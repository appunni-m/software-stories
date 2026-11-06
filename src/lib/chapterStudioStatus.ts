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

/** Give a stale revision a next step that matches whether current history can be read. */
export function chapterStudioStaleRevisionGuidance(
  availability: ChapterHistoryAvailability,
  nativeRuntime: boolean,
  snapshotId: string,
): string {
  const source = `Its citations stay pinned to snapshot ${snapshotId.slice(-12)}. Open a citation to inspect the exact captured code.`;
  switch (availability) {
    case 'loading':
      return `${source} Current revision history is still loading. Wait for it to finish before choosing or creating an updated draft.`;
    case 'unavailable':
      return nativeRuntime
        ? `${source} Current revision history is unavailable. Use Retry above before choosing or creating an updated draft.`
        : `${source} Open Chapter Studio in the Mac app to check current revision history before choosing an updated draft.`;
    case 'ready':
      return `${source} Select a current revision in the history above, or use the action above to make an updated draft.`;
  }
}

/** Generation is safe only after a successful read proves the target is missing or stale. */
export function chapterStudioCanGenerate(
  availability: ChapterHistoryAvailability,
  status: ChapterStudioStatus,
): boolean {
  return availability === 'ready' && (status === 'missing' || status === 'stale');
}
