import { lazy, Suspense, useContext, useEffect, useRef, useState, useSyncExternalStore, type FormEvent } from 'react';
import { AlertTriangle, ArrowDownUp, FileCode2, GitBranch, GitCommitHorizontal, RefreshCw } from 'lucide-react';
import type { AgentGitRecovery } from './lib/agentRecovery';
import { matchesCapturedHead, matchingCapturedHeadPath, type CapturedGitSourceSnapshot } from './lib/gitChangeAvailability';
import { GitHistoryEmptyContext } from './workspace/gitHistoryContext';
import {
  compareLocalGitRevisions,
  cancelLocalGitComparison,
  isLocalGitComparisonBusy,
  subscribeLocalGitComparisons,
  isNativeRuntime,
  type GitComparisonResult,
  type GitFileChange,
  type GitSourceAnchor,
} from './lib/commands';

const PrGuideStudio = lazy(() => import('./PrGuideStudio').then((module) => ({ default: module.PrGuideStudio })));

interface GitChangesScreenProps {
  compact?: boolean;
  projectId: string | null;
  baseRef: string;
  headRef: string;
  selectedChangePath?: string;
  comparison: { baseRef: string; headRef: string; result: GitComparisonResult } | null;
  onRefsChange: (baseRef: string, headRef: string) => void;
  onSelectChange: (path: string | undefined) => void;
  onComparison: (projectId: string, baseRef: string, headRef: string, result: GitComparisonResult) => void;
  onOpenRepository: () => void;
  onOpenProviderSettings: () => void;
  recovery?: AgentGitRecovery | null;
  onRecoveryReviewReady?: (operationId: string) => void;
  capturedSource?: CapturedGitSourceSnapshot | null;
  onOpenCapturedSource?: (path: string, snapshotId: string) => void;
}

