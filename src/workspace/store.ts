import { createQuestionQueue } from './queue';
import { createRefreshQueue, type RefreshQueue } from './refreshQueue';
import { useRef, useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { cancelCodexChapterRequest, getSnapshotIndexProgress, isNativeRuntime, listChapterRevisions, loadSnapshotIndex, startSnapshotIndexing, type ChapterRevision, type SnapshotIndex } from '../lib/commands';
import {ConversationPersistence,type ConversationRecord} from './conversationPersistence';
import { ensureCodexConnection, readCodexSelection } from '../lib/connectionStore';
import { errorMessageOr } from '../lib/errorMessage';
import { createQuestionOutcomeSelector, recordQuestionOutcomes, type QuestionOutcome } from './outcomes';
import { addTurn, cleanAnswerNotes, learningQuestion, mergePublishedSections, mergeRepositoryGuide, parseWorkspace, type RepositoryGuide, type AnswerSummary, type PublishedSection, type Context, type Turn, type View, type WorkspaceState } from './model';

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
    const deadline = Date.now() + 300000;
    for (;;) {
      const progress = await getSnapshotIndexProgress(started.jobId);
      if (!progress) throw new Error('The local code index could not be read. Try again.');
      if (progress.status !== 'running') {
        const result = await loadSnapshotIndex(project,snapshot);
        if (!result) throw new Error(progress.message || 'The local code index could not be built. Try again.');
        return result;
      }
      if (Date.now() >= deadline) throw new Error('The local code index is still building. Try this question again when it finishes.');
      await new Promise((resolve) => setTimeout(resolve,200));
    }
  })().finally(() => indexLoads.delete(id));
  indexLoads.set(id,pending);
  return pending;
}

const states = new Map<string, WorkspaceState>();
const unsavedWorkspaceEpisodes = new Map<string,string>();
const listeners = new Set<() => void>();
const jobs = new Map<string, { cancelled: boolean; runId: string | null }>();
const key = (project:string,snapshot:string) => JSON.stringify([project,snapshot]);
const browserPreviewKey = (project: string, snapshot: string) => `code-atlas:workspace:v1:${project}:${snapshot}`;
const conversations=new Map<string,ConversationPersistence>();
const closingMessage='Code Atlas is closing. This question can be tried again.';
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
  await Promise.all([...conversations.values()].map(entry=>entry.flushBeforeClose()));
}
function savedConversation(project:string,snapshot:string):ConversationPersistence {
  const id=key(project,snapshot);let entry=conversations.get(id);
  if(!entry){entry=new ConversationPersistence({projectId:project,snapshotId:snapshot,
    load:()=>invoke<ConversationRecord|null>('load_workspace_conversation',{projectId:project,snapshotId:snapshot}),
    save:(expectedRevision,state)=>invoke<ConversationRecord>('save_workspace_conversation',{projectId:project,snapshotId:snapshot,expectedRevision,state}),
    restore:state=>{states.set(id,state);listeners.forEach(notify=>notify());},
    availability:()=>listeners.forEach(notify=>notify()),
    failed:()=>reportUnsavedConversation(project,snapshot,true),saved:()=>resolveUnsavedConversation(project,snapshot),
  });conversations.set(id,entry);}return entry;
}
function resolveUnsavedConversation(project:string,snapshot:string){
  const id=key(project,snapshot),occurrenceId=unsavedWorkspaceEpisodes.get(id);
  if(occurrenceId){unsavedWorkspaceEpisodes.delete(id);window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice-resolved',{detail:{projectId:project,snapshotId:snapshot,occurrenceId}}));}
}
function reportUnsavedConversation(project:string,snapshot:string,native=false){
  const id=key(project,snapshot);if(unsavedWorkspaceEpisodes.has(id))return;
  const occurrenceId=crypto.randomUUID();unsavedWorkspaceEpisodes.set(id,occurrenceId);
  window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{title:'Conversation could not be saved',
    message:native?'Some conversation changes could not be saved. Existing explanations are preserved.':'Local workspace storage is unavailable. Generated explanations remain in the native library.',
    projectId:project,snapshotId:snapshot,occurrenceId,...(native?{target:{kind:'retryConversation'}}:{})}}));
}
export function prepareWorkspaceConversation(project:string,snapshot:string):Promise<void>{
  if(closingWorkspace)return new Promise((resolve,reject)=>{deferredCloseUpdates.push(()=>{void prepareWorkspaceConversation(project,snapshot).then(resolve,reject);});});
  return isNativeRuntime()?savedConversation(project,snapshot).start():Promise.resolve();
}
export function flushWorkspaceConversation(project:string,snapshot:string):Promise<void>{return isNativeRuntime()?savedConversation(project,snapshot).flush():Promise.resolve();}
export function retryWorkspaceConversation(project:string,snapshot:string):Promise<void>{return isNativeRuntime()?savedConversation(project,snapshot).retry():Promise.resolve();}
export function useConversationAvailability(project:string,snapshot:string){
  return useSyncExternalStore(notify=>{listeners.add(notify);return()=>{listeners.delete(notify);};},()=>isNativeRuntime()?savedConversation(project,snapshot).availability:'ready');
}
export function getWorkspace(project: string, snapshot: string): WorkspaceState {
  const id = key(project, snapshot);
  if (!states.has(id)) { let raw = null; if(!isNativeRuntime())try { raw = localStorage.getItem(browserPreviewKey(project,snapshot)); } catch { /* Browser preview remains usable when storage is unavailable. */ } states.set(id, parseWorkspace(raw, project, snapshot)); }
  return states.get(id)!;
}
export function updateWorkspace(project: string, snapshot: string, update: (state: WorkspaceState) => WorkspaceState) {
  if(closingWorkspace){deferredCloseUpdates.push(()=>updateWorkspace(project,snapshot,update));return;}
  if(isNativeRuntime()&&savedConversation(project,snapshot).availability!=='ready')throw new Error('The saved conversation is still opening.');
  const id = key(project, snapshot); const previous = getWorkspace(project, snapshot); const next = recordQuestionOutcomes(previous, update(previous), Date.now());
  if (next === previous) return;
  states.set(id, next);
  if(isNativeRuntime())savedConversation(project,snapshot).enqueue(next);
  else try {
    localStorage.setItem(browserPreviewKey(project,snapshot), JSON.stringify(next));
    resolveUnsavedConversation(project,snapshot);
  } catch {reportUnsavedConversation(project,snapshot);}
  listeners.forEach((notify) => notify());
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
  if (job) job.cancelled = true;
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === id);
  if (turn?.guideId) {
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
  if (runId) void cancelCodexChapterRequest(runId).catch(() => {});
  patchTurn(project,snapshot,id,{ status:'stopped', error: undefined });
}
const enterQuestionQueue = createQuestionQueue();
/** Store section references only after the native draft is independently read
 * and saved. Retrying adoption cannot trigger another model request. */
