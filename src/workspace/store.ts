import { createQuestionQueue } from './queue';
import { createRefreshQueue, type RefreshQueue } from './refreshQueue';
import { useRef, useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { finishQueuedWorkspaceGenerationJob, getLocalNarrationPreparationStatus, getSnapshotIndexProgress, isNativeRuntime, listChapterRevisions, listWorkspaceGenerationJobsForScope, loadSnapshotIndex, prewarmLocalNarration, queueWorkspaceGenerationJob, startSnapshotIndexing, stopWorkspaceGenerationJob, type ChapterRevision, type SnapshotIndex, type WorkspaceGenerationJob } from '../lib/commands';
import {ConversationPersistence,type ConversationConflictChoice,type ConversationConflictSummary,type ConversationRecord} from './conversationPersistence';
import { ensureCodexConnection, readCodexSelection } from '../lib/connectionStore';
import { errorMessageOr } from '../lib/errorMessage';
import { readVoiceConnection, readVoiceConnectionIssue } from '../lib/voiceConnectionStore';
import type { NoticeTarget } from '../lib/noticeHistory';
import { localNarrationPrewarmInputs } from './narrationPrewarm';
import { createNarrationPrewarmQueue } from './narrationQueue';
import { createQuestionOutcomeSelector, recordQuestionOutcomes, type QuestionOutcome } from './outcomes';
import { addTurn, cleanAnswerNotes, cleanExplanationFailureCode, createWorkspace, decodeWorkspace, learningQuestion, mergePublishedSections, mergeRepositoryGuide, rememberComposerDraft, serializeWorkspace, type ComposerDraft, type RepositoryGuide, type RepositoryGuideList, type AnswerSummary, type PublishedSection, type Context, type Turn, type View, type WorkspaceState } from './model';

const indexLoads = new Map<string, Promise<SnapshotIndex>>();
/** One local index build per snapshot, shared by reading and questions. No model call. */
export function ensureWorkspaceIndex(project: string, snapshot: string): Promise<SnapshotIndex> {
  const id = key(project,snapshot);
  const existing = indexLoads.get(id);
  if (existing) return existing;
  const pending = (async () => {
    const saved = await loadSnapshotIndex(project,snapshot);
    if (saved) return saved;
    const started = await startSnapshotIndexing(project,snapshot);
    let pollDelayMs = 200;
    for (;;) {
      const progress = await getSnapshotIndexProgress(started.jobId);
      if (!progress) throw new Error('The local code index could not be read. Try again.');
      if (progress.status !== 'running') {
        const result = await loadSnapshotIndex(project,snapshot);
        if (!result) throw new Error(progress.message || 'The local code index could not be built. Try again.');
        return result;
      }
      // Indexing is an owned bounded local job, not an inference request with a
      // wall-clock budget. Keep waiting for its terminal state without polling
      // the native job store at a fixed five calls per second indefinitely.
      await new Promise((resolve) => setTimeout(resolve,pollDelayMs));
      pollDelayMs = Math.min(1000, Math.ceil(pollDelayMs * 1.5));
    }
  })().finally(() => indexLoads.delete(id));
  indexLoads.set(id,pending);
  return pending;
}

const states = new Map<string, WorkspaceState>();
const browserPreviewWriteBlocked = new Set<string>();
const unsavedWorkspaceEpisodes = new Map<string,string>();
const listeners = new Set<() => void>();
const jobs = new Map<string, { cancelled: boolean; runId: string | null; pendingGuideAdvances: string[] }>();
const workspaceDraftTimers = new Map<string,{project:string;snapshot:string;timer:ReturnType<typeof setTimeout>}>();
const narrationPrewarmer = createNarrationPrewarmQueue({prepare:prewarmLocalNarration,status:getLocalNarrationPreparationStatus});
// A new identity on WebView reload lets native recovery distinguish live
// renderer-owned queue entries from entries whose owner disappeared.
const rendererSessionId = crypto.randomUUID();
const key = (project:string,snapshot:string) => JSON.stringify([project,snapshot]);
const browserPreviewKey = (project: string, snapshot: string) => `code-atlas:workspace:v1:${project}:${snapshot}`;
const conversations=new Map<string,ConversationPersistence>();
const closingMessage='Code Atlas is closing. This question can be tried again.';
const rendererClock = typeof globalThis.performance?.now === 'function'
  ? () => globalThis.performance.now()
  : () => Date.now();
let closingWorkspace=false;
let deferredCloseUpdates:(()=>void)[]=[];
/** Freeze after reader capture. Late native-job reconciliation is replayed if
 * close fails; its source artifacts/run references already live natively. */
export function beginWorkspaceClose():()=>void {
  if(closingWorkspace)throw new Error('Closing is already in progress.');
  closingWorkspace=true;
  let resumed=false;
  return()=>{
    if(resumed)return;resumed=true;closingWorkspace=false;
    const updates=deferredCloseUpdates;deferredCloseUpdates=[];
    for(const update of updates)update();
  };
}
export async function flushAllWorkspaceConversations():Promise<void> {
  flushPendingWorkspaceDraftWrites();
  await Promise.all([...conversations.values()].map(entry=>entry.flushBeforeClose()));
}
function savedConversation(project:string,snapshot:string):ConversationPersistence {
  const id=key(project,snapshot);let entry=conversations.get(id);
  if(!entry){entry=new ConversationPersistence({projectId:project,snapshotId:snapshot,
    load:()=>invoke<ConversationRecord|null>('load_workspace_conversation',{projectId:project,snapshotId:snapshot}),
    save:(expectedRevision,state)=>invoke<ConversationRecord>('save_workspace_conversation',{projectId:project,snapshotId:snapshot,expectedRevision,state}),
    restore:state=>{
      const current=states.get(id);
      const drafts=new Map<string,ComposerDraft>();
      for(const exploration of current?.explorations??[])if(exploration.draft)drafts.set(exploration.id,exploration.draft);
      const restored=drafts.size?{...state,explorations:state.explorations.map(exploration=>{
        const draft=drafts.get(exploration.id);return draft?{...exploration,draft}:exploration;
      })}:state;
      states.set(id,restored);listeners.forEach(notify=>notify());
    },
    availability:()=>listeners.forEach(notify=>notify()),
    failed:message=>reportUnsavedConversation(project,snapshot,true,message),saved:()=>resolveUnsavedConversation(project,snapshot),
  });conversations.set(id,entry);}return entry;
}
function resolveUnsavedConversation(project:string,snapshot:string){
  const id=key(project,snapshot),occurrenceId=unsavedWorkspaceEpisodes.get(id);
  if(occurrenceId){unsavedWorkspaceEpisodes.delete(id);window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice-resolved',{detail:{projectId:project,snapshotId:snapshot,occurrenceId}}));}
}
function reportUnsavedConversation(project:string,snapshot:string,native=false,failureMessage?:string,target?:NoticeTarget){
  const id=key(project,snapshot);if(unsavedWorkspaceEpisodes.has(id))return;
  const occurrenceId=crypto.randomUUID();unsavedWorkspaceEpisodes.set(id,occurrenceId);
  window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{title:'Conversation could not be saved',
    message:failureMessage??(native?'Some conversation changes could not be saved. Existing explanations are preserved.':'Local workspace storage is unavailable. Generated explanations remain in the native library.'),
    projectId:project,snapshotId:snapshot,occurrenceId,...(target?{target}:native?{target:{kind:'retryConversation'}}:{})}}));
}
function saveBrowserPreview(project:string,snapshot:string,state:WorkspaceState){
  let serialized:string|null;
  try{serialized=serializeWorkspace(state);}
  catch{reportUnsavedConversation(project,snapshot);return;}
  if(serialized===null){
    reportUnsavedConversation(project,snapshot,false,'The browser preview cannot save this conversation because it exceeds the 8 MiB limit. This update stays on screen but may be lost if you reload; the earlier saved version is intact.');
    return;
  }
  try{
    localStorage.setItem(browserPreviewKey(project,snapshot),serialized);
    resolveUnsavedConversation(project,snapshot);
  }catch{reportUnsavedConversation(project,snapshot);}
}
export function prepareWorkspaceConversation(project:string,snapshot:string):Promise<void>{
  if(closingWorkspace)return new Promise((resolve,reject)=>{deferredCloseUpdates.push(()=>{void prepareWorkspaceConversation(project,snapshot).then(resolve,reject);});});
  return isNativeRuntime()?savedConversation(project,snapshot).start():Promise.resolve();
}
export function flushWorkspaceConversation(project:string,snapshot:string):Promise<void>{flushPendingWorkspaceDraftWrite(project,snapshot);return isNativeRuntime()?savedConversation(project,snapshot).flush():Promise.resolve();}
export function flushWorkspaceConversationBeforeGeneration(project:string,snapshot:string):Promise<void>{flushPendingWorkspaceDraftWrite(project,snapshot);return isNativeRuntime()?savedConversation(project,snapshot).flushForGeneration():Promise.resolve();}
export function retryWorkspaceConversation(project:string,snapshot:string):Promise<void>{return isNativeRuntime()?savedConversation(project,snapshot).retry():Promise.resolve();}
export function resolveWorkspaceConversationConflict(project:string,snapshot:string,choice:ConversationConflictChoice):Promise<void>{
  return isNativeRuntime()?savedConversation(project,snapshot).resolveConflict(choice):Promise.resolve();
}
export function useConversationAvailability(project:string,snapshot:string){
  return useSyncExternalStore(notify=>{listeners.add(notify);return()=>{listeners.delete(notify);};},()=>isNativeRuntime()?savedConversation(project,snapshot).availability:'ready');
}
export function useWorkspaceConversationConflict(project:string,snapshot:string):ConversationConflictSummary|null{
  return useSyncExternalStore(notify=>{listeners.add(notify);return()=>{listeners.delete(notify);};},()=>
    isNativeRuntime()?savedConversation(project,snapshot).conflictSummary():null);
}
export function getWorkspace(project: string, snapshot: string): WorkspaceState {
  const id = key(project, snapshot);
  if (!states.has(id)) {
    let raw: string | null = null;
    let readFailed = false;
    if (!isNativeRuntime()) try { raw = localStorage.getItem(browserPreviewKey(project,snapshot)); } catch { readFailed = true; }
    const restored = decodeWorkspace(raw, project, snapshot);
    if (!isNativeRuntime() && (readFailed || (raw !== null && restored === null))) browserPreviewWriteBlocked.add(id);
    states.set(id, restored ?? createWorkspace(project, snapshot));
  }
  return states.get(id)!;
}
/** Explicitly discard only an unsupported browser-preview record after the user
 * chooses the notification action. The original bytes stay in place otherwise. */
