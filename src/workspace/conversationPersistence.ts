import {createWorkspace,decodeWorkspace,type WorkspaceState} from './model';
import {cleanReaderProgress} from '../lib/readerProgress';
import {cleanAnswerVersions} from './answerVersions';

export interface ConversationRecord {schemaVersion:1;revision:number;state:WorkspaceState}
export type ConversationAvailability='loading'|'ready'|'unavailable';
interface IO {
  projectId:string;snapshotId:string;
  load:()=>Promise<ConversationRecord|null>;
  save:(expectedRevision:number|null,state:WorkspaceState)=>Promise<ConversationRecord>;
  restore:(state:WorkspaceState)=>void;
  availability:()=>void;
  failed:(message:string)=>void;
  saved:()=>void;
}
const unavailable=()=>new Error('Your conversation could not be saved. Existing explanations have been preserved.');
const genericSaveFailure='Some conversation changes could not be saved. Existing explanations are preserved.';
const overlapSaveFailure='This conversation changed in another window. Your edits are still here; retry to merge separate changes. If the same item changed in both windows, choose which version to keep, then retry.';
const nativeRevisionConflict='Another window saved this conversation. Your unsaved changes have been preserved in this window.';
const isConversationRevisionConflict=(failure:unknown):boolean=>{
  const message=typeof failure==='string'?failure:failure instanceof Error?failure.message:
    failure&&typeof failure==='object'&&'message' in failure&&typeof failure.message==='string'?failure.message:'';
  return message===nativeRevisionConflict||message===overlapSaveFailure;
};
const safeSaveFailure=(failure:unknown):string=>{
  const message=typeof failure==='string'?failure:failure instanceof Error?failure.message:
    failure&&typeof failure==='object'&&'message' in failure&&typeof failure.message==='string'?failure.message:'';
  if(message==='A conversation save is still in progress. Wait a moment, then retry.')return message;
  if(isConversationRevisionConflict(message))return overlapSaveFailure;
  if(message==='Local conversation storage could not be written. Check available disk space and folder access; your changes remain open in this window.')return message;
  if(message==='The Mac reported insufficient local storage space. Check available disk space, then retry saving; your changes remain open in this window.')return message;
  if(message==='A local storage quota was reached. Check the app or filesystem storage limit, then retry saving; your changes remain open in this window.')return message;
  if(message==='Code Atlas cannot write conversation data. Check local storage permissions, then retry; your changes remain open in this window.')return message;
  if(message==='Code Atlas rejected this conversation update. Your saved work is intact and the unsaved changes remain open.')return message;
  if(message==='This conversation exceeds the local save limit. Your saved work is intact and the unsaved changes remain open.')return message;
  return genericSaveFailure;
};

const MISSING=Symbol('missing');
type Missing=typeof MISSING;
type MergeValue=unknown|Missing;
type MergePath=readonly string[];
const isObject=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const equal=(left:MergeValue,right:MergeValue):boolean=>{
  if(left===right)return true;
  if(left===MISSING||right===MISSING||typeof left!=='object'||left===null||typeof right!=='object'||right===null)return false;
  if(Array.isArray(left)||Array.isArray(right))return Array.isArray(left)&&Array.isArray(right)&&left.length===right.length&&left.every((item,index)=>equal(item,right[index]));
  if(!isObject(left)||!isObject(right))return false;
  const leftKeys=Object.keys(left).sort(),rightKeys=Object.keys(right).sort();
  return leftKeys.length===rightKeys.length&&leftKeys.every((key,index)=>key===rightKeys[index]&&equal(left[key],right[key]));
};
const copy=(value:MergeValue):MergeValue=>value===MISSING?MISSING:Array.isArray(value)?value.map(item=>copy(item as MergeValue))
  :isObject(value)?Object.fromEntries(Object.entries(value).map(([key,item])=>[key,copy(item as MergeValue)])):value;
