import type { ChapterRevision, ChapterSection } from './commands';

export function orderedSections(revision: ChapterRevision): ChapterSection[] {
  return [...revision.sections].sort((left, right) => left.order - right.order || left.sectionId.localeCompare(right.sectionId));
}

/** One chapter-list entry per chapter: always its newest accepted revision. */
export function latestAcceptedRevisions(revisions: ChapterRevision[]): ChapterRevision[] {
  const latest = new Map<string, ChapterRevision>();
  for (const revision of revisions) {
    if (revision.lifecycle !== 'accepted' || revision.sections.length === 0) continue;
    const previous = latest.get(revision.chapterId);
    if (!previous
      || revision.revisionNumber > previous.revisionNumber
      || (revision.revisionNumber === previous.revisionNumber && revision.createdAtUnixMs > previous.createdAtUnixMs)
      || (revision.revisionNumber === previous.revisionNumber
        && revision.createdAtUnixMs === previous.createdAtUnixMs
        && revision.revisionId.localeCompare(previous.revisionId) > 0)) {
      latest.set(revision.chapterId, revision);
    }
  }
  return [...latest.values()].sort((left, right) => left.title.localeCompare(right.title) || left.chapterId.localeCompare(right.chapterId));
}

export function chooseSectionId(
  sections: ChapterSection[],
  routeSectionId?: string | null,
  rememberedSectionId?: string | null,
): string | null {
  if (sections.length === 0) return null;
  if (routeSectionId && sections.some((section) => section.sectionId === routeSectionId)) return routeSectionId;
  if (rememberedSectionId && sections.some((section) => section.sectionId === rememberedSectionId)) return rememberedSectionId;
  return sections[0].sectionId;
}

export function resumeStorageKey(projectId: string, chapterId: string, revisionId: string): string {
  return `code-atlas.walkthrough.v1:${encodeURIComponent(projectId)}:${encodeURIComponent(chapterId)}:${encodeURIComponent(revisionId)}`;
}

export function parseRememberedSection(
  serialized: string | null,
  projectId: string,
  chapterId: string,
  revisionId: string,
): string | null {
  if (!serialized) return null;
  try {
    const value = JSON.parse(serialized) as Record<string, unknown>;
    if (value.version !== 1 || value.projectId !== projectId || value.chapterId !== chapterId || value.revisionId !== revisionId) return null;
    return typeof value.sectionId === 'string' && value.sectionId.length > 0 ? value.sectionId : null;
  } catch {
    return null;
  }
}

export function rememberedSectionRecord(projectId: string, chapterId: string, revisionId: string, sectionId: string): string {
  return JSON.stringify({ version: 1, projectId, chapterId, revisionId, sectionId });
}