export function discardUnavailableBrowserPreview(project: string, snapshot: string): void {
  if (isNativeRuntime()) throw new Error('This action is only available for the browser preview.');
  const id=key(project,snapshot);
  if (!browserPreviewWriteBlocked.has(id)) throw new Error('This browser preview has no unavailable saved record to discard.');
  try { localStorage.removeItem(browserPreviewKey(project,snapshot)); }
  catch { throw new Error('The saved browser preview could not be discarded. Its stored data remains untouched.'); }
  browserPreviewWriteBlocked.delete(id);
  states.set(id,createWorkspace(project,snapshot));
  resolveUnsavedConversation(project,snapshot);
  listeners.forEach((notify) => notify());
}
export function updateWorkspace(project: string, snapshot: string, update: (state: WorkspaceState) => WorkspaceState) {
  if(closingWorkspace){deferredCloseUpdates.push(()=>updateWorkspace(project,snapshot,update));return;}
  const native=isNativeRuntime();
  const persistence=native?savedConversation(project,snapshot):undefined;
  if(native&&persistence!.availability==='loading')throw new Error('The saved conversation is still opening.');
  const id = key(project, snapshot); const previous = getWorkspace(project, snapshot); const next = recordQuestionOutcomes(previous, update(previous), Date.now());
  if (next === previous) return;
  clearWorkspaceDraftTimer(id);
  states.set(id, next);
  if(native) {
    // When an existing record is unreadable, keep source navigation usable in
    // memory without replacing or attempting to migrate the preserved bytes.
    if(persistence!.availability==='ready')persistence!.enqueue(next);
  }
  else if (browserPreviewWriteBlocked.has(id)) {
    reportUnsavedConversation(project,snapshot,false,'Saved browser workspace data could not be opened. Its original bytes were preserved. Discard it to start fresh; changes in this preview cannot be saved.',{kind:'discardBrowserPreview'});
  } else saveBrowserPreview(project,snapshot,next);
  listeners.forEach((notify) => notify());
}
/** Draft edits update the reader immediately and save after typing pauses. */
export function updateWorkspaceDraft(project:string,snapshot:string,explorationId:string,draft:ComposerDraft|null):void {
  if(closingWorkspace){deferredCloseUpdates.push(()=>updateWorkspaceDraft(project,snapshot,explorationId,draft));return;}
  const native=isNativeRuntime();
  const persistence=native?savedConversation(project,snapshot):undefined;
  if(native&&persistence!.availability==='loading')throw new Error('The saved conversation is still opening.');
  const id=key(project,snapshot),previous=getWorkspace(project,snapshot);
  const next=rememberComposerDraft(previous,explorationId,draft);
  if(next===previous)return;
  states.set(id,next);
  // Native conversation records deliberately omit unsent composer drafts.
  // Do not turn every typing pause into a durable no-op write (and a possible
  // cross-window revision conflict); browser preview still persists drafts.
  if(!native)scheduleWorkspaceDraftWrite(project,snapshot);
  listeners.forEach(notify=>notify());
}
function clearWorkspaceDraftTimer(id:string){
  const pending=workspaceDraftTimers.get(id);
  if(!pending)return;
  clearTimeout(pending.timer);
  workspaceDraftTimers.delete(id);
}
function persistWorkspaceDraftState(project:string,snapshot:string){
  const state=states.get(key(project,snapshot));
  if(!state)return;
  if(isNativeRuntime()){
    const persistence=savedConversation(project,snapshot);
    if(persistence.availability==='ready')persistence.enqueue(state);
    else if(persistence.availability==='unavailable')reportUnsavedConversation(project,snapshot,true,'The current question draft could not be saved.');
    return;
  }
  if(browserPreviewWriteBlocked.has(key(project,snapshot))){
    reportUnsavedConversation(project,snapshot,false,'Saved browser workspace data could not be opened. Its original bytes were preserved. Discard it to start fresh; changes in this preview cannot be saved.',{kind:'discardBrowserPreview'});
    return;
  }
  saveBrowserPreview(project,snapshot,state);
}
function scheduleWorkspaceDraftWrite(project:string,snapshot:string){
  const id=key(project,snapshot);
  clearWorkspaceDraftTimer(id);
  const timer=setTimeout(()=>{
    workspaceDraftTimers.delete(id);
    persistWorkspaceDraftState(project,snapshot);
  },250);
  workspaceDraftTimers.set(id,{project,snapshot,timer});
}
function flushPendingWorkspaceDraftWrite(project:string,snapshot:string){
  const id=key(project,snapshot),pending=workspaceDraftTimers.get(id);
  if(!pending)return;
  clearWorkspaceDraftTimer(id);
  persistWorkspaceDraftState(project,snapshot);
}
function flushPendingWorkspaceDraftWrites(){
  for(const pending of [...workspaceDraftTimers.values()])flushPendingWorkspaceDraftWrite(pending.project,pending.snapshot);
}
export function useWorkspace(project: string, snapshot: string) {
  return useSyncExternalStore((notify) => { listeners.add(notify); return () => { listeners.delete(notify); }; }, () => getWorkspace(project,snapshot));
}
/** Scrolling/code selection must not rerender the entire application header.
 * Subscribe to outcomes only; no empty project scope is created. */
