import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, LoaderCircle, X } from 'lucide-react';
import {
  cancelCodexChapterRequest, discardGeneratedChapterDraft, discardProviderRequestReview,
  listChapterRevisions, saveGeneratedChapterDraft, sendCodexChapterRequest,
  type ProviderCommandResult,
  type ChapterRevision,
} from './lib/commands';
import { ensureCodexConnection, readCodexSelection } from './lib/connectionStore';
import { buildCodexPlannedChapterReview, matchesPlannedChapterReview, preparePlannedChapterScope } from './lib/plannedChapters';
import { chapterForStep, runChapterBatch, type ReviewedBatchStep } from './lib/chapterBatch';
import type { RepositoryPlanRevisionDetail } from './lib/repositoryPlans';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { errorMessageOr } from './lib/errorMessage';

function value<T>(result: ProviderCommandResult<T>): T {
  if (result.status === 'error') throw Error(result.error.message);
  return result.data;
}

/** One reviewed batch, sequential account usage, independently saved drafts. */
export function ChapterBatchDialog({ projectId, plan, onClose, onReviewChapter }: {
  projectId: string; plan: RepositoryPlanRevisionDetail; onClose: () => void;
  onReviewChapter: (chapter: ChapterRevision) => void;
}) {
  const [steps, setSteps] = useState<ReviewedBatchStep[]>([]);
  const [busy, setBusy] = useState(true);
  const [running, setRunning] = useState(false);
  const [done, setDone] = useState(false);
  const [progress, setProgress] = useState('Preparing chapter requests locally…');
  const [states, setStates] = useState<Record<string, string>>({});
  const [ready, setReady] = useState<Record<string, ChapterRevision>>({});
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLElement>(null);
  const mounted = useRef(false);
  const cancelled = useRef(false);
  const active = useRef<string | null>(null);
  const started = useRef(false);
  const selection = useRef(readCodexSelection());
  useModalFocusTrap(dialog);
  useEffect(() => {
    mounted.current = true;
    let current = true;
    const previous = document.activeElement as HTMLElement | null;
    dialog.current?.focus();
    const key = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); onClose(); } };
    document.addEventListener('keydown', key);
    void (async () => {
      const connection = await ensureCodexConnection();
      if (!current) return;
      if (!connection.status.authenticated) throw Error('Connect Codex in the header first.');
      const saved = await listChapterRevisions(projectId, plan.revision.snapshotId);
      const prepared: ReviewedBatchStep[] = [];
      for (const step of plan.plan.preview.plan.steps) {
        if (!current) return;
        const scope = await preparePlannedChapterScope(projectId, { revisionId: plan.revision.revisionId,
          recordSha256: plan.plan.recordSha256, stepId: step.stepId });
        if (!current) return;
        const existing = chapterForStep(saved, scope, projectId);
        if (existing) {
          setStates((all) => ({ ...all, [step.stepId]: 'Already saved' }));
          setReady((all) => ({ ...all, [step.stepId]: existing }));
          continue;
        }
        const review = value(await buildCodexPlannedChapterReview({ projectId, target: scope.target, ...selection.current, approvalPolicy: 'never' }));
        try {
          if (!current) return;
          if (!matchesPlannedChapterReview(review, scope) || review.projectId !== projectId) throw Error('A chapter request no longer matches this accepted route. Reopen the plan.');
          prepared.push({ scope, review });
        } finally { await discardProviderRequestReview(review.reviewId).catch(() => {}); }
      }
      if (current) { setSteps(prepared); setProgress(prepared.length ? 'Ready to build the remaining chapters.' : 'Every step already has a saved chapter.'); }
    })().catch((failure) => { if (current) setError(errorMessageOr(failure, 'The chapter batch could not be prepared.')); })
      .finally(() => { if (current) setBusy(false); });
    return () => {
      current = false; mounted.current = false; cancelled.current = true;
      document.removeEventListener('keydown', key);
      if (active.current) void cancelCodexChapterRequest(active.current).catch(() => {});
      if (previous?.isConnected) previous.focus();
    };
  }, [projectId, plan, onClose]);

  async function build() {
    if (busy || running || done || !steps.length || started.current) return;
    started.current = true;
    cancelled.current = false; setBusy(true); setRunning(true); setError(null);
    try {
      await runChapterBatch(steps, {
        build: async (scope) => value(await buildCodexPlannedChapterReview({ projectId, target: scope.target, ...selection.current, approvalPolicy: 'never' })),
        matches: matchesPlannedChapterReview,
        send: async (review) => value(await sendCodexChapterRequest(review.reviewId, review.fingerprint, true)),
        save: saveGeneratedChapterDraft,
        list: () => listChapterRevisions(projectId, plan.revision.snapshotId),
        discardReview: discardProviderRequestReview, discardDraft: discardGeneratedChapterDraft,
      }, {
        cancelled: () => cancelled.current,
        active: (id) => { active.current = id; },
        progress: (index, phase, revision) => {
          if (revision) {
            window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision } }));
            window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
          }
          if (!mounted.current) return;
          if (revision) setReady((all) => ({ ...all, [steps[index].scope.target.stepId]: revision }));
          setStates((all) => ({ ...all, [steps[index].scope.target.stepId]: phase === 'generating' ? 'Generating…' : phase === 'saved' ? 'Draft saved' : 'Already saved' }));
          setProgress(`Chapter ${index + 1} of ${steps.length} · ${phase === 'generating' ? 'Codex is explaining the code' : 'Saved locally'}`);
        },
      });
      if (mounted.current) { setDone(true); setProgress(cancelled.current ? 'Stopped. Completed drafts are saved.' : 'Chapters are ready. Choose a saved chapter below to review it.'); }
    } catch (failure) {
      if (mounted.current) { setError(errorMessageOr(failure, 'Chapter generation stopped. Saved drafts are preserved.')); setDone(true); }
    } finally { if (mounted.current) { setBusy(false); setRunning(false); } }
  }
  const sourcePaths = [...new Set(steps.flatMap((step) => step.review.selectedFiles.map((file) => file.path)))];
  return <div className="provider-overlay"><section className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="batch-title" ref={dialog} tabIndex={-1}>
    <header className="provider-dialog-header"><div><h2 id="batch-title">Build the route’s chapters</h2><p>{plan.revision.title}</p></div><button className="icon-button" aria-label="Close chapter batch" onClick={onClose}><X size={18} /></button></header>
    <div className="provider-dialog-content">
      <p className="discovery-progress" role="status">{busy ? <LoaderCircle size={18} /> : done ? <CheckCircle2 size={18} /> : null}{progress}</p>
      <ol className="chapter-batch-list">{plan.plan.preview.plan.steps.map((step) => <li key={step.stepId}><span>{step.question}</span><small>{states[step.stepId] ?? 'Ready'}</small>
        {!busy && ready[step.stepId] && <button className="secondary-button" onClick={() => { onClose(); onReviewChapter(ready[step.stepId]); }}>Review chapter</button>}
      </li>)}</ol>
      {!running && !done && steps.length > 0 && <>
        <p>Codex will generate {steps.length} chapters in order using <strong>{selection.current.modelId} · {selection.current.effort}</strong> and your header connection. Each chapter is saved as an unaccepted draft. Existing step chapters are skipped.</p>
        <p className="provider-small-note">Starting shares the reviewed code and prompts with OpenAI through Codex. Account quota applies for each chapter. Closing stops the queue and cancels an active run. A save already in progress can finish.</p>
        <details><summary>Review code and exact requests · {sourcePaths.length} files</summary>{steps.map(({ scope, review }) => <section key={scope.target.stepId}><h3>{scope.question}</h3>
          <p>{review.selectedFiles.map((file) => file.path).join(' · ')}</p>
          <details><summary>Prompt and settings</summary><pre className="discovery-metadata">{JSON.stringify({ systemInstruction: review.systemInstruction, userPrompt: review.userPrompt, generationConfig: review.generationConfig }, null, 2)}</pre></details>
          {review.selectedFiles.map((file) => <details key={file.path}><summary>{file.path} · {file.sizeBytes} bytes</summary><pre className="discovery-metadata">{file.contents}</pre></details>)}
        </section>)}</details>
      </>}
      {error && <p role="alert" className="provider-error-text">{error} Completed drafts are preserved; reopen this action to review only the remaining steps. Check Edit and recovery notices before retrying a failed step.</p>}
    </div><footer className="provider-dialog-footer"><button className="secondary-button" onClick={onClose}>{running ? 'Stop and close' : 'Close'}</button>
      {!done && <button className="primary-button" disabled={busy || !steps.length || !!error} onClick={() => { void build(); }}>Build {steps.length || ''} chapters</button>}
    </footer>
  </section></div>;
}