function arrayIdentity(field:string,value:unknown):string|null {
  if(field==='runIds'||field==='notes')return typeof value==='string'?value:null;
  if(!isObject(value))return null;
  if(field==='explorations'||field==='turns')return typeof value.id==='string'?value.id:null;
  if(field==='publishedSections')return typeof value.runId==='string'&&typeof value.generationId==='string'?`${value.runId}\u0000${value.generationId}`:null;
  if(field==='answerVersions')return typeof value.originalRevisionId==='string'&&typeof value.revisionId==='string'?`${value.originalRevisionId}\u0000${value.revisionId}`:null;
  if(field==='stories'||field==='diagrams')return typeof value.revisionId==='string'&&typeof value.sectionId==='string'&&typeof value.sectionRevisionId==='string'
    ?`${value.revisionId}\u0000${value.sectionId}\u0000${value.sectionRevisionId}`:null;
  return null;
}
const mergeableArrays=new Set(['explorations','turns','runIds','notes','publishedSections','answerVersions','stories','diagrams']);
// There is one saved reader location per snapshot. A retry from the visible
// window keeps that window's navigation while still merging other turns.
const currentWindowFields=new Set(['currentId','file','view','focusedTurnId','reveal','codeOpen','fileFlow','scrollPositions',
  'reading','presentation','answerVersions','selected','guide','flow','history','positions','sceneId','fullMap','viewport','focusedNodeId','focusedEdgeId']);

/** Merge two native windows' snapshots against their last shared saved state.
 * Stable record collections keep independent additions; a same-field edit is
 * reported as a conflict instead of silently overwriting either window. */
export function mergeConversationStates(base:WorkspaceState,local:WorkspaceState,remote:WorkspaceState):{state:WorkspaceState|null;conflictCount:number} {
  if(base.projectId!==local.projectId||base.projectId!==remote.projectId||base.snapshotId!==local.snapshotId||base.snapshotId!==remote.snapshotId)
    return {state:null,conflictCount:1};
  const conflicts:string[]=[];
  const merge=(before:MergeValue,ours:MergeValue,theirs:MergeValue,path:MergePath):MergeValue=>{
    if(equal(ours,theirs))return copy(ours);
    if(equal(ours,before))return copy(theirs);
    if(equal(theirs,before))return copy(ours);
    // An unsent composer belongs to the currently edited window. Prefer its
    // latest draft if two windows type into the same exploration concurrently.
    if(path.at(-1)==='draft')return copy(ours);
    if(isObject(ours)&&isObject(theirs)&&(before===MISSING||isObject(before))){
      const oldObject=before===MISSING?{}:before;
      const result:Record<string,unknown>={};
      for(const key of new Set([...Object.keys(oldObject),...Object.keys(ours),...Object.keys(theirs)])){
        const value=merge(Object.hasOwn(oldObject,key)?oldObject[key]:MISSING,Object.hasOwn(ours,key)?ours[key]:MISSING,
          Object.hasOwn(theirs,key)?theirs[key]:MISSING,[...path,key]);
        if(value!==MISSING)result[key]=value;
      }
      return result;
    }
    if(Array.isArray(ours)&&Array.isArray(theirs)&&(before===MISSING||Array.isArray(before))&&mergeableArrays.has(path.at(-1)??'')){
      const oldArray=before===MISSING?[]:before;
      const index=(values:unknown[])=>{
        const map=new Map<string,unknown>();
        for(const item of values){const key=arrayIdentity(path.at(-1)??'',item);if(key===null||map.has(key))return null;map.set(key,item);}
        return map;
      };
      const oldMap=index(oldArray),ourMap=index(ours),theirMap=index(theirs);
      if(!oldMap||!ourMap||!theirMap){conflicts.push(path.join('.'));return copy(theirs);}
      const keys=[...theirMap.keys(),...[...ourMap.keys()].filter(key=>!theirMap.has(key))];
      const result:unknown[]=[];
      for(const key of keys){
        const item=merge(oldMap.has(key)?oldMap.get(key)!:MISSING,ourMap.has(key)?ourMap.get(key)!:MISSING,
          theirMap.has(key)?theirMap.get(key)!:MISSING,[...path,key]);
        if(item!==MISSING)result.push(item);
      }
      return result;
    }
    if(currentWindowFields.has(path.at(-1)??''))return copy(ours);
    conflicts.push(path.join('.'));
    return copy(theirs);
  };
  const value=merge(base,local,remote,[]);
  if(value===MISSING||!isObject(value)||conflicts.length)return {state:null,conflictCount:conflicts.length||1};
  return {state:value as unknown as WorkspaceState,conflictCount:0};
}

