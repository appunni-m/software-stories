import type { ChapterRevision } from './commands.ts';
import type { InventoryRow } from './inventory.ts';

export interface ChapterStudioTarget {
  key: string;
  targetId: string | null;
  paths: string[];
  overview?: boolean;
  revisions: ChapterRevision[];
  latestCurrent: ChapterRevision | null;
  latestAny: ChapterRevision | null;
  missing: boolean;
}

/** Keep prior chapter targets visible even when the current snapshot no longer
 * contains readable source for them. Their citations still open the old snapshot. */
export function buildChapterStudioTargets(
  rows: readonly InventoryRow[],
  revisions: readonly ChapterRevision[],
  snapshotId: string,
): ChapterStudioTarget[] {
  const result: ChapterStudioTarget[] = [];
  const seen = new Set<string>();
  const historyFor = (targetId: string) => revisions
    .filter((revision) => revision.target.targetId === targetId)
    .sort((left, right) => left.revisionNumber - right.revisionNumber);
  const addTarget = (
    key: string,
    targetId: string | null,
    paths: string[],
    history: ChapterRevision[],
    overview = false,
  ) => {
    if (seen.has(key)) return;
    seen.add(key);
    const latestCurrent = history
      .filter((revision) => revision.snapshotId === snapshotId)
      .sort((left, right) => right.revisionNumber - left.revisionNumber)[0] ?? null;
    const latestAny = [...history]
      .sort((left, right) => right.revisionNumber - left.revisionNumber)[0] ?? null;
    result.push({ key, targetId, paths, overview, revisions: history, latestCurrent, latestAny, missing: history.length === 0 });
  };

  for (const row of rows) {
    if (row.disposition !== 'captured' || row.kind !== 'file' || !['text', 'unknown'].includes(row.contentKind)) continue;
    const matches = revisions.filter((revision) => revision.target.scope.kind === 'file' && revision.target.scope.path === row.path);
    if (matches.length > 0) {
      const targetIds = Array.from(new Set(matches.map((revision) => revision.target.targetId)));
      for (const targetId of targetIds) addTarget(targetId, targetId, [row.path], historyFor(targetId));
    } else {
      addTarget(`file:${row.path}`, null, [row.path], []);
    }
  }

  for (const revision of revisions) {
    if (revision.target.scope.kind === 'file') {
      addTarget(revision.target.targetId, revision.target.targetId,
        [revision.target.scope.path], historyFor(revision.target.targetId));
    }
  }

  const groupIds = Array.from(new Set(revisions
    .filter((revision) => revision.target.scope.kind === 'group')
    .map((revision) => revision.target.targetId)));
  for (const targetId of groupIds) {
    const history = historyFor(targetId);
    const scope = history[0]?.target.scope;
    if (scope?.kind === 'group') addTarget(targetId, targetId, scope.paths, history);
  }

  const overviewHistory = revisions
    .filter((revision) => revision.target.scope.kind === 'overview')
    .sort((left, right) => left.revisionNumber - right.revisionNumber);
  if (overviewHistory.length > 0) addTarget('overview', 'target:overview', [], overviewHistory, true);

  return result.sort((left, right) => {
    const a = left.overview ? '' : left.paths[0] ?? left.key;
    const b = right.overview ? '' : right.paths[0] ?? right.key;
    return a.localeCompare(b);
  });
}

/** A stale or missing target may be regenerated only from captured text in the
 * current snapshot; deleted, skipped, excluded, and binary paths stay review-only. */
export function chapterStudioHasReadableCurrentSource(
  path: string,
  rows: readonly Pick<InventoryRow, 'path' | 'disposition' | 'kind' | 'contentKind'>[],
): boolean {
  return rows.some((row) => row.path === path
    && row.disposition === 'captured'
    && row.kind === 'file'
    && (row.contentKind === 'text' || row.contentKind === 'unknown'));
}
