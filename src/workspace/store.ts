import { createQuestionQueue } from './queue';
import { useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { cancelCodexChapterRequest, getSnapshotIndexProgress, listChapterRevisions, loadSnapshotIndex, saveGeneratedChapterDraft, startSnapshotIndexing, type ChapterDraftPreview, type ChapterRevision, type SnapshotIndex } from '../lib/commands';
import { ensureCodexConnection, readCodexSelection } from '../lib/connectionStore';
import { errorMessageOr } from '../lib/errorMessage';
import { addTurn, cleanAnswerNotes, learningQuestion, parseWorkspace, type Context, type Turn, type View, type WorkspaceState } from './model';

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
  for (const e of getWorkspace(projectId,snapshotId).explorations) for (const t of e.turns) if (jobs.has(t.id)) stopQuestion(projectId,snapshotId,t.id);
  if (grantId) await invoke('revoke_workspace_exploration', { grantId });
  updateWorkspace(projectId,snapshotId,(s) => ({ ...s, grantId: undefined }));
}
export function stopQuestion(project: string, snapshot: string, id: string) {
  const job = jobs.get(id);
  if (job) { job.cancelled = true; if (job.runId) void cancelCodexChapterRequest(job.runId).catch(() => {}); }
  patchTurn(project,snapshot,id,{ status:'stopped', error: undefined });
}
const enterQuestionQueue = createQuestionQueue();
/** Module-owned operation: changing tab, file, repository or opening settings does not cancel it. */
export async function runQuestion(project: string, snapshot: string, id: string, grantId: string): Promise<ChapterRevision | null> {
  if (jobs.has(id)) return null;
  const owner = getWorkspace(project,snapshot).explorations.find((e) => e.turns.some((t) => t.id === id));
  const turn = owner?.turns.find((t) => t.id === id);
  if (!turn || !owner) return null;
  const job = { cancelled: false, runId: null as string | null }; jobs.set(id, job);
  const check = () => { if (job.cancelled) throw new Error('Stopped'); };
  patchTurn(project,snapshot,id,{ status:'queued', error: undefined });
  const release = await enterQuestionQueue();
  try {
    check();
    patchTurn(project,snapshot,id,{ status:'working', error: undefined });
    // A previous queued operation may have cancelled and reaped the native
    // runtime. Observe it after taking the queue, before preparing any review;
    // cached header status is not proof that its process still exists.
    await ensureCodexConnection(true); check();
    const selection = readCodexSelection();
    if (selection.modelId !== 'gpt-6-luna' || selection.effort !== 'max') throw new Error('Choose GPT-6 Luna with max reasoning in the connection settings.');
    const runId = crypto.randomUUID();
    job.runId = runId; check();
    const preview = await invoke<ChapterDraftPreview>('generate_workspace_answer', { input: {
      runId, projectId: project, snapshotId: snapshot, grantId,
      question: learningQuestion(getWorkspace(project,snapshot),owner.id,id,1200),
      references: turn.context.map((c) => ({ path:c.path, sha256:c.sha256 ?? null, from:c.from ?? null, to:c.to ?? null })),
      ...selection,
    } });
    check(); job.runId = null;
    patchTurn(project,snapshot,id,{ generationId:preview.generationId, notes:cleanAnswerNotes(preview.updateNotes) });
    let revision: ChapterRevision;
    try { revision = await saveGeneratedChapterDraft(preview.generationId); }
    catch (failure) {
      // A lost save reply is not permission to duplicate a native revision.
      const saved = await listChapterRevisions(project,snapshot).catch(() => []);
      const existing = saved.find((r) => r.revisionId === preview.revision.revisionId);
      if (!existing) throw failure;
      revision = existing;
    }
    patchTurn(project,snapshot,id,{ status:'ready', revisionId:revision.revisionId, generationId:undefined });
    window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed',{ detail:{ projectId:project } }));
    window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{ detail:{ title:'Explanation ready', message:turn.question,projectId:project,snapshotId:snapshot,explorationId:owner.id } }));
    return revision;
  } catch (failure) {
    if (!job.cancelled) {
      const message=errorMessageOr(failure,'This question could not be answered. Saved work is preserved.');
      if (/permission|allow.*explor|snapshot or Codex account changed/i.test(message)) updateWorkspace(project,snapshot,(s)=>({...s,grantId:undefined}));
      patchTurn(project,snapshot,id,{ status:'failed', error:message });
      window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{title:'Explanation needs attention',message:turn.question,projectId:project,snapshotId:snapshot,explorationId:owner.id}}));
    }
    return null;
  } finally {
    release();
    jobs.delete(id);
  }
}
