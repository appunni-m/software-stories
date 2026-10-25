import { useEffect, useRef, useState } from 'react';
import { FileSearch, LoaderCircle, X } from 'lucide-react';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { ensureCodexConnection, readCodexSelection, useConnectionState } from './lib/connectionStore';
import { prepareSourceDiscovery, sendSourceDiscovery, discardSourceDiscovery, type DiscoveryCoverageGapCategory, type SourceDiscoveryScope, type SourceDiscoveryReview, type SourceDiscoveryResult } from './lib/sourceDiscovery';
import { loadSourceDiscoveryResult, saveSourceDiscoveryResult, SOURCE_DISCOVERY_SAVED_EVENT } from './lib/sourceDiscoveryStore';
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

export function SourceDiscoveryDialog({ scope, onReady, onClose, persistSuccessfulResult = true }: {
  scope: SourceDiscoveryScope; onReady: (result: SourceDiscoveryResult, review?: SourceDiscoveryReview) => void; onClose: () => void;
  persistSuccessfulResult?: boolean;
}) {
  const [review, setReview] = useState<SourceDiscoveryReview | null>(null);
  const [phase, setPhase] = useState('Reading the saved code index…');
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
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
    const wait = () => new Promise<void>((resolve) => { timer = setTimeout(resolve, 250); });
    dialog.current?.focus();
    setReview(null); setError(null); setBusy(true); preparing.current = true;
    void (async () => {
      if (!isNativeRuntime()) throw Error('Open the Mac app to discover code with your saved agent.');
      if (persistSuccessfulResult) {
        try {
          const manifest=await loadSnapshotManifest(scope.projectId,scope.snapshotId);
          if (!current) return;
          const availablePaths=manifest.files.filter((file)=>file.kind==='file'&&file.disposition==='captured'&&!file.secretLike).map((file)=>file.path);
          const saved=loadSourceDiscoveryResult(scope,availablePaths);
          if (saved) { onReady(saved); return; }
        } catch {
          // Cached selections are used only after their paths can be checked
          // against the current immutable manifest. Discovery may still retry.
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
          setPhase('Finding names and connections in the code…');
          const job = await startSnapshotIndexing(scope.projectId, scope.snapshotId);
          while (current) {
            const progress = await getSnapshotIndexProgress(job.jobId);
            if (!progress) break;
            if (progress.status !== 'running') {
              indexReady = progress.status === 'complete' || progress.status === 'partial';
              break;
            }
            await wait();
          }
        }
      } catch {
        // The local map is an optimization. Backend preparation can still use
        // the immutable captured file inventory when it is absent or unreadable.
      }
      if (!current) return;
      setPhase(indexReady ? 'Preparing code discovery…' : 'Preparing discovery from the captured code…');
      const selection = readCodexSelection();
      const value = await prepareSourceDiscovery({ ...scope, ...selection });
      if (!current) { void discardSourceDiscovery(value.reviewId).catch(() => {}); return; }
      prepared.current = value.reviewId; setReview(value);
    })().catch((failure) => { if (current) setError(errorMessageOr(failure, 'The code index could not be prepared.')); })
      .finally(() => { if (current) { setBusy(false); preparing.current = false; } });
    return () => {
      current = false; mounted.current = false;
      if (timer) clearTimeout(timer);
      if (active.current) void cancelCodexChapterRequest(active.current).catch(() => {});
      if (prepared.current) void discardSourceDiscovery(prepared.current).catch(() => {});
    };
  }, [scope.projectId, scope.snapshotId, scope.question, scope.targetPath, retry, persistSuccessfulResult, onReady]);
  async function discover() {
    if (!review || busy || preparing.current || active.current) return;
    setBusy(true); setError(null); setPhase('Finding the files related to your question…');
    active.current = review.reviewId;
    try {
      const result = await sendSourceDiscovery(review);
      if (!mounted.current) return;
      if (!result.selectedFiles.length) throw Error(result.limitation || 'Codex could not find enough relevant code. Try a more specific question.');
      if (persistSuccessfulResult && saveSourceDiscoveryResult(scope, result) && typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent(SOURCE_DISCOVERY_SAVED_EVENT, { detail: scope }));
      }
      active.current = null;
      onReady(result, review);
    } catch (failure) {
      if (mounted.current) { setError(errorMessageOr(failure, 'Code discovery could not finish.')); setReview(null); }
    } finally { active.current = null; if (mounted.current) setBusy(false); }
  }
  if (needsConnection) return <CodexChapterDialog mode="settings" native={isNativeRuntime()} onClose={() => {
    if (!connectionState.status?.authenticated) { onClose(); return; }
    setNeedsConnection(false); setRetry((value) => value + 1);
  }} />;
  return <div className="provider-overlay"><section className="provider-dialog discovery-dialog" ref={dialog} role="dialog" aria-modal="true" aria-labelledby="discovery-title" tabIndex={-1}>
    <header className="provider-dialog-header"><div><div className="step-pill"><FileSearch size={15} /> AI FINDS THE CODE</div><h2 id="discovery-title">Start with your question</h2><p>{scope.question}</p></div><button className="icon-button" onClick={onClose} aria-label="Close code discovery"><X size={18} /></button></header>
    <div className="provider-dialog-content">
      {busy ? <p role="status" className="discovery-progress"><LoaderCircle size={18} />{phase}</p> : review && <>
        <p>Codex will search <strong>{review.candidateCount.toLocaleString()} captured code files</strong> and use the available names and connections to choose what matters. You do not need to select references.</p>
        <p className="provider-small-note">{review.modelId} · {review.effort} · uses your header connection. This discovery shares your question, captured code paths, code-map coverage, available symbol names and static links with OpenAI through Codex. Account quota applies. File contents are reviewed separately before chapter generation.</p>
        {review.indexStatus === 'partial' && <p className="source-warning">The local code map is incomplete. Discovery can still use captured files and the symbols and connections that were found; some may be missing.</p>}
        {review.indexStatus === 'cancelled' && <p className="source-warning">Building the local code map was cancelled. Discovery can use captured files and any saved partial map; some connections may be missing.</p>}
        {review.indexStatus === 'missing' && <p className="source-warning">No local code map is available. Discovery will use captured file paths only; code relationships are unknown.</p>}
        {review.indexStatus === 'unreadable' && <p className="source-warning">The saved local code map could not be read. Discovery will use captured file paths only; code relationships are unknown.</p>}
        {review.omittedCount > 0 && <p className="source-warning">{review.omittedCount.toLocaleString()} code paths do not fit this discovery window. The result will show this scope limit.</p>}
        {review.coverageGaps.length > 0 && (
          <details>
            <summary>Code outside this discovery pass · {review.coverageGaps.reduce((total, gap) => total + gap.count, 0).toLocaleString()} paths</summary>
            <ul>{review.coverageGaps.map((gap) => (
              <li key={gap.category}>
                <strong>{gap.count.toLocaleString()} {coverageGapLabels[gap.category]}</strong>
                <p>{gap.message}</p>
                {gap.examples.length > 0 && <p><code>{gap.examples.join(', ')}</code></p>}
              </li>
            ))}</ul>
          </details>
        )}
        <details><summary>What Codex will inspect</summary><pre className="discovery-metadata">{review.metadata}</pre></details>
        {review.documentation.length > 0 && <details><summary>Documentation is guidance, code is the evidence</summary><ul>{review.documentation.map((doc) => <li key={doc.path}><code>{doc.path}</code> · {doc.status === 'declaredDeprecated' ? 'Declared deprecated / archived' : 'Not verified against code'}<p>{doc.reason}</p></li>)}</ul></details>}
      </>}
      {error && <p role="alert" className="provider-error-text">{error}</p>}
    </div><footer className="provider-dialog-footer"><button className="secondary-button" onClick={onClose}>{active.current ? 'Cancel discovery' : 'Cancel'}</button>
      {error ? <button className="primary-button" onClick={() => setRetry((value) => value + 1)}>Try again</button> : <button className="primary-button" disabled={busy || !review} onClick={() => { void discover(); }}>Find relevant code</button>}
    </footer>
  </section></div>;
}