export function GitChangesScreen({
  compact = false,
  projectId,
  baseRef: initialBaseRef,
  headRef: initialHeadRef,
  selectedChangePath,
  comparison,
  onRefsChange,
  onSelectChange,
  onComparison,
  onOpenRepository,
  onOpenProviderSettings,
  recovery,
  onRecoveryReviewReady,
  capturedSource,
  onOpenCapturedSource,
}: GitChangesScreenProps) {
  const commitHistoryEmpty = useContext(GitHistoryEmptyContext);
  const [baseRef, setBaseRef] = useState(initialBaseRef);
  const [headRef, setHeadRef] = useState(initialHeadRef);
  const busy = useSyncExternalStore(subscribeLocalGitComparisons, () => isLocalGitComparisonBusy(projectId), () => false);
  const [comparisonOptionsOpen, setComparisonOptionsOpen] = useState(false);
  const activeRequest = useRef<AbortController | null>(null);
  const [error, setError] = useState<string | null>(null);
  const requestSequence = useRef(0);

  useEffect(() => {
    if (busy) setComparisonOptionsOpen(true);
  }, [busy]);

  useEffect(() => {
    setBaseRef(initialBaseRef);
    setHeadRef(initialHeadRef);
  }, [initialBaseRef, initialHeadRef]);

  useEffect(() => {
    setError(null);
    return () => {
      requestSequence.current += 1;
      activeRequest.current?.abort();
      activeRequest.current = null;
    };
  }, [projectId]);

  const matchingComparison = comparison
    && comparison.baseRef === baseRef.trim()
    && comparison.headRef === headRef.trim()
    ? comparison
    : null;
  const canCompare = Boolean(projectId && baseRef.trim() && headRef.trim() && isNativeRuntime());

  async function handleCompare(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !baseRef.trim() || !headRef.trim() || !isNativeRuntime() || busy) return;
    const nextBaseRef = baseRef.trim();
    const nextHeadRef = headRef.trim();
    const sequence = ++requestSequence.current;
    const controller = new AbortController();
    activeRequest.current = controller;
    setError(null);
    onRefsChange(nextBaseRef, nextHeadRef);
    try {
      const result = await compareLocalGitRevisions(projectId, nextBaseRef, nextHeadRef, controller.signal);
      if (sequence === requestSequence.current) onComparison(projectId, nextBaseRef, nextHeadRef, result);
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') return;
      if (sequence === requestSequence.current) {
        setError('Code Atlas could not compare those versions. Check that the repository is available, then try again.');
      }
    } finally {
      if (activeRequest.current === controller) activeRequest.current = null;
    }
  }

  function stopComparison() {
    // Abort the request owned by this screen first. The controller also sends
    // the native cancellation RPC, while the project-scoped fallback handles
    // any request whose local screen reference has already been cleared.
    activeRequest.current?.abort();
    if (projectId) cancelLocalGitComparison(projectId);
  }

  function swapRefs() {
    const nextBase = headRef;
    const nextHead = baseRef;
    setBaseRef(nextBase);
    setHeadRef(nextHead);
    onRefsChange(nextBase, nextHead);
  }

  if (!projectId) {
    return (
      <div className="git-changes-empty empty-workspace">
        <GitBranch size={23} />
        <h2>Open a repository to compare versions.</h2>
        <p>The comparison uses versions already on this Mac. It will not switch branches, fetch from a remote, or change your working files.</p>
        <button className="primary-button" onClick={onOpenRepository}>Open repository</button>
      </div>
    );
  }

  return (
    <div className="git-changes-workflow">
      <section className="git-ref-card" aria-labelledby="git-ref-title">
        {compact && !matchingComparison && <p className="git-commit-hint">{commitHistoryEmpty ?? 'Select a commit on the left to inspect its changes.'}</p>}
        <details
          className="git-compare-options"
          open={!compact || busy || comparisonOptionsOpen}
          onToggle={(event) => {
            if (compact && !busy) setComparisonOptionsOpen(event.currentTarget.open);
          }}
        ><summary onClick={(event) => { if (busy) event.preventDefault(); }}>Compare branches or tags</summary>
        <div className="git-changes-section-heading">
          <div>
            <span className="git-changes-kicker">LOCAL COMPARISON</span>
            <h2 id="git-ref-title">Choose the two versions</h2>
          </div>
          <span className="git-readonly-badge"><GitCommitHorizontal size={14} /> Read-only</span>
        </div>
        <p className="git-ref-explainer">Enter a starting and ending version already on this Mac. A branch, tag, or commit ID works. Code Atlas checks both before showing the changes.</p>
        <form className="git-ref-form" aria-busy={busy} onSubmit={(event) => { void handleCompare(event); }}>
          <label>
            Starting version
            <input
              value={baseRef}
              disabled={busy}
              onChange={(event) => {
                const value = event.target.value;
                setBaseRef(value);
                onRefsChange(value, headRef);
              }}
              autoComplete="off"
              autoCapitalize="none"
              spellCheck={false}
              placeholder="For example, main"
              aria-label="Starting Git version"
            />
          </label>
          <button type="button" className="git-swap-button" disabled={busy} onClick={swapRefs} aria-label="Swap starting and ending versions" title="Swap versions">
            <ArrowDownUp size={16} />
          </button>
          <label>
            Ending version
            <input
              value={headRef}
              disabled={busy}
              onChange={(event) => {
                const value = event.target.value;
                setHeadRef(value);
                onRefsChange(baseRef, value);
              }}
              autoComplete="off"
              autoCapitalize="none"
              spellCheck={false}
              placeholder="For example, feature branch"
              aria-label="Ending Git version"
            />
          </label>
          {busy ? <button key="stop" type="button" className="secondary-button git-compare-button" onClick={stopComparison}>
            <RefreshCw size={15} className="git-spin" /> Stop comparison
          </button> : <button key="compare" className="primary-button git-compare-button" type="submit" disabled={!canCompare}>
            <RefreshCw size={15} /> Compare versions
          </button>}
        </form>
        <p className="git-safety-note"><strong>No checkout, fetch, or Git write.</strong> Comparison reads existing local objects only. The result is bounded; skipped or uncertain evidence is labeled.</p>
        {!isNativeRuntime() && (
          <p className="git-preview-note" role="status">Local Git comparison runs in the packaged Mac app. This browser preview does not access your repository or execute Git commands.</p>
        )}
        {!matchingComparison && comparison && baseRef.trim() && headRef.trim() && (
          <p className="git-stale-result" role="status">The displayed versions changed. Compare again to inspect the new pair.</p>
        )}
        {error && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {error}</p>}
        </details>
      </section>

      {matchingComparison && <ComparisonResults compact={compact}
        key={`${projectId}:${matchingComparison.result.comparisonId ?? `${matchingComparison.baseRef}:${matchingComparison.headRef}`}`}
        comparison={matchingComparison.result}
        projectId={projectId}
        baseRef={matchingComparison.baseRef}
        headRef={matchingComparison.headRef}
        selectedChangePath={selectedChangePath}
        onSelectChange={onSelectChange}
        onOpenProviderSettings={onOpenProviderSettings}
        recovery={recovery}
        onRecoveryReviewReady={onRecoveryReviewReady}
        capturedSource={capturedSource}
        onOpenCapturedSource={onOpenCapturedSource}
      />}
    </div>
  );
}