export function useWorkspaceOutcomes(project?: string, snapshot?: string): QuestionOutcome[] {
  const select = useRef<ReturnType<typeof createQuestionOutcomeSelector> | null>(null);
  if (!select.current) select.current = createQuestionOutcomeSelector();
  return useSyncExternalStore((notify) => { listeners.add(notify); return () => { listeners.delete(notify); }; }, () => {
    const state = project && snapshot && (!isNativeRuntime()||savedConversation(project,snapshot).availability==='ready') ? getWorkspace(project, snapshot) : undefined;
    return select.current!(state);
  });
}
function patchTurn(project: string, snapshot: string, id: string, patch: Partial<Turn>) {
  updateWorkspace(project, snapshot, (s) => ({ ...s, explorations: s.explorations.map((e) => ({ ...e, turns: e.turns.map((t) => t.id === id ? { ...t, ...patch } : t) })) }));
}
function patchTurnForLatestRun(project: string, snapshot: string, id: string, runId: string, patch: Partial<Turn>): boolean {
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((candidate) => candidate.id === id);
  if (turn?.runIds?.at(-1) !== runId) return false;
  patchTurn(project,snapshot,id,patch);
  return true;
}
export function appendQuestion(project: string, snapshot: string, question: string, context: Context[], view: View, parentTurn?: string): string {
  const id = crypto.randomUUID();
  updateWorkspace(project, snapshot, (s) => addTurn(s,{id,question,context,status:'queued'},view,parentTurn?{originTurnId:parentTurn,id:crypto.randomUUID()}:undefined));
  return id;
}
export async function authorizeWorkspace(projectId: string, snapshotId: string) {
  await ensureCodexConnection();
  const grantId = await invoke<string>('authorize_workspace_exploration', { projectId, snapshotId, consent: true });
  updateWorkspace(projectId, snapshotId, (s) => ({ ...s, grantId }));
  return grantId;
}
export async function revokeWorkspace(projectId: string, snapshotId: string) {
  const grantId = getWorkspace(projectId,snapshotId).grantId;
  // Stop first, including a queued operation. Revalidation rejects revoked grants at dispatch.
  for (const e of getWorkspace(projectId,snapshotId).explorations) for (const t of e.turns) if (jobs.has(t.id) || ['queued','working','interrupted'].includes(t.status)) stopQuestion(projectId,snapshotId,t.id);
  if (grantId) await invoke('revoke_workspace_exploration', { grantId });
  updateWorkspace(projectId,snapshotId,(s) => ({ ...s, grantId: undefined }));
}
export function stopQuestion(project: string, snapshot: string, id: string) {
  const job = jobs.get(id);
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === id);
  if (turn?.guideId) {
    if (job) job.cancelled = true;
    if ((job && !job.runId) || (!turn.guide && !turn.runIds?.length && !job?.runId)) {
      patchTurn(project,snapshot,id,{status:'stopped',error:undefined});
      return;
    }
    void invoke('stop_repository_guide',{projectId:project,snapshotId:snapshot,id:turn.guideId}).then(() => restoreRepositoryGuides(project,snapshot)).catch((failure) => {
      patchTurn(project,snapshot,id,{error:errorMessageOr(failure,'The guide could not be stopped. Try Stop again.')});
    });
    return;
  }
  const runId = job?.runId ?? turn?.runIds?.at(-1);
  if (runId) {
    void stopWorkspaceGenerationJob(runId,project,snapshot).then((saved) => {
      if (job) job.cancelled = true;
      const status = saved.state === 'completed' ? 'ready'
        : saved.state === 'partiallyCompleted' ? 'partial'
        : saved.state === 'failed' ? 'failed'
        : saved.state === 'interrupted' ? 'interrupted'
        : 'stopped';
      if (patchTurnForLatestRun(project,snapshot,id,runId,{status,error:undefined}) && (status === 'ready' || status === 'partial')) {
        void restoreWorkspaceAnswers(project,snapshot).catch(() => {});
      }
    }).catch((failure) => {
      patchTurnForLatestRun(project,snapshot,id,runId,{error:errorMessageOr(failure,'The question could not be stopped. Try Stop again.')});
    });
    return;
  }
  if (job) job.cancelled = true;
  patchTurn(project,snapshot,id,{ status:'stopped', error: undefined });
}
const enterQuestionQueue = createQuestionQueue();
function prewarmRevisionNarration(revision: ChapterRevision) {
  if (!isNativeRuntime() || readVoiceConnectionIssue()) return;
  const connection = readVoiceConnection();
  if (connection.backend !== 'pocket-tts') return;
  const read = (key: string) => {
    try { return window.localStorage.getItem(key); }
    catch { return null; }
  };
  narrationPrewarmer.enqueue(localNarrationPrewarmInputs(revision, connection, read));
}
/** Store section references only after the native draft is independently read
 * and saved. Retrying adoption cannot trigger another model request. */
