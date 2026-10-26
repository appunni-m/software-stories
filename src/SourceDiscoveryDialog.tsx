import { useEffect, useRef, useState } from 'react';
import { LoaderCircle, X } from 'lucide-react';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { ensureCodexConnection, readCodexSelection, useConnectionState } from './lib/connectionStore';
import { prepareSourceDiscovery, sendSourceDiscovery, discardSourceDiscovery, type DiscoveryCoverageGapCategory, type SourceDiscoveryScope, type SourceDiscoveryReview, type SourceDiscoveryResult } from './lib/sourceDiscovery';
import { readSourceDiscoveryResult, saveSourceDiscoveryResult, SOURCE_DISCOVERY_SAVED_EVENT } from './lib/sourceDiscoveryStore';
import { cancelCodexChapterRequest, isNativeRuntime, loadSnapshotIndex, loadSnapshotManifest, startSnapshotIndexing, getSnapshotIndexProgress } from './lib/commands';
import { errorMessageOr } from './lib/errorMessage';
import { CodexChapterDialog } from './CodexChapterDialog';

const coverageGapLabels: Record<DiscoveryCoverageGapCategory, string> = {
  largeFiles: 'large files',
  unreadableSource: 'unreadable source files',
  unsupportedLanguage: 'files without a local parser',
  indexLimit: 'files beyond local index limits',
  partialParse: 'partially mapped files',
  unavailableSource: 'unreadable source files',
  incompleteIndex: 'files missing from the local map',
  unindexedFiles: 'files missing from the local map',
};

