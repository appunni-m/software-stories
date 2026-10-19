import { useEffect, useRef, useState } from 'react';
import { FileSearch, LoaderCircle, X } from 'lucide-react';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { ensureCodexConnection, readCodexSelection, useConnectionState } from './lib/connectionStore';
import { prepareSourceDiscovery, sendSourceDiscovery, discardSourceDiscovery, type SourceDiscoveryScope, type SourceDiscoveryReview, type SourceDiscoveryResult } from './lib/sourceDiscovery';
import { saveSourceDiscoveryResult, SOURCE_DISCOVERY_SAVED_EVENT } from './lib/sourceDiscoveryStore';
import { cancelCodexChapterRequest, isNativeRuntime, loadSnapshotIndex, startSnapshotIndexing, getSnapshotIndexProgress } from './lib/commands';
import { errorMessageOr } from './lib/errorMessage';
import { CodexChapterDialog } from './CodexChapterDialog';

export function SourceDiscoveryDialog({ scope, onReady, onClose, persistSuccessfulResult = false }: {
  scope: SourceDiscoveryScope; onReady: (result: SourceDiscoveryResult, review: SourceDiscoveryReview) => void; onClose: () => void;
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
      const connection = await ensureCodexConnection(retry > 0);
      if (!current) return;
      if (!connection.status.authenticated) { setNeedsConnection(true); return; }
      const existing = await loadSnapshotIndex(scope.projectId, scope.snapshotId);
      if (!current) return;
      if (!existing) {
        setPhase('Building a local map of files, symbols and relationships…');
        const job = await startSnapshotIndexing(scope.projectId, scope.snapshotId);
        while (current) {
          const progress = await getSnapshotIndexProgress(job.jobId);
          if (!progress) throw Error('The local index job is unavailable. Try again to use the captured file inventory.');
          if (progress.status !== 'running') break;
          await wait();
        }
      }
      if (!current) return;
      setPhase('Preparing AI code discovery…');
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
  }, [scope.projectId, scope.snapshotId, scope.question, scope.targetPath, retry]);
  async function discover() {
    if (!review || busy || preparing.current || active.current) return;
    setBusy(true); setError(null); setPhase('Codex is following the index to choose implementation files…');
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
        <p>Codex will use <strong>{review.candidateCount.toLocaleString()} indexed code paths</strong>, named symbols and static connections to choose the relevant files. You do not need to select references.</p>
        <p className="provider-small-note">{review.modelId} · {review.effort} · uses your header connection. This discovery shares indexed names, relationships, documentation labels and your question with OpenAI through Codex. Account quota applies. File contents are reviewed separately before chapter generation.</p>
        {!review.indexAvailable && <p className="source-warning">No usable symbol index is available. Discovery can use captured file paths only; relationships remain unknown.</p>}
        {review.omittedCount > 0 && <p className="source-warning">{review.omittedCount.toLocaleString()} code paths do not fit this discovery window. The result will show this scope limit.</p>}
        <details><summary>What Codex will inspect</summary><pre className="discovery-metadata">{review.metadata}</pre></details>
        {review.documentation.length > 0 && <details><summary>Documentation is guidance, code is the evidence</summary><ul>{review.documentation.map((doc) => <li key={doc.path}><code>{doc.path}</code> · {doc.status === 'declaredDeprecated' ? 'Declared deprecated / archived' : 'Not verified against code'}<p>{doc.reason}</p></li>)}</ul></details>}
      </>}
      {error && <p role="alert" className="provider-error-text">{error}</p>}
    </div><footer className="provider-dialog-footer"><button className="secondary-button" onClick={onClose}>{active.current ? 'Cancel discovery' : 'Cancel'}</button>
      {error ? <button className="primary-button" onClick={() => setRetry((value) => value + 1)}>Try again</button> : <button className="primary-button" disabled={busy || !review} onClick={() => { void discover(); }}>Find relevant code</button>}
    </footer>
  </section></div>;
}
