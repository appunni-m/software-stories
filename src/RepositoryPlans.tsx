import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import { ArrowLeft, ArrowRight, FileCode2, RefreshCw, Sparkles, Trash2 } from 'lucide-react';
import { isNativeRuntime, type ChapterClaim, type ChapterRevision, type SnapshotManifest } from './lib/commands';
import { samePlannedTarget } from './lib/chapterOrigin';
import { GenerationRecord } from './GenerationRecord';
import { codeLensRangeFromChapterAnchor } from './lib/codeLens';
import { errorMessageOr } from './lib/errorMessage';
import type { SourceLineRange } from './lib/navigation';
import { preparePlannedChapterScope, type PlannedChapterScope } from './lib/plannedChapters';
import {
  acceptRepositoryPlanDraft, discardRepositoryPlanDraft, listRepositoryPlanDrafts, loadRepositoryPlanDraft,
  listRepositoryPlanRevisions, loadRepositoryPlanRevision, resolveRepositoryPlanNotice,
  type PrepareRepositoryPlanInput, type RepositoryPlanDraftDetail, type RepositoryPlanDraftSummary,
  type RepositoryPlanRevisionDetail, type RepositoryPlanRevisionSummary, type ReviewedPlanSource,
} from './lib/repositoryPlans';

interface Props {
  manifest: SnapshotManifest;
  onPlan: (input: PrepareRepositoryPlanInput) => void;
  onOpenFile: (path: string, snapshotId?: string, range?: SourceLineRange) => void;
  onGenerateFile: (path: string, selectedPaths: string[]) => void;
  onGeneratePlannedChapter: (scope: PlannedChapterScope) => void;
  chapters: ChapterRevision[];
  chaptersLoading: boolean;
  chaptersError: string | null;
  onReadChapter: (chapter: ChapterRevision) => void;
  onReviewChapter: (chapter: ChapterRevision) => void;
}

/** A saved plan never becomes chapter coverage or reading progress. Metadata
 * discovery is followed by a native evidence-revalidating load before display. */