async function adoptSection(project: string, snapshot: string, turnId: string, section: PublishedSection): Promise<ChapterRevision> {
  const revision = await invoke<ChapterRevision>('save_workspace_answer_section', {
    projectId:project,snapshotId:snapshot,runId:section.runId,generationId:section.generationId,
  });
  if (revision.revisionId !== section.revisionId || revision.projectId !== project || revision.snapshotId !== snapshot) throw new Error('The saved section identity changed.');
  updateWorkspace(project,snapshot,(state) => ({...state,explorations:state.explorations.map((e) => ({...e,
    turns:e.turns.map((t) => t.id === turnId ? mergePublishedSections(t,[section]) : t),
  }))}));
  window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed',{detail:{projectId:project}}));
  return revision;
}
async function reconcileAnswer(project: string, snapshot: string, turnId: string, summary: AnswerSummary) {
  if (summary.projectId !== project || summary.snapshotId !== snapshot) throw new Error('The saved answer belongs to another snapshot.');
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (!turn?.runIds?.includes(summary.runId)) throw new Error('The saved answer belongs to another question.');
  let revision: ChapterRevision | null = null;
  const failures: unknown[] = [];
  for (const section of summary.sections) {
    try { revision = await adoptSection(project,snapshot,turnId,section); } catch (error) { failures.push(error); }
  }
  if (failures.length) throw failures[0];
  const owner = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (owner?.runIds?.at(-1) === summary.runId) patchTurn(project,snapshot,turnId,{status:summary.outcome === 'complete' ? 'ready' : summary.outcome,
    notes:cleanAnswerNotes(summary.incompleteTopics),error:undefined});
  return revision;
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
    const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.runIds?.includes(scope.runId));
    if (turn && jobs.has(turn.id) && !turn.guideId) return;
    void restoreWorkspaceAnswers(project,snapshot).catch(() => {});
  };
  const section = await listen<Pick<AnswerSummary,'projectId'|'snapshotId'|'runId'>>('workspace-answer-section',({payload}) => refresh(payload));
  try {
    const finished = await listen<AnswerSummary>('workspace-answer-finished',({payload}) => refresh(payload));
    try {
      const guide = await listen<RepositoryGuide>('repository-guide-changed',({payload}) => {
        if (closed || payload.projectId !== project || payload.snapshotId !== snapshot) return;
        // Events invalidate saved data. Reading it again also verifies the
        // latest guide instead of allowing an older event payload to win.
        void restoreWorkspaceAnswers(project,snapshot).catch(() => {});
      });
      const unobserve = onAvailability ? observeRestoration(key(project,snapshot),(availability) => { if (!closed) onAvailability(availability); }) : undefined;
      return () => {closed=true;unobserve?.();section();finished();guide();};
    } catch (error) { finished(); throw error; }
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
    const summary = await invoke<AnswerSummary>('load_workspace_answer',{projectId:project,snapshotId:snapshot,runId:entry.runId});
    await reconcileAnswer(project,snapshot,turn.id,summary);
  }
}
/** Explicitly remove a terminal investigation receipt after its drafts have
 * been saved. Questions, source links and chapter revisions remain readable. */
