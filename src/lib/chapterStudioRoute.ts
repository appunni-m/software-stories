import type { ChapterStudioTarget } from './chapterStudioTargets.ts';

export type ChapterHistoryAvailability = 'loading' | 'unavailable' | 'ready';

export type ChapterStudioRouteResolution =
  | { kind: 'none' }
  | { kind: 'loading' }
  | { kind: 'unavailable' }
  | { kind: 'chapter-missing' }
  | { kind: 'revision-missing'; targetKey: string }
  | { kind: 'resolved'; targetKey: string; revisionId: string | null };

/** Resolve a saved chapter route without silently substituting another file or revision. */
export function resolveChapterStudioRoute(
  chapterId: string | null,
  revisionId: string | null,
  historyAvailability: ChapterHistoryAvailability,
  targets: readonly ChapterStudioTarget[],
): ChapterStudioRouteResolution {
  if (!chapterId) return revisionId ? { kind: 'chapter-missing' } : { kind: 'none' };
  if (historyAvailability === 'loading') return { kind: 'loading' };
  if (historyAvailability === 'unavailable') return { kind: 'unavailable' };

  const target = targets.find((candidate) => candidate.revisions.some((revision) => revision.chapterId === chapterId));
  if (!target) return { kind: 'chapter-missing' };

  if (revisionId) {
    const revision = target.revisions.find((candidate) =>
      candidate.chapterId === chapterId && candidate.revisionId === revisionId);
    return revision
      ? { kind: 'resolved', targetKey: target.key, revisionId: revision.revisionId }
      : { kind: 'revision-missing', targetKey: target.key };
  }

  const latest = target.latestCurrent ?? target.latestAny;
  return { kind: 'resolved', targetKey: target.key, revisionId: latest?.revisionId ?? null };
}
