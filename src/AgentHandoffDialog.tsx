import { useEffect, useMemo, useRef, useState } from 'react';
import { FileCode2, X } from 'lucide-react';
import { providerFailureMessage } from './lib/errorMessage';
import { InlineCodeText } from './InlineCodeText';
import { AppButton } from './design/AppButton';
import { AppSelect } from './design/AppSelect';
import {
  buildAgentHandoffReview, discardGeneratedChapterDraft, discardProviderRequestReview,
  exportAgentHandoff, importAgentHandoffResult, listChapterRevisions, loadSnapshotManifest,
  saveGeneratedChapterDraft,
  type ChapterDraftPreview, type ChapterRevision, type HandoffAgent, type ProviderRequestReview, type SnapshotManifest,
} from './lib/commands';
import type { CodexChapterDialogMode } from './CodexChapterDialog';

interface Props {
  mode: Extract<CodexChapterDialogMode, 'settings' | { kind: 'review' | 'overview' }>;
  native: boolean;
  initialAgent: HandoffAgent;
  onAgentChange: (agent: HandoffAgent) => void;
  onClose: () => void;
  onChooseBackend: () => void;
}

const agents: { id: HandoffAgent; name: string }[] = [
  { id: 'claude-code', name: 'Claude Code' }, { id: 'codex', name: 'Codex' },
  { id: 'antigravity', name: 'Antigravity' }, { id: 'other', name: 'Another agent' },
];

/** Manual transport only. Runtime settings and account access belong to the
 * user's agent; the app validates the imported explanation independently. */