export async function removeAnswerRecord(project: string, snapshot: string, runId: string) {
  await invoke('remove_workspace_answer_record',{projectId:project,snapshotId:snapshot,runId});
}
export function appendRepositoryGuide(project: string, snapshot: string, question: string): string {
  const id = appendQuestion(project,snapshot,question,[],'guide');
  patchTurn(project,snapshot,id,{guideId:crypto.randomUUID()});
  return id;
}
async function reconcileGuide(project: string, snapshot: string, guide: RepositoryGuide) {
  if (guide.projectId !== project || guide.snapshotId !== snapshot) throw new Error('This guide belongs to another snapshot.');
  updateWorkspace(project,snapshot,(state) => mergeRepositoryGuide(state,guide));
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.guideId === guide.id);
  if (!turn) return;
  // Adoption is local and idempotent. A freed terminal receipt may already have
  // all immutable drafts in the library; never regenerate it to restore cards.
  const saved = await listChapterRevisions(project,snapshot);
  for (const answer of guide.answers) for (const section of answer.sections) {
    if (!saved.some((r) => r.revisionId === section.revisionId)) await adoptSection(project,snapshot,turn.id,section);
  }
  window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed',{detail:{projectId:project}}));
}
async function restoreRepositoryGuides(project: string, snapshot: string) {
  const guides = await invoke<RepositoryGuide[]>('list_repository_guides',{projectId:project,snapshotId:snapshot});
  for (const guide of guides) await reconcileGuide(project,snapshot,guide);
}
/** Explicit start/continue only. The native worker survives webview navigation;
 * restored metadata never calls this function. */