function ComparisonResults({
  compact,
  comparison,
  projectId,
  baseRef,
  headRef,
  selectedChangePath,
  onSelectChange,
  onOpenProviderSettings,
  recovery,
  onRecoveryReviewReady,
  capturedSource,
  onOpenCapturedSource,
}: {
  compact?: boolean;
  comparison: GitComparisonResult;
  projectId: string;
  baseRef: string;
  headRef: string;
  selectedChangePath?: string;
  onSelectChange: (path: string | undefined) => void;
  onOpenProviderSettings: () => void;
  recovery?: AgentGitRecovery | null;
  onRecoveryReviewReady?: (operationId: string) => void;
  capturedSource?: CapturedGitSourceSnapshot | null;
  onOpenCapturedSource?: (path: string, snapshotId: string) => void;
}) {
  const [focusedCitation, setFocusedCitation] = useState<{ identity: string; hunkIndex: number } | null>(null);
  const [compactSurface, setCompactSurface] = useState<'diff' | 'pr-guide'>('diff');
  const isPartial = comparison.status === 'limited';
  const resultLabel = comparison.status === 'ready'
    ? `${comparison.changes.length} changed ${comparison.changes.length === 1 ? 'file' : 'files'}`
    : comparison.status === 'chooseAgain'
      ? 'Choose valid versions'
      : isPartial
        ? 'Partial comparison'
        : comparison.status === 'cancelled' ? 'Comparison stopped' : 'Comparison unavailable';
  const activeChange = comparison.changes.find((change) => changeIdentity(change) === selectedChangePath) ?? comparison.changes[0];
  const activeIdentity = activeChange ? changeIdentity(activeChange) : undefined;
  const activePath = activeChange?.headPath ?? activeChange?.basePath ?? 'Unknown path';
  const prGuide = <Suspense fallback={<p className="pr-guide-muted" role="status">Loading PR Guide review controls…</p>}>
    <PrGuideStudio accountOnly={compact}
      key={JSON.stringify([projectId, baseRef, headRef, comparison.comparisonId])}
      projectId={projectId}
      baseRef={baseRef}
      headRef={headRef}
      comparison={comparison}
      onFocusChange={(identity, hunkIndex) => {
        if (identity && hunkIndex !== undefined) setFocusedCitation({ identity, hunkIndex });
        if (compact) setCompactSurface('diff');
        onSelectChange(identity);
      }}
      onOpenProviderSettings={onOpenProviderSettings}
      recovery={recovery}
      onRecoveryReviewReady={onRecoveryReviewReady}
    />
  </Suspense>;

  return (
    <section className="git-comparison-results" aria-labelledby="git-results-title">
      <header className="git-results-header">
        <div>
          <span className="git-changes-kicker">{isPartial ? 'PARTIAL RESULT' : 'COMPARE RESULTS'}</span>
          <h2 id="git-results-title">{resultLabel}</h2>
        </div>
      </header>
      {(comparison.comparisonId || (comparison.base && comparison.head)) && <details className="git-version-details">
        <summary>Version details</summary>
        {comparison.comparisonId && <code className="git-comparison-id" title="Stable identity derived from both requested versions and resolved commits">{comparison.comparisonId}</code>}
        {comparison.base && comparison.head && <div className="git-resolved-refs" role="group" aria-label="Resolved commit IDs">
          <CommitIdentity label="From" ref={comparison.base} />
          <CommitIdentity label="To" ref={comparison.head} />
        </div>}
      </details>}
      {comparison.refProblem && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {comparison.refProblem.message} ({comparison.refProblem.side === 'base' ? 'starting version' : 'ending version'}: “{comparison.refProblem.requested}”)</p>}
      {comparison.message && !comparison.refProblem && <p className={isPartial ? 'git-stale-result' : 'git-error-note'} role={comparison.status === 'error' ? 'alert' : 'status'}>{comparison.message}</p>}
      {comparison.omittedChangeCount > 0 && <p className="git-stale-result" role="status">{comparison.omittedChangeCount} more {comparison.omittedChangeCount === 1 ? 'change was' : 'changes were'} omitted. {comparison.omittedReason}</p>}
      {comparison.changes.length > 0 && comparison.relationshipImpact.status !== 'notNeeded' && (
        <div className={`git-relationship-status impact-${comparison.relationshipImpact.status}`} role="status">
          <strong>Static cross-file relationships · {formatRelationshipStatus(comparison.relationshipImpact.status)}</strong>
          <p>{comparison.relationshipImpact.message}</p>
        </div>
      )}
      {comparison.changes.length === 0 && comparison.status !== 'chooseAgain' && comparison.status !== 'error' ? (
        <p className="git-no-changes">These commits have no file changes in the comparison result.</p>
      ) : comparison.changes.length > 0 && compact ? (
        <div className="git-compact-comparison">
          <div className="git-compact-surfaces">
            <button type="button" aria-pressed={compactSurface === 'diff'} onClick={() => setCompactSurface('diff')}>Diff</button>
            <button type="button" aria-pressed={compactSurface === 'pr-guide'} onClick={() => setCompactSurface('pr-guide')}>PR Guide</button>
          </div>
          <div className="git-compact-browser" hidden={compactSurface !== 'diff'}>
            <nav className="git-compact-files" aria-label="Changed files">
              <header><strong>Files changed</strong><span>{comparison.changes.length}</span></header>
              {comparison.changes.map((change) => {
                const identity = changeIdentity(change);
                const path = change.headPath ?? change.basePath ?? 'Unknown path';
                const selected = identity === activeIdentity;
                return <button type="button" key={identity} className={selected ? 'active' : ''} aria-pressed={selected} onClick={() => { setFocusedCitation(null); onSelectChange(identity); }}>
                  <span className={`git-change-kind kind-${change.kind.toLowerCase()}`}>{change.kind}</span>
                  <code title={path}>{path}</code>
                  <small>{change.hunks.length} {change.hunks.length === 1 ? 'hunk' : 'hunks'} · {formatClassification(change.classification)}</small>
                </button>;
              })}
            </nav>
            <section className="git-compact-detail" aria-label={`Diff for ${activePath}`}>
              {activeChange ? <ChangeDisclosure
                change={activeChange}
                relationshipImpact={comparison.relationshipImpact}
                canOpenCapturedSource={canOpenCapturedSource(activeChange, comparison, capturedSource, onOpenCapturedSource)}
                capturedHeadMatches={matchesCapturedHead(comparison, capturedSource)}
                capturedSnapshotId={capturedSource?.snapshotId}
                onOpenCapturedSource={onOpenCapturedSource}
                expanded
                interactive={false}
                focusedHunkIndex={focusedCitation && focusedCitation.identity === activeIdentity ? focusedCitation.hunkIndex : undefined}
                onToggle={() => undefined}
              /> : <p className="git-no-changes">Select a changed file to inspect its diff.</p>}
            </section>
          </div>
          <div className="git-compact-pr-guide" hidden={compactSurface !== 'pr-guide'}>{prGuide}</div>
        </div>
      ) : (
        <div className="git-change-list" role="group" aria-label="Changed files">
          {comparison.changes.map((change) => {
            const identity = changeIdentity(change);
            const expanded = selectedChangePath === identity;
            return <ChangeDisclosure
              key={identity}
              change={change}
              relationshipImpact={comparison.relationshipImpact}
              canOpenCapturedSource={canOpenCapturedSource(change, comparison, capturedSource, onOpenCapturedSource)}
              capturedHeadMatches={matchesCapturedHead(comparison, capturedSource)}
              capturedSnapshotId={capturedSource?.snapshotId}
              onOpenCapturedSource={onOpenCapturedSource}
              expanded={expanded}
              focusedHunkIndex={focusedCitation?.identity === identity ? focusedCitation.hunkIndex : undefined}
              onToggle={() => { setFocusedCitation(null); onSelectChange(expanded ? undefined : identity); }}
            />;
          })}
        </div>
      )}
      {compact && comparison.changes.length > 0 ? null : prGuide}
    </section>
  );
}