async function adoptSection(project: string, snapshot: string, turnId: string | undefined, section: PublishedSection, prewarmAudio = false): Promise<ChapterRevision> {
  const revision = await invoke<ChapterRevision>('save_workspace_answer_section', {
    projectId:project,snapshotId:snapshot,runId:section.runId,generationId:section.generationId,
  });
  if (revision.revisionId !== section.revisionId || revision.projectId !== project || revision.snapshotId !== snapshot) throw new Error('The saved section identity changed.');
  if (turnId) updateWorkspace(project,snapshot,(state) => ({...state,explorations:state.explorations.map((e) => ({...e,
    turns:e.turns.map((t) => {
      if (t.id !== turnId) return t;
      const owner = t.guideId && !t.runIds?.includes(section.runId)
        ? {...t,runIds:[...(t.runIds ?? []),section.runId].slice(-32)}
        : t;
      return mergePublishedSections(owner,[section]);
    }),
  }))}));
  // Queue local narration before notifying the reader. Otherwise the player can
  // check preparation status in the gap, see "idle", and stop polling before
  // this new section reaches the bounded native prewarm queue. This remains
  // fire-and-forget; synthesis cannot delay publishing the saved section.
  if (prewarmAudio) prewarmRevisionNarration(revision);
  window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed',{detail:{projectId:project,revision}}));
  return revision;
}
async function reconcileAnswer(project: string, snapshot: string, turnId: string, summary: AnswerSummary, prewarmAudio = false) {
  if (summary.projectId !== project || summary.snapshotId !== snapshot) throw new Error('The saved answer belongs to another snapshot.');
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (!turn?.runIds?.includes(summary.runId)) throw new Error('The saved answer belongs to another question.');
  let revision: ChapterRevision | null = null;
  const failures: unknown[] = [];
  for (const section of summary.sections) {
    try { revision = await adoptSection(project,snapshot,turnId,section,prewarmAudio); } catch (error) { failures.push(error); }
  }
  if (failures.length) throw failures[0];
  const owner = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (owner?.runIds?.at(-1) === summary.runId) patchTurn(project,snapshot,turnId,{status:summary.outcome === 'complete' ? 'ready' : summary.outcome,
    notes:cleanAnswerNotes(summary.incompleteTopics),failureCode:cleanExplanationFailureCode(summary.failureCode),error:undefined});
  return revision;
}
/** Startup summaries are already replay-validated by native code, and their
 * drafts are adopted before the list command returns. Merge these identities
 * directly so restoring a saved answer does not replay it a second time. */