/** Explicit wire fields exclude transient focus requests and native job copies. */
export function conversationSnapshot(state:WorkspaceState):WorkspaceState {
  // A live Guide carries native metadata beside its turn in memory. The
  // conversation contract stores its ID and section references, not that
  // duplicate runtime object; remove it before strict record validation.
  const durableState={...state,explorations:state.explorations.map(exploration=>({...exploration,
    turns:exploration.turns.map(({guide:_guide,...turn})=>turn),
  }))};
  const current=decodeWorkspace(JSON.stringify(durableState),state.projectId,state.snapshotId);
  if(!current)throw unavailable();
  const {version,projectId,snapshotId,currentId,file,grantId}=current;
  const context=({path,sha256,from,to}:WorkspaceState['explorations'][number]['turns'][number]['context'][number])=>({path,sha256,from,to});
  return JSON.parse(JSON.stringify({version,projectId,snapshotId,currentId,file,grantId,
    explorations:current.explorations.map(e=>({id:e.id,parentId:e.parentId,originTurnId:e.originTurnId,title:e.title,view:e.view,
      file:e.file,reveal:e.reveal?context(e.reveal):undefined,codeOpen:e.codeOpen,fileFlow:e.fileFlow?{path:e.fileFlow.path,showAll:e.fileFlow.showAll}:undefined,focusedTurnId:e.focusedTurnId,scrollPositions:e.scrollPositions,
      turns:e.turns.map(t=>({id:t.id,question:t.question,context:t.context.map(context),status:t.status,revisionId:t.revisionId,
        generationId:t.generationId,error:t.error,runIds:t.runIds,publishedSections:t.publishedSections,notes:t.notes,
        guideId:t.guideId,reading:t.reading,presentation:t.presentation?cleanReaderProgress(t.presentation):undefined,answerVersions:t.answerVersions?cleanAnswerVersions(t.answerVersions,t):undefined,outcomeAt:t.outcomeAt,outcomeSequence:t.outcomeSequence,outcomeNotice:t.outcomeNotice}))}))}));
}

/** One serial writer per scope. A failed attempt and subsequent edits are kept
 * separately so a lost acknowledgment can be replayed before newer edits. */
