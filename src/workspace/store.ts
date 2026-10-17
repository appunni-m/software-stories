import { createQuestionQueue } from './queue';
import { useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { cancelCodexChapterRequest, getSnapshotIndexProgress, loadSnapshotIndex, startSnapshotIndexing, type ChapterRevision, type SnapshotIndex } from '../lib/commands';
import { ensureCodexConnection, readCodexSelection } from '../lib/connectionStore';
import { errorMessageOr } from '../lib/errorMessage';
import { addTurn, cleanAnswerNotes, learningQuestion, mergePublishedSections, parseWorkspace, type AnswerSummary, type PublishedSection, type Context, type Turn, type View, type WorkspaceState } from './model';

const indexLoads = new Map<string, Promise<SnapshotIndex>>();
/** One local index build per snapshot, shared by reading and questions. No model call. */
export function ensureWorkspaceIndex(project: string, snapshot: string): Promise<SnapshotIndex> {
  const id = `${project}:${snapshot}`;
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
const listeners = new Set<() => void>();
const jobs = new Map<string, { cancelled: boolean; runId: string | null }>();
const key = (project: string, snapshot: string) => `code-atlas:workspace:v1:${project}:${snapshot}`;
export function getWorkspace(project: string, snapshot: string): WorkspaceState {
  const id = key(project, snapshot);
  if (!states.has(id)) { let raw = null; try { raw = localStorage.getItem(id); } catch { /* Storage failure never sends or loses native revisions. */ } states.set(id, parseWorkspace(raw, project, snapshot)); }
  return states.get(id)!;
}
export function updateWorkspace(project: string, snapshot: string, update: (state: WorkspaceState) => WorkspaceState) {
  const id = key(project, snapshot); const previous = getWorkspace(project, snapshot); const next = update(previous);
  if (next === previous) return;
  states.set(id, next);
  try { localStorage.setItem(id, JSON.stringify(next)); } catch { window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice', { detail: { title: 'Conversation could not be saved', message: 'Local workspace storage is full. Generated explanations remain in the native library.' } })); }
  listeners.forEach((notify) => notify());
}
export function useWorkspace(project: string, snapshot: string) {
  return useSyncExternalStore((notify) => { listeners.add(notify); return () => { listeners.delete(notify); }; }, () => getWorkspace(project,snapshot));
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
const restorations = new Map<string, Promise<void>>();
/** A reloaded webview can observe a still-owned native worker. Reading its
 * events does not dispatch a question; Stop uses the persisted native run ID. */
export async function watchWorkspaceAnswers(project: string, snapshot: string): Promise<() => void> {
  let closed = false; let pending = Promise.resolve();
  const refresh = (scope: {projectId:string;snapshotId:string;runId:string}) => {
    if (closed || scope.projectId !== project || scope.snapshotId !== snapshot) return;
    const turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.runIds?.includes(scope.runId));
    if (turn && jobs.has(turn.id)) return;
    pending = pending.then(async () => { if (!closed) await restoreWorkspaceAnswers(project,snapshot); }).catch(() => {});
  };
  const section = await listen<Pick<AnswerSummary,'projectId'|'snapshotId'|'runId'>>('workspace-answer-section',({payload}) => refresh(payload));
  try {
    const finished = await listen<AnswerSummary>('workspace-answer-finished',({payload}) => refresh(payload));
    return () => {closed=true;section();finished();};
  } catch (error) { section(); throw error; }
}
/** Restart recovery is local reading only. It never reconnects or replays AI. */
export function restoreWorkspaceAnswers(project: string, snapshot: string): Promise<void> {
  const scope = key(project,snapshot);
  const existing = restorations.get(scope); if (existing) return existing;
  const pending = (async () => {
    const saved = await invoke<AnswerSummary[]>('list_workspace_answers',{projectId:project,snapshotId:snapshot});
    for (const entry of saved) {
      let turn = getWorkspace(project,snapshot).explorations.flatMap((e) => e.turns).find((t) => t.runIds?.includes(entry.runId));
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
  })().finally(() => restorations.delete(scope));
  restorations.set(scope,pending); return pending;
}
/** Explicitly remove a terminal investigation receipt after its drafts have
 * been saved. Questions, source links and chapter revisions remain readable. */
export async function removeAnswerRecord(project: string, snapshot: string, runId: string) {
  await invoke('remove_workspace_answer_record',{projectId:project,snapshotId:snapshot,runId});
}
/** Module-owned operation: changing tab, file, repository or opening settings does not cancel it. */
export async function runQuestion(project: string, snapshot: string, id: string, grantId: string): Promise<ChapterRevision | null> {
  if (jobs.has(id)) return null;
  const owner = getWorkspace(project,snapshot).explorations.find((e) => e.turns.some((t) => t.id === id));
  const turn = owner?.turns.find((t) => t.id === id);
  if (!turn || !owner) return null;
  if ((turn.runIds?.length ?? 0) >= 32) { patchTurn(project,snapshot,id,{error:'This question has reached its retry limit. Ask a follow-up to continue.'}); return null; }
  const job = {cancelled:false,runId:null as string | null}; jobs.set(id,job);
  const check = () => {if (job.cancelled) throw new Error('Stopped');};
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
    window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{
      title:summary.outcome === 'complete' ? 'Explanation ready' : 'Saved explanation available',
      message:turn.question,projectId:project,snapshotId:snapshot,explorationId:owner.id,
    }}));
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
      patchTurn(project,snapshot,id,{status:job.cancelled ? 'stopped' : saved?.length ? 'partial' : 'failed',error:job.cancelled ? undefined : message});
    }
    return null;
  } finally { unlisten?.(); release(); jobs.delete(id); }
}