export function AgentHandoffDialog({ mode, native, initialAgent, onAgentChange, onClose, onChooseBackend }: Props) {
  const projectId = mode === 'settings' ? null : mode.projectId;
  const snapshotId = mode === 'settings' ? null : mode.snapshotId;
  const targetPath = mode !== 'settings' && mode.kind === 'review' ? mode.targetPath ?? null : null;
  const initialPaths = mode === 'settings' ? undefined : mode.selectedPaths;
  const [agent, setAgent] = useState(initialAgent);
  const [manifest, setManifest] = useState<SnapshotManifest | null>(null);
  const [paths, setPaths] = useState<string[]>([]);
  const [query, setQuery] = useState('');
  const [review, setReview] = useState<ProviderRequestReview | null>(null);
  const [consent, setConsent] = useState(false);
  const [taskText, setTaskText] = useState('');
  const [response, setResponse] = useState('');
  const [preview, setPreview] = useState<ChapterDraftPreview | null>(null);
  const [saved, setSaved] = useState<ChapterRevision | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const lifetime = useRef(0);
  const activeBusy = useRef(false);
  const closeRef = useRef(onClose);
  const resources = useRef<{ review: string | null; draft: string | null }>({ review: null, draft: null });
  closeRef.current = onClose;
  const agentName = agents.find((value) => value.id === agent)?.name ?? 'Your agent';
  const files = useMemo(() => (manifest?.files ?? []).filter((file) => file.kind === 'file'
    && ['text', 'unknown'].includes(file.contentKind) && !file.secretLike && file.sizeBytes <= 250_000)
    .sort((left, right) => left.path.localeCompare(right.path)), [manifest]);
  const visibleFiles = files.filter((file) => file.path.toLowerCase().includes(query.toLowerCase())).slice(0, 100);
  const sourceBytes = paths.reduce((sum, path) => sum + (files.find((file) => file.path === path)?.sizeBytes ?? 0), 0);
  const sourceReady = Boolean(manifest && paths.length && paths.length <= 12 && sourceBytes <= 250_000
    && paths.every((path) => files.some((file) => file.path === path))
    && (mode === 'settings' || mode.kind !== 'review' || targetPath && paths.includes(targetPath)));
  const locked = busy !== null || preview !== null;

  const releaseReview = () => {
    const id = resources.current.review;
    resources.current.review = null;
    if (id && native) void discardProviderRequestReview(id).catch(() => {});
    setReview(null); setConsent(false); setTaskText(''); setResponse(''); setNote(null); setError(null);
  };
  const begin = (operation: string) => {
    if (!native || activeBusy.current) return null;
    activeBusy.current = true; setBusy(operation); setError(null); setNote(null);
    return lifetime.current;
  };
  const finish = (epoch: number) => {
    if (epoch !== lifetime.current) return;
    activeBusy.current = false; setBusy(null);
  };

  useEffect(() => {
    const epoch = ++lifetime.current;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialogRef.current?.focus();
    setManifest(null); setPaths([...new Set([...(targetPath ? [targetPath] : []), ...(initialPaths ?? [])])]);
    setReview(null); setPreview(null); setSaved(null); setConsent(false); setTaskText(''); setResponse(''); setError(null); setNote(null);
    activeBusy.current = Boolean(native && projectId && snapshotId);
    setBusy(activeBusy.current ? 'Loading captured files…' : null);
    if (native && projectId && snapshotId) {
      void loadSnapshotManifest(projectId, snapshotId).then((value) => {
        if (epoch === lifetime.current) setManifest(value);
      }).catch(() => { if (epoch === lifetime.current) setError('Captured files could not be loaded. Close and reopen this handoff to retry.'); })
        .finally(() => finish(epoch));
    }
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); if (!activeBusy.current) closeRef.current(); }
      if (event.key !== 'Tab' || !dialogRef.current) return;
      const elements = Array.from(dialogRef.current.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), summary, a[href], [tabindex]:not([tabindex="-1"])'))
        .filter((element) => element.getClientRects().length > 0);
      const index = elements.indexOf(document.activeElement as HTMLElement);
      if (!elements.length) { event.preventDefault(); dialogRef.current.focus(); }
      else if (index < 0 || event.shiftKey && index === 0 || !event.shiftKey && index === elements.length - 1) {
        event.preventDefault(); (event.shiftKey ? elements[elements.length - 1] : elements[0]).focus();
      }
    };
    document.addEventListener('keydown', keydown, true);
    return () => {
      ++lifetime.current;
      document.removeEventListener('keydown', keydown, true);
      const { review: reviewId, draft } = resources.current;
      resources.current = { review: null, draft: null };
      if (native && reviewId) void discardProviderRequestReview(reviewId).catch(() => {});
      if (native && draft) void discardGeneratedChapterDraft(draft).catch(() => {}).finally(() => window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed')));
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, [native, projectId, snapshotId, targetPath, initialPaths]);

  const build = async () => {
    if (!projectId || !snapshotId || !sourceReady || locked) return;
    const epoch = begin('Preparing local task…'); if (epoch === null) return;
    releaseReview();
    try {
      const result = await buildAgentHandoffReview({ projectId, snapshotId, targetPath, selectedPaths: paths, agentId: agent });
      if (epoch !== lifetime.current) { if (result.status === 'ok') void discardProviderRequestReview(result.data.reviewId).catch(() => {}); return; }
      if (result.status === 'error') { setError(providerFailureMessage(result.error)); return; }
      resources.current.review = result.data.reviewId; setReview(result.data);
    } catch { if (epoch === lifetime.current) setError('The local task could not be prepared. No agent was started.'); }
    finally { finish(epoch); }
  };
  const copy = async () => {
    if (!review || !consent || locked) return;
    const epoch = begin('Preparing task text…'); if (epoch === null) return;
    try {
      const result = await exportAgentHandoff(review.reviewId, review.fingerprint, consent);
      if (epoch !== lifetime.current) return;
      if (result.status === 'error') { setError(providerFailureMessage(result.error)); return; }
      setTaskText(result.data);
      try {
        await navigator.clipboard.writeText(result.data);
        if (epoch === lifetime.current) setNote(`Copied. Paste into ${agentName}, then bring its complete JSON response back here.`);
      } catch { if (epoch === lifetime.current) setNote('Clipboard access was unavailable. Select and copy the task text below.'); }
    } catch { if (epoch === lifetime.current) setError('The task could not be exported. Rebuild the review and try again.'); }
    finally { finish(epoch); }
  };
  const importResult = async () => {
    if (!review || !taskText || !response.trim() || locked) return;
    const epoch = begin('Validating imported evidence…'); if (epoch === null) return;
    try {
      const result = await importAgentHandoffResult(review.reviewId, review.fingerprint, response);
      if (epoch !== lifetime.current) { if (result.status === 'ok') void discardGeneratedChapterDraft(result.data.generationId).catch(() => {}); return; }
      if (result.status === 'error') { setError(providerFailureMessage(result.error)); return; }
      resources.current.draft = result.data.generationId; resources.current.review = null;
      setPreview(result.data); setTaskText(''); setResponse(''); setConsent(false);
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    } catch { if (epoch === lifetime.current) setError('The import could not be confirmed. Check recovered previews before retrying; accepted chapters are unchanged.'); }
    finally { finish(epoch); }
  };
  const save = async () => {
    if (!preview || saved || preview.validation.status !== 'valid') return;
    const epoch = begin('Saving draft…'); if (epoch === null) return;
    try {
      let revision: ChapterRevision;
      try { revision = await saveGeneratedChapterDraft(preview.generationId); }
      catch {
        const existing = await listChapterRevisions(preview.revision.projectId, preview.revision.snapshotId)
          .then((values) => values.find((value) => value.revisionId === preview.revision.revisionId));
        if (!existing) throw new Error('Save could not be confirmed');
        revision = existing;
      }
      if (epoch !== lifetime.current) return;
      resources.current.draft = null; setSaved(revision);
      window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision } }));
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    } catch { if (epoch === lifetime.current) setError('The draft save could not be confirmed. Retry saving before closing.'); }
    finally { finish(epoch); }
  };

  return <div className="provider-overlay"><section className="provider-dialog" ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="handoff-title">
    <header className="provider-dialog-header"><div><div className="step-pill"><span>CODING AGENT</span> MANUAL HANDOFF</div><h2 id="handoff-title">Explain with your agent</h2>
      <p>Review a task, run it in your agent, then import the explanation. No API key is needed in Code Atlas.</p></div>
      <AppButton intent="icon" disabled={busy !== null} onClick={onClose} aria-label="Close agent handoff"><X size={17} /></AppButton></header>
    <div className="provider-dialog-content">
      <section className="provider-review-card"><h3>1. Choose your agent</h3>
        <label className="provider-model-select">Agent<AppSelect ariaLabel="Coding agent" size="1" value={agent} disabled={locked} options={agents.map((value)=>({value:value.id,label:value.name}))} onValueChange={(value)=>{const next=value as HandoffAgent;releaseReview();setAgent(next);onAgentChange(next);}}/></label>
        <p>Choose model, reasoning and approvals inside {agentName}. Code Atlas cannot apply or verify those settings, manage this run, cancel it or constrain its tools. Use a fresh conversation and review the agent’s own account, cost and permissions before pasting.</p>
        {agent === 'antigravity' && <p className="provider-small-note" role="note">Manual handoff only: paste the reviewed task into Antigravity yourself. Google does not allow third-party apps to connect with a consumer Antigravity account.</p>}
        <p className="provider-small-note">This is a manual exchange of text, separate from automatic runtime integration. Your agent’s normal terms, access rules and usage limits apply. Imported output does not attest which agent or model produced it.</p>
      </section>
      {!native ? <p role="status">Open the native app to prepare immutable source evidence and validate imported chapters.</p> : mode === 'settings' ? <p role="status">Open a captured file and choose Explain, or create a repository tour from Guide, then select a manual agent handoff. Configure model and permissions in your agent.</p> : <>
        <section className="provider-review-card"><h3>2. Review the captured source</h3>
          <p>{targetPath ? <code>{targetPath}</code> : 'Selected-source repository overview'} · {paths.length}/12 files · {sourceBytes.toLocaleString()}/250,000 source bytes</p>
          <label className="provider-context-search">Find a captured file<input type="search" value={query} disabled={locked} onChange={(event) => setQuery(event.target.value)} /></label>
          <div className="provider-context-files">{visibleFiles.map((file) => <label className={paths.includes(file.path) ? 'selected' : ''} key={file.path}>
            <input type="checkbox" checked={paths.includes(file.path)} disabled={locked || file.path === targetPath || !paths.includes(file.path) && (paths.length >= 12 || sourceBytes + file.sizeBytes > 250_000)}
              onChange={() => { releaseReview(); setPaths((values) => values.includes(file.path) ? values.filter((value) => value !== file.path) : [...values, file.path]); }} /><code>{file.path}</code><small>{file.sizeBytes.toLocaleString()} bytes</small></label>)}{!visibleFiles.length && manifest && <p>No eligible files match this search.</p>}</div>
          <p className="provider-small-note">Showing up to 100 matching files. Search to narrow the list. Selected paths: {paths.join(', ') || 'none'}.</p>
          {paths.length > 0 && !sourceReady && manifest && <p className="provider-error-text">The target or a selected file is unavailable, or the selection exceeds 12 files or 250,000 bytes. Remove supporting files, or choose an eligible captured target.</p>}
          <AppButton intent="secondary" disabled={locked || !sourceReady} onClick={() => { void build(); }}>{review ? 'Rebuild local task' : 'Prepare local task'}</AppButton>
          {review && <><p>Review expires at {new Date(review.expiresAtUnixMs).toLocaleTimeString()}. Keep this dialog open while using the agent. Restarting the app requires a new task.</p>
            {review.selectedFiles.map((file) => <details className="provider-source-file" key={file.path}><summary><code>{file.path}</code><span>SHA-256 {file.sha256}</span></summary><pre><code>{file.contents}</code></pre></details>)}
            <details className="provider-prompt-preview"><summary>Exact instructions, evidence and response schema</summary><pre>{JSON.stringify(review.requestBodyPreview, null, 2)}</pre></details>
            <div className="provider-fingerprint"><span>Task fingerprint</span><code>{review.fingerprint}</code></div>
            {!preview && <><div className="provider-consent-list"><label><input type="checkbox" checked={consent} disabled={busy !== null} onChange={(event) => setConsent(event.target.checked)} /><span>I reviewed this exact source and want to copy it for use in {agentName}. Clipboard contents can be read or retained by my system’s clipboard tools. Pasting shares the code under that agent’s account and terms.</span></label></div>
              <AppButton intent="primary" disabled={busy !== null || !consent} onClick={() => { void copy(); }}>Copy reviewed task</AppButton></>}
          </>}
        </section>
        {taskText && <section className="provider-review-card"><h3>3. Run in {agentName} and import</h3><p>Paste the complete task into your agent. Ask it to return only the JSON response. Stop or cancel generation in that agent if needed. Closing this dialog does not stop its work.</p>
          <details><summary>Task text for manual copying</summary><textarea className="agent-handoff-text" aria-label="Reviewed task text" readOnly value={taskText} onFocus={(event) => event.currentTarget.select()} /></details>
          <label>Complete JSON response<textarea className="agent-handoff-text" value={response} maxLength={1_000_000} disabled={busy !== null} placeholder="Paste the response including handoffId, fingerprint and chapter" onChange={(event) => setResponse(event.target.value)} /></label>
          <AppButton intent="primary" disabled={busy !== null || !response.trim()} onClick={() => { void importResult(); }}>Validate imported explanation</AppButton>
        </section>}
        {preview && <section className="provider-response"><span>{saved ? 'SAVED · UNACCEPTED DRAFT' : 'SOURCE VALIDATED · UNSAVED PREVIEW'}</span><h3><InlineCodeText text={preview.revision.title}/></h3>
          <p>Source citations passed local checks. Explanation quality still needs your review. Agent identity, model, reasoning and permission settings are unverified.</p>
          <div className="generated-chapter-sections">{preview.revision.sections.map((section) => <article className="generated-chapter-section" key={section.sectionId}><h4>{section.order + 1}. <InlineCodeText text={section.title}/></h4><p><InlineCodeText text={section.prose}/></p>
            <ul>{section.claims.map((claim) => <li key={claim.claimId}><span className="chapter-claim-category">{claim.category.kind === 'inference' ? claim.category.visibleLabel : claim.category.kind === 'codeBehavior' ? 'Code behavior' : 'Repository context'}</span><p><InlineCodeText text={claim.prose}/></p>{claim.category.kind === 'inference' && <p>Basis: <InlineCodeText text={claim.category.basis}/></p>}{claim.anchors.map((anchor) => <details className="chapter-evidence" key={anchor.anchorId}><summary><FileCode2 size={12} /><code>{anchor.path}:{anchor.startLine}–{anchor.endLine}</code></summary><pre>{anchor.excerpt}</pre></details>)}</li>)}</ul>
            <details><summary>Narration script</summary><p><InlineCodeText text={section.spokenText}/></p></details></article>)}</div>
          <p>{saved ? `Revision ${saved.revisionNumber} saved. Open Guide or Edit to review and accept it.` : 'Saving adds an unaccepted draft. Acceptance remains a separate action.'}</p></section>}
      </>}
      {busy && <p role="status" aria-live="polite">{busy}</p>}{error && <p className="provider-error-text" role="alert">{error}</p>}{note && <p className="provider-status-note" role="status">{note}</p>}
    </div>
    <footer className="provider-dialog-footer"><div className="provider-draft-actions"><AppButton intent="secondary" disabled={busy !== null} onClick={onChooseBackend}>Change backend{preview && !saved ? ' and discard preview' : ''}</AppButton><AppButton intent="secondary" disabled={busy !== null} onClick={onClose}>{preview && !saved ? 'Close and discard preview' : 'Done'}</AppButton></div>
      {preview && !saved && <AppButton intent="primary" disabled={busy !== null || preview.validation.status !== 'valid'} onClick={() => { void save(); }}>Save unaccepted draft</AppButton>}</footer>
  </section></div>;
}
