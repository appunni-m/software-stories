import type { ChapterRevision } from './commands.ts';

export type ChapterStudioStatus = 'missing' | 'draft' | 'rejected' | 'stale' | 'accepted';

/** A target with only older-snapshot revisions is stale, not missing. */
export function chapterStudioStatus(
  hasHistory: boolean,
  selectedRevision: ChapterRevision | null,
  currentSnapshotId: string,
  latestCurrent: ChapterRevision | null,
): ChapterStudioStatus {
  if (!hasHistory) return 'missing';
  if (selectedRevision) {
    return selectedRevision.snapshotId === currentSnapshotId ? selectedRevision.lifecycle : 'stale';
  }
  return latestCurrent?.lifecycle ?? 'stale';
}

/** Target-list status describes the newest available version, even when an
 * older revision is selected in the main review pane. */
export function chapterStudioRowStatus(
  hasHistory: boolean,
  latestCurrent: ChapterRevision | null,
): ChapterStudioStatus {
  return hasHistory ? latestCurrent?.lifecycle ?? 'stale' : 'missing';
}

export function chapterStudioStatusLabel(status: ChapterStudioStatus): string {
  switch (status) {
    case 'missing': return 'Needs explanation';
    case 'stale': return 'Older snapshot';
    case 'draft': return 'Draft';
    case 'rejected': return 'Rejected';
    case 'accepted': return 'Accepted';
  }
}
