import {createWorkspace,decodeWorkspace,type WorkspaceState} from './model';

export interface ConversationRecord {schemaVersion:1;revision:number;state:WorkspaceState}
export type ConversationAvailability='loading'|'ready'|'unavailable';
interface IO {
  projectId:string;snapshotId:string;
  load:()=>Promise<ConversationRecord|null>;
  save:(expectedRevision:number|null,state:WorkspaceState)=>Promise<ConversationRecord>;
  legacy:()=>string|null;
  restore:(state:WorkspaceState)=>void;
  availability:()=>void;
  failed:()=>void;
  saved:()=>void;
}
const unavailable=()=>new Error('Your conversation could not be saved. Existing explanations have been preserved.');

/** Explicit wire fields exclude transient focus requests and native job copies. */
export function conversationSnapshot(state:WorkspaceState):WorkspaceState {
  const {version,projectId,snapshotId,currentId,file,grantId}=state;
  const context=({path,sha256,from,to}:WorkspaceState['explorations'][number]['turns'][number]['context'][number])=>({path,sha256,from,to});
  return JSON.parse(JSON.stringify({version,projectId,snapshotId,currentId,file,grantId,
    explorations:state.explorations.map(e=>({id:e.id,parentId:e.parentId,originTurnId:e.originTurnId,title:e.title,view:e.view,
      file:e.file,reveal:e.reveal?context(e.reveal):undefined,focusedTurnId:e.focusedTurnId,scrollPositions:e.scrollPositions,
      turns:e.turns.map(t=>({id:t.id,question:t.question,context:t.context.map(context),status:t.status,revisionId:t.revisionId,
        generationId:t.generationId,error:t.error,runIds:t.runIds,publishedSections:t.publishedSections,notes:t.notes,
        guideId:t.guideId,reading:t.reading,outcomeAt:t.outcomeAt,outcomeSequence:t.outcomeSequence,outcomeNotice:t.outcomeNotice}))}))}));
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
  private blocked=false;
  constructor(private readonly io:IO){}
  private record(value:ConversationRecord):WorkspaceState {
    if(value?.schemaVersion!==1||!Number.isSafeInteger(value.revision)||value.revision<1)throw unavailable();
    const state=decodeWorkspace(JSON.stringify(value.state),this.io.projectId,this.io.snapshotId);
    if(!state)throw unavailable();return state;
  }
  start():Promise<void> {
    if(this.availability==='ready')return Promise.resolve();
    if(this.starting)return this.starting;
    this.availability='loading';this.io.availability();
    this.starting=(async()=>{
      let record=await this.io.load();
      if(!record){
        const raw=this.io.legacy();
        const state=raw===null?createWorkspace(this.io.projectId,this.io.snapshotId):decodeWorkspace(raw,this.io.projectId,this.io.snapshotId);
        if(!state)throw unavailable(); // Preserve the original preference for recovery.
        record=await this.io.save(null,conversationSnapshot(state));
      }
      const state=this.record(record);this.revision=record.revision;this.availability='ready';
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
    if(this.blocked)return Promise.reject(unavailable());
    this.active=(async()=>{
      while(this.attempt||this.pending){
        if(!this.attempt){this.attempt={revision:this.revision,state:this.pending!};this.pending=null;}
        const record=await this.io.save(this.attempt.revision,this.attempt.state);this.record(record);
        if(record.revision<(this.revision??0))throw unavailable();
        this.revision=record.revision;this.attempt=null;
      }
      this.io.saved();
    })().catch(()=>{this.blocked=true;this.io.failed();throw unavailable();}).finally(()=>{
      this.active=null;
      if(this.pending&&!this.blocked)void this.drain().catch(()=>{});
    });
    return this.active;
  }
  async flush():Promise<void> {
    if(this.availability!=='ready'||this.blocked)throw unavailable();
    while(this.active||this.pending||this.attempt)await this.drain();
  }
  async retry():Promise<void> {
    await this.start();this.blocked=false;await this.flush();
  }
}