export class ConversationPersistence {
  availability:ConversationAvailability='loading';
  private revision:number|null=null;
  private starting:Promise<void>|null=null;
  private active:Promise<void>|null=null;
  private pending:WorkspaceState|null=null;
  private attempt:{revision:number|null;state:WorkspaceState}|null=null;
  private persisted:WorkspaceState|null=null;
  private blocked=false;
  private failure=genericSaveFailure;
  private retrying:Promise<void>|null=null;
  constructor(private readonly io:IO){}
  private record(value:ConversationRecord):WorkspaceState {
    if(value?.schemaVersion!==1||!Number.isSafeInteger(value.revision)||value.revision<1)throw unavailable();
    const state=decodeWorkspace(JSON.stringify(value.state),this.io.projectId,this.io.snapshotId);
    if(!state)throw unavailable();return state;
  }
  private async reconcileRevisionConflict():Promise<void> {
    const latest=await this.io.load();
    if(!latest)throw unavailable();
    const remote=this.record(latest),failedAttempt=this.attempt;
    if(!failedAttempt)throw new Error(overlapSaveFailure);
    const local=this.pending??failedAttempt.state;
    // A save can reach disk and then lose its acknowledgment. Treat the exact
    // attempted snapshot as committed, then replay only newer local edits.
    if(equal(remote,failedAttempt.state)){
      this.revision=latest.revision;this.persisted=remote;
      if(this.pending){this.attempt={revision:latest.revision,state:this.pending};this.pending=null;}
      else this.attempt=null;
      return;
    }
    if(!this.persisted)throw new Error(overlapSaveFailure);
    const merged=mergeConversationStates(this.persisted,local,remote);
    if(!merged.state)throw new Error(overlapSaveFailure);
    const safeMerged=this.record({schemaVersion:1,revision:latest.revision,state:merged.state});
    this.revision=latest.revision;this.persisted=remote;this.attempt={revision:latest.revision,state:safeMerged};this.pending=null;
    this.io.restore(safeMerged);
  }
  start():Promise<void> {
    if(this.availability==='ready')return Promise.resolve();
    if(this.starting)return this.starting;
    this.availability='loading';this.io.availability();
    this.starting=(async()=>{
      let record=await this.io.load();
      if(!record)record=await this.io.save(null,conversationSnapshot(createWorkspace(this.io.projectId,this.io.snapshotId)));
      const state=this.record(record);this.revision=record.revision;this.persisted=state;this.availability='ready';
      this.io.restore(state);this.io.availability();
    })().catch(()=>{this.availability='unavailable';this.io.availability();throw unavailable();}).finally(()=>{this.starting=null;});
    return this.starting;
  }
  enqueue(state:WorkspaceState):void {
    if(this.availability!=='ready')throw unavailable();
    this.pending=conversationSnapshot(state);
    if(!this.blocked)void this.drain().catch(()=>{});
  }
  private drain():Promise<void> {
    if(this.active)return this.active;
    if(this.blocked)return Promise.reject(new Error(this.failure));
    this.active=(async()=>{
      let conflictRecoveries=0;
      while(this.attempt||this.pending){
        if(!this.attempt){this.attempt={revision:this.revision,state:this.pending!};this.pending=null;}
        let record:ConversationRecord;
        try{record=await this.io.save(this.attempt.revision,this.attempt.state);}
        catch(failure){
          if(!isConversationRevisionConflict(failure)||conflictRecoveries>=3)throw failure;
          await this.reconcileRevisionConflict();conflictRecoveries++;continue;
        }
        const saved=this.record(record);
        if(record.revision<(this.revision??0))throw unavailable();
        this.revision=record.revision;this.persisted=saved;this.attempt=null;
      }
      this.io.saved();
    })().catch(failure=>{this.blocked=true;this.failure=safeSaveFailure(failure);this.io.failed(this.failure);throw new Error(this.failure);}).finally(()=>{
      this.active=null;
      if(this.pending&&!this.blocked)void this.drain().catch(()=>{});
    });
    return this.active;
  }
  async flush():Promise<void> {
    if(this.availability!=='ready')throw unavailable();
    if(this.blocked)throw new Error(this.failure);
    while(this.active||this.pending||this.attempt)await this.drain();
  }
  /** A new explicit generation action may safely reconcile one earlier window conflict. */
  async flushForGeneration():Promise<void> {
    try{await this.flush();}
    catch(failure){
      if(!isConversationRevisionConflict(failure))throw failure;
      await this.retry();
      await this.flush();
    }
  }
  async flushBeforeClose():Promise<void> {
    // A failed initial read with no edits has nothing to flush and must not trap startup.
    if(this.starting)try{await this.starting;}catch{/* Original bytes are preserved by start. */}
    if(this.active||this.pending||this.attempt)await this.flush();
  }
  retry():Promise<void> {
    if(this.retrying)return this.retrying;
    const pending=this.retryOnce().finally(()=>{this.retrying=null;});this.retrying=pending;return pending;
  }
  private async retryOnce():Promise<void> {
    await this.start();
    if(this.active)try{await this.active;}catch{/* Inspect the latest durable record after a failed save. */}
    if(!this.blocked){await this.flush();return;}
    let latest:ConversationRecord|null;
    try{latest=await this.io.load();}catch(failure){this.failure=safeSaveFailure(failure);this.io.failed(this.failure);throw new Error(this.failure);}
    if(!latest){this.io.failed(this.failure);throw new Error(this.failure);}
    let remote:WorkspaceState;
    try{remote=this.record(latest);}catch(failure){this.failure=safeSaveFailure(failure);this.io.failed(this.failure);throw new Error(this.failure);}
    const failedAttempt=this.attempt;
    const local=this.pending??failedAttempt?.state;
    if(!local){
      this.revision=latest.revision;this.persisted=remote;this.blocked=false;this.failure=genericSaveFailure;this.io.restore(remote);this.io.saved();return;
    }
    // A save can reach disk and then lose its acknowledgment. Treat the exact
    // attempted snapshot as committed, then save any newer local edits once.
    if(failedAttempt&&equal(remote,failedAttempt.state)){
      this.revision=latest.revision;this.persisted=remote;this.attempt=null;
      this.io.restore(this.pending??remote);this.blocked=false;this.failure=genericSaveFailure;
      if(!this.pending){this.io.saved();return;}
      await this.flush();return;
    }
    if(!this.persisted){this.io.failed(this.failure);throw new Error(this.failure);}
    const merged=mergeConversationStates(this.persisted,local,remote);
    if(!merged.state){
      this.failure=overlapSaveFailure;this.blocked=true;this.io.failed(this.failure);throw new Error(this.failure);
    }
    let safeMerged:WorkspaceState;
    try{safeMerged=this.record({schemaVersion:1,revision:latest.revision,state:merged.state});}
    catch{this.failure=overlapSaveFailure;this.blocked=true;this.io.failed(this.failure);throw new Error(this.failure);}
    this.revision=latest.revision;this.persisted=remote;this.attempt={revision:latest.revision,state:safeMerged};this.pending=null;
    this.io.restore(safeMerged);this.blocked=false;this.failure=genericSaveFailure;
    await this.flush();
  }
}