function reconcileListedAnswer(project: string, snapshot: string, turnId: string, summary: AnswerSummary) {
  if (summary.projectId !== project || summary.snapshotId !== snapshot) throw new Error('The saved answer belongs to another snapshot.');
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (!turn?.runIds?.includes(summary.runId)) throw new Error('The saved answer belongs to another question.');
  updateWorkspace(project,snapshot,(state) => ({...state,explorations:state.explorations.map((exploration) => ({...exploration,
    turns:exploration.turns.map((candidate) => candidate.id === turnId ? mergePublishedSections(candidate,summary.sections) : candidate),
  }))}));
  const owner = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (owner?.runIds?.at(-1) === summary.runId) patchTurn(project,snapshot,turnId,{status:summary.outcome === 'complete' ? 'ready' : summary.outcome,
    notes:cleanAnswerNotes(summary.incompleteTopics),failureCode:cleanExplanationFailureCode(summary.failureCode),error:undefined});
}
interface Restoration { queue: RefreshQueue; check: () => void }
const restorations = new Map<string, Restoration>();
type RestorationAvailability = 'loading' | 'ready' | 'unavailable';
interface RestorationObservers { callbacks: Set<(availability: RestorationAvailability) => void>; version: number }
const restorationObservers = new Map<string, RestorationObservers>();
function observeRestoration(scope: string, callback: (availability: RestorationAvailability) => void): () => void {
  let observers = restorationObservers.get(scope);
  if (!observers) { observers = {callbacks:new Set(),version:0}; restorationObservers.set(scope,observers); }
  const group = observers;
  group.callbacks.add(callback);
  return () => {
    group.callbacks.delete(callback);
    if (!group.callbacks.size && restorationObservers.get(scope) === group) restorationObservers.delete(scope);
  };
}
function publishRestoration(scope: string, availability: RestorationAvailability) {
  const observers = restorationObservers.get(scope);
  if (!observers) return;
  const version = ++observers.version;
  for (const callback of [...observers.callbacks]) {
    // A listener can begin a newer read. Do not deliver the older idle state
    // afterward to another observer of that same repository/snapshot.
    if (observers.version !== version) break;
    if (observers.callbacks.has(callback)) callback(availability);
  }
}
/** A reloaded webview can observe a still-owned native worker. Reading its
 * events does not dispatch a question; Stop uses the persisted native run ID. */
export async function watchWorkspaceAnswers(project: string, snapshot: string, onAvailability?: (availability: RestorationAvailability) => void): Promise<() => void> {
  let closed = false;
  const refresh = (scope: {projectId:string;snapshotId:string;runId:string}) => {
    if (closed || scope.projectId !== project || scope.snapshotId !== snapshot) return;
    const turns = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns);
    const turn = turns.find((candidate) => candidate.runIds?.includes(scope.runId));
    if (turn && jobs.has(turn.id)) return;
    // Live guides have a guide-scoped section listener below. Avoid a full
    // saved-answer/guide read for each slide while that owned run is active;
    // after reload there is no renderer job, so the generic listener still
    // performs the normal local reconciliation.
    if (turns.some((candidate) => candidate.guideId && jobs.has(candidate.id))) return;
    void restoreWorkspaceAnswers(project,snapshot).catch(() => {});
  };
  const section = await listen<Pick<AnswerSummary,'projectId'|'snapshotId'|'runId'>>('workspace-answer-section',({payload}) => refresh(payload));
  try {
    const finished = await listen<AnswerSummary>('workspace-answer-finished',({payload}) => refresh(payload));
    let guide: (() => void) | undefined;
    let guideSection: (() => void) | undefined;
    try {
      guide = await listen<RepositoryGuide>('repository-guide-changed',({payload}) => {
        if (closed || payload.projectId !== project || payload.snapshotId !== snapshot) return;
        // Events invalidate saved data. Reading it again also verifies the
        // latest guide instead of allowing an older event payload to win.
        void restoreChangedRepositoryGuide(project,snapshot,payload.id).catch(() => {});
      });
      guideSection = await listen<{guideId:string;section:PublishedSection & {projectId:string;snapshotId:string}}>('repository-guide-section-published',({payload}) => {
        const section = payload.section;
        if (closed || !payload.guideId || section.projectId !== project || section.snapshotId !== snapshot) return;
        const owner = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.guideId === payload.guideId);
        // The progressive answer receipt is already durable, but repository-guide
        // summaries arrive only after a whole investigation unit. Adopt this exact
        // validated section now so local narration can warm while that unit continues.
        void adoptSection(project,snapshot,owner?.id,section,true).catch(() => {
          void restoreChangedRepositoryGuide(project,snapshot,payload.guideId).catch(() => {});
        });
      });
      const unobserve = onAvailability ? observeRestoration(key(project,snapshot),(availability) => { if (!closed) onAvailability(availability); }) : undefined;
      return () => {closed=true;unobserve?.();section();finished();guide?.();guideSection?.();};
    } catch (error) { guideSection?.();guide?.();finished();throw error; }
  } catch (error) { section(); throw error; }
}
/** Restart recovery is local reading only. It never reconnects or replays AI. */
export function restoreWorkspaceAnswers(project: string, snapshot: string): Promise<void> {
  const scope = key(project,snapshot);
  let restoration = restorations.get(scope);
  const started = !restoration;
  if (!restoration) {
    let failure: { value: unknown } | null = null;
    const entry: Restoration = {
      queue: createRefreshQueue<void>({
        load: () => readSavedWorkspaceAnswers(project,snapshot),
        onValue: () => { failure = null; },
        onError: (value) => { failure = {value}; },
        onIdle: () => { if (restorations.get(scope) === entry) {
          restorations.delete(scope);
          publishRestoration(scope,failure ? 'unavailable' : 'ready');
        } },
      }),
      check: () => { if (failure) throw failure.value; },
    };
    restorations.set(scope,entry); restoration = entry;
  }
  // A concurrent invalidation schedules one fresh serial read. Every caller
  // waits for that read and receives its final failure; no error retries itself.
  const pending = restoration.queue.refresh();
  if (started) publishRestoration(scope,'loading');
  return pending.then(restoration.check);
}
async function readSavedWorkspaceAnswers(project: string, snapshot: string): Promise<void> {
  await prepareWorkspaceConversation(project,snapshot);
  await restoreRepositoryGuides(project,snapshot);
  const saved = await invoke<AnswerSummary[]>('list_workspace_answers',{projectId:project,snapshotId:snapshot});
  let failed = false;
  let firstFailure: unknown;
  for (const entry of saved) {
    let turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.runIds?.includes(entry.runId));
    if (turn?.guideId) continue;
    if (turn && jobs.has(turn.id)) continue;
    if (!turn) {
      const id = crypto.randomUUID();
      turn = {id,question:entry.question,context:[],status:'interrupted',runIds:[entry.runId],publishedSections:[]};
      const restored = turn;
      updateWorkspace(project,snapshot,(state) => ({...state,explorations:state.explorations.map((e) => e.id === 'root' ? {...e,turns:[...e.turns,restored]} : e)}));
    }
    try {
      reconcileListedAnswer(project,snapshot,turn.id,entry);
    } catch (error) {
      // One damaged or concurrently removed receipt must not prevent later
      // independently saved answers from being restored. Keep the overall
      // scope unavailable so Notifications still offers a local retry.
      if (!failed) firstFailure = error;
      failed = true;
    }
  }
  try { restoreWorkspaceJobStates(project,snapshot,await listWorkspaceGenerationJobsForScope(project,snapshot,rendererSessionId)); }
  catch { /* Saved answers remain readable; an unavailable job ledger is conservative. */ }
  if (failed) throw firstFailure;
}