function CommitIdentity({ label, ref }: { label: string; ref: { requested: string; objectId: string } }) {
  return (
    <div className="git-commit-identity">
      <span>{label} · {ref.requested}</span>
      <code>{ref.objectId}</code>
    </div>
  );
}

function canOpenCapturedSource(
  change: GitFileChange,
  comparison: GitComparisonResult,
  capturedSource: GitChangesScreenProps['capturedSource'],
  onOpen: GitChangesScreenProps['onOpenCapturedSource'],
) {
  return Boolean(onOpen && matchingCapturedHeadPath(change, comparison, capturedSource));
}

function ChangeDisclosure({ change, relationshipImpact, canOpenCapturedSource = false, capturedHeadMatches = false, capturedSnapshotId, onOpenCapturedSource, expanded, interactive = true, focusedHunkIndex, onToggle }: { change: GitFileChange; relationshipImpact: GitComparisonResult['relationshipImpact']; canOpenCapturedSource?: boolean; capturedHeadMatches?: boolean; capturedSnapshotId?: string; onOpenCapturedSource?: (path: string, snapshotId: string) => void; expanded: boolean; interactive?: boolean; focusedHunkIndex?: number; onToggle: () => void }) {
  const path = change.headPath ?? change.basePath ?? 'Unknown path';
  const cardRef = useRef<HTMLElement>(null);
  const heading = <>
    <span className="git-change-kind">{change.kind}</span>
    <span className="git-change-paths">
      {change.basePath === change.headPath
        ? <span className="git-change-same-path"><code>{path}</code></span>
        : <><code>{change.basePath ?? '∅'}</code><span aria-hidden="true">→</span><code>{change.headPath ?? '∅'}</code></>}
    </span>
    <span className={`git-evidence-classification classification-${change.classification}`}>{formatClassification(change.classification)}</span>
    <span className="git-change-count">{change.hunks.length} {change.hunks.length === 1 ? 'hunk' : 'hunks'}</span>
  </>;

  useEffect(() => {
    if (!expanded || focusedHunkIndex === undefined) return;
    const hunk = cardRef.current?.querySelector<HTMLElement>(`[data-hunk-index="${focusedHunkIndex}"]`);
    if (!hunk) return;
    const behavior = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth';
    hunk.scrollIntoView({ behavior, block: 'center' });
  }, [expanded, focusedHunkIndex]);

  return (
    <article ref={cardRef} className={`git-change-card${expanded ? ' expanded' : ''}`}>
      {interactive
        ? <button className="git-change-toggle" aria-expanded={expanded} onClick={onToggle}>{heading}</button>
        : <h3 className="git-change-toggle git-change-static-title">{heading}</h3>}
      {expanded && (
        <div className="git-change-detail">
          {change.skippedReason && <p className="git-stale-result" role="status">{change.skippedReason}</p>}
          <section className="git-relationship-evidence" aria-label={`Cross-file static relationships for ${path}`}>
            <div>
              <strong>Cross-file static references</strong>
              <p>Parser-resolved source references to this changed file or its changed declarations. These do not establish runtime impact.</p>
            </div>
            <div className="git-relationship-groups">
              <RelationshipList side="From" relationships={change.baseRelationships} />
              <RelationshipList side="To" relationships={change.headRelationships} />
            </div>
            {change.baseRelationships.length === 0 && change.headRelationships.length === 0 && (
              <p className="git-relationship-empty">
                {relationshipImpact.status === 'complete'
                  ? 'No high-confidence parser-resolved cross-file references were found for this file.'
                  : 'No relationship records are available for this file. The analysis status above explains which results may be missing.'}
              </p>
            )}
          </section>
          {change.hunks.length === 0 ? (
            <div className="git-no-changes-group" role="group" aria-label="Diff unavailable">
              <p className="git-no-changes" role="status">
                {change.classification === 'oversized'
                  ? 'This file is too large to diff here.'
                  : 'No bounded text hunk is available for this path.'}
              </p>
              {canOpenCapturedSource && capturedSnapshotId && onOpenCapturedSource
                ? <button className="text-button git-no-changes-action" onClick={() => onOpenCapturedSource(path, capturedSnapshotId)}><FileCode2 size={13} /> Open captured source</button>
                : change.classification === 'oversized'
                  ? <p className="git-no-changes-next">{capturedHeadMatches
                    ? 'This capture has no readable text for this file. Try a smaller revision or inspect another changed file.'
                    : 'Select a capture that matches this comparison to inspect its source.'}</p>
                  : null}
            </div>
          ) : change.hunks.map((hunk, index) => (
            <section className={`git-hunk${focusedHunkIndex === index ? ' is-focused' : ''}`} key={`${path}:${index}`} data-hunk-index={index} aria-label={`Diff hunk ${index + 1}`}>
              <div className="git-hunk-anchors">
                <AnchorIdentity side="From" anchor={hunk.baseAnchor} />
                <AnchorIdentity side="To" anchor={hunk.headAnchor} />
              </div>
              {(hunk.baseChangedRanges.length > 0 || hunk.headChangedRanges.length > 0) && (
                <div className="git-hunk-changed-ranges" role="group" aria-label="Exact changed line ranges">
                  <strong>Changed source lines</strong>
                  <span><b>From</b> {formatChangedRanges(hunk.baseChangedRanges)}</span>
                  <span><b>To</b> {formatChangedRanges(hunk.headChangedRanges)}</span>
                </div>
              )}
              {(hunk.baseSymbols.length > 0 || hunk.headSymbols.length > 0) && (
                <div className="git-hunk-symbols">
                  <strong>Parsed declarations intersecting changed lines</strong>
                  <ul>
                    {[...hunk.baseSymbols.map((symbol) => ({ ...symbol, side: 'From' })), ...hunk.headSymbols.map((symbol) => ({ ...symbol, side: 'To' }))].map((symbol) => (
                      <li key={`${symbol.side}:${symbol.anchor.revisionId}:${symbol.anchor.path}:${symbol.anchor.startLine}:${symbol.name}`}>
                        <span>{symbol.side}</span><code>{symbol.kind} {symbol.name}</code>
                        <small>{symbol.anchor.path}:{symbol.anchor.startLine}–{symbol.anchor.endLine}</small>
                      </li>
                    ))}
                  </ul>
                  <p>Declaration ranges may extend beyond the changed lines. These are parser-backed source structures, not claims about runtime impact.</p>
                </div>
              )}
              <pre><code>{hunk.patchExcerpt}</code></pre>
            </section>
          ))}
        </div>
      )}
    </article>
  );
}

