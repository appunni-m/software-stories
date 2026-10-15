import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import { ArrowLeft, ArrowRight, BookOpen, FileCode2, RotateCcw, Sparkles, Trash2 } from 'lucide-react';
import { isNativeRuntime, loadSnapshotIndex, type ChapterRevision, type SnapshotIndex, type SnapshotManifest } from './lib/commands';
import { codeLensRangeFromIndexedAnchor } from './lib/codeLens';
import { errorMessageOr } from './lib/errorMessage';
import type { SourceLineRange } from './lib/navigation';
import {
  createRepositoryJourney, journeyFileRevision, journeyGenerationPaths, journeySourceFiles,
  journeyStartingSymbols, journeyStepIssue, journeyStorageKey, loadJourneyStore, saveJourneyStore,
  type JourneyStore, type RepositoryJourney,
} from './lib/repositoryJourneys';

interface Props {
  manifest: SnapshotManifest;
  revisions: ChapterRevision[];
  revisionsReady: boolean;
  returnFocusTarget?: string;
  onOpenFile: (path: string, snapshotId?: string, range?: SourceLineRange) => void;
  onGenerateFile: (path: string, selectedPaths: string[]) => void;
  onReadRevision: (revision: ChapterRevision) => void;
  onReviewRevision: (revision: ChapterRevision) => void;
}