export function RepositoryPlans({ manifest, onPlan, onOpenFile, onGenerateFile, onGeneratePlannedChapter,
  chapters, chaptersLoading, chaptersError, onReadChapter, onReviewChapter,
}: Props) {
  const { projectId, snapshotId } = manifest.summary;
  const native = isNativeRuntime();
  const [question, setQuestion] = useState('');
  const [query, setQuery] = useState('');
  const [paths, setPaths] = useState<string[]>([]);
  const [creating, setCreating] = useState(false);
  const [summaries, setSummaries] = useState<RepositoryPlanDraftSummary[]>([]);
  const [detail, setDetail] = useState<RepositoryPlanDraftDetail | null>(null);
  const [revisions, setRevisions] = useState<RepositoryPlanRevisionSummary[]>([]);
  const [revision, setRevision] = useState<RepositoryPlanRevisionDetail | null>(null);
  const [parentId, setParentId] = useState('');
  const [parent, setParent] = useState<RepositoryPlanRevisionDetail | null>(null);
  const [parentLoading, setParentLoading] = useState(false);
  const [acceptConsent, setAcceptConsent] = useState(false);
  const [decisionBusy, setDecisionBusy] = useState<'accept' | 'resolve' | 'prepare-step' | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [position, setPosition] = useState(0);
  const [loading, setLoading] = useState(native);
  const [reading, setReading] = useState(false);
  const [discarding, setDiscarding] = useState(false);
  const [confirmDiscard, setConfirmDiscard] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const epoch = useRef(0);
  const readVersion = useRef(0);
  const parentVersion = useRef(0);
  const mutating = useRef(false);
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
  const linkedChapters = revision && detail && step ? chapters.filter((chapter) => {
    const origin = chapter.generationOrigin;
    const planned = origin?.plannedChapter;
    if (!origin || !planned) return false;
    return chapter.projectId === projectId && origin.projectId === projectId
      && planned.snapshotId === detail.preview.plan.snapshotId && planned.primaryPath === step.primaryPath
      && planned.question === step.question && samePlannedTarget(planned.target, {
        revisionId: revision.revision.revisionId, recordSha256: detail.recordSha256, stepId: step.stepId,
      });
  }).sort((left, right) => right.createdAtUnixMs - left.createdAtUnixMs || right.revisionNumber - left.revisionNumber) : [];
  const locked = discarding || decisionBusy !== null;
  const alreadyAccepted = detail ? revisions.find((item) => item.revision.draftId === detail.preview.plan.draftId) : undefined;
  const availableParents = revisions.filter((item) => item.isHead && item.revision.question === detail?.preview.plan.question);
  async function explainStep() {
    if (!native || locked || mutating.current || !revision || !step || !detail || detail.preview.plan.snapshotId !== snapshotId) return;
    mutating.current = true;
    setDecisionBusy('prepare-step'); setError(null);
    const version = readVersion.current;
    const selected = selectionRef.current;
    const target = { revisionId: revision.revision.revisionId, recordSha256: detail.recordSha256, stepId: step.stepId };
    try {
      const scope = await preparePlannedChapterScope(projectId, target);
      if (!mounted.current || readVersion.current !== version || selectionRef.current !== selected) return;
      const expectedPaths = new Set([step.primaryPath, ...step.claim.anchors.map((anchor) => anchor.path)]);
      if (scope.target.revisionId !== target.revisionId || scope.target.recordSha256 !== target.recordSha256
        || scope.target.stepId !== target.stepId || scope.snapshotId !== snapshotId || scope.primaryPath !== step.primaryPath
        || scope.question !== step.question || scope.planQuestion !== detail.preview.plan.question
        || scope.selectedPaths.length !== expectedPaths.size || new Set(scope.selectedPaths).size !== expectedPaths.size
        || scope.selectedPaths.some((path) => !expectedPaths.has(path))) throw new Error('The prepared chapter does not match this accepted reading step. Open the original plan revision again.');
      onGeneratePlannedChapter(scope);
    } catch (failure) {
      if (mounted.current && readVersion.current === version) setError(errorMessageOr(failure, 'This accepted reading step could not be prepared. No agent request was started.'));
    } finally {
      mutating.current = false;
      if (mounted.current) setDecisionBusy(null);
    }
  }
  useEffect(() => {
    if (focusStep.current) { focusStep.current = false; heading.current?.focus(); }
  }, [position, detail]);

  const refresh = useCallback(async () => {
    if (!native) return;
    const version = ++epoch.current;
    setLoading(true); setListError(null); setHistoryError(null);
    try {
      const [draftsResult, revisionsResult] = await Promise.allSettled([
        listRepositoryPlanDrafts(projectId), listRepositoryPlanRevisions(projectId),
      ]);
      if (!mounted.current || epoch.current !== version) return;
      if (draftsResult.status === 'fulfilled') setSummaries(draftsResult.value);
      else setListError(errorMessageOr(draftsResult.reason, 'Saved drafts could not be listed. Existing records were preserved.'));
      if (revisionsResult.status === 'fulfilled') setRevisions(revisionsResult.value);
      else setHistoryError(errorMessageOr(revisionsResult.reason, 'Accepted plan history could not be listed. Existing revisions were preserved.'));
      const selected = selectionRef.current;
      const missing = selected?.startsWith('plan:') && draftsResult.status === 'fulfilled' && !draftsResult.value.some((item) => item.draftId === selected)
        || selected?.startsWith('plan-revision:') && revisionsResult.status === 'fulfilled' && !revisionsResult.value.some((item) => item.revision.revisionId === selected);
      if (missing) {
        ++readVersion.current; ++parentVersion.current; selectionRef.current = null;
        setSelectedId(null); setDetail(null); setRevision(null); setParent(null); setParentId(''); setParentLoading(false); setAcceptConsent(false); setReading(false);
      }
    } catch (failure) {
      if (mounted.current && epoch.current === version) setListError(errorMessageOr(failure, 'Saved plans could not be listed. Existing local records were preserved.'));
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
      mounted.current = false; ++epoch.current; ++readVersion.current; ++parentVersion.current;
      window.removeEventListener('code-atlas:repository-plans-changed', changed);
    };
  }, [projectId, snapshotId, refresh]);

  async function open(draftId: string) {
    if (!native || mutating.current) return;
    const version = ++readVersion.current;
    selectionRef.current = draftId;
    setSelectedId(draftId); setReading(true); setDetail(null); setError(null); setNotice(null); setPosition(0);
    ++parentVersion.current; setRevision(null); setParent(null); setParentId(''); setParentLoading(false); setAcceptConsent(false);
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
    if (!native || mutating.current || confirmDiscard !== draftId) return;
    mutating.current = true;
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
    } finally { mutating.current = false; if (mounted.current) setDiscarding(false); }
  }

  async function openRevision(revisionId: string) {
    if (!native || mutating.current) return;
    const version = ++readVersion.current; ++parentVersion.current;
    selectionRef.current = revisionId;
    setSelectedId(null); setDetail(null); setRevision(null); setParent(null); setParentId(''); setParentLoading(false); setAcceptConsent(false);
    setReading(true); setError(null); setNotice(null); setPosition(0);
    try {
      const loaded = await loadRepositoryPlanRevision(projectId, revisionId);
      if (!mounted.current || readVersion.current !== version || selectionRef.current !== revisionId) return;
      if (loaded.revision.revisionId !== revisionId || loaded.plan.preview.plan.projectId !== projectId
        || loaded.plan.recordSha256 !== loaded.revision.recordSha256) throw new Error('The returned accepted plan has a different identity.');
      setDetail(loaded.plan); setRevision(loaded);
    } catch (failure) {
      if (mounted.current && readVersion.current === version) setError(errorMessageOr(failure, 'The accepted plan could not be revalidated. Existing history was preserved.'));
    } finally { if (mounted.current && readVersion.current === version) setReading(false); }
  }

  async function chooseParent(id: string) {
    if (!native || mutating.current) return;
    const version = ++parentVersion.current;
    setParentId(id); setParent(null); setParentLoading(!!id); setAcceptConsent(false); setError(null);
    if (!id) return;
    try {
      const loaded = await loadRepositoryPlanRevision(projectId, id);
      if (!mounted.current || parentVersion.current !== version) return;
      if (loaded.revision.revisionId !== id || !loaded.isHead || loaded.plan.preview.plan.projectId !== projectId
        || loaded.revision.question !== detail?.preview.plan.question || loaded.plan.recordSha256 !== loaded.revision.recordSha256) {
        throw new Error('This plan is no longer the current revision for the same question. Refresh and review history again.');
      }
      setParent(loaded);
    } catch (failure) {
      if (mounted.current && parentVersion.current === version) setError(errorMessageOr(failure, 'The selected parent could not be reviewed. No acceptance was written.'));
    } finally { if (mounted.current && parentVersion.current === version) setParentLoading(false); }
  }

  const notifyPlans = () => window.dispatchEvent(new CustomEvent('code-atlas:repository-plans-changed', { detail: { projectId } }));

  async function accept() {
    if (!native || !detail || revision || mutating.current || !acceptConsent || loading || historyError || alreadyAccepted
      || parentId && (!parent || parent.revision.revisionId !== parentId || !availableParents.some((item) => item.revision.revisionId === parentId))) return;
    const reviewed = detail;
    mutating.current = true; setDecisionBusy('accept'); setError(null); setNotice(null);
    try {
      const saved = await acceptRepositoryPlanDraft({ projectId, draftId: reviewed.preview.plan.draftId,
        expectedRecordSha256: reviewed.recordSha256, basedOnRevisionId: parentId || null });
      if (!mounted.current) return;
      if (saved.plan.preview.plan.projectId !== projectId || saved.revision.draftId !== reviewed.preview.plan.draftId
        || saved.revision.recordSha256 !== reviewed.recordSha256 || saved.plan.recordSha256 !== reviewed.recordSha256
        || saved.revision.basedOnRevisionId !== (parentId || null)) throw new Error('The acceptance reply did not match the reviewed record. Refresh history before retrying.');
      ++readVersion.current; selectionRef.current = saved.revision.revisionId;
      setSelectedId(null); setDetail(saved.plan); setRevision(saved); setAcceptConsent(false);
      setNotice(`Accepted plan revision ${saved.revision.revisionNumber} is saved. Earlier revisions, the original draft and separate recovery notices were preserved.`);
    } catch (failure) {
      if (mounted.current) setError(errorMessageOr(failure, 'Acceptance could not be confirmed. An immutable revision may already exist; refresh history before retrying.'));
    } finally { mutating.current = false; if (mounted.current) setDecisionBusy(null); notifyPlans(); }
  }

  async function resolveNotice() {
    if (!native || !detail || mutating.current) return;
    const source: ReviewedPlanSource = revision
      ? { kind: 'revision', revisionId: revision.revision.revisionId, recordSha256: detail.recordSha256 }
      : { kind: 'draft', draftId: detail.preview.plan.draftId, recordSha256: detail.recordSha256 };
    mutating.current = true; setDecisionBusy('resolve'); setError(null); setNotice(null);
    try {
      const removed = await resolveRepositoryPlanNotice(projectId, source);
      if (mounted.current) setNotice(removed ? 'This plan’s matching generation notice was resolved. The plan and unrelated notices were preserved.' : 'No matching generation notice remains. The plan and unrelated notices were preserved.');
    } catch (failure) {
      if (mounted.current) setError(errorMessageOr(failure, 'Notice resolution could not be confirmed. Refresh recovery before retrying.'));
    } finally { mutating.current = false; if (mounted.current) setDecisionBusy(null); notifyPlans(); }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!native || mutating.current) return;
    setFormError(null);
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
          <button className="text-button" disabled={locked} onClick={() => onOpenFile(anchor.path, anchor.snapshotId, codeLensRangeFromChapterAnchor(anchor))}><FileCode2 size={14} /> {anchor.path}:{anchor.startLine}–{anchor.endLine}</button>
          <pre>{anchor.excerpt}</pre>
        </div>)}
      </details>
    </article>;
  }

  return <section className="repository-journeys repository-plans" aria-labelledby="learning-plans-title">
    <header className="plan-heading"><div><h2 id="learning-plans-title">Learning plans</h2><p>Ask a question, choose source, then use Codex or Claude to plan a reading sequence.</p></div>
      <button className="primary-button" disabled={!native || locked} onClick={() => setCreating(!creating)}><Sparkles size={15} /> {creating ? 'Hide new plan' : 'New learning plan'}</button></header>
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
      <button type="submit" className="primary-button" disabled={!native || locked || !paths.length || bytes > 250_000}>Choose agent and review request <ArrowRight size={14} /></button>
    </form>}
    <div className="plan-saved-heading"><h3>Saved drafts</h3><button className="text-button" disabled={!native || loading || locked} onClick={() => { setError(null); void refresh(); }}><RefreshCw size={14} /> Refresh</button></div>
    <p className="provider-small-note">This list is metadata only. Opening a draft rechecks its original proposal and captured evidence before showing citations.</p>
    {loading && <p role="status">Loading saved plan metadata…</p>}
    {!loading && native && !summaries.length && !listError && <p>No saved plans yet. Start with one question and a small source selection.</p>}
    <ul className="plan-saved-list">{summaries.map((item) => <li key={item.draftId}>
      <button className="text-button" aria-current={selectedId === item.draftId ? 'true' : undefined} disabled={locked || !!item.unavailableReason} onClick={() => { void open(item.draftId); }}>{item.title}</button>
      <small>{item.unavailableReason ?? `${item.providerId === 'codex' ? 'Codex' : 'Claude'} · ${item.modelId} · ${item.snapshotId === snapshotId ? 'current snapshot' : 'earlier snapshot'}`}</small>
      {confirmDiscard === item.draftId ? <span><span>Discard this local draft?</span> <button className="text-button" disabled={locked} onClick={() => { void discard(item.draftId); }}>Discard draft</button> <button className="text-button" disabled={locked} onClick={() => setConfirmDiscard(null)}>Keep</button></span>
        : <button className="text-button" disabled={locked} onClick={() => setConfirmDiscard(item.draftId)} aria-label={`Discard ${item.title}`}><Trash2 size={14} /> Discard</button>}
    </li>)}</ul>
    <div className="plan-saved-heading"><h3>Accepted plan history</h3><span>Immutable revisions · chapters are reviewed separately</span></div>
    {!loading && !historyError && !revisions.length && <p>No accepted plans yet. Open a saved draft, review its steps and evidence, then accept it explicitly.</p>}
    <ul className="plan-saved-list">{revisions.map((item) => <li key={item.revision.revisionId}>
      <button className="text-button" disabled={locked} aria-current={revision?.revision.revisionId === item.revision.revisionId ? 'true' : undefined} onClick={() => { void openRevision(item.revision.revisionId); }}>{item.revision.title} · revision {item.revision.revisionNumber}</button>
      <small>{item.isHead ? 'Latest in this plan' : 'Earlier revision'} · {item.revision.snapshotId === snapshotId ? 'current snapshot' : 'earlier snapshot'} · {item.revision.providerId === 'codex' ? 'Codex' : 'Claude'}</small>
    </li>)}</ul>
    {reading && <p role="status">Revalidating the original question, proposal and captured source…</p>}
    {detail && step && <section className="plan-reader" aria-label="Saved learning plan review">
      <h3>{detail.preview.plan.title}</h3><p>{detail.preview.plan.question}</p>
      <p className="provider-small-note">{revision ? `Accepted revision ${revision.revision.revisionNumber}` : 'Unaccepted plan'} · {detail.preview.plan.plannedSelectedPaths.length} primary files · {detail.preview.plan.citedSelectedPaths.length} cited files · {detail.preview.plan.omittedSelectedPaths.length} selected files without a step. {detail.scope.coverage.unselectedEligibleFileCount} other eligible captured files remain outside the selection.</p>
      <p className="provider-small-note">Capture: {detail.scope.captureStatus} · index: {detail.scope.indexStatus} · {detail.scope.coverage.skippedPathCount} skipped paths · {detail.scope.coverage.excludedCapturedPathCount} excluded captured paths. This is not a whole-repository explanation.</p>
      <details><summary>Snapshot and recorded agent settings</summary><p><code>{detail.preview.plan.snapshotId}</code> · {detail.preview.modelId}</p><pre>{JSON.stringify(detail.settings, null, 2)}</pre><p>These are recorded dispatch labels, not tamper-proof account proof or authorization for a new request. Capture: {detail.scope.captureStatus}; index: {detail.scope.indexStatus}.</p></details>
      <label>Reading step<select value={position} disabled={locked} onChange={(event) => { focusStep.current = true; setPosition(Number(event.target.value)); }}>
        {detail.preview.plan.steps.map((item, index) => <option key={item.stepId} value={index}>{index + 1}. {item.question}</option>)}
      </select></label>
      <h4 ref={heading} tabIndex={-1}>{position + 1}. {step.question}</h4>
      {step.prerequisiteStepIds.length > 0 && <p>Builds on {step.prerequisiteStepIds.map((id) => detail.preview.plan.steps.findIndex((item) => item.stepId === id) + 1).join(', ')}. This is a reading order, not a proven runtime call.</p>}
      {claim(step.claim)}
      <div className="plan-reader-actions"><button className="secondary-button" disabled={locked || position === 0} onClick={() => { focusStep.current = true; setPosition(position - 1); }}><ArrowLeft size={14} /> Back</button>
        <button className="secondary-button" disabled={locked || position + 1 >= detail.preview.plan.steps.length} onClick={() => { focusStep.current = true; setPosition(position + 1); }}>Next <ArrowRight size={14} /></button>
        {revision ? <button className="primary-button" disabled={locked || detail.preview.plan.snapshotId !== snapshotId} onClick={() => { void explainStep(); }}><Sparkles size={14} /> {decisionBusy === 'prepare-step' ? 'Preparing step…' : 'Explain this step with an agent'}</button>
          : <button className="secondary-button" disabled={locked || detail.preview.plan.snapshotId !== snapshotId} onClick={() => onGenerateFile(step.primaryPath, [...new Set([step.primaryPath, ...step.claim.anchors.map((anchor) => anchor.path)])])}><Sparkles size={14} /> General file explanation</button>}</div>
      {!revision && <p>Accept and open this plan revision to generate chapters that answer its exact step questions. A general file explanation does not carry this reading plan.</p>}
      {revision && <section aria-label="Saved chapters with this learning step origin"><h4>Chapters started from this step</h4>
        {chaptersLoading ? <p role="status">Reading saved chapter history…</p> : chaptersError ? <p role="alert">{chaptersError}</p> : <>
          {linkedChapters.length ? <ul>{linkedChapters.slice(0, 20).map((chapter) => <li key={chapter.revisionId}>
            <strong>{chapter.title}</strong> · {chapter.lifecycle} · revision {chapter.revisionNumber} · {chapter.snapshotId === detail.preview.plan.snapshotId ? 'original snapshot' : 'another snapshot'}
            <GenerationRecord origin={chapter.generationOrigin} />
            {chapter.lifecycle === 'accepted' && <button className="secondary-button" disabled={locked} onClick={() => onReadChapter(chapter)}>Read this accepted revision</button>}
            <button className="text-button" disabled={locked} onClick={() => onReviewChapter(chapter)}>Review revision</button>
          </li>)}</ul> : <p>No saved chapters record this exact plan revision and step. Older same-file chapters are not automatically linked.</p>}
          {linkedChapters.length > 20 && <p>Showing 20 of {linkedChapters.length} linked revisions. All remain in Edit.</p>}
        </>}
        <p className="provider-small-note">This is recorded origin, not step completion. Later edits and source changes may follow; chapter acceptance and reading progress remain separate.</p>
      </section>}
      {detail.preview.plan.snapshotId !== snapshotId && <p>Open a citation to switch to this plan’s original snapshot before requesting a file explanation.</p>}
      <details><summary>Known unknowns and omitted selected files</summary>{detail.preview.plan.unknowns.map(claim)}<ul>{detail.preview.plan.omittedSelectedPaths.map((path) => <li key={path}><code>{path}</code></li>)}</ul></details>
      {!revision && alreadyAccepted && <section className="plan-decision"><h4>This draft already has an accepted revision</h4>
        <p>Accepting it again cannot create a second decision or overwrite history. The saved draft can be discarded independently.</p>
        <button className="secondary-button" disabled={locked} onClick={() => { void openRevision(alreadyAccepted.revision.revisionId); }}>Open accepted revision {alreadyAccepted.revision.revisionNumber}</button>
      </section>}
      {!revision && !alreadyAccepted && <section className="plan-decision" aria-label="Accept this reviewed plan">
        <h4>Accept this plan</h4><p>Acceptance records this exact plan locally. It does not generate or accept file chapters, prove comprehension or resolve a generation notice.</p>
        <label>Revision destination<select value={parentId} disabled={locked || loading || !!historyError} onChange={(event) => { void chooseParent(event.target.value); }}>
          <option value="">New independent plan</option>
          {parentId && !availableParents.some((item) => item.revision.revisionId === parentId) && <option value={parentId} disabled>Previous choice changed · review the current history</option>}
          {availableParents.map((item) => <option key={item.revision.revisionId} value={item.revision.revisionId}>Update {item.revision.title} · after revision {item.revision.revisionNumber}</option>)}
        </select></label>
        <p>Only the latest revision of a plan with the same question can be updated. A different question starts a new plan.</p>
        {parentLoading && <p role="status">Revalidating the selected previous revision…</p>}
        {parent && <details className="plan-parent-review"><summary>Compare with accepted revision {parent.revision.revisionNumber}</summary>
          <p>Previous snapshot: <code>{parent.revision.snapshotId}</code>. Proposed snapshot: <code>{detail.preview.plan.snapshotId}</code>.</p>
          <p>{parent.plan.preview.plan.steps.length} previous steps → {detail.preview.plan.steps.length} proposed steps. Every previous revision remains unchanged. This acceptance does not claim selective regeneration or preserved identities between separate model generations.</p>
          {parent.plan.preview.plan.steps.map((item, index) => <section key={item.stepId}><h5>{index + 1}. {item.question}</h5>{claim(item.claim)}</section>)}
          <h5>Previous known unknowns</h5>{parent.plan.preview.plan.unknowns.map(claim)}
        </details>}
        <label className="provider-consent"><input type="checkbox" checked={acceptConsent} disabled={locked || loading || !!historyError || parentLoading || !!parentId && !parent} onChange={(event) => setAcceptConsent(event.target.checked)} />I reviewed this plan’s steps, source evidence, known unknowns and coverage limits{parent ? ', and the selected previous revision' : ''}.</label>
        <button className="primary-button" disabled={locked || loading || !!historyError || !acceptConsent || parentLoading || !!parentId && (!parent || !availableParents.some((item) => item.revision.revisionId === parentId))} onClick={() => { void accept(); }}>{decisionBusy === 'accept' ? 'Recording acceptance…' : parentId ? 'Accept as next revision' : 'Accept as a new plan'}</button>
      </section>}
      {revision && <p className="provider-small-note">Immutable accepted revision <code>{revision.revision.revisionId}</code>{revision.revision.basedOnRevisionId ? ` · based on ${revision.revision.basedOnRevisionId}` : ' · first revision of this plan'}. Deleting the original draft cannot remove this accepted copy.</p>}
      <p className="provider-small-note">Steps advance only when you choose. Plan acceptance does not accept chapters, complete coverage or mark anything read. Resolve only this plan’s matching generation notice with the explicit action below.</p>
      <button className="secondary-button" disabled={locked} onClick={() => { void resolveNotice(); }}>{decisionBusy === 'resolve' ? 'Resolving matching notice…' : 'Resolve this plan’s generation notice'}</button>
    </section>}
    {listError && <p className="provider-error-text" role="alert">{listError}</p>}
    {historyError && <p className="provider-error-text" role="alert">{historyError}</p>}
    {error && <p className="provider-error-text" role="alert">{error}</p>}
    {notice && <p role="status">{notice}</p>}
  </section>;
}