function RelationshipList({ side, relationships }: { side: 'From' | 'To'; relationships: GitFileChange['baseRelationships'] }) {
  if (relationships.length === 0) return null;
  return (
    <section className="git-relationship-side" aria-label={`Relationships in the ${side.toLowerCase()} version`}>
      <h4>{side} version</h4>
      <ul>
        {relationships.map((relationship) => (
          <li key={`${relationship.evidenceAnchor.revisionId}:${relationship.sourcePath}:${relationship.evidenceAnchor.startLine}:${relationship.kind}:${relationship.targetPath}:${relationship.targetName ?? ''}`}>
            <div>
              <strong>{formatRelationshipKind(relationship.kind)}</strong>
              <span>{relationship.confidence} confidence</span>
            </div>
            <p>
              <code>{relationship.sourcePath}:{relationship.evidenceAnchor.startLine}–{relationship.evidenceAnchor.endLine}</code>
              {relationship.sourceName && <> in <code>{relationship.sourceName}</code></>}
              {' → '}
              <code>{relationship.targetPath}</code>
              {relationship.targetName && <> · <code>{relationship.targetName}</code></>}
              {relationship.targetAnchor && <small> declared at {relationship.targetAnchor.path}:{relationship.targetAnchor.startLine}–{relationship.targetAnchor.endLine}</small>}
            </p>
          </li>
        ))}
      </ul>
    </section>
  );
}

