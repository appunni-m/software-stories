import { useEffect, useMemo, useRef, useState } from 'react';
import {
  discardProviderRequestReview, loadSnapshotManifest,
  type ClaudeGenerationSettings, type ClaudeModelDiscoveryObservation, type ClaudeReasoningEffort,
  type ProviderFailure, type ProviderRequestReview, type SnapshotManifest,
} from './lib/commands';
import {
  claudeOperationLabel, claudeReviewScopeKey, gitChangeLabel, isClaudeChapterScope,
  matchesClaudeOperationReview, prepareClaudeOperationReview, type ClaudeOperationReviewMode,
} from './lib/claudeReview';

type Selection = Pick<ClaudeGenerationSettings, 'modelId' | 'effort' | 'maxTurns'>;
const preferenceKey = 'code-atlas:claude-chapter-preferences';

function isEffort(value: unknown): value is ClaudeReasoningEffort {
  return value === 'low' || value === 'medium' || value === 'high' || value === 'xhigh' || value === 'max';
}

function preferences(): Selection {
  try {
    const text = localStorage.getItem(preferenceKey) ?? '';
    const value: unknown = text.length <= 1024 ? JSON.parse(text) : null;
    if (value && typeof value === 'object' && 'modelId' in value && 'effort' in value && 'maxTurns' in value
      && typeof value.modelId === 'string' && value.modelId.length <= 200
      && (value.effort === null || isEffort(value.effort))
      && typeof value.maxTurns === 'number' && Number.isInteger(value.maxTurns) && value.maxTurns >= 1 && value.maxTurns <= 8) {
      return { modelId: value.modelId, effort: value.effort, maxTurns: value.maxTurns };
    }
  } catch { /* Optional defaults contain no account data or consent. */ }
  return { modelId: '', effort: null, maxTurns: 4 };
}

function remember(value: Selection) {
  try { localStorage.setItem(preferenceKey, JSON.stringify(value)); } catch { /* Optional. */ }
}

function reviewError(error: ProviderFailure): string {
  const messages: Partial<Record<ProviderFailure['kind'], string>> = {
    reviewChanged: 'The native connection or reviewed context changed. Check the Claude catalog and prepare a fresh preview.',
    reviewExpired: 'This source review expired. Prepare a fresh preview.',
    reviewUnavailable: 'The local review is unavailable. Check the original evidence and prepare it again.',
    invalidReview: 'The source or Claude settings could not be validated. Reopen the original snapshot range or Git comparison, then check model and effort.',
    unavailableModel: 'Choose a model and effort from the current Claude catalog.',
    contextTooLarge: 'This context exceeds the request limit. Select fewer or smaller files.',
    secretLikeContext: 'A selected path or source pattern is blocked as secret-like. Remove it from the source selection.',
    localStorageUnavailable: 'The local evidence is unavailable. Reopen the repository and selected snapshot or comparison.',
  };
  return messages[error.kind] ?? 'The local request preview could not be prepared. Check the captured source and settings.';
}

/** Offline source/settings preview only. No consent or inference control is
 * present here until the qualified native sender is implemented. */