function restoreWorkspaceJobStates(project:string,snapshot:string,jobs:WorkspaceGenerationJob[]) {
  for (const job of jobs) {
    if (job.kind!=='workspaceQuestion'||job.projectId!==project||job.snapshotId!==snapshot||!job.conversationTurnId) continue;
    const turns=getWorkspace(project,snapshot).explorations.flatMap((e)=>e.turns);
    const turn=turns.find((candidate)=>candidate.id===job.conversationTurnId);
    if (!turn||!turn.runIds?.includes(job.jobId)||turn.runIds.at(-1)!==job.jobId) continue;
    if(job.state==='queued'&&job.rendererSessionId===rendererSessionId&&turn.status==='working')continue;
    const status:Turn['status']=job.state==='queued'?'queued'
      :job.state==='preparing'||job.state==='running'||job.state==='stopRequested'?'working'
      :job.state==='completed'?'ready':job.state==='partiallyCompleted'?'partial'
      :job.state==='stopped'?'stopped':job.state==='failed'?'failed':'interrupted';
    patchTurn(project,snapshot,turn.id,status==='working'?{status,error:undefined}:{status});
  }
}
/** Explicitly remove a terminal investigation receipt after its drafts have
 * been saved. Questions, source links and chapter revisions remain readable. */
export async function removeAnswerRecord(project: string, snapshot: string, runId: string) {
  await invoke('remove_workspace_answer_record',{projectId:project,snapshotId:snapshot,runId});
}
export function appendRepositoryGuide(project: string, snapshot: string, question: string, parentTurn?: string): string {
  const id = appendQuestion(project,snapshot,question,[],'guide',parentTurn);
  patchTurn(project,snapshot,id,{guideId:crypto.randomUUID()});
  return id;
}
async function reconcileGuide(project: string, snapshot: string, guide: RepositoryGuide, prewarmAudio = false) {
  if (guide.projectId !== project || guide.snapshotId !== snapshot) throw new Error('This guide belongs to another snapshot.');
  const previousTurn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.guideId === guide.id);
  const previouslyKnownRevisions = new Set(previousTurn?.publishedSections?.map((section) => section.revisionId) ?? []);
  updateWorkspace(project,snapshot,(state) => mergeRepositoryGuide(state,guide));
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.guideId === guide.id);
  if (!turn) return;
  // Adoption is local and idempotent. A freed terminal receipt may already have
  // all immutable drafts in the library; never regenerate it to restore cards.
  const saved = await listChapterRevisions(project,snapshot);
  for (const answer of guide.answers) for (const section of answer.sections) {
    const isNew = !previouslyKnownRevisions.has(section.revisionId);
    const revision = saved.find((r) => r.revisionId === section.revisionId);
    if (revision) {
      if (prewarmAudio && isNew) prewarmRevisionNarration(revision);
      continue;
    }
    await adoptSection(project,snapshot,turn.id,section,prewarmAudio && isNew);
  }
  window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed',{detail:{projectId:project}}));
}
async function restoreChangedRepositoryGuide(project: string, snapshot: string, guideId: string) {
  const result = await invoke<RepositoryGuideList>('list_repository_guides',{projectId:project,snapshotId:snapshot});
  const guide = result.guides.find((candidate) => candidate.id === guideId);
  if (guide) await reconcileGuide(project,snapshot,guide,true);
}
async function restoreRepositoryGuides(project: string, snapshot: string) {
  const result = await invoke<RepositoryGuideList>('list_repository_guides',{projectId:project,snapshotId:snapshot});
  let unavailableCount = result.unavailableCount;
  for (const guide of result.guides) {
    // One revision with an old or damaged visual contract can fail during
    // chapter adoption. Keep restoring the remaining independent guides.
    try { await reconcileGuide(project,snapshot,guide); }
    catch { unavailableCount += 1; }
  }
  if (unavailableCount > 0) window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{
    title:'Some saved guides need attention',
    message:`${unavailableCount} saved guide item${unavailableCount===1?'':'s'} could not be fully reopened. Their saved work remains on this Mac.`,
    projectId:project,snapshotId:snapshot,
  }}));
}
/** Explicit start/continue only. The native worker survives webview navigation;
 * restored metadata never calls this function. */