function formatRelationshipStatus(status: GitComparisonResult['relationshipImpact']['status']): string {
  switch (status) {
    case 'complete': return 'indexed';
    case 'partial': return 'partial';
    case 'unavailable': return 'unavailable';
    default: return 'not needed';
  }
}

function formatRelationshipKind(kind: string): string {
  return kind.replace(/([A-Z])/g, ' $1').toLowerCase();
}

function AnchorIdentity({ side, anchor }: { side: string; anchor: GitSourceAnchor | null }) {
  return (
    <span className={anchor ? '' : 'git-anchor-missing'}>
      <strong>{side}</strong>{' '}
      {anchor ? <><code>{anchor.path}:{anchor.startLine}–{anchor.endLine}</code><small>commit {anchor.revisionId.slice(0, 12)} · blob {anchor.blobId.slice(0, 12)}</small></> : <small>No source lines on this side</small>}
    </span>
  );
}

function changeIdentity(change: GitFileChange): string {
  return `${change.basePath ?? ''}\u0000${change.headPath ?? ''}`;
}

function formatClassification(value: GitFileChange['classification']): string {
  switch (value) {
    case 'parserUncertain': return 'Parser uncertain';
    case 'oversized': return 'Oversized';
    default: return value[0].toUpperCase() + value.slice(1);
  }
}

function formatChangedRanges(ranges: Array<{ startLine: number; endLine: number }>): string {
  return ranges.length === 0
    ? 'no lines (addition/deletion on the other side)'
    : ranges.map((range) => range.startLine === range.endLine ? `${range.startLine}` : `${range.startLine}–${range.endLine}`).join(', ');
}
