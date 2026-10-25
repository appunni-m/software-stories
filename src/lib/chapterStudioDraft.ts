export interface ChapterStudioDraftRouteIdentity {
  baseRevisionId: string;
  candidate: { chapterId: string };
}

/** A route mismatch must keep a saved draft recoverable, not silently discard it. */
export function chapterStudioDraftMatchesRoute(
  draft: ChapterStudioDraftRouteIdentity,
  routeChapterId: string | null,
  routeRevisionId: string | null,
): boolean {
  return (!routeRevisionId || draft.baseRevisionId === routeRevisionId)
    && (!routeChapterId || draft.candidate.chapterId === routeChapterId);
}