export function ClaudeOperationReview({ native, mode, discovery }: {
  native: boolean;
  mode: ClaudeOperationReviewMode | 'settings' | null;
  discovery: ClaudeModelDiscoveryObservation;
}) {
  const scope = typeof mode === 'object' ? mode : null;
  const chapterScope = isClaudeChapterScope(scope) ? scope : null;
  const rangeScope = scope?.kind === 'code-lens-range' ? scope : null;
  const gitScope = scope?.kind === 'pr-guide' ? scope : scope?.kind === 'pr-change' ? scope.target : null;
  const gitChanges = scope?.kind === 'pr-guide' ? scope.selectedChanges : scope?.kind === 'pr-change' ? [scope.target.selectedChange] : [];
  const scopeKey = claudeReviewScopeKey(mode);
  const projectId = scope?.projectId ?? null;
  const snapshotId = chapterScope?.snapshotId ?? null;
  const targetPath = chapterScope && chapterScope.kind !== 'overview' ? chapterScope.targetPath : null;
  const sectionUpdate = scope?.kind === 'section-update' ? scope.sectionUpdate : null;
  const connection = discovery.reviewConnection;
  const connectionId = connection?.connectionId ?? '';
  const connectionExpiry = connection?.expiresAtUnixMs ?? 0;
  const [selection, setSelection] = useState(preferences);
  // Preserve the full requested scope. Oversized selections must be corrected
  // explicitly; in particular, never truncate a frozen section-repair scope.
  const [paths, setPaths] = useState<string[]>(() => [...new Set([
    ...(targetPath ? [targetPath] : []), ...(chapterScope?.selectedPaths ?? []),
  ])]);
  const [manifest, setManifest] = useState<SnapshotManifest | null>(null);
  const [manifestLoading, setManifestLoading] = useState(false);
  const [manifestError, setManifestError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [review, setReview] = useState<ProviderRequestReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [expired, setExpired] = useState(Date.now() >= connectionExpiry);
  const [error, setError] = useState<string | null>(null);
  const summary = useRef<HTMLDivElement>(null);
  const resources = useRef<{ mounted: boolean; version: number; active: number | null; reviewId: string | null }>({
    mounted: false, version: 0, active: null, reviewId: null,
  });

  useEffect(() => {
    const current = resources.current;
    current.mounted = true;
    let abandoned = false;
    setManifest(null); setManifestError(null); setReview(null); setBusy(false);
    setPaths([...new Set([...(targetPath ? [targetPath] : []), ...(chapterScope?.selectedPaths ?? [])])]);
    setExpired(Date.now() >= connectionExpiry);
    const release = () => {
      ++current.version;
      const id = current.reviewId;
      current.reviewId = null;
      if (native && id) void discardProviderRequestReview(id).catch(() => {});
    };
    // Native monotonic expiry remains authoritative. Bound each UI timer to
    // five minutes as well; the renderer cannot extend the native lease.
    const timer = window.setTimeout(() => {
      release();
      if (!abandoned) { setExpired(true); setReview(null); setError(null); }
    }, Math.min(300_000, Math.max(0, connectionExpiry - Date.now())));
    if (native && projectId && snapshotId && connectionId) {
      setManifestLoading(true);
      void loadSnapshotManifest(projectId, snapshotId).then((value) => {
        if (abandoned) return;
        if (value.summary.projectId !== projectId || value.summary.snapshotId !== snapshotId) {
          setManifestError('The returned snapshot does not match this source selection. Reopen the selected snapshot.');
          return;
        }
        setManifest(value);
      }).catch(() => {
        if (!abandoned) setManifestError('The captured file list is unavailable. Reopen this repository and snapshot.');
      }).finally(() => { if (!abandoned) setManifestLoading(false); });
    } else setManifestLoading(false);
    return () => {
      abandoned = true;
      current.mounted = false;
      current.active = null;
      window.clearTimeout(timer);
      release();
    };
  }, [native, projectId, snapshotId, connectionId, connectionExpiry, scopeKey]);

  useEffect(() => { if (review) summary.current?.focus(); }, [review]);

  const eligibleFiles = useMemo(() => (manifest?.files ?? []).filter((file) => file.kind === 'file'
    && (file.contentKind === 'text' || file.contentKind === 'unknown') && !file.secretLike && file.sizeBytes <= 250_000)
    .sort((left, right) => left.path.localeCompare(right.path)), [manifest]);
  const matches = eligibleFiles.filter((file) => (!sectionUpdate || paths.includes(file.path))
    && file.path.toLowerCase().includes(query.trim().toLowerCase()));
  const selectedBytes = eligibleFiles.filter((file) => paths.includes(file.path))
    .reduce((total, file) => total + file.sizeBytes, 0);
  const selectedModel = discovery.catalog.models.find((model) => model.modelId === selection.modelId);
  const effortKnown = selection.effort === null || !!selectedModel?.supportedEfforts.includes(selection.effort);
  const modelReady = !!selectedModel && effortKnown;
  const chapterReady = !!chapterScope && !!manifest && paths.length > 0 && paths.length <= 12 && selectedBytes <= 250_000
    && paths.every((path) => eligibleFiles.some((file) => file.path === path))
    && (chapterScope.kind === 'overview' || !!targetPath && paths.includes(targetPath));
  const sourceReady = chapterScope ? chapterReady : rangeScope ? rangeScope.target.startLine >= 1 && rangeScope.target.endLine >= rangeScope.target.startLine
    : scope?.kind === 'pr-guide' ? scope.selectedChanges.length > 0 && scope.selectedChanges.length <= 12
      : scope?.kind === 'pr-change' ? Number.isSafeInteger(scope.target.hunkIndex) && scope.target.hunkIndex >= 0 : false;
  const locked = busy || expired || !native;

  const releaseReview = () => {
    const current = resources.current;
    ++current.version;
    const id = current.reviewId;
    current.reviewId = null;
    if (native && id) void discardProviderRequestReview(id).catch(() => {});
    setReview(null); setError(null);
  };
  const changeSettings = (next: Selection) => {
    if (locked || resources.current.active !== null) return;
    releaseReview(); setSelection(next); remember(next);
  };
  const changePath = (path: string, checked: boolean) => {
    if (locked || resources.current.active !== null || sectionUpdate || path === targetPath) return;
    releaseReview();
    setPaths((previous) => checked
      ? previous.includes(path) || previous.length >= 12 ? previous : [...previous, path]
      : previous.filter((selected) => selected !== path));
  };

  const prepare = async () => {
    const current = resources.current;
    if (!native || !scope || !projectId || !connectionId || locked
      || current.active !== null || !modelReady || !sourceReady) return;
    if (Date.now() >= connectionExpiry) { releaseReview(); setExpired(true); return; }
    releaseReview();
    const version = ++current.version;
    current.active = version;
    setBusy(true);
    let unclaimedReview: string | null = null;
    const settings: ClaudeGenerationSettings = { ...selection, approvalPolicy: 'dontAsk' };
    try {
      const value = await prepareClaudeOperationReview(connectionId, scope, settings, paths);
      if (value.status === 'error') {
        if (current.mounted && current.version === version) setError(reviewError(value.error));
        return;
      }
      const prepared = value.data;
      unclaimedReview = prepared.reviewId;
      if (!current.mounted || current.version !== version) {
        await discardProviderRequestReview(prepared.reviewId).catch(() => {});
        unclaimedReview = null;
        return;
      }
      if (!Number.isFinite(prepared.expiresAtUnixMs) || prepared.expiresAtUnixMs <= Date.now()
        || prepared.expiresAtUnixMs > connectionExpiry
        || !matchesClaudeOperationReview(prepared, scope, settings, connectionId, paths)) {
        await discardProviderRequestReview(prepared.reviewId).catch(() => {});
        unclaimedReview = null;
        if (current.mounted && current.version === version) setError('The returned review expired or did not match this operation, source and configuration. Refresh the native connection.');
        return;
      }
      current.reviewId = prepared.reviewId;
      unclaimedReview = null;
      setReview(prepared);
    } catch {
      if (current.mounted && current.version === version) setError('The local review could not be prepared. Check the selected evidence and native connection.');
    } finally {
      if (unclaimedReview) void discardProviderRequestReview(unclaimedReview).catch(() => {});
      if (current.active === version) {
        current.active = null;
        if (current.mounted) setBusy(false);
      }
    }
  };

  return <section className="provider-review-card" aria-labelledby="claude-review-title">
    <h3 id="claude-review-title">{scope ? 'Preview a Claude request' : 'Claude defaults'}</h3>
    <p>Choose supported settings for this installed runtime. Automatic generation is being implemented; this preview prepares source locally.</p>
    <label className="provider-model-select">Model
      <select value={selectedModel ? selection.modelId : ''} disabled={locked} onChange={(event) => changeSettings({ ...selection, modelId: event.target.value, effort: null })}>
        <option value="">Choose from this catalog…</option>
        {discovery.catalog.models.map((model) => <option key={model.modelId} value={model.modelId}>{model.displayName} · {model.modelId}</option>)}
      </select>
    </label>
    <label className="provider-model-select">Reasoning effort
      <select value={effortKnown ? selection.effort ?? '' : 'unavailable'} disabled={locked || !selectedModel} onChange={(event) => {
        const effort = event.target.value;
        if (effort === '') changeSettings({ ...selection, effort: null });
        else if (isEffort(effort) && selectedModel?.supportedEfforts.includes(effort)) changeSettings({ ...selection, effort });
      }}>
        {!effortKnown && <option value="unavailable" disabled>Saved effort unavailable</option>}
        <option value="">Runtime default</option>
        {selectedModel?.supportedEfforts.map((effort) => <option key={effort} value={effort}>{effort}</option>)}
      </select>
    </label>
    <label className="provider-model-select">Turn limit
      <select value={selection.maxTurns} disabled={locked} onChange={(event) => changeSettings({ ...selection, maxTurns: Number(event.target.value) })}>
        {[1, 2, 3, 4, 5, 6, 7, 8].map((turns) => <option key={turns} value={turns}>{turns}</option>)}
      </select>
    </label>
    <p className="provider-small-note"><strong>Approval: Don’t ask · fixed.</strong> This explanation adapter requests no command, file, web or external tools. Runtime default effort omits the effort override. These defaults are saved for Claude only.</p>
    {!!selection.modelId && !selectedModel && <p role="status">The saved model is absent from this catalog. Choose an available model.</p>}
    {!!selectedModel && !modelReady && <p role="status">The saved effort is unavailable for this model. Choose an advertised effort or Runtime default.</p>}
    {expired && <p className="provider-status-note" role="status">This review connection expired. Check the catalog again.</p>}
    {scope && <>
      {chapterScope && <>
        {manifestLoading && <p role="status">Loading the captured file list…</p>}
        {manifestError && <p className="provider-error-text" role="alert">{manifestError}</p>}
        <details open={chapterScope.kind === 'overview'}>
          <summary>{sectionUpdate ? 'Fixed section-repair source' : 'Selected captured source'} · {paths.length} files · {selectedBytes.toLocaleString()}/250,000 current source bytes</summary>
          {sectionUpdate && <p>Only the selected section will be requested. Its declared file scope is fixed for preservation checks. Previous source and section text are context only; current source is citable, and the native byte limit counts both snapshots.</p>}
          <ul className="provider-section-path-list">{paths.map((path) => <li key={path}><code>{path}</code>{path === targetPath ? ' · target' : ''}
            {!sectionUpdate && path !== targetPath && <button className="text-button" disabled={locked} onClick={() => changePath(path, false)}>Remove</button>}</li>)}</ul>
          <label className="provider-context-search">Filter captured files<input value={query} onChange={(event) => setQuery(event.target.value)} disabled={locked} /></label>
          <div className="provider-context-files">{matches.slice(0, 100).map((file) => <label key={file.path} className={paths.includes(file.path) ? 'selected' : undefined}>
            <input type="checkbox" checked={paths.includes(file.path)} disabled={locked || !!sectionUpdate || file.path === targetPath
              || !paths.includes(file.path) && (paths.length >= 12 || selectedBytes + file.sizeBytes > 250_000)} onChange={(event) => changePath(file.path, event.target.checked)} />
            <code>{file.path}</code><small>{file.sizeBytes.toLocaleString()} bytes</small>
          </label>)}</div>
          {matches.length > 100 && <p>Showing 100 of {matches.length} matching files. Refine the filter to see other files.</p>}
          {!manifestLoading && !manifestError && !matches.length && <p>No eligible captured files match this filter.</p>}
        </details>
        {!sourceReady && !manifestLoading && !manifestError && <p role="status">{sectionUpdate
          ? 'This repair requires its exact declared files and a bounded captured context. Reopen the section and current snapshot to correct its scope.'
          : targetPath ? 'Select 1–12 eligible captured text files within 250,000 bytes, including the target. Recapture source if the target is unavailable.'
            : 'Choose 1–12 eligible captured text files within 250,000 bytes for the overview.'}</p>}
      </>}
      {rangeScope && <section className="provider-review-card" aria-labelledby="claude-range-title">
        <h4 id="claude-range-title">Fixed source selection</h4>
        <p><code>{rangeScope.target.path}</code> · lines {rangeScope.target.startLine}–{rangeScope.target.endLine}</p>
        <p>File SHA-256 <code>{rangeScope.target.sourceSha256}</code></p>
        {rangeScope.target.startByte !== null && rangeScope.target.endByteExclusive !== null
          ? <p>UTF-8 bytes [{rangeScope.target.startByte}, {rangeScope.target.endByteExclusive})</p>
          : <p>The selected complete lines are included.</p>}
        <p>Only this captured excerpt enters the local preview. Native preparation verifies the file hash, bounds, UTF-8 and 32 KiB excerpt limit. Reselect in Files to change the range.</p>
      </section>}
      {gitScope && <section className="provider-review-card" aria-labelledby="claude-git-title">
        <h4 id="claude-git-title">{scope.kind === 'pr-guide' ? 'Fixed PR Guide selection' : 'Fixed diff hunk'}</h4>
        <dl><dt>Starting version</dt><dd><code>{gitScope.baseRef}</code></dd>
          <dt>Ending version</dt><dd><code>{gitScope.headRef}</code></dd>
          <dt>Comparison</dt><dd><code>{gitScope.comparisonId}</code></dd></dl>
        <ul className="provider-section-path-list">{gitChanges.map((change) => <li key={JSON.stringify([change.basePath, change.headPath])}><code>{gitChangeLabel(change)}</code></li>)}</ul>
        {scope.kind === 'pr-change' && <p>Original hunk {scope.target.hunkIndex + 1} only.</p>}
        <p>Preview preparation rereads existing local Git objects and rejects changed refs, unavailable hunks or excluded evidence. It starts no agent. Return to Changes to alter this selection.</p>
      </section>}
      <button className="secondary-button" disabled={locked || !modelReady || !sourceReady} onClick={() => { void prepare(); }}>{busy ? 'Preparing local preview…' : review ? 'Refresh request preview' : 'Preview exact request'}</button>
      {review && <>
        <div ref={summary} className="provider-request-summary" tabIndex={-1} role="group" aria-label="Exact local Claude request">
          <div><span>Operation</span><strong>{claudeOperationLabel(scope)}</strong></div>
          <div><span>{gitScope ? 'Comparison' : 'Snapshot'}</span><code>{review.snapshotId}</code></div>
          <div><span>Model</span><strong>{review.modelId}</strong></div>
          <div><span>Effort · turns · approval</span><strong>{selection.effort ?? 'Runtime default'} · {selection.maxTurns} · Don’t ask</strong></div>
          <div><span>Reviewed evidence</span><strong>{review.selectedFiles.length} {gitScope ? 'patches' : 'files'} · {review.sourceBytes.toLocaleString()} bytes</strong></div>
          <div><span>Review expiry</span><strong>{new Date(review.expiresAtUnixMs).toLocaleTimeString()}</strong></div>
        </div>
        {review.selectedFiles.map((file) => <details className="provider-source-file" key={file.path}><summary><code>{file.path}</code><span>SHA-256 {file.sha256}</span></summary><pre><code>{file.contents}</code></pre></details>)}
        <details className="provider-index-preview"><summary>Exact declarations and relationships</summary><pre>{JSON.stringify({ symbols: review.selectedSymbols, relationships: review.selectedRelationships, truncated: review.indexEvidenceTruncated }, null, 2)}</pre></details>
        <details className="provider-prompt-preview"><summary>Exact prompt, settings and output schema</summary><pre>{JSON.stringify(review.requestBodyPreview, null, 2)}</pre></details>
        <div className="provider-fingerprint"><span>Local review identity</span><code>{review.fingerprint}</code><small>Changing source, settings or connection discards this preview. This is not consent to send source.</small></div>
        <button className="text-button" disabled={busy} onClick={releaseReview}>Discard request preview</button>
      </>}
    </>}
    {mode === null && <p role="status">Open Claude from a captured file, overview, section, code range or Git selection to preview that exact operation. Automatic generation remains unavailable.</p>}
    {error && <p className="provider-error-text" role="alert">{error}</p>}
  </section>;
}