export async function runRepositoryGuide(project: string, snapshot: string, id: string, grantId: string, resume = false) {
  if (jobs.has(id)) return;
  if(closingWorkspace){patchTurn(project,snapshot,id,{status:'interrupted',error:closingMessage});return;}
  const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === id);
  if (!turn?.guideId) return;
  const job = {cancelled:false,runId:null as string|null};jobs.set(id,job);
  const release = await enterQuestionQueue();
  try {
    if (job.cancelled) return;
    if(closingWorkspace)throw new Error(closingMessage);
    patchTurn(project,snapshot,id,{status:'working',error:undefined});
    await flushWorkspaceConversation(project,snapshot);
    if(closingWorkspace)throw new Error(closingMessage);
    await ensureCodexConnection(true);
    if (job.cancelled) return;
    if(closingWorkspace)throw new Error(closingMessage);
    const selection = readCodexSelection();
    if (selection.modelId !== 'gpt-6-luna' || selection.effort !== 'max') throw new Error('Choose GPT-6 Luna with max reasoning in the connection settings.');
    job.runId = turn.guideId;
    const guide = await invoke<RepositoryGuide>(resume ? 'resume_repository_guide' : 'start_repository_guide', resume
      ? {id:turn.guideId,projectId:project,snapshotId:snapshot,consent:true}
      : {input:{id:turn.guideId,projectId:project,snapshotId:snapshot,grantId,question:turn.question,consent:true}});
    await reconcileGuide(project,snapshot,guide);
  } catch (failure) {
    try { await restoreRepositoryGuides(project,snapshot); } catch { /* Preserve saved cards and the original error. */ }
    const message=errorMessageOr(failure,'The guide could not be completed. Saved sections remain available.');
    patchTurn(project,snapshot,id,{status:job.cancelled?'stopped':message===closingMessage?'interrupted':'partial',error:message});
  } finally { release();jobs.delete(id); }
}
/** Module-owned operation: changing tab, file, repository or opening settings does not cancel it. */
export async function runQuestion(project: string, snapshot: string, id: string, grantId: string): Promise<ChapterRevision | null> {
  if (jobs.has(id)) return null;
  if(closingWorkspace){patchTurn(project,snapshot,id,{status:'interrupted',error:closingMessage});return null;}
  const owner = getWorkspace(project,snapshot).explorations.find((e) => e.turns.some((t) => t.id === id));
  const turn = owner?.turns.find((t) => t.id === id);
  if (!turn || !owner) return null;
  if ((turn.runIds?.length ?? 0) >= 32) { patchTurn(project,snapshot,id,{error:'This question has reached its retry limit. Ask a follow-up to continue.'}); return null; }
  const job = {cancelled:false,runId:null as string | null}; jobs.set(id,job);
  const check = () => {if (job.cancelled)throw new Error('Stopped');if(closingWorkspace)throw new Error(closingMessage);};
  patchTurn(project,snapshot,id,{status:'queued',error:undefined});
  const release = await enterQuestionQueue();
  let unlisten: (() => void) | undefined;
  let adoption = Promise.resolve();
  try {
    check(); patchTurn(project,snapshot,id,{status:'working',error:undefined});
    await ensureCodexConnection(true); check();
    const selection = readCodexSelection();
    if (selection.modelId !== 'gpt-6-luna' || selection.effort !== 'max') throw new Error('Choose GPT-6 Luna with max reasoning in the connection settings.');
    const runId = crypto.randomUUID(); job.runId = runId;
    patchTurn(project,snapshot,id,{runIds:[...(turn.runIds ?? []),runId]});
    await flushWorkspaceConversation(project,snapshot);check();
    // Subscribe before dispatch. Events carry identities only; the native save
    // command verifies the full checkpoint before any content is rendered.
    unlisten = await listen<PublishedSection & {projectId:string;snapshotId:string}>('workspace-answer-section',({payload}) => {
      if (payload.runId !== runId || payload.projectId !== project || payload.snapshotId !== snapshot) return;
      adoption = adoption.then(async () => { await adoptSection(project,snapshot,id,payload); }).catch(() => {
        // Final reconciliation retries local adoption after a lost event/reply.
        // Existing cards stay visible throughout; inference is never replayed.
      });
    });
    check();
    const summary = await invoke<AnswerSummary>('generate_progressive_workspace_answer',{input:{
      runId,projectId:project,snapshotId:snapshot,grantId,
      question:learningQuestion(getWorkspace(project,snapshot),owner.id,id,1200),
      references:turn.context.map((c) => ({path:c.path,sha256:c.sha256 ?? null,from:c.from ?? null,to:c.to ?? null})),...selection,
    }});
    await adoption;
    const revision = await reconcileAnswer(project,snapshot,id,summary);
    return revision;
  } catch (failure) {
    await adoption;
    let recovered = false;
    if (job.runId) {
      try {
        const summary = await invoke<AnswerSummary>('load_workspace_answer',{projectId:project,snapshotId:snapshot,runId:job.runId});
        await reconcileAnswer(project,snapshot,id,summary); recovered = true;
      } catch { /* Preserve existing cards even if local inspection fails. */ }
    }
    if (!recovered) {
      const message = errorMessageOr(failure,'This question could not be completed. Saved sections remain available.');
      if (/permission|allow.*explor|snapshot or Codex account changed/i.test(message)) updateWorkspace(project,snapshot,(s) => ({...s,grantId:undefined}));
      const saved = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.id === id)?.publishedSections;
      patchTurn(project,snapshot,id,{status:job.cancelled ? 'stopped' : message===closingMessage ? 'interrupted' : saved?.length ? 'partial' : 'failed',error:job.cancelled ? undefined : message});
    }
    return null;
  } finally { unlisten?.(); release(); jobs.delete(id); }
}