export function RepositoryJourneys({ manifest, revisions, revisionsReady, returnFocusTarget, onOpenFile, onGenerateFile, onReadRevision, onReviewRevision }: Props) {
  const { projectId, snapshotId } = manifest.summary;
  const [store, setStore] = useState<JourneyStore | null>(null);
  const storeRef = useRef<JourneyStore | null>(null);
  const [storageError, setStorageError] = useState<string | null>(null);
  const [selectedJourneyId, setSelectedJourneyId] = useState<string | null>(null);
  const [removed, setRemoved] = useState<RepositoryJourney | null>(null);
  const [index, setIndex] = useState<SnapshotIndex | null>(null);
  const [indexLoading, setIndexLoading] = useState(isNativeRuntime());
  const [indexError, setIndexError] = useState<string | null>(null);
  const [indexRefresh, setIndexRefresh] = useState(0);
  const [question, setQuestion] = useState('');
  const [startingPath, setStartingPath] = useState('');
  const [startingSymbolId, setStartingSymbolId] = useState('');
  const [formError, setFormError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const focusStep = useRef(false);
  const restoreTarget = useRef(returnFocusTarget);
  const stepHeading = useRef<HTMLHeadingElement>(null);
  const files = useMemo(() => journeySourceFiles(manifest), [manifest]);
  const matchingFiles = files.filter((file) => file.path.toLowerCase().includes(startingPath.trim().toLowerCase())).slice(0, 60);
  const symbols = useMemo(() => journeyStartingSymbols(manifest, index, startingPath), [manifest, index, startingPath]);
  const journey = store?.journeys.find((item) => item.journeyId === selectedJourneyId) ?? store?.journeys[0] ?? null;
  const stepIndex = journey ? journey.steps.findIndex((item) => item.stepId === journey.selectedStepId) : -1;
  const step = journey?.steps[stepIndex] ?? null;
  const issue = journey && step ? journeyStepIssue(journey, step, manifest, index) : null;
  const sourceMatches = Boolean(step && files.some((file) => file.path === step.path && file.sha256 === step.sourceSha256));
  const requestPaths = journey && step ? journeyGenerationPaths(journey, step, manifest) : [];
  const generationPaths = issue && step ? requestPaths.filter((path) => path === step.path) : requestPaths;
  const revision = journey && step && revisionsReady && sourceMatches ? journeyFileRevision(journey, step, revisions) : null;
  const plannedFileCount = new Set(journey?.steps.map((item) => item.path)).size;
  const prerequisite = journey?.steps.find((item) => item.stepId === step?.prerequisiteStepId) ?? null;

  const reloadSaved = useCallback(() => {
    try {
      const loaded = loadJourneyStore(projectId, snapshotId);
      const restoredJourney = loaded.journeys.find((item) => item.steps.some((savedStep) => restoreTarget.current?.startsWith(`journey:${item.journeyId}:${savedStep.stepId}:`)));
      if (restoredJourney) {
        const restoredStep = restoredJourney.steps.find((savedStep) => restoreTarget.current?.startsWith(`journey:${restoredJourney.journeyId}:${savedStep.stepId}:`))!;
        loaded.journeys = loaded.journeys.map((item) => item.journeyId === restoredJourney.journeyId ? { ...item, selectedStepId: restoredStep.stepId } : item);
      }
      storeRef.current = loaded;
      setStore(loaded);
      setSelectedJourneyId((current) => restoredJourney?.journeyId ?? (loaded.journeys.some((item) => item.journeyId === current) ? current : loaded.journeys[0]?.journeyId ?? null));
      restoreTarget.current = undefined;
      setStorageError(null);
    } catch (error) {
      storeRef.current = null;
      setStore(null);
      setStorageError(errorMessageOr(error, 'Saved journey plans could not be read. Their stored data was preserved.'));
    }
  }, [projectId, snapshotId]);

  useEffect(() => {
    reloadSaved();
    const refresh = (event: StorageEvent) => { if (event.key === journeyStorageKey(projectId, snapshotId) || event.key === null) reloadSaved(); };
    window.addEventListener('storage', refresh);
    return () => window.removeEventListener('storage', refresh);
  }, [projectId, snapshotId, reloadSaved]);

  useEffect(() => {
    let active = true;
    setIndex(null);
    setIndexError(null);
    setIndexLoading(isNativeRuntime());
    if (isNativeRuntime()) void loadSnapshotIndex(projectId, snapshotId)
      .then((loaded) => {
        if (!active) return;
        if (loaded && (loaded.projectId !== projectId || loaded.snapshotId !== snapshotId)) throw new Error('The local index belongs to another snapshot.');
        setIndex(loaded);
      })
      .catch(() => { if (active) setIndexError('The local index could not be loaded. Retry, or create a single-file reading step.'); })
      .finally(() => { if (active) setIndexLoading(false); });
    return () => { active = false; };
  }, [projectId, snapshotId, indexRefresh]);

  useEffect(() => {
    if (!focusStep.current) return;
    focusStep.current = false;
    stepHeading.current?.focus();
  }, [step?.stepId, selectedJourneyId]);

  function persist(journeys: RepositoryJourney[]): boolean {
    const current = storeRef.current;
    if (!current) return false;
    try {
      const saved = saveJourneyStore(projectId, snapshotId, journeys, current.token);
      storeRef.current = saved;
      setStore(saved);
      setStorageError(null);
      return true;
    } catch (error) {
      setStorageError(errorMessageOr(error, 'This plan change could not be saved. Previous progress was preserved.'));
      return false;
    }
  }

  function updateJourney(next: RepositoryJourney) {
    const current = storeRef.current;
    if (current) return persist(current.journeys.map((item) => item.journeyId === next.journeyId ? next : item));
    return false;
  }

  function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!storeRef.current || indexLoading) return;
    setFormError(null);
    try {
      const next = createRepositoryJourney(manifest, index, { question, path: startingPath, symbolId: startingSymbolId || null });
      if (persist([next, ...storeRef.current.journeys])) {
        setSelectedJourneyId(next.journeyId);
        setCreating(false);
        setRemoved(null);
      }
    } catch (error) { setFormError(errorMessageOr(error, 'The source reading plan could not be created.')); }
  }

  function chooseStep(stepId: string) {
    if (journey && updateJourney({ ...journey, selectedStepId: stepId })) focusStep.current = true;
  }

  function chooseJourney(journeyId: string) {
    const current = storeRef.current;
    const chosen = current?.journeys.find((item) => item.journeyId === journeyId);
    if (current && chosen && persist([chosen, ...current.journeys.filter((item) => item.journeyId !== journeyId)])) {
      setSelectedJourneyId(journeyId);
      focusStep.current = true;
    }
  }

  function continueReading() {
    if (!journey || !step) return;
    const next = journey.steps[stepIndex + 1];
    if (updateJourney({ ...journey, readStepIds: [...new Set([...journey.readStepIds, step.stepId])], selectedStepId: next?.stepId ?? step.stepId })) {
      focusStep.current = Boolean(next);
    }
  }

  function removeJourney() {
    if (!journey || !storeRef.current) return;
    if (persist(storeRef.current.journeys.filter((item) => item.journeyId !== journey.journeyId))) {
      setRemoved(journey);
      setSelectedJourneyId(null);
    }
  }

  return <section className="repository-journeys" aria-labelledby="repository-journeys-title">
    <header className="journey-header">
      <div><span className="guide-home-kicker">FOLLOW A BEHAVIOR</span><h2 id="repository-journeys-title">Read the code as a journey</h2><p>Choose a starting declaration. Follow its verified call links, inspect the code, and request each file explanation when you need it.</p></div>
      {store && <button className="secondary-button" onClick={() => setCreating((value) => !value)} aria-expanded={creating}>{creating ? 'Close planner' : 'Plan a journey'}</button>}
    </header>
    {storageError && <p className="journey-error" role="alert">{storageError} <button className="text-button" onClick={reloadSaved}>Reload saved plans</button></p>}
    {removed && store && <p className="journey-note" role="status">Plan removed. Saved file explanations remain available. <button className="text-button" onClick={() => {
      const current = storeRef.current;
      if (current && persist([removed, ...current.journeys])) { setSelectedJourneyId(removed.journeyId); setRemoved(null); }
    }}>Undo</button></p>}
    {store && creating && <form className="journey-form" onSubmit={create}>
      <label>What do you want to understand?<input value={question} onChange={(event) => setQuestion(event.target.value)} minLength={8} maxLength={300} required placeholder="How does a request reach storage?" /></label>
      <label>Starting file<input value={startingPath} onChange={(event) => { setStartingPath(event.target.value); setStartingSymbolId(''); }} list="journey-start-files" required placeholder="Search or enter a captured file path" autoComplete="off" spellCheck={false} /></label>
      <datalist id="journey-start-files">{matchingFiles.map((file) => <option key={file.path} value={file.path} />)}</datalist>
      <label>Starting declaration<select value={startingSymbolId} disabled={indexLoading || symbols.length === 0} onChange={(event) => setStartingSymbolId(event.target.value)}>
        <option value="">{indexLoading ? 'Loading local index…' : 'File only · no inferred call path'}</option>
        {symbols.map((symbol) => <option key={symbol.symbolId} value={symbol.symbolId}>{symbol.qualifiedName} · line {symbol.anchor.startLine}</option>)}
      </select></label>
      {indexError && <p className="journey-note" role="status">{indexError} <button type="button" className="text-button" onClick={() => setIndexRefresh((value) => value + 1)}>Retry index</button></p>}
      {!indexLoading && !index && !indexError && <p className="journey-note">No saved local index is available. You can plan a file reading step; index the snapshot in Files to discover supported call links.</p>}
      {formError && <p className="journey-error" role="alert">{formError}</p>}
      <div><button className="primary-button" type="submit" disabled={indexLoading || files.length === 0}>Create reading plan</button><small>Saved on this Mac. Creating a plan sends no AI request.</small></div>
    </form>}
    {journey && step && <>
      <div className="journey-selector">
        <label>Saved journey<select value={journey.journeyId} onChange={(event) => chooseJourney(event.target.value)}>{store!.journeys.map((item) => <option key={item.journeyId} value={item.journeyId}>{item.question}</option>)}</select></label>
        <button className="text-button" onClick={() => chooseStep(journey.steps.find((item) => !journey.readStepIds.includes(item.stepId))?.stepId ?? journey.steps[0].stepId)}><RotateCcw size={13} /> Resume next unread step</button>
        <button className="icon-button" onClick={removeJourney} aria-label="Remove this reading plan" title="Remove this reading plan"><Trash2 size={14} /></button>
      </div>
      <p className="journey-note">{journey.readStepIds.length} / {journey.steps.length} steps marked read · {plannedFileCount} / {files.length} captured text files in this journey · {manifest.skipped.length} uncaptured paths. Reading progress does not count files as explained.</p>
      <div className="journey-layout">
        <nav className="journey-step-list" aria-label="Journey reading steps">{journey.steps.map((item, position) => <button key={item.stepId} aria-current={step.stepId === item.stepId ? 'step' : undefined} onClick={() => chooseStep(item.stepId)}><span>{position + 1}</span><span><strong>{item.symbolName ?? item.path}</strong><small>{journey.readStepIds.includes(item.stepId) ? 'Read' : 'Not marked read'} · {item.path}</small></span></button>)}</nav>
        <section className="journey-step-detail" aria-labelledby="journey-step-title">
          <span className="guide-home-kicker">STEP {stepIndex + 1} OF {journey.steps.length}</span>
          <h3 id="journey-step-title" ref={stepHeading} tabIndex={-1}>{step.question}</h3>
          <code>{step.path}{step.anchor ? `:${step.anchor.startLine}–${step.anchor.endLine}` : ''}</code>
          {prerequisite && <p className="journey-note">Read <button className="text-button" onClick={() => chooseStep(prerequisite.stepId)}>{prerequisite.symbolName ?? prerequisite.path}</button> first. This reading step follows a statically resolved call, rather than establishing the full runtime sequence.</p>}
          {issue && <p className="journey-error" role="status">{issue} <button className="text-button" onClick={() => setIndexRefresh((value) => value + 1)}>Reload index</button></p>}
          <div className="journey-step-actions">
            <button className="secondary-button" data-focus-key={`journey:${journey.journeyId}:${step.stepId}:source`} disabled={!sourceMatches} onClick={() => onOpenFile(step.path, snapshotId, step.anchor && !issue ? codeLensRangeFromIndexedAnchor(step.anchor) : undefined)}><FileCode2 size={14} /> Explore source</button>
            {step.call && !issue && <button className="secondary-button" data-focus-key={`journey:${journey.journeyId}:${step.stepId}:call`} onClick={() => onOpenFile(step.call!.evidence.path, snapshotId, codeLensRangeFromIndexedAnchor(step.call!.evidence))}>Inspect the call · line {step.call.evidence.startLine}</button>}
            {revision?.lifecycle === 'accepted' ? <button className="primary-button" data-focus-key={`journey:${journey.journeyId}:${step.stepId}:read`} onClick={() => onReadRevision(revision)}><BookOpen size={14} /> Read file explanation</button>
              : revision ? <button className="primary-button" data-focus-key={`journey:${journey.journeyId}:${step.stepId}:review`} onClick={() => onReviewRevision(revision)}>Review file draft</button>
                : <button className="primary-button" disabled={!isNativeRuntime() || !sourceMatches || !revisionsReady || generationPaths.length === 0} onClick={() => onGenerateFile(step.path, generationPaths)}><Sparkles size={14} /> Generate file explanation</button>}
          </div>
          {!revisionsReady && <p className="journey-note">Waiting for saved explanations. Source inspection and reading progress remain available.</p>}
          {generationPaths.length === 0 && sourceMatches && <p className="journey-note">This file exceeds the current per-request source limit. Inspect its source or explain a bounded range from Files.</p>}
          {!revision && generationPaths.length > 0 && <details className="journey-context"><summary>Next generation scope · {generationPaths.length} file{generationPaths.length === 1 ? '' : 's'}</summary><p>The existing backend chooser and exact-context review open before a request. Supporting files come from the adjacent reading steps, within the 12-file / 250 KB request limit.</p><ul>{generationPaths.map((path) => <li key={path}><code>{path}</code></li>)}</ul></details>}
          <footer className="journey-reader-controls"><button className="secondary-button" disabled={stepIndex === 0} onClick={() => chooseStep(journey.steps[stepIndex - 1].stepId)}><ArrowLeft size={14} /> Previous</button><span role="status">{journey.readStepIds.includes(step.stepId) ? 'Marked read' : 'Read at your own pace'}</span><button className="primary-button" onClick={continueReading}>{stepIndex + 1 < journey.steps.length ? 'Mark read and continue' : 'Mark read'} <ArrowRight size={14} /></button></footer>
        </section>
      </div>
      <details className="journey-boundaries"><summary>What this journey leaves open{journey.truncated ? ' · bounded plan' : ''}</summary>
        <p>This plan follows at most 24 declarations. {Math.max(0, files.length - plannedFileCount)} captured text files are outside this journey and remain available in Files and file-by-file coverage.{journey.truncated ? ' Additional call paths or boundary notes were omitted by the plan limit; create another journey from a different starting declaration.' : ''}</p>
        <ul>{journey.boundaries.map((note, position) => <li key={position}>{note}</li>)}</ul>
      </details>
    </>}
  </section>;
}
