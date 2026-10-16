import { useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { buildCodexChapterReview, cancelCodexChapterRequest, discardProviderRequestReview, getSnapshotIndexProgress, listChapterRevisions, loadSnapshotIndex, loadSnapshotManifest, saveGeneratedChapterDraft, sendCodexChapterRequest, startSnapshotIndexing, type ChapterRevision } from '../lib/commands';
import { ensureCodexConnection, readCodexSelection } from '../lib/connectionStore';
import { discardSourceDiscovery, prepareSourceDiscovery, sendSourceDiscovery } from '../lib/sourceDiscovery';
import { errorMessageOr } from '../lib/errorMessage';
import { addTurn, ancestors, parseWorkspace, type Context, type Turn, type View, type WorkspaceState } from './model';

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
  const id = key(project, snapshot); const next = update(getWorkspace(project, snapshot)); states.set(id, next);
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
/** Module-owned operation: changing tab, file, repository or opening settings does not cancel it. */
export async function runQuestion(project: string, snapshot: string, id: string, grantId: string): Promise<ChapterRevision | null> {
  if (jobs.has(id)) return null;
  const owner = getWorkspace(project,snapshot).explorations.find((e) => e.turns.some((t) => t.id === id));
  const turn = owner?.turns.find((t) => t.id === id);
  if (!turn || !owner) return null;
  const job = { cancelled: false, runId: null as string | null }; jobs.set(id, job);
  let discoveryId: string | null = null; let reviewId: string | null = null;
  const check = () => { if (job.cancelled) throw new Error('Stopped'); };
  patchTurn(project,snapshot,id,{ status:'working', error: undefined });
  try {
    await ensureCodexConnection(); check();
    const selection = readCodexSelection();
    if (selection.modelId !== 'gpt-6-luna' || selection.effort !== 'max') throw new Error('Choose GPT-6 Luna with max reasoning in the connection settings.');
    // Reuse the existing bounded immutable symbol index; this is all local.
    if (!await loadSnapshotIndex(project,snapshot)) {
      check(); const indexJob = await startSnapshotIndexing(project,snapshot);
      for (;;) {
        check(); const progress = await getSnapshotIndexProgress(indexJob.jobId);
        if (!progress || progress.status !== 'running') break;
        await new Promise((resolve) => setTimeout(resolve,200));
      }
    }
    check();
    const contextQuestion = ancestors(getWorkspace(project,snapshot),owner.id).slice(1).map((e) => e.title).concat(turn.question).join(' → ').slice(-300);
    const discovery = await prepareSourceDiscovery({ projectId:project, snapshotId:snapshot, question:contextQuestion.length >= 8 ? contextQuestion : `Explain ${contextQuestion}`, targetPath:turn.context[0]?.path, ...selection, workspaceGrant:grantId });
    discoveryId = discovery.reviewId; check(); job.runId = discovery.reviewId;
    const result = await sendSourceDiscovery(discovery); check(); job.runId = null;
    const manifest = await loadSnapshotManifest(project,snapshot); check();
    const sizes = new Map(manifest.files.map((f) => [f.path,f.sizeBytes]));
    const explicit = [...new Set(turn.context.map((c) => c.path))];
    let bytes = explicit.reduce((n,p) => n + (sizes.get(p) ?? 250001),0);
    if (explicit.length > 12 || bytes > 250000) throw new Error('The attached code is too large for one question. Remove an attachment or ask about a smaller file.');
    const paths = [...explicit];
    for (const f of result.selectedFiles) { const size=sizes.get(f.path); if (!paths.includes(f.path) && size !== undefined && paths.length < 12 && bytes+size <= 250000) {paths.push(f.path);bytes+=size;} }
    if (!paths.length) throw new Error(result.limitation || 'There is not enough captured code to answer this question. Try a more specific question.');
    const focus = turn.context.filter((c) => c.from && c.to && c.sha256).map((c) => ({ path:c.path, sourceSha256:c.sha256!, startLine:c.from!, endLine:c.to!, startByte:null, endByteExclusive:null }));
    const reviewResult = await buildCodexChapterReview({ projectId:project, snapshotId:snapshot,
      targetPath:turn.context[0]?.path ?? null, selectedPaths:paths, ...selection, approvalPolicy:'never', workspaceGrant:grantId,
      question:ancestors(getWorkspace(project,snapshot),owner.id).slice(1).map((e) => e.title).concat(turn.question).join(' → ').slice(-1200), focus });
    if (reviewResult.status === 'error') throw new Error(reviewResult.error.message || 'The explanation request could not be prepared.');
    const review = reviewResult.data; reviewId = review.reviewId; check(); job.runId = review.reviewId;
    const generated = await sendCodexChapterRequest(review.reviewId,review.fingerprint,true); check(); job.runId = null;
    if (generated.status === 'error') throw new Error(generated.error.message || 'The explanation did not pass its source checks. Try a smaller question.');
    const preview = generated.data;
    patchTurn(project,snapshot,id,{ generationId:preview.generationId });
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
    jobs.delete(id);
    if (discoveryId) void discardSourceDiscovery(discoveryId).catch(() => {});
    if (reviewId) void discardProviderRequestReview(reviewId).catch(() => {});
  }
}