export function SourceDiscoveryDialog({ scope, onReady, onClose, persistSuccessfulResult = true, autoDiscover = false }: {
  scope: SourceDiscoveryScope; onReady: (result: SourceDiscoveryResult, review?: SourceDiscoveryReview) => void; onClose: () => void;
  persistSuccessfulResult?: boolean;
  autoDiscover?: boolean;
}) {
  const [review, setReview] = useState<SourceDiscoveryReview | null>(null);
  const [phase, setPhase] = useState(autoDiscover ? 'Preparing your explanation…' : 'Preparing your repository…');
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [savedSelectionNotice, setSavedSelectionNotice] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [needsConnection, setNeedsConnection] = useState(false);
  const connectionState = useConnectionState();
  const dialog = useRef<HTMLElement>(null);
  const active = useRef<string | null>(null);
  const prepared = useRef<string | null>(null);
  const mounted = useRef(false);
  const preparing = useRef(false);
  useModalFocusTrap(dialog);
  useEffect(() => {
    if (needsConnection) return;
    const previous = document.activeElement as HTMLElement | null;
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); onClose(); } };
    document.addEventListener('keydown', escape);
    return () => { document.removeEventListener('keydown', escape); if (previous?.isConnected) previous.focus(); };
  }, [onClose, needsConnection]);
  useEffect(() => {
    mounted.current = true;
    let current = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let finishWait: (() => void) | null = null;
    let pollDelayMs = 200;
    const wait = () => new Promise<void>((resolve) => {
      finishWait = resolve;
      timer = setTimeout(() => {
        timer = undefined;
        finishWait = null;
        resolve();
      }, pollDelayMs);
    });
    dialog.current?.focus();
    setReview(null); setError(null); setSavedSelectionNotice(null); setBusy(true); preparing.current = true;
    setPhase(autoDiscover ? 'Preparing your explanation…' : 'Preparing your repository…');
    void (async () => {
      if (!isNativeRuntime()) throw Error('Open the Mac app to discover code with your saved agent.');
      if (persistSuccessfulResult) {
        try {
          const manifest=await loadSnapshotManifest(scope.projectId,scope.snapshotId);
          if (!current) return;
          const availablePaths=manifest.files.filter((file)=>file.kind==='file'&&!file.secretLike).map((file)=>file.path);
          const saved=readSourceDiscoveryResult(scope,availablePaths);
          if (saved.status === 'available') { onReady(saved.result); return; }
          if (saved.status === 'unavailable') {
            const issue = saved.issue === 'unsupported' ? 'The saved code selection uses an unsupported format.'
              : saved.issue === 'stale-path' ? 'A saved code path is no longer present in this snapshot.'
                : saved.issue === 'oversized' ? 'The saved code selection exceeds its storage limit.'
                  : saved.issue === 'storage' ? 'The saved code selection could not be read from local storage.'
                    : saved.issue === 'unreadable' ? 'The saved code selection is unreadable.'
                      : 'The saved code selection is invalid for this snapshot.';
            setSavedSelectionNotice(`${issue} The stored bytes were left untouched; you can find code again below.`);
          }
        } catch {
          if (current) setSavedSelectionNotice('The saved code selection could not be checked against this snapshot. The stored bytes were left untouched; you can find code again below.');
        }
      }
      const connection = await ensureCodexConnection(retry > 0);
      if (!current) return;
      if (!connection.status.authenticated) { setNeedsConnection(true); return; }
      let indexReady = false;
      try {
        const existing = await loadSnapshotIndex(scope.projectId, scope.snapshotId);
        if (!current) return;
        indexReady = existing !== null;
        if (!existing) {
          if (!autoDiscover) setPhase('Organizing the code for search…');
          const job = await startSnapshotIndexing(scope.projectId, scope.snapshotId);
          while (current) {
            const progress = await getSnapshotIndexProgress(job.jobId);
            if (!current) return;
            if (!progress) break;
            if (progress.status !== 'running') {
              indexReady = progress.status === 'complete' || progress.status === 'partial';
              break;
            }
            await wait();
            pollDelayMs = Math.min(1000, Math.ceil(pollDelayMs * 1.5));
          }
        }
      } catch {
        // The local map is an optimization. Backend preparation can still use
        // the immutable captured file inventory when it is absent or unreadable.
      }
      if (!current) return;
      if (!autoDiscover) setPhase(indexReady ? 'Almost ready…' : 'Preparing your question…');
      const selection = readCodexSelection();
      const value = await prepareSourceDiscovery({ ...scope, ...selection });
      if (!current) { void discardSourceDiscovery(value.reviewId).catch(() => {}); return; }
      prepared.current = value.reviewId; setReview(value);
      if (autoDiscover) {
        preparing.current = false;
        await discover(value);
      }
    })().catch((failure) => { if (current) setError(errorMessageOr(failure, 'The code index could not be prepared.')); })
      .finally(() => { if (current) { setBusy(false); preparing.current = false; } });
    return () => {
      current = false; mounted.current = false;
      if (timer !== undefined) {
        clearTimeout(timer);
        timer = undefined;
        const resolve = finishWait;
        finishWait = null;
        resolve?.();
      }
      if (active.current) void cancelCodexChapterRequest(active.current).catch(() => {});
      if (prepared.current) void discardSourceDiscovery(prepared.current).catch(() => {});
    };
  }, [scope.projectId, scope.snapshotId, scope.question, scope.targetPath, retry, persistSuccessfulResult, onReady, autoDiscover]);
  async function discover(selectedReview: SourceDiscoveryReview | null = review) {
    if (!selectedReview || active.current) return;
    setBusy(true); setError(null); setPhase(autoDiscover ? 'Preparing your explanation…' : 'Finding the files related to your question…');
    active.current = selectedReview.reviewId;
    try {
      const result = await sendSourceDiscovery(selectedReview);
      if (!mounted.current) return;
      if (!result.selectedFiles.length) throw Error(result.limitation || 'Codex could not find enough relevant code. Try a more specific question.');
      if (persistSuccessfulResult && saveSourceDiscoveryResult(scope, result) && typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent(SOURCE_DISCOVERY_SAVED_EVENT, { detail: scope }));
      }
      active.current = null;
      onReady(result, selectedReview);
    } catch (failure) {
      if (mounted.current) { setError(errorMessageOr(failure, 'Code discovery could not finish.')); setReview(null); }
    } finally { active.current = null; if (mounted.current) setBusy(false); }
  }
  if (needsConnection) return <CodexChapterDialog mode="settings" native={isNativeRuntime()} onClose={() => {
    if (!connectionState.status?.authenticated) { onClose(); return; }
    setNeedsConnection(false); setRetry((value) => value + 1);
  }} />;
  return <div className="provider-overlay"><section className="provider-dialog discovery-dialog" ref={dialog} role="dialog" aria-modal="true" aria-labelledby="discovery-title" tabIndex={-1}>
    <header className="provider-dialog-header"><div><h2 id="discovery-title">{autoDiscover ? 'Preparing your explanation' : 'Find code for this question'}</h2><p>{scope.question}</p></div><button className="icon-button" onClick={onClose} aria-label="Close code discovery"><X size={18} /></button></header>
    <div className="provider-dialog-content">
      {busy ? <p role="status" className="discovery-progress"><LoaderCircle size={18} />{phase}</p> : review && <>
        <p>Code Atlas will choose relevant files for you. You do not need to select references.</p>
        <p className="provider-small-note">Your question and repository details (file paths, code names and relationships) go to your saved AI connection. File contents are not sent in this step; you can review them before asking for the explanation. This may count toward your connected account’s usage.</p>
        {review.indexStatus === 'partial' && <p className="source-warning">Some code relationships are missing, so file selection may be incomplete.</p>}
        {review.indexStatus === 'cancelled' && <p className="source-warning">The repository map is incomplete, so file selection may be incomplete.</p>}
        {review.indexStatus === 'missing' && <p className="source-warning">Code relationships are unavailable. Codex can still search file paths, but may miss related code.</p>}
        {review.indexStatus === 'unreadable' && <p className="source-warning">Code relationships could not be read, so file selection may be incomplete.</p>}
        {review.omittedCount > 0 && <p className="source-warning">{review.omittedCount.toLocaleString()} paths are outside this search, so the result may miss relevant code.</p>}
        {review.coverageGaps.length > 0 && (
          <details>
            <summary>Coverage notes · {review.coverageGaps.reduce((total, gap) => total + gap.count, 0).toLocaleString()} code paths</summary>
            <ul>{review.coverageGaps.map((gap) => (
              <li key={gap.category}>
                <strong>{gap.count.toLocaleString()} {coverageGapLabels[gap.category]}</strong>
                <p>{gap.message}</p>
                {gap.examples.length > 0 && <p><code>{gap.examples.join(', ')}</code></p>}
              </li>
            ))}</ul>
          </details>
        )}
        <details><summary>Details shared with your connection</summary><pre className="discovery-metadata">{review.metadata}</pre></details>
        {review.documentation.length > 0 && <details><summary>Documentation is guidance, code is the evidence</summary><ul>{review.documentation.map((doc) => <li key={doc.path}><code>{doc.path}</code> · {doc.status === 'declaredDeprecated' ? 'Declared deprecated / archived' : 'Not verified against code'}<p>{doc.reason}</p></li>)}</ul></details>}
      </>}
      {savedSelectionNotice && <p className="source-warning" role="status">{savedSelectionNotice}</p>}
      {error && <p role="alert" className="provider-error-text">{error}</p>}
    </div><footer className="provider-dialog-footer"><button className="secondary-button" onClick={onClose}>{active.current ? 'Cancel discovery' : 'Cancel'}</button>
      {error ? <button className="primary-button" onClick={() => setRetry((value) => value + 1)}>Try again</button> : !autoDiscover && <button className="primary-button" disabled={busy || !review} onClick={() => { void discover(); }}>Find relevant code</button>}
    </footer>
  </section></div>;
}
