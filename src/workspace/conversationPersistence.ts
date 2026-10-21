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
const safeSaveFailure=(failure:unknown):string=>{
  const message=typeof failure==='string'?failure:failure instanceof Error?failure.message:
    failure&&typeof failure==='object'&&'message' in failure&&typeof failure.message==='string'?failure.message:'';
  if(message==='A conversation save is still in progress. Wait a moment, then retry.')return message;
  if(message==='Another window saved this conversation. Your unsaved changes have been preserved in this window.')return message;
  if(message==='Local conversation storage could not be written. Check available disk space and folder access; your changes remain open in this window.')return message;
  if(message==='Code Atlas rejected this conversation update. Your saved work is intact and the unsaved changes remain open.')return message;
  if(message==='This conversation exceeds the local save limit. Your saved work is intact and the unsaved changes remain open.')return message;
  return genericSaveFailure;
};

/** Explicit wire fields exclude transient focus requests and native job copies. */
export function conversationSnapshot(state:WorkspaceState):WorkspaceState {
  const current=decodeWorkspace(JSON.stringify(state),state.projectId,state.snapshotId);
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
  private blocked=false;
  private failure=genericSaveFailure;
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
      if(!record)record=await this.io.save(null,conversationSnapshot(createWorkspace(this.io.projectId,this.io.snapshotId)));
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
    if(this.blocked)return Promise.reject(new Error(this.failure));
    this.active=(async()=>{
      while(this.attempt||this.pending){
        if(!this.attempt){this.attempt={revision:this.revision,state:this.pending!};this.pending=null;}
        const record=await this.io.save(this.attempt.revision,this.attempt.state);this.record(record);
        if(record.revision<(this.revision??0))throw unavailable();
        this.revision=record.revision;this.attempt=null;
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
  async flushBeforeClose():Promise<void> {
    // A failed initial read with no edits has nothing to flush and must not trap startup.
    if(this.starting)try{await this.starting;}catch{/* Original bytes are preserved by start. */}
    if(this.active||this.pending||this.attempt)await this.flush();
  }
  async retry():Promise<void> {
    await this.start();this.blocked=false;this.failure=genericSaveFailure;await this.flush();
  }
}
