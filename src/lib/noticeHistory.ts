/** Saved UI outcomes contain navigation targets, never callbacks or consent. */
export type NoticeTarget = {kind:'openRepository'} | {kind:'retryRecents'} | {kind:'retryConversation'} | {kind:'discardBrowserPreview'} | {kind:'openExploration';explorationId:string};
export interface AppNoticeInput {
  title:string; message:string; tone?:'info'|'warning'|'error';
  projectId?:string|null; snapshotId?:string|null; target?:NoticeTarget|null; occurrenceId?:string|null;
}
export interface SavedNoticeInput {
  title:string; message:string; tone:'info'|'warning'|'error';
  projectId:string|null; snapshotId:string|null; target:NoticeTarget|null; occurrenceId:string|null;
}
export interface AppNotice {id:string;input:SavedNoticeInput;createdAtUnixMs:number;read:boolean}
export interface NoticeHistoryData {schemaVersion:1;items:AppNotice[];dismissed:string[]}
export type NoticeOperation = {kind:'publish';input:SavedNoticeInput} | {kind:'read'|'dismiss';id:string} | {kind:'readMany'|'dismissMany';ids:string[]};
type Pending = {kind:'load'} | {operation:NoticeOperation;item?:AppNotice};
const MAX_BATCH_DISMISSALS=2048;
const isDismissOperation=(operation:NoticeOperation) => operation.kind==='dismiss'||operation.kind==='dismissMany';
export type NoticeHistoryIssue = 'full'|'unavailable';
export interface NoticeHistorySnapshot {items:AppNotice[];availability:'loading'|'ready'|'unavailable';issue:NoticeHistoryIssue|null}
export interface NoticeHistoryApi {load():Promise<NoticeHistoryData>;update(operation:NoticeOperation):Promise<NoticeHistoryData>}
export const emptyNoticeHistory = ():NoticeHistoryData => ({schemaVersion:1,items:[],dismissed:[]});

export function normalizeNotice(input:AppNoticeInput):SavedNoticeInput {
  const text=(value:string,max:number) => Array.from(value.replace(/\s+/g,' ').trim()).slice(0,max).join('');
  const normalized:SavedNoticeInput={title:text(input.title,100),message:text(input.message,700),tone:input.tone??'info',
    projectId:input.projectId??null,snapshotId:input.snapshotId??null,target:input.target??null,occurrenceId:input.occurrenceId??null};
  const validId=(id:string|null) => id===null || /^[A-Za-z0-9_.:-]{1,256}$/.test(id);
  if(!normalized.title || !normalized.message || /[\u0000-\u001f\u007f-\u009f]/u.test(normalized.title+normalized.message)
    || !['info','warning','error'].includes(normalized.tone)
    || (normalized.projectId!==null && (!normalized.projectId.trim() || new TextEncoder().encode(normalized.projectId).length>4096 || /[\u0000-\u001f\u007f-\u009f]/u.test(normalized.projectId))) || !validId(normalized.snapshotId)
    || !validId(normalized.occurrenceId) || (normalized.snapshotId!==null && normalized.projectId===null)) throw new Error('Invalid update context');
  const target=normalized.target;
  if(target) {
    if(target.kind==='openExploration') {
      if(!validId(target.explorationId) || !target.explorationId || !normalized.projectId || !normalized.snapshotId) throw new Error('Invalid exploration context');
      normalized.target={kind:'openExploration',explorationId:target.explorationId};
    } else if(target.kind==='retryConversation' || target.kind==='discardBrowserPreview') {
      if(!normalized.projectId||!normalized.snapshotId)throw new Error('Invalid conversation context');
      normalized.target={kind:target.kind};
    } else if(target.kind==='openRepository' || target.kind==='retryRecents') normalized.target={kind:target.kind};
    else throw new Error('Invalid update action');
  }
  return normalized;
}

/** Field order and nulls match native Input serialization; native checks again. */
export async function noticeKey(input:SavedNoticeInput):Promise<string> {
  const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(input)));
  return Array.from(new Uint8Array(digest),byte=>byte.toString(16).padStart(2,'0')).join('');
}

function historyIssue(error:unknown):NoticeHistoryIssue {
  const message=typeof error==='string'?error:error instanceof Error?error.message:'';
  return message.startsWith('Notification history is full.')?'full':'unavailable';
}

