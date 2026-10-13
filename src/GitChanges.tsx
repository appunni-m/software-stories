import { lazy, Suspense, useEffect, useRef, useState, type FormEvent } from 'react';
import { AlertTriangle, ArrowDownUp, GitBranch, GitCommitHorizontal, RefreshCw } from 'lucide-react';
import {
  compareLocalGitRevisions,
  isNativeRuntime,
  type GitComparisonResult,
  type GitFileChange,
  type GitSourceAnchor,
} from './lib/commands';

const PrGuideStudio = lazy(() => import('./PrGuideStudio').then((module) => ({ default: module.PrGuideStudio })));

interface GitChangesScreenProps {
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
}

export function GitChangesScreen({
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
}: GitChangesScreenProps) {
  const [baseRef, setBaseRef] = useState(initialBaseRef);
  const [headRef, setHeadRef] = useState(initialHeadRef);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestSequence = useRef(0);

  useEffect(() => {
    setBaseRef(initialBaseRef);
    setHeadRef(initialHeadRef);
  }, [initialBaseRef, initialHeadRef]);

  useEffect(() => () => { requestSequence.current += 1; }, []);

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
    setBusy(true);
    setError(null);
    onRefsChange(nextBaseRef, nextHeadRef);
    try {
      const result = await compareLocalGitRevisions(projectId, nextBaseRef, nextHeadRef);
      if (sequence === requestSequence.current) onComparison(projectId, nextBaseRef, nextHeadRef, result);
    } catch {
      if (sequence === requestSequence.current) {
        setError('Code Atlas could not compare those local refs. Check that the repository is available, then try again.');
      }
    } finally {
      if (sequence === requestSequence.current) setBusy(false);
    }
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
        <h2>Open a local repository to compare revisions.</h2>
        <p>The comparison uses commits already available in that repository. It will not fetch a remote or change your working copy.</p>
        <button className="primary-button" onClick={onOpenRepository}>Open repository</button>
      </div>
    );
  }

  return (
    <div className="git-changes-workflow">
      <section className="git-ref-card" aria-labelledby="git-ref-title">
        <div className="git-changes-section-heading">
          <div>
            <span className="git-changes-kicker">LOCAL COMPARISON</span>
            <h2 id="git-ref-title">Choose the two revisions</h2>
          </div>
          <span className="git-readonly-badge"><GitCommitHorizontal size={14} /> Read-only</span>
        </div>
        <p className="git-ref-explainer">Use a local branch, tag, full commit ID, or an unambiguous short ref. Code Atlas resolves both to immutable commit IDs before it reads the diff.</p>
        <form className="git-ref-form" onSubmit={(event) => { void handleCompare(event); }}>
          <label>
            Base revision
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
              placeholder="main or a full commit ID"
              aria-label="Base Git revision"
            />
          </label>
          <button type="button" className="git-swap-button" disabled={busy} onClick={swapRefs} aria-label="Swap base and head revisions" title="Swap revisions">
            <ArrowDownUp size={16} />
          </button>
          <label>
            Head revision
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
              placeholder="feature branch or full commit ID"
              aria-label="Head Git revision"
            />
          </label>
          <button className="primary-button git-compare-button" type="submit" disabled={!canCompare || busy}>
            <RefreshCw size={15} className={busy ? 'git-spin' : ''} /> {busy ? 'Comparing local refs…' : 'Compare revisions'}
          </button>
        </form>
        <p className="git-safety-note"><strong>No checkout, fetch, or Git write.</strong> Comparison reads existing local objects only. The result is bounded; skipped or uncertain evidence is labeled.</p>
        {!isNativeRuntime() && (
          <p className="git-preview-note" role="status">Local Git comparison runs in the packaged Mac app. This browser preview does not access your repository or execute Git commands.</p>
        )}
        {!matchingComparison && comparison && baseRef.trim() && headRef.trim() && (
          <p className="git-stale-result" role="status">The displayed refs changed. Run the comparison again to inspect the new pair.</p>
        )}
        {error && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {error}</p>}
      </section>

      {matchingComparison && <ComparisonResults
        key={`${projectId}:${matchingComparison.result.comparisonId ?? `${matchingComparison.baseRef}:${matchingComparison.headRef}`}`}
        comparison={matchingComparison.result}
        projectId={projectId}
        baseRef={matchingComparison.baseRef}
        headRef={matchingComparison.headRef}
        selectedChangePath={selectedChangePath}
        onSelectChange={onSelectChange}
        onOpenProviderSettings={onOpenProviderSettings}
      />}
    </div>
  );
}

