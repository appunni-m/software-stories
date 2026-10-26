import { CHAPTER_SCHEMA_VERSION } from './chapterSchema.ts';
import type { ChapterRevision, ChapterSection } from './commands.ts';

const MAX_REMEMBERED_PROGRESS_CHARS = 16 * 1024;
const MAX_REMEMBERED_ID_CHARS = 1024;
const MAX_REMEMBERED_SECTION_ID_CHARS = 128;
const MAX_REMEMBERED_PLAYBACK_SECONDS = 60 * 60;
const MAX_REMEMBERED_TRANSCRIPT_PIXELS = 10_000_000;

function boundedRememberedId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_REMEMBERED_ID_CHARS
    && value.trim().length > 0 && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

function boundedSectionId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_REMEMBERED_SECTION_ID_CHARS
    && /^[A-Za-z0-9][A-Za-z0-9:_-]*$/u.test(value);
}

export function orderedSections(revision: ChapterRevision): ChapterSection[] {
  return [...revision.sections].sort((left, right) => left.order - right.order || left.sectionId.localeCompare(right.sectionId));
}

/** One chapter-list entry per chapter: always its newest accepted revision. */
export function latestAcceptedRevisions(revisions: ChapterRevision[]): ChapterRevision[] {
  const latest = new Map<string, ChapterRevision>();
  for (const revision of revisions) {
    if (revision.schemaVersion !== CHAPTER_SCHEMA_VERSION || revision.lifecycle !== 'accepted' || revision.sections.length === 0) continue;
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

/** Explicit draft previews never widen the accepted library used by exports. */
export function readerSelection(revisions: ChapterRevision[], chapterId?: string | null, revisionId?: string | null) {
  const current = revisions.filter(revision => revision.schemaVersion === CHAPTER_SCHEMA_VERSION);
  const exportable = latestAcceptedRevisions(current);
  const exact = chapterId && revisionId ? current.find((r) => r.chapterId === chapterId && r.revisionId === revisionId && (r.lifecycle === 'draft' || r.lifecycle === 'accepted') && r.sections.length > 0) ?? null : null;
  const selected = revisionId ? exact : exportable.find((r) => r.chapterId === chapterId) ?? null;
  const library = exact?.lifecycle === 'draft' ? [exact, ...exportable.filter((r) => r.chapterId !== exact.chapterId)] : exportable;
  return { selected, library, exportable };
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
  return `code-atlas.walkthrough.v2:${encodeURIComponent(projectId)}:${encodeURIComponent(chapterId)}:${encodeURIComponent(revisionId)}`;
}

export interface RememberedWalkthroughProgress {
  sectionId: string;
  playbackPosition: number;
  transcriptPosition: number;
}

export type RememberedWalkthroughProgressRead =
  | { status: 'missing' }
  | { status: 'available'; progress: RememberedWalkthroughProgress }
  | { status: 'unavailable' };

export function readRememberedProgress(
  serialized: string | null,
  projectId: string,
  chapterId: string,
  revisionId: string,
): RememberedWalkthroughProgressRead {
  if (serialized === null) return { status: 'missing' };
  const progress = parseRememberedProgress(serialized, projectId, chapterId, revisionId);
  return progress ? { status: 'available', progress } : { status: 'unavailable' };
}

export function parseRememberedProgress(
  serialized: string | null,
  projectId: string,
  chapterId: string,
  revisionId: string,
): RememberedWalkthroughProgress | null {
  if (typeof serialized !== 'string' || serialized.length === 0 || serialized.length > MAX_REMEMBERED_PROGRESS_CHARS
    || !boundedRememberedId(projectId) || !boundedRememberedId(chapterId) || !boundedRememberedId(revisionId)) return null;
  try {
    const parsed: unknown = JSON.parse(serialized);
    const expected = ['version', 'projectId', 'chapterId', 'revisionId', 'sectionId', 'playbackPosition', 'transcriptPosition'];
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null;
    const value = parsed as Record<string, unknown>;
    if (Object.keys(value).length !== expected.length || !expected.every(key => Object.hasOwn(value, key))
      || value.version !== 2
      || !boundedRememberedId(value.projectId)
      || value.projectId !== projectId
      || !boundedRememberedId(value.chapterId)
      || value.chapterId !== chapterId
      || !boundedRememberedId(value.revisionId)
      || value.revisionId !== revisionId
      || !boundedSectionId(value.sectionId)) return null;
    const playbackPosition = value.playbackPosition;
    const transcriptPosition = value.transcriptPosition;
    if (typeof playbackPosition !== 'number' || !Number.isFinite(playbackPosition)
      || playbackPosition < 0 || playbackPosition > MAX_REMEMBERED_PLAYBACK_SECONDS
      || typeof transcriptPosition !== 'number' || !Number.isFinite(transcriptPosition)
      || transcriptPosition < 0 || transcriptPosition > MAX_REMEMBERED_TRANSCRIPT_PIXELS) return null;
    return { sectionId: value.sectionId, playbackPosition, transcriptPosition };
  } catch {
    return null;
  }
}

export function parseRememberedSection(
  serialized: string | null,
  projectId: string,
  chapterId: string,
  revisionId: string,
): string | null {
  return parseRememberedProgress(serialized, projectId, chapterId, revisionId)?.sectionId ?? null;
}

export function rememberedSectionRecord(
  projectId: string,
  chapterId: string,
  revisionId: string,
  sectionId: string,
  playbackPosition = 0,
  transcriptPosition = 0,
): string {
  return JSON.stringify({ version: 2, projectId, chapterId, revisionId, sectionId, playbackPosition, transcriptPosition });
}

/** Preserve unsupported reader bytes during passive playback/scroll writes.
 * An explicit section or chapter selection may start a fresh current record. */
export function saveRememberedProgress(
  storage: Pick<Storage, 'getItem' | 'setItem'>,
  projectId: string,
  chapterId: string,
  revisionId: string,
  progress: RememberedWalkthroughProgress,
  replaceUnavailable = false,
): boolean {
  let key: string;
  try { key = resumeStorageKey(projectId, chapterId, revisionId); }
  catch { return false; }
  const serialized = rememberedSectionRecord(
    projectId, chapterId, revisionId, progress.sectionId, progress.playbackPosition, progress.transcriptPosition,
  );
  if (readRememberedProgress(serialized, projectId, chapterId, revisionId).status !== 'available') return false;
  let previous: string | null;
  try { previous = storage.getItem(key); }
  catch { return false; }
  if (previous !== null && readRememberedProgress(previous, projectId, chapterId, revisionId).status === 'unavailable'
    && !replaceUnavailable) return false;
  try { storage.setItem(key, serialized); return true; }
  catch { return false; }
}
