import type { ChapterRevision } from './commands.ts';

export type ChapterHistoryAvailability = 'loading' | 'unavailable' | 'ready';
export type ChapterStudioStatus = 'loading' | 'unavailable' | 'missing' | 'draft' | 'rejected' | 'stale' | 'accepted';

/** A target with only older-snapshot revisions is stale, not missing. */
export function chapterStudioStatus(
  availability: ChapterHistoryAvailability,
  hasHistory: boolean,
  selectedRevision: ChapterRevision | null,
  currentSnapshotId: string,
  latestCurrent: ChapterRevision | null,
): ChapterStudioStatus {
  if (!hasHistory) return availability === 'ready' ? 'missing' : availability;
  if (selectedRevision) {
    return selectedRevision.snapshotId === currentSnapshotId ? selectedRevision.lifecycle : 'stale';
  }
  return latestCurrent?.lifecycle ?? 'stale';
}

/** Target-list status describes the newest available version, even when an
 * older revision is selected in the main review pane. */
export function chapterStudioRowStatus(
  availability: ChapterHistoryAvailability,
  hasHistory: boolean,
  latestCurrent: ChapterRevision | null,
): ChapterStudioStatus {
  if (!hasHistory) return availability === 'ready' ? 'missing' : availability;
  return latestCurrent?.lifecycle ?? 'stale';
}

export function chapterStudioStatusLabel(status: ChapterStudioStatus): string {
  switch (status) {
    case 'loading': return 'Loading';
    case 'unavailable': return 'History unavailable';
    case 'missing': return 'Needs explanation';
    case 'stale': return 'Older snapshot';
    case 'draft': return 'Draft';
    case 'rejected': return 'Rejected';
    case 'accepted': return 'Accepted';
  }
}

/** Generation is safe only after a successful read proves the target is missing or stale. */
export function chapterStudioCanGenerate(
  availability: ChapterHistoryAvailability,
  status: ChapterStudioStatus,
): boolean {
  return availability === 'ready' && (status === 'missing' || status === 'stale');
}