export function projectNotices(base:NoticeHistoryData,pending:readonly Pending[]):NoticeHistoryData {
  let items=[...base.items];const dismissed=new Set(base.dismissed);
  for(const entry of pending) {
    if('kind' in entry) continue;
    const op=entry.operation;
    if(op.kind==='publish') {
      if(entry.item && !dismissed.has(entry.item.id) && !items.some(item=>item.id===entry.item!.id)) items.push(entry.item);
    } else if(op.kind==='read') items=items.map(item=>item.id===op.id?{...item,read:true}:item);
    else if(op.kind==='readMany') {const ids=new Set(op.ids);items=items.map(item=>ids.has(item.id)?{...item,read:true}:item);}
    else if(op.kind==='dismiss') {items=items.filter(item=>item.id!==op.id);dismissed.add(op.id);}
    else if(op.kind==='dismissMany') {const ids=new Set(op.ids);items=items.filter(item=>!ids.has(item.id));for(const id of ids)dismissed.add(id);}
  }
  return {schemaVersion:1,items,dismissed:[...dismissed]};
}

/** All native reads/writes are serial. Failed operations stay queued for an
 * explicit local retry; optimistic notices remain visible without claiming
 * their acknowledgments have been durably saved. */
export class NoticeHistory {
  private base=emptyNoticeHistory();
  private pending:Pending[]=[];
  private preparing=new Set<{input:SavedNoticeInput;cancelled:boolean}>();
  private publications=new Set<Promise<void>>();
  private active:Promise<void>|null=null;
  private closing=false;
  private deferred:(()=>void)[]=[];
  private listeners=new Set<()=>void>();
  private started=false;
  private running=false;
  private blocked:NoticeHistoryIssue|null=null;
  private capacityRecovery=false;
  private snapshot:NoticeHistorySnapshot={items:[],availability:'loading',issue:null};
  private api:NoticeHistoryApi;
  constructor(api:NoticeHistoryApi) {this.api=api;}
  subscribe=(callback:()=>void) => {this.listeners.add(callback);return()=>{this.listeners.delete(callback);};};
  getSnapshot=() => this.snapshot;
  private emit() {
    this.snapshot={items:projectNotices(this.base,this.pending).items.slice().reverse(),availability:this.blocked?'unavailable':this.running||this.pending.length?'loading':'ready',issue:this.blocked};
    for(const callback of this.listeners) callback();
  }
  start() {if(this.started)return;this.started=true;this.pending.unshift({kind:'load'});this.run();}
  publish(input:AppNoticeInput):Promise<void> {
    if(this.closing){this.deferred.push(()=>{void this.publish(input);});return Promise.resolve();}
    const task=this.publishNow(input);this.publications.add(task);
    void task.finally(()=>{this.publications.delete(task);});
    return task;
  }
  private async publishNow(input:AppNoticeInput):Promise<void> {
    let preparing:{input:SavedNoticeInput;cancelled:boolean}|undefined;
    try {
      const normalized=normalizeNotice(input);preparing={input:normalized,cancelled:false};this.preparing.add(preparing);
      const id=await noticeKey(normalized);
      if(preparing.cancelled)return;
      this.start();
      const current=projectNotices(this.base,this.pending);
      if(current.dismissed.includes(id)||current.items.some(item=>item.id===id))return;
      this.pending.push({operation:{kind:'publish',input:normalized},item:{id,input:normalized,createdAtUnixMs:Date.now(),read:false}});
      this.emit();this.run();
    } catch {this.blocked='unavailable';this.emit();}
    finally {if(preparing)this.preparing.delete(preparing);}
  }
  resolveOccurrence(projectId:string,snapshotId:string,occurrenceId:string) {
    const matches=(input:SavedNoticeInput)=>input.projectId===projectId&&input.snapshotId===snapshotId&&input.occurrenceId===occurrenceId;
    for(const entry of this.preparing)if(matches(entry.input))entry.cancelled=true;
    // A save completing during close can resolve its old failure notice. This
    // acknowledgment belongs to the already admitted write, not a new UI edit.
    for(const item of projectNotices(this.base,this.pending).items)if(matches(item.input))this.enqueueNow({kind:'dismiss',id:item.id});
  }
  read(id:string) {this.enqueue({kind:'read',id});}
  readMany(ids:readonly string[]) {
    const requested=[...new Set(ids)];
    if(this.closing){this.deferred.push(()=>this.readMany(requested));return;}
    if(!requested.length||requested.length>MAX_BATCH_DISMISSALS)return;
    this.enqueueNow({kind:'readMany',ids:requested});
  }
  dismiss(id:string) {
    if(this.closing){this.deferred.push(()=>this.dismiss(id));return;}
    if(this.blocked==='full') {
      const index=this.pending.findIndex((entry)=>'operation' in entry&&entry.operation.kind==='publish'&&entry.item?.id===id);
      if(index>=0) {
        this.pending.splice(index,1);
        if(!this.pending.some((entry)=>'operation' in entry&&entry.operation.kind==='publish')){this.blocked=null;this.capacityRecovery=false;}
        this.emit();this.run();return;
      }
    }
    this.enqueueNow({kind:'dismiss',id});
  }
  dismissMany(ids:readonly string[]) {
    const requested=[...new Set(ids)];
    if(this.closing){this.deferred.push(()=>this.dismissMany(requested));return;}
    if(!requested.length||requested.length>MAX_BATCH_DISMISSALS)return;
    let persistedIds=requested;
    if(this.blocked==='full') {
      const saved=new Set(this.base.items.map(item=>item.id));
      const selected=new Set(requested);
      this.pending=this.pending.filter(entry=>{
        if('operation' in entry&&entry.operation.kind==='publish'&&entry.item&&selected.has(entry.item.id)&&!saved.has(entry.item.id))return false;
        return true;
      });
      persistedIds=requested.filter(id=>saved.has(id));
      if(!persistedIds.length) {
        if(!this.pending.some(entry=>'operation' in entry&&entry.operation.kind==='publish')){this.blocked=null;this.capacityRecovery=false;}
        this.emit();this.run();return;
      }
    }
    this.enqueueNow({kind:'dismissMany',ids:persistedIds});
  }
  retry() {if(this.closing||this.blocked==='full')return;this.blocked=null;if(!this.pending.length)this.pending.push({kind:'load'});this.run();}
  beginClose():()=>void {
    if(this.closing)throw new Error('Closing is already in progress.');
    this.closing=true;let resumed=false;
    return()=>{
      if(resumed)return;resumed=true;this.closing=false;
      const deferred=this.deferred;this.deferred=[];
      for(const update of deferred)update();
    };
  }
  async flush():Promise<void> {
    for(;;){
      if(this.blocked){
        if(this.pending.some(entry=>'operation' in entry)||this.publications.size)throw new Error('Your updates could not be saved.');
        return; // An initial read failure has not changed any saved updates.
      }
      await Promise.all([...this.publications]);
      if(this.blocked)throw new Error('Your updates could not be saved.');
      if(this.active)await this.active;
      else if(this.pending.length)this.run();
      else if(!this.publications.size)return;
    }
  }
  private enqueue(operation:NoticeOperation) {
    if(this.closing){this.deferred.push(()=>this.enqueue(operation));return;}
    this.enqueueNow(operation);
  }
  private enqueueNow(operation:NoticeOperation) {
    this.start();
    const entry:Pending={operation};
    if(isDismissOperation(operation)&&(this.blocked==='full'||this.capacityRecovery)) {
      const firstQueued=this.running?1:0;
      let index=this.pending.findIndex((item,position)=>position>=firstQueued&&'operation' in item&&item.operation.kind==='publish');
      if(index<0)index=this.pending.length;
      this.pending.splice(index,0,entry);
      this.capacityRecovery=true;
      this.blocked=null;
    } else this.pending.push(entry);
    this.emit();this.run();
  }
  private run() {
    if(this.running||this.blocked)return;
    this.running=true;this.emit();
    const finishing=this.drain().finally(()=>{if(this.active===finishing)this.active=null;});
    this.active=finishing;
  }
  private async drain() {
    try {
      while(this.pending.length) {
        const next=this.pending[0];
        const value=await ('kind' in next?this.api.load():this.api.update(next.operation));
        this.base=value;this.pending.shift();this.emit();
      }
      this.capacityRecovery=false;
    } catch (failure) {
      this.blocked=historyIssue(failure);
      this.capacityRecovery=this.blocked==='full';
      if(this.capacityRecovery&&this.pending.length>1) {
        const [head,...tail]=this.pending;
        this.pending=[head!,...tail.filter((entry)=>'operation' in entry&&isDismissOperation(entry.operation)),...tail.filter((entry)=>!('operation' in entry&&isDismissOperation(entry.operation)))];
      }
    } finally {this.running=false;this.emit();}
  }
}
