import { useEffect, useRef, useState } from 'react';
import { Check, FileCode2, LoaderCircle, Play, ShieldCheck, Square, X } from 'lucide-react';
import { AppButton } from './design/AppButton';
import { InlineCodeText } from './InlineCodeText';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import {
  cancelCodexChapterRequest, discardGeneratedChapterDraft, discardProviderRequestReview,
  listChapterRevisions, listPendingGeneratedChapterPreviews, saveGeneratedChapterDraft,
  sendCodexChapterRequest, type ChapterRevision,
  type PendingChapterPreviewSummary, type ProviderRequestReview,
} from './lib/commands';
import { ensureCodexConnection, readCodexSelection } from './lib/connectionStore';
import { codexSelectionIsSupported } from './lib/codexConnection';
import { providerFailureMessage } from './lib/errorMessage';
import { readGenerationBackend } from './lib/generationPreferences';
import {
  acquirePlannedChapterBatchRun,
  exactSavedPlannedChapter, matchingPlannedChapterPendingPreview, matchesPlannedChapterPreview,
  plannedChapterBatchBoundary,
  sameReviewedPlannedChapterRequest,
} from './lib/plannedChapterBatch';
import {
  buildCodexPlannedChapterReview, matchesPlannedChapterReview,
  preparePlannedChapterBatch, type PlannedChapterBatchTarget, type PlannedChapterScope,
} from './lib/plannedChapters';

type Phase = 'preparing' | 'review' | 'running' | 'complete' | 'stopped' | 'failed' | 'blocked';
type BatchEntry = {
  scope: PlannedChapterScope;
  review?: ProviderRequestReview;
  saved?: ChapterRevision;
  status: 'ready' | 'saved' | 'accepted' | 'running' | 'failed';
};

function messageOf(error: unknown, fallback: string): string {
  return error instanceof Error && error.message.trim() ? error.message : fallback;
}