function ComparisonResults({
  comparison,
  projectId,
  baseRef,
  headRef,
  selectedChangePath,
  onSelectChange,
  onOpenProviderSettings,
}: {
  comparison: GitComparisonResult;
  projectId: string;
  baseRef: string;
  headRef: string;
  selectedChangePath?: string;
  onSelectChange: (path: string | undefined) => void;
  onOpenProviderSettings: () => void;
}) {
  const [focusedCitation, setFocusedCitation] = useState<{ identity: string; hunkIndex: number } | null>(null);
  const isPartial = comparison.status === 'limited';
  const resultLabel = comparison.status === 'ready'
    ? `${comparison.changes.length} changed ${comparison.changes.length === 1 ? 'file' : 'files'}`
    : comparison.status === 'chooseAgain'
      ? 'Choose valid local refs'
      : isPartial
        ? 'Partial comparison'
        : 'Comparison unavailable';

  return (
    <section className="git-comparison-results" aria-labelledby="git-results-title">
      <header className="git-results-header">
        <div>
          <span className="git-changes-kicker">{isPartial ? 'BOUNDED RESULT' : 'RESOLVED COMMIT PAIR'}</span>
          <h2 id="git-results-title">{resultLabel}</h2>
        </div>
        {comparison.comparisonId && <code className="git-comparison-id" title="Stable identity derived from both requested refs and resolved commits">{comparison.comparisonId.slice(0, 18)}</code>}
      </header>
      {comparison.base && comparison.head && (
        <div className="git-resolved-refs" aria-label="Resolved immutable commit IDs">
          <CommitIdentity label="Base" ref={comparison.base} />
          <CommitIdentity label="Head" ref={comparison.head} />
        </div>
      )}
      {comparison.refProblem && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {comparison.refProblem.message} ({comparison.refProblem.side}: “{comparison.refProblem.requested}”)</p>}
      {comparison.message && !comparison.refProblem && <p className={isPartial ? 'git-stale-result' : 'git-error-note'} role={comparison.status === 'error' ? 'alert' : 'status'}>{comparison.message}</p>}
      {comparison.omittedChangeCount > 0 && <p className="git-stale-result" role="status">{comparison.omittedChangeCount} more {comparison.omittedChangeCount === 1 ? 'change was' : 'changes were'} omitted. {comparison.omittedReason}</p>}
      {comparison.changes.length === 0 && comparison.status !== 'chooseAgain' && comparison.status !== 'error' ? (
        <p className="git-no-changes">These commits have no file changes in the comparison result.</p>
      ) : (
        <div className="git-change-list" aria-label="Changed files">
          {comparison.changes.map((change) => {
            const identity = changeIdentity(change);
            const expanded = selectedChangePath === identity;
            return <ChangeDisclosure
              key={identity}
              change={change}
              expanded={expanded}
              focusedHunkIndex={focusedCitation?.identity === identity ? focusedCitation.hunkIndex : undefined}
              onToggle={() => { setFocusedCitation(null); onSelectChange(expanded ? undefined : identity); }}
            />;
          })}
        </div>
      )}
      <Suspense fallback={<p className="pr-guide-muted" role="status">Loading PR Guide review controls…</p>}>
        <PrGuideStudio
          projectId={projectId}
          baseRef={baseRef}
          headRef={headRef}
          comparison={comparison}
          onFocusChange={(identity, hunkIndex) => {
            if (identity && hunkIndex !== undefined) setFocusedCitation({ identity, hunkIndex });
            onSelectChange(identity);
          }}
          onOpenProviderSettings={onOpenProviderSettings}
        />
      </Suspense>
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

function ChangeDisclosure({ change, expanded, focusedHunkIndex, onToggle }: { change: GitFileChange; expanded: boolean; focusedHunkIndex?: number; onToggle: () => void }) {
  const path = change.headPath ?? change.basePath ?? 'Unknown path';
  const cardRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!expanded || focusedHunkIndex === undefined) return;
    const hunk = cardRef.current?.querySelector<HTMLElement>(`[data-hunk-index="${focusedHunkIndex}"]`);
    if (!hunk) return;
    const behavior = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth';
    hunk.scrollIntoView({ behavior, block: 'center' });
  }, [expanded, focusedHunkIndex]);

  return (
    <article ref={cardRef} className={`git-change-card${expanded ? ' expanded' : ''}`}>
      <button className="git-change-toggle" aria-expanded={expanded} onClick={onToggle}>
        <span className="git-change-kind">{change.kind}</span>
        <span className="git-change-paths">
          <code>{change.basePath ?? '∅'}</code>
          {change.basePath !== change.headPath && <span aria-hidden="true">→</span>}
          {change.basePath !== change.headPath && <code>{change.headPath ?? '∅'}</code>}
          {change.basePath === change.headPath && <span className="git-change-same-path"><code>{path}</code></span>}
        </span>
        <span className={`git-evidence-classification classification-${change.classification}`}>{formatClassification(change.classification)}</span>
        <span className="git-change-count">{change.hunks.length} {change.hunks.length === 1 ? 'hunk' : 'hunks'}</span>
      </button>
      {expanded && (
        <div className="git-change-detail">
          {change.skippedReason && <p className="git-stale-result" role="status">{change.skippedReason}</p>}
          {change.hunks.length === 0 ? <p className="git-no-changes">No bounded text hunk is available for this path.</p> : change.hunks.map((hunk, index) => (
            <section className={`git-hunk${focusedHunkIndex === index ? ' is-focused' : ''}`} key={`${path}:${index}`} data-hunk-index={index} aria-label={`Diff hunk ${index + 1}`}>
              <div className="git-hunk-anchors">
                <AnchorIdentity side="Base" anchor={hunk.baseAnchor} />
                <AnchorIdentity side="Head" anchor={hunk.headAnchor} />
              </div>
              {(hunk.baseChangedRanges.length > 0 || hunk.headChangedRanges.length > 0) && (
                <div className="git-hunk-changed-ranges" aria-label="Exact changed line ranges">
                  <strong>Changed source lines</strong>
                  <span><b>Base</b> {formatChangedRanges(hunk.baseChangedRanges)}</span>
                  <span><b>Head</b> {formatChangedRanges(hunk.headChangedRanges)}</span>
                </div>
              )}
              {(hunk.baseSymbols.length > 0 || hunk.headSymbols.length > 0) && (
                <div className="git-hunk-symbols">
                  <strong>Parsed declarations intersecting changed lines</strong>
                  <ul>
                    {[...hunk.baseSymbols.map((symbol) => ({ ...symbol, side: 'Base' })), ...hunk.headSymbols.map((symbol) => ({ ...symbol, side: 'Head' }))].map((symbol) => (
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