export async function runRepositoryGuide(project: string, snapshot: string, id: string, grantId: string, resume = false, retryUnitKey?: string) {
  if (jobs.has(id)) return;
  if(closingWorkspace){patchTurn(project,snapshot,id,{status:'interrupted',error:closingMessage});return;}
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === id);
  if (!turn?.guideId) return;
  const job = {cancelled:false,runId:null as string|null,pendingGuideAdvances:[]};jobs.set(id,job);
  const queueWaitStarted = rendererClock();
  const release = await enterQuestionQueue();
  const rendererQueueWaitMs = Math.min(86_400_000, Math.max(0, Math.round(rendererClock() - queueWaitStarted)));
  try {
    if (job.cancelled) return;
    if(closingWorkspace)throw new Error(closingMessage);
    patchTurn(project,snapshot,id,{status:'working',error:undefined});
    await flushWorkspaceConversationBeforeGeneration(project,snapshot);
    if(closingWorkspace)throw new Error(closingMessage);
    await ensureCodexConnection(true);
    if (job.cancelled) return;
    if(closingWorkspace)throw new Error(closingMessage);
    const selection = readCodexSelection();
    job.runId = turn.guideId;
    const guide = retryUnitKey
      ? await invoke<RepositoryGuide>('retry_repository_guide_unit',{id:turn.guideId,projectId:project,snapshotId:snapshot,unitKey:retryUnitKey,grantId,consent:true,rendererQueueWaitMs})
      : await invoke<RepositoryGuide>(resume ? 'resume_repository_guide' : 'start_repository_guide', resume
        ? {id:turn.guideId,projectId:project,snapshotId:snapshot,consent:true,rendererQueueWaitMs}
        : {input:{id:turn.guideId,projectId:project,snapshotId:snapshot,grantId,question:turn.question,
            modelId:selection.modelId,reasoningEffort:selection.effort,consent:true,rendererQueueWaitMs}});
    await reconcileGuide(project,snapshot,guide,true);
  } catch (failure) {
    try { await restoreRepositoryGuides(project,snapshot); } catch { /* Preserve saved cards and the original error. */ }
    const message=errorMessageOr(failure,'The guide could not be completed. Saved sections remain available.');
    patchTurn(project,snapshot,id,{status:job.cancelled?'stopped':message===closingMessage?'interrupted':'partial',error:message});
  } finally {
    const pending = job.pendingGuideAdvances.slice();
    release();jobs.delete(id);
    if(!job.cancelled)for(const revisionId of pending)void advanceRepositoryGuide(project,snapshot,id,revisionId);
  }
}
/** A listened/finished guide slide frees exactly one idempotent buffer slot. */
export async function advanceRepositoryGuide(project:string,snapshot:string,turnId:string,consumedRevisionId:string) {
  if(closingWorkspace||!consumedRevisionId)return;
  const turn=getWorkspace(project,snapshot).explorations.flatMap((e)=>e.turns).find((candidate)=>candidate.id===turnId);
  if(!turn?.guideId||!canContinueGuide(turn))return;
  const active=jobs.get(turnId);
  if(active){
    if(!active.pendingGuideAdvances.includes(consumedRevisionId)&&active.pendingGuideAdvances.length<16)active.pendingGuideAdvances.push(consumedRevisionId);
    return;
  }
  const job={cancelled:false,runId:null as string|null,pendingGuideAdvances:[] as string[]};jobs.set(turnId,job);
  const queueWaitStarted=rendererClock();
  const release=await enterQuestionQueue();
  const rendererQueueWaitMs=Math.min(86_400_000,Math.max(0,Math.round(rendererClock()-queueWaitStarted)));
  try{
    if(job.cancelled||closingWorkspace)return;
    const current=getWorkspace(project,snapshot).explorations.flatMap((e)=>e.turns).find((candidate)=>candidate.id===turnId);
    if(!current?.guideId||!canContinueGuide(current))return;
    patchTurn(project,snapshot,turnId,{status:'working',error:undefined});
    await flushWorkspaceConversationBeforeGeneration(project,snapshot);
    if(job.cancelled||closingWorkspace)return;
    await ensureCodexConnection(true);
    if(job.cancelled||closingWorkspace)return;
    job.runId=current.guideId;
    const guide=await invoke<RepositoryGuide>('advance_repository_guide',{id:current.guideId,projectId:project,snapshotId:snapshot,consumedRevisionId,rendererQueueWaitMs});
    await reconcileGuide(project,snapshot,guide,true);
  }catch(failure){
    try{await restoreRepositoryGuides(project,snapshot);}catch{/* Keep the current reader content. */}
    const message=errorMessageOr(failure,'The next slide could not be prepared. Saved slides remain available.');
    patchTurn(project,snapshot,turnId,{status:job.cancelled?'stopped':message===closingMessage?'interrupted':'partial',error:message});
  }finally{
    const pending=job.pendingGuideAdvances.slice();
    release();jobs.delete(turnId);
    if(!job.cancelled)for(const revisionId of pending)void advanceRepositoryGuide(project,snapshot,turnId,revisionId);
  }
}
function canContinueGuide(turn:Turn){return turn.guide?.canContinue===true;}
/** Module-owned operation: changing tab, file, repository or opening settings does not cancel it. */
export async function runQuestion(project: string, snapshot: string, id: string, grantId: string): Promise<ChapterRevision | null> {
  if (jobs.has(id)) return null;
  if(closingWorkspace){patchTurn(project,snapshot,id,{status:'interrupted',error:closingMessage});return null;}
  const owner = getWorkspace(project,snapshot).explorations.find((e) => e.turns.some((t) => t.id === id));
  const turn = owner?.turns.find((t) => t.id === id);
  if (!turn || !owner) return null;
  if ((turn.runIds?.length ?? 0) >= 32) { patchTurn(project,snapshot,id,{error:'This question has reached its retry limit. Ask a follow-up to continue.'}); return null; }
  const runId = crypto.randomUUID();
  const selection = readCodexSelection();
  const job = {cancelled:false,runId,pendingGuideAdvances:[] as string[]}; jobs.set(id,job);
  const check = () => {if (job.cancelled)throw new Error('Stopped');if(closingWorkspace)throw new Error(closingMessage);};
  patchTurn(project,snapshot,id,{status:'queued',failureCode:undefined,error:undefined,runIds:[...(turn.runIds ?? []),runId]});
  let release:(()=>void)|null=null;
  let unlisten: (() => void) | undefined;
  let generationDispatched = false;
  let adoption = Promise.resolve();
  try {
    // Save turn identity first so either side of the following native write
    // can be reconciled after a WebView reload.
    await flushWorkspaceConversationBeforeGeneration(project,snapshot);check();
    await queueWorkspaceGenerationJob({jobId:runId,conversationTurnId:id,rendererSessionId,projectId:project,snapshotId:snapshot,
      modelId:selection.modelId,reasoningEffort:selection.effort});
    const queueWaitStarted=rendererClock();
    release=await enterQuestionQueue();
    const queueWaitMs = Math.min(86_400_000,Math.max(0,Math.round(rendererClock()-queueWaitStarted)));
    check(); patchTurn(project,snapshot,id,{status:'working',error:undefined});
    await ensureCodexConnection(true); check();
    // Subscribe before dispatch. Events carry identities only; the native save
    // command verifies the full checkpoint before any content is rendered.
    unlisten = await listen<PublishedSection & {projectId:string;snapshotId:string}>('workspace-answer-section',({payload}) => {
      if (payload.runId !== runId || payload.projectId !== project || payload.snapshotId !== snapshot) return;
      adoption = adoption.then(async () => { await adoptSection(project,snapshot,id,payload,true); }).catch(() => {
        // Final reconciliation retries local adoption after a lost event/reply.
        // Existing cards stay visible throughout; inference is never replayed.
      });
    });
    check();
    generationDispatched = true;
    const summary = await invoke<AnswerSummary>('generate_progressive_workspace_answer',{input:{
      runId,turnId:id,projectId:project,snapshotId:snapshot,grantId,rendererQueueWaitMs:queueWaitMs,
      question:learningQuestion(getWorkspace(project,snapshot),owner.id,id,1200),
      references:(getWorkspace(project,snapshot).explorations.find((e)=>e.id===owner.id)?.turns.find((t)=>t.id===id)?.context ?? [])
        .map((c) => ({path:c.path,sha256:c.sha256 ?? null,from:c.from ?? null,to:c.to ?? null})),...selection,
    }});
    await adoption;
    const revision = await reconcileAnswer(project,snapshot,id,summary,true);
    return revision;
  } catch (failure) {
    await adoption;
    if (!generationDispatched) {
      // Local save conflicts, connection failures, and pre-dispatch Stops did
      // not consume a model attempt. Keep any other-window run IDs intact.
      const latest=getWorkspace(project,snapshot).explorations.flatMap((e)=>e.turns).find((candidate)=>candidate.id===id);
      if (latest?.runIds?.includes(runId)) {
        patchTurn(project,snapshot,id,{runIds:latest.runIds.filter((candidate)=>candidate!==runId)});
      }
    }
    try {
      const state=job.cancelled?'stopped':closingWorkspace?'interrupted':'failed';
      // Native ownership is checked atomically. This closes the queued record
      // if IPC failed before worker registration and is a no-op once native
      // execution or terminal reconciliation owns it.
      await finishQueuedWorkspaceGenerationJob(runId,project,snapshot,state);
    } catch { /* A native worker may have taken ownership during dispatch. */ }
    let recovered = false;
    if (job.runId) {
      try {
        const summary = await invoke<AnswerSummary>('load_workspace_answer',{projectId:project,snapshotId:snapshot,runId:job.runId});
        await reconcileAnswer(project,snapshot,id,summary,true); recovered = true;
      } catch { /* Preserve existing cards even if local inspection fails. */ }
    }
    if (!recovered) {
      const message = errorMessageOr(failure,'This question could not be completed. Saved sections remain available.');
      if (/permission|allow.*explor|snapshot or Codex account changed/i.test(message)) updateWorkspace(project,snapshot,(s) => ({...s,grantId:undefined}));
      const saved = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === id)?.publishedSections;
      patchTurn(project,snapshot,id,{status:job.cancelled ? 'stopped' : message===closingMessage ? 'interrupted' : saved?.length ? 'partial' : 'failed',error:job.cancelled ? undefined : message});
    }
    return null;
  } finally { unlisten?.(); release?.(); jobs.delete(id); }
}
