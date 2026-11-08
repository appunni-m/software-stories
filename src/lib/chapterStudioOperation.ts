/** Identity captured when an asynchronous Chapter Studio action starts. */
export interface ChapterStudioOperationIdentity {
  projectId: string;
  snapshotId: string;
  targetKey: string;
  selectedRevisionId: string | null;
  routeChapterId: string | null;
  routeRevisionId: string | null;
  routeSectionId: string | null;
  editing: boolean;
  candidate: object | null;
}

/** A late native reply may update the reader only while it still owns this view. */
export function chapterStudioOperationIsCurrent(
  started: ChapterStudioOperationIdentity,
  current: ChapterStudioOperationIdentity,
): boolean {
  return started.projectId === current.projectId
    && started.snapshotId === current.snapshotId
    && started.targetKey === current.targetKey
    && started.selectedRevisionId === current.selectedRevisionId
    && started.routeChapterId === current.routeChapterId
    && started.routeRevisionId === current.routeRevisionId
    && started.routeSectionId === current.routeSectionId
    && started.editing === current.editing
    && started.candidate === current.candidate;
}