export function RepositoryPlanBatchDialog({ projectId, target, native, onClose, onReviewDraft, onOpenPendingPreview }: {
  projectId: string;
  target: PlannedChapterBatchTarget;
  native: boolean;
  onClose: () => void;
  onReviewDraft: (revision: ChapterRevision) => void;
  onOpenPendingPreview: (generationId: string) => void;
}) {
  const dialog = useRef<HTMLElement>(null);
  const alive = useRef(true);
  const cancelled = useRef(false);
  const stopAfterCurrentSave = useRef(false);
  const buildRunStarted = useRef(false);
  const activeReviewId = useRef<string | null>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const [phase, setPhase] = useState<Phase>('preparing');
  const [entries, setEntries] = useState<BatchEntry[]>([]);
  const [selection, setSelection] = useState<{ modelId: string; effort: string } | null>(null);
  const [accountEmail, setAccountEmail] = useState<string | null>(null);
  const [pendingPreview, setPendingPreview] = useState<PendingChapterPreviewSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('Reviewing the accepted route locally…');
  useModalFocusTrap(dialog);

  useEffect(() => {
    returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.focus();
    return () => { if (returnFocus.current?.isConnected) returnFocus.current.focus(); };
  }, []);

  const close = () => {
    if (phase === 'running' || phase === 'preparing') cancelled.current = true;
    const active = activeReviewId.current;
    if (active) void cancelCodexChapterRequest(active).catch(() => {});
    onClose();
  };

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      cancelled.current = true;
      const active = activeReviewId.current;
      if (active) void cancelCodexChapterRequest(active).catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (!native) {
      setPhase('blocked');
      setError('Building chapters is available in the Code Atlas Mac app.');
      return;
    }
    let disposed = false;
    const prepare = async () => {
      const openedReviews: string[] = [];
      try {
        if (readGenerationBackend() !== 'codex') {
          throw new Error('This batch uses the saved Codex connection. Switch to Codex once in Connections, then reopen the route.');
        }
        setStatus('Checking your saved Codex connection…');
        const connection = await ensureCodexConnection(true);
        const currentSelection = readCodexSelection();
        if (!connection.status.authenticated || !connection.status.runtimeRunning || connection.status.activeRunId) {
          throw new Error(connection.status.activeRunId
            ? 'Codex is handling another request. Wait for it to finish, then reopen this route.'
            : 'Connect Codex in Connections before building these chapters.');
        }
        if (!codexSelectionIsSupported(connection.models, currentSelection)) {
          throw new Error('The saved Codex model or reasoning level is unavailable. Choose a supported setting in Connections.');
        }
        if (disposed || cancelled.current) return;
        setSelection(currentSelection);
        setAccountEmail(connection.status.accountEmail);
        setStatus('Checking saved chapters and rebuilding the accepted route…');
        const batch = await preparePlannedChapterBatch(projectId, target);
        const [revisions, pending] = await Promise.all([
          listChapterRevisions(projectId, batch.snapshotId),
          listPendingGeneratedChapterPreviews(),
        ]);
        if (disposed || cancelled.current) return;
        if (batch.projectId !== projectId || batch.revisionId !== target.revisionId
          || batch.recordSha256 !== target.recordSha256 || batch.schemaVersion !== 1
          || batch.steps.length < 1 || batch.steps.length > 24
          || batch.steps.some((step) => step.target.revisionId !== target.revisionId
            || step.target.recordSha256 !== target.recordSha256 || step.snapshotId !== batch.snapshotId)) {
          throw new Error('The accepted route did not match its complete chapter batch. Refresh saved routes before continuing.');
        }
        const unresolved = matchingPlannedChapterPendingPreview(pending, projectId, batch.snapshotId, batch.steps);
        if (unresolved) {
          setPendingPreview(unresolved);
          setEntries(batch.steps.map((scope) => {
            const saved = revisions.filter((revision) => exactSavedPlannedChapter(revision, scope))
              .sort((a, b) => b.createdAtUnixMs - a.createdAtUnixMs)[0];
            return { scope, ...(saved ? { saved, status: saved.lifecycle === 'accepted' ? 'accepted' as const : 'saved' as const } : { status: 'ready' as const }) };
          }));
          setPhase('blocked');
          setError('A chapter result from this route needs review first. Open that saved preview, then return here; no step was sent again.');
          return;
        }

        const prepared: BatchEntry[] = [];
        for (const scope of batch.steps) {
          if (disposed || cancelled.current) return;
          const saved = revisions.filter((revision) => exactSavedPlannedChapter(revision, scope))
            .sort((a, b) => b.createdAtUnixMs - a.createdAtUnixMs)[0];
          if (saved) {
            prepared.push({ scope, saved, status: saved.lifecycle === 'accepted' ? 'accepted' : 'saved' });
            continue;
          }
          setStatus(`Reviewing step ${prepared.length + 1} of ${batch.steps.length} locally…`);
          let reviewId: string | null = null;
          try {
            const result = await buildCodexPlannedChapterReview({
              projectId, target: scope.target, modelId: currentSelection.modelId,
              effort: currentSelection.effort, approvalPolicy: 'never',
            });
            if (result.status === 'error') throw new Error(providerFailureMessage(result.error));
            reviewId = result.data.reviewId;
            if (disposed || cancelled.current) return;
            if (result.data.providerId !== 'codex' || result.data.modelId !== currentSelection.modelId
              || result.data.generationConfig.effort !== currentSelection.effort
              || result.data.generationConfig.approvalPolicy !== 'never'
              || !matchesPlannedChapterReview(result.data, scope)) {
              throw new Error('A step did not match the accepted route, source, or saved Codex settings. No chapter was sent.');
            }
            prepared.push({ scope, review: result.data, status: 'ready' });
          } finally {
            if (reviewId) {
              openedReviews.push(reviewId);
              try {
                await discardProviderRequestReview(reviewId);
                openedReviews.splice(openedReviews.indexOf(reviewId), 1);
              } catch {
                throw new Error('A temporary request review could not be cleared. Close and reopen the route before building.');
              }
            }
          }
          if (disposed || cancelled.current) return;
          if (alive.current) setEntries(prepared.slice());
        }
        if (disposed || cancelled.current) return;
        setEntries(prepared);
        setPhase('review');
        setStatus(prepared.some((entry) => entry.status === 'ready')
          ? 'Review every remaining request before building the chapters.'
          : 'Every route step already has a saved draft or accepted chapter.');
      } catch (failure) {
        if (!disposed && !cancelled.current && alive.current) {
          setPhase('failed');
          setError(messageOf(failure, 'The chapter batch could not be prepared. No provider request was sent.'));
        }
      } finally {
        for (const reviewId of openedReviews) await discardProviderRequestReview(reviewId).catch(() => {});
      }
    };
    void prepare();
    const connectionChanged = () => {
      if (readGenerationBackend() === 'codex') return;
      cancelled.current = true;
      const active = activeReviewId.current;
      if (active) void cancelCodexChapterRequest(active).catch(() => {});
      if (alive.current) {
        setPhase('stopped');
        setError('The selected connection changed. No further chapter was sent. Open the route again to review the remaining steps.');
      }
    };
    window.addEventListener('code-atlas:connection-changed', connectionChanged);
    return () => {
      disposed = true;
      window.removeEventListener('code-atlas:connection-changed', connectionChanged);
    };
  }, [native, projectId, target]);

  const build = async () => {
    if (phase !== 'review' || !selection || cancelled.current) return;
    if (!acquirePlannedChapterBatchRun(buildRunStarted, true)) return;
    setPhase('running');
    setError(null);
    cancelled.current = false;
    stopAfterCurrentSave.current = false;
    let currentIndex = -1;
    try {
      for (let index = 0; index < entries.length; index += 1) {
        if (!plannedChapterBatchBoundary({ alive: alive.current, cancelled: cancelled.current, stopAfterCurrentSave: stopAfterCurrentSave.current }).continueBatch) break;
        const entry = entries[index];
        if (entry.status === 'saved' || entry.status === 'accepted') continue;
        if (!entry.review) throw new Error('A step is missing its reviewed request. Reopen the route before building.');
        currentIndex = index;
        setEntries((current) => current.map((item, itemIndex) => itemIndex === index ? { ...item, status: 'running' } : item));
        setStatus(`Building chapter ${index + 1} of ${entries.length}…`);

        if (readGenerationBackend() !== 'codex') throw new Error('The selected AI connection changed. No further chapter was sent.');
        const connection = await ensureCodexConnection(true);
        const currentSelection = readCodexSelection();
        if (!connection.status.authenticated || !connection.status.runtimeRunning || connection.status.activeRunId
          || connection.status.accountEmail !== accountEmail
          || !codexSelectionIsSupported(connection.models, currentSelection)
          || currentSelection.modelId !== selection.modelId || currentSelection.effort !== selection.effort) {
          throw new Error('The Codex account, runtime, model, or reasoning setting changed. No further chapter was sent. Reopen the route to review the remaining steps.');
        }

        const [latestRevisions, pending] = await Promise.all([
          listChapterRevisions(projectId, entry.scope.snapshotId),
          listPendingGeneratedChapterPreviews(),
        ]);
        if (latestRevisions.some((revision) => exactSavedPlannedChapter(revision, entry.scope))) {
          const saved = latestRevisions.filter((revision) => exactSavedPlannedChapter(revision, entry.scope))
            .sort((a, b) => b.createdAtUnixMs - a.createdAtUnixMs)[0];
          setEntries((current) => current.map((item, itemIndex) => itemIndex === index
            ? { ...item, saved, status: saved.lifecycle === 'accepted' ? 'accepted' : 'saved' } : item));
          continue;
        }
        const unresolved = matchingPlannedChapterPendingPreview(pending, projectId, entry.scope.snapshotId, [entry.scope]);
        if (unresolved) {
          setPendingPreview(unresolved);
          throw new Error('A saved result for this step needs review before the batch can continue. Open that preview, then reopen the route.');
        }

        const rebuilt = await buildCodexPlannedChapterReview({
          projectId, target: entry.scope.target, modelId: selection.modelId,
          effort: selection.effort, approvalPolicy: 'never',
        });
        if (rebuilt.status === 'error') throw new Error(providerFailureMessage(rebuilt.error));
        const review = rebuilt.data;
        if (cancelled.current || !alive.current) {
          await discardProviderRequestReview(review.reviewId).catch(() => {});
          break;
        }
        if (!sameReviewedPlannedChapterRequest(entry.review, review, entry.scope, selection)
          || Date.now() >= review.expiresAtUnixMs) {
          await discardProviderRequestReview(review.reviewId).catch(() => {});
          throw new Error('This step changed after the batch review. No changed request was sent; reopen the route to review it again.');
        }
        activeReviewId.current = review.reviewId;
        const result = await sendCodexChapterRequest(review.reviewId, review.fingerprint, true);
        activeReviewId.current = null;
        if (result.status === 'error') throw new Error(providerFailureMessage(result.error));
        const preview = result.data;
        if (!plannedChapterBatchBoundary({ alive: alive.current, cancelled: cancelled.current, stopAfterCurrentSave: stopAfterCurrentSave.current }).keepCurrent) {
          await discardGeneratedChapterDraft(preview.generationId).catch(() => {});
          break;
        }
        if (!matchesPlannedChapterPreview(preview, review, entry.scope)) {
          await discardGeneratedChapterDraft(preview.generationId).catch(() => {});
          throw new Error('The generated chapter did not match its reviewed route step. It was not saved.');
        }

        setStatus(`Saving chapter ${index + 1} as an unaccepted draft…`);
        let saved: ChapterRevision;
        try {
          saved = await saveGeneratedChapterDraft(preview.generationId);
        } catch {
          const existing = await listChapterRevisions(projectId, entry.scope.snapshotId)
            .then((all) => all.find((revision) => revision.revisionId === preview.revision.revisionId))
            .catch(() => undefined);
          if (!existing || !exactSavedPlannedChapter(existing, entry.scope)) {
            throw new Error('The save could not be confirmed. The result remains in recovery; do not run this step again until it is reviewed.');
          }
          saved = existing;
          await discardGeneratedChapterDraft(preview.generationId).catch(() => {});
        }
        if (saved.revisionId !== preview.revision.revisionId || !exactSavedPlannedChapter(saved, entry.scope)) {
          throw new Error('The saved chapter identity did not match its reviewed step. Stop and inspect the saved draft before continuing.');
        }
        setEntries((current) => current.map((item, itemIndex) => itemIndex === index
          ? { ...item, saved, status: saved.lifecycle === 'accepted' ? 'accepted' : 'saved' } : item));
        window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision: saved } }));
        window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
        currentIndex = -1;
        if (cancelled.current || !alive.current) break;
      }
      if (!alive.current) return;
      if (plannedChapterBatchBoundary({ alive: alive.current, cancelled: cancelled.current, stopAfterCurrentSave: stopAfterCurrentSave.current }).stopped) {
        setPhase('stopped');
        setStatus('Stopped. Saved drafts remain available; no later step was sent.');
      } else {
        setPhase('complete');
        setStatus('Batch finished. Each saved chapter is still an unaccepted draft.');
      }
    } catch (failure) {
      activeReviewId.current = null;
      if (!alive.current) return;
      if (currentIndex >= 0) {
        setEntries((current) => current.map((item, itemIndex) => itemIndex === currentIndex && item.status === 'running'
          ? { ...item, status: 'failed' } : item));
      }
      setPhase(cancelled.current ? 'stopped' : 'failed');
      setError(messageOf(failure, 'The batch stopped. Saved drafts remain available; no step was retried.'));
      setStatus(cancelled.current ? 'Stopped. No later step was sent.' : 'The batch stopped at this step.');
      window.dispatchEvent(new CustomEvent('code-atlas:agent-recovery-changed'));
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    }
  };

  const readyCount = entries.filter((entry) => entry.status === 'ready' || entry.status === 'failed').length;
  const finished = phase === 'complete' || phase === 'stopped' || phase === 'failed' || phase === 'blocked';
  return <div className="provider-overlay repository-plan-batch-overlay">
    <section ref={dialog} className="provider-dialog repository-plan-batch-dialog" role="dialog" aria-modal="true"
      aria-labelledby="repository-plan-batch-title" tabIndex={-1}>
      <header className="provider-dialog-header">
        <div><span className="eyebrow"><FileCode2 size={13}/> ACCEPTED ROUTE</span><h2 id="repository-plan-batch-title">Build route chapters</h2>
          <p>Review each exact request once. Chapters are saved in order as separate drafts.</p></div>
        <AppButton intent="icon" onClick={close} aria-label="Close chapter batch"><X size={17}/></AppButton>
      </header>
      <div className="provider-dialog-content repository-plan-batch-content">
        {selection && <div className="repository-plan-batch-connection"><ShieldCheck size={15}/><span>Using Codex · {selection.modelId}</span></div>}
        <p className="provider-small-note">One request per remaining step. Each request uses only that step’s captured files and the saved Codex settings. Your account quota applies; nothing is sent before you choose Build chapters. Every result stays a draft until you review it.</p>
        {error && <p className="provider-error-text" role="alert">{error}</p>}
        {!error && <p className="provider-status-note" role="status">{phase === 'preparing' ? <LoaderCircle size={14}/> : phase === 'running' ? <LoaderCircle size={14}/> : phase === 'review' ? <Check size={14}/> : null}{status}</p>}
        {pendingPreview && <div className="repository-plan-batch-recovery"><strong>Earlier result needs review</strong><span>{pendingPreview.title}</span><AppButton intent="secondary" onClick={() => onOpenPendingPreview(pendingPreview.generationId)}>Open saved result</AppButton></div>}
        {entries.length > 0 && <ol className="repository-plan-batch-steps">
          {entries.map((entry, index) => <li key={entry.scope.target.stepId} className={`repository-plan-batch-step is-${entry.status}`}>
            <div className="repository-plan-batch-step-heading">
              <span>{entry.status === 'saved' ? <Check size={14}/> : entry.status === 'accepted' ? <Check size={14}/> : entry.status === 'running' ? <LoaderCircle size={14}/> : index + 1}</span>
              <div><strong>Step {index + 1}</strong><small>{entry.status === 'saved' ? 'Draft saved' : entry.status === 'accepted' ? 'Already accepted' : entry.status === 'running' ? 'Building' : entry.status === 'failed' ? 'Stopped here' : 'Ready to build'}</small></div>
              <code>{entry.scope.primaryPath}</code>
            </div>
            <h3><InlineCodeText text={entry.scope.question}/></h3>
            {entry.saved && <AppButton size="1" intent="quiet" onClick={() => onReviewDraft(entry.saved!)}>Review saved chapter</AppButton>}
            {entry.review && <details className="repository-plan-batch-request">
              <summary>Review prompt and {entry.review.selectedFiles.length} captured {entry.review.selectedFiles.length === 1 ? 'file' : 'files'} · {entry.review.sourceBytes.toLocaleString()} bytes</summary>
              <details><summary>System instructions</summary><pre>{entry.review.systemInstruction}</pre></details>
              <details><summary>Question sent to Codex</summary><pre>{entry.review.userPrompt}</pre></details>
              {entry.review.selectedFiles.map((file) => <details key={`${entry.scope.target.stepId}:${file.path}`}>
                <summary><code>{file.path}</code> · {file.sizeBytes.toLocaleString()} bytes</summary>
                <pre>{file.contents}</pre>
              </details>)}
            </details>}
          </li>)}
        </ol>}
      </div>
      <footer className="provider-dialog-footer repository-plan-batch-footer">
        <AppButton intent="secondary" onClick={close}>{phase === 'running' ? 'Stop and close' : 'Close'}</AppButton>
        {phase === 'running' && <AppButton intent="quiet" onClick={() => { stopAfterCurrentSave.current = true; setStatus('Stop requested. The current chapter will be saved before the batch stops.'); }}><Square size={13}/>Stop after current save</AppButton>}
        {phase === 'review' && readyCount > 0 && <AppButton intent="primary" onClick={() => void build()}><Play size={14}/>Build {readyCount} chapters</AppButton>}
        {finished && <AppButton intent="primary" onClick={close}>{phase === 'complete' ? 'Done' : 'Close'}</AppButton>}
      </footer>
    </section>
  </div>;
}
