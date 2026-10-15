import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import { ArrowLeft, ArrowRight, FileCode2, RefreshCw, Sparkles, Trash2 } from 'lucide-react';
import { isNativeRuntime, type ChapterClaim, type SnapshotManifest } from './lib/commands';
import { codeLensRangeFromChapterAnchor } from './lib/codeLens';
import { errorMessageOr } from './lib/errorMessage';
import type { SourceLineRange } from './lib/navigation';
import {
  discardRepositoryPlanDraft, listRepositoryPlanDrafts, loadRepositoryPlanDraft,
  type PrepareRepositoryPlanInput, type RepositoryPlanDraftDetail, type RepositoryPlanDraftSummary,
} from './lib/repositoryPlans';

interface Props {
  manifest: SnapshotManifest;
  onPlan: (input: PrepareRepositoryPlanInput) => void;
  onOpenFile: (path: string, snapshotId?: string, range?: SourceLineRange) => void;
  onGenerateFile: (path: string, selectedPaths: string[]) => void;
}

/** A saved plan never becomes chapter coverage or reading progress. Metadata
 * discovery is followed by a native evidence-revalidating load before display. */
export function RepositoryPlans({ manifest, onPlan, onOpenFile, onGenerateFile }: Props) {
  const { projectId, snapshotId } = manifest.summary;
  const native = isNativeRuntime();
  const [question, setQuestion] = useState('');
  const [query, setQuery] = useState('');
  const [paths, setPaths] = useState<string[]>([]);
  const [creating, setCreating] = useState(false);
  const [summaries, setSummaries] = useState<RepositoryPlanDraftSummary[]>([]);
  const [detail, setDetail] = useState<RepositoryPlanDraftDetail | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [position, setPosition] = useState(0);
  const [loading, setLoading] = useState(native);
  const [reading, setReading] = useState(false);
  const [discarding, setDiscarding] = useState(false);
  const [confirmDiscard, setConfirmDiscard] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const epoch = useRef(0);
  const readVersion = useRef(0);
  const mounted = useRef(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const focusStep = useRef(false);
  const selectionRef = useRef<string | null>(null);
  const files = useMemo(() => manifest.files.filter((file) => file.kind === 'file'
    && (file.contentKind === 'text' || file.contentKind === 'unknown') && !file.secretLike && file.sizeBytes <= 250_000)
    .sort((a, b) => a.path.localeCompare(b.path)), [manifest]);
  const bytes = files.filter((file) => paths.includes(file.path)).reduce((sum, file) => sum + file.sizeBytes, 0);
  const matches = files.filter((file) => file.path.toLowerCase().includes(query.trim().toLowerCase()));
  const step = detail?.preview.plan.steps[position];
  useEffect(() => {
    if (focusStep.current) { focusStep.current = false; heading.current?.focus(); }
  }, [position, detail]);

  const refresh = useCallback(async () => {
    if (!native) return;
    const version = ++epoch.current;
    setLoading(true); setError(null);
    try {
      const saved = await listRepositoryPlanDrafts(projectId);
      if (!mounted.current || epoch.current !== version) return;
      setSummaries(saved);
      if (selectionRef.current && !saved.some((item) => item.draftId === selectionRef.current)) {
        ++readVersion.current; selectionRef.current = null;
        setSelectedId(null); setDetail(null); setReading(false);
      }
    } catch (failure) {
      if (mounted.current && epoch.current === version) setError(errorMessageOr(failure, 'Saved plans could not be listed. Existing local records were preserved.'));
    } finally { if (mounted.current && epoch.current === version) setLoading(false); }
  }, [native, projectId]);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const changed = (event: Event) => {
      const id = (event as CustomEvent<{ projectId?: string }>).detail?.projectId;
      if (!id || id === projectId) void refresh();
    };
    window.addEventListener('code-atlas:repository-plans-changed', changed);
    return () => {
      mounted.current = false; ++epoch.current; ++readVersion.current;
      window.removeEventListener('code-atlas:repository-plans-changed', changed);
    };
  }, [projectId, snapshotId, refresh]);

  async function open(draftId: string) {
    if (!native || discarding) return;
    const version = ++readVersion.current;
    selectionRef.current = draftId;
    setSelectedId(draftId); setReading(true); setDetail(null); setError(null); setNotice(null); setPosition(0);
    try {
      const loaded = await loadRepositoryPlanDraft(projectId, draftId);
      if (!mounted.current || readVersion.current !== version || selectionRef.current !== draftId) return;
      if (loaded.preview.plan.projectId !== projectId || loaded.preview.plan.draftId !== draftId || !loaded.preview.savedLocally) throw new Error('The returned plan has a different native identity.');
      setDetail(loaded);
    } catch (failure) {
      if (mounted.current && readVersion.current === version) setError(errorMessageOr(failure, 'The saved plan could not be revalidated. Its local copy was preserved.'));
    } finally { if (mounted.current && readVersion.current === version) setReading(false); }
  }

  async function discard(draftId: string) {
    if (!native || discarding || confirmDiscard !== draftId) return;
    setDiscarding(true); setError(null);
    try {
      await discardRepositoryPlanDraft(projectId, draftId);
      if (!mounted.current) return;
      if (selectionRef.current === draftId) {
        ++readVersion.current; selectionRef.current = null;
        setSelectedId(null); setDetail(null); setReading(false);
      }
      setConfirmDiscard(null); setNotice('The local draft was discarded. Chapters, accepted history and separate recovery notices were preserved.');
      await refresh();
    } catch (failure) {
      if (mounted.current) setError(errorMessageOr(failure, 'Discard could not be confirmed. Refresh saved plans before trying again.'));
    } finally { if (mounted.current) setDiscarding(false); }
  }

  function submit(event: FormEvent) {
    event.preventDefault(); setFormError(null);
    const clean = question.trim();
    if ([...clean].length < 8 || [...clean].length > 300 || /[\u0000-\u001f\u007f-\u009f]/u.test(clean)) { setFormError('Write a learning question using 8–300 characters without control characters.'); return; }
    if (!paths.length || paths.length > 12 || bytes > 250_000 || paths.some((path) => !files.some((file) => file.path === path))) { setFormError('Select 1–12 captured text files within 250,000 source bytes.'); return; }
    setQuestion(clean);
    onPlan({ projectId, snapshotId, question: clean, selectedPaths: [...paths] });
  }

  function claim(value: ChapterClaim) {
    return <article className="plan-rationale" key={value.claimId}>
      <span className="chapter-claim-category">{value.category.kind === 'inference' ? value.category.visibleLabel : value.category.kind === 'codeBehavior' ? 'Code evidence' : 'Repository context'}</span>
      <p>{value.prose}</p>
      {value.category.kind === 'inference' && <p className="provider-small-note">Basis: {value.category.basis}</p>}
      <details><summary>Explore source · {value.anchors.length} citation{value.anchors.length === 1 ? '' : 's'}</summary>
        {value.anchors.map((anchor) => <div className="plan-citation" key={anchor.anchorId}>
          <button className="text-button" onClick={() => onOpenFile(anchor.path, anchor.snapshotId, codeLensRangeFromChapterAnchor(anchor))}><FileCode2 size={14} /> {anchor.path}:{anchor.startLine}–{anchor.endLine}</button>
          <pre>{anchor.excerpt}</pre>
        </div>)}
      </details>
    </article>;
  }

  return <section className="repository-journeys repository-plans" aria-labelledby="learning-plans-title">
    <header className="plan-heading"><div><h2 id="learning-plans-title">Learning plans</h2><p>Ask a question, choose source, then use Codex or Claude to plan a reading sequence.</p></div>
      <button className="primary-button" disabled={!native} onClick={() => setCreating(!creating)}><Sparkles size={15} /> {creating ? 'Hide new plan' : 'New learning plan'}</button></header>
    {!native && <p role="status">Open the macOS app to generate and reopen native plans. No API key is required for the coding-agent routes.</p>}
    {creating && <form className="plan-create-form" onSubmit={submit}>
      <label>What do you want to understand?<input value={question} onChange={(event) => setQuestion(event.target.value)} placeholder="How does a file become a generated chapter?" /></label>
      <p>Select the captured files relevant to your question. Each request covers at most 12 files and 250,000 source bytes; other files remain outside this plan.</p>
      <details open><summary>Source selection · {paths.length}/12 files · {bytes.toLocaleString()}/250,000 bytes</summary>
        {paths.length > 0 && <ul>{paths.map((path) => <li key={path}><code>{path}</code> <button type="button" className="text-button" onClick={() => setPaths((current) => current.filter((value) => value !== path))}>Remove</button></li>)}</ul>}
        <label>Find captured files<input value={query} onChange={(event) => setQuery(event.target.value)} /></label>
        <div className="provider-context-files">{matches.slice(0, 100).map((file) => <label key={file.path}>
          <input type="checkbox" checked={paths.includes(file.path)} disabled={!paths.includes(file.path) && (paths.length >= 12 || bytes + file.sizeBytes > 250_000)} onChange={(event) => {
            const checked = event.target.checked;
            setPaths((current) => checked ? current.includes(file.path) ? current : [...current, file.path] : current.filter((path) => path !== file.path));
          }} /><code>{file.path}</code><small>{file.sizeBytes.toLocaleString()} B</small>
        </label>)}</div>
        {matches.length > 100 && <p>Showing 100 of {matches.length} matches. Refine the filter to see other files.</p>}
        {!matches.length && <p>No eligible captured files match.</p>}
      </details>
      {formError && <p role="alert">{formError}</p>}
      <button type="submit" className="primary-button" disabled={!native || !paths.length || bytes > 250_000}>Choose agent and review request <ArrowRight size={14} /></button>
    </form>}
    <div className="plan-saved-heading"><h3>Saved drafts</h3><button className="text-button" disabled={!native || loading || discarding} onClick={() => { void refresh(); }}><RefreshCw size={14} /> Refresh</button></div>
    <p className="provider-small-note">This list is metadata only. Opening a draft rechecks its original proposal and captured evidence before showing citations.</p>
    {loading && <p role="status">Loading saved plan metadata…</p>}
    {!loading && native && !summaries.length && !error && <p>No saved plans yet. Start with one question and a small source selection.</p>}
    <ul className="plan-saved-list">{summaries.map((item) => <li key={item.draftId}>
      <button className="text-button" aria-current={selectedId === item.draftId ? 'true' : undefined} disabled={discarding || !!item.unavailableReason} onClick={() => { void open(item.draftId); }}>{item.title}</button>
      <small>{item.unavailableReason ?? `${item.providerId === 'codex' ? 'Codex' : 'Claude'} · ${item.modelId} · ${item.snapshotId === snapshotId ? 'current snapshot' : 'earlier snapshot'}`}</small>
      {confirmDiscard === item.draftId ? <span><span>Discard this local draft?</span> <button className="text-button" disabled={discarding} onClick={() => { void discard(item.draftId); }}>Discard draft</button> <button className="text-button" disabled={discarding} onClick={() => setConfirmDiscard(null)}>Keep</button></span>
        : <button className="text-button" disabled={discarding} onClick={() => setConfirmDiscard(item.draftId)} aria-label={`Discard ${item.title}`}><Trash2 size={14} /> Discard</button>}
    </li>)}</ul>
    {reading && <p role="status">Revalidating the original question, proposal and captured source…</p>}
    {detail && step && <section className="plan-reader" aria-label="Saved learning plan review">
      <h3>{detail.preview.plan.title}</h3><p>{detail.preview.plan.question}</p>
      <p className="provider-small-note">Unaccepted plan · {detail.preview.plan.plannedSelectedPaths.length} primary files · {detail.preview.plan.citedSelectedPaths.length} cited files · {detail.preview.plan.omittedSelectedPaths.length} selected files without a step. {detail.scope.coverage.unselectedEligibleFileCount} other eligible captured files remain outside the selection.</p>
      <p className="provider-small-note">Capture: {detail.scope.captureStatus} · index: {detail.scope.indexStatus} · {detail.scope.coverage.skippedPathCount} skipped paths · {detail.scope.coverage.excludedCapturedPathCount} excluded captured paths. This is not a whole-repository explanation.</p>
      <details><summary>Snapshot and recorded agent settings</summary><p><code>{detail.preview.plan.snapshotId}</code> · {detail.preview.modelId}</p><pre>{JSON.stringify(detail.settings, null, 2)}</pre><p>These are recorded dispatch labels, not tamper-proof account proof or authorization for a new request. Capture: {detail.scope.captureStatus}; index: {detail.scope.indexStatus}.</p></details>
      <label>Reading step<select value={position} onChange={(event) => { focusStep.current = true; setPosition(Number(event.target.value)); }}>
        {detail.preview.plan.steps.map((item, index) => <option key={item.stepId} value={index}>{index + 1}. {item.question}</option>)}
      </select></label>
      <h4 ref={heading} tabIndex={-1}>{position + 1}. {step.question}</h4>
      {step.prerequisiteStepIds.length > 0 && <p>Builds on {step.prerequisiteStepIds.map((id) => detail.preview.plan.steps.findIndex((item) => item.stepId === id) + 1).join(', ')}. This is a reading order, not a proven runtime call.</p>}
      {claim(step.claim)}
      <div className="plan-reader-actions"><button className="secondary-button" disabled={position === 0} onClick={() => { focusStep.current = true; setPosition(position - 1); }}><ArrowLeft size={14} /> Back</button>
        <button className="secondary-button" disabled={position + 1 >= detail.preview.plan.steps.length} onClick={() => { focusStep.current = true; setPosition(position + 1); }}>Next <ArrowRight size={14} /></button>
        <button className="primary-button" disabled={detail.preview.plan.snapshotId !== snapshotId} onClick={() => onGenerateFile(step.primaryPath, [...new Set([step.primaryPath, ...step.claim.anchors.map((anchor) => anchor.path)])])}><Sparkles size={14} /> Explain this file</button></div>
      {detail.preview.plan.snapshotId !== snapshotId && <p>Open a citation to switch to this plan’s original snapshot before requesting a file explanation.</p>}
      <details><summary>Known unknowns and omitted selected files</summary>{detail.preview.plan.unknowns.map(claim)}<ul>{detail.preview.plan.omittedSelectedPaths.map((path) => <li key={path}><code>{path}</code></li>)}</ul></details>
      <p className="provider-small-note">Steps advance only when you choose. This draft does not accept chapters, complete coverage or mark anything read. Separate generation notices remain in Recovery until explicitly dismissed.</p>
    </section>}
    {error && <p className="provider-error-text" role="alert">{error}</p>}
    {notice && <p role="status">{notice}</p>}
  </section>;
}
