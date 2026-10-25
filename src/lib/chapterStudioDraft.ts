import { CHAPTER_SCHEMA_VERSION } from './chapterSchema.ts';
import type { ChapterRevision } from './commands.ts';

export interface ChapterStudioDraftRouteIdentity {
  baseRevisionId: string;
  candidate: { chapterId: string };
}

export interface ChapterStudioDraftCheckpoint {
  version: 1;
  projectId: string;
  snapshotId: string;
  targetKey: string;
  baseRevisionId: string;
  candidate: ChapterRevision;
}

export type ChapterStudioDraftLoad =
  | { status: 'empty' }
  | { status: 'restored'; checkpoint: ChapterStudioDraftCheckpoint }
  | { status: 'pending'; checkpoint: ChapterStudioDraftCheckpoint }
  | { status: 'preserved'; reason: 'unreadable' | 'unsupported' | 'invalid' };

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function isAnchor(value: unknown): boolean {
  return isRecord(value)
    && typeof value.anchorId === 'string'
    && typeof value.path === 'string'
    && typeof value.snapshotId === 'string'
    && typeof value.sourceSha256 === 'string'
    && typeof value.excerpt === 'string'
    && Number.isSafeInteger(value.startLine)
    && Number.isSafeInteger(value.endLine)
    && Number.isSafeInteger(value.startByte)
    && Number.isSafeInteger(value.endByteInclusive);
}

function isDependency(value: unknown): boolean {
  return isRecord(value)
    && ['sourceFile', 'symbol', 'relationship'].includes(String(value.kind))
    && typeof value.path === 'string'
    && typeof value.sourceSha256 === 'string'
    && (value.kind === 'sourceFile'
      || value.kind === 'symbol' && typeof value.symbolId === 'string'
      || value.kind === 'relationship' && typeof value.relationshipId === 'string');
}

function isRestorableCandidate(value: unknown, projectId: string, snapshotId: string): value is ChapterRevision {
  if (!isRecord(value) || value.schemaVersion !== CHAPTER_SCHEMA_VERSION
    || value.projectId !== projectId || value.snapshotId !== snapshotId
    || typeof value.chapterId !== 'string' || typeof value.revisionId !== 'string'
    || typeof value.title !== 'string' || !isRecord(value.target)
    || typeof value.target.targetId !== 'string' || !isRecord(value.target.scope)
    || !['overview', 'file', 'group'].includes(String(value.target.scope.kind))
    || !Array.isArray(value.sections) || value.sections.length === 0) return false;
  return value.sections.every((section) => isRecord(section)
    && typeof section.sectionId === 'string'
    && typeof section.title === 'string'
    && typeof section.prose === 'string'
    && typeof section.spokenText === 'string'
    && Array.isArray(section.claims)
    && section.claims.every((claim) => isRecord(claim)
      && typeof claim.claimId === 'string'
      && typeof claim.prose === 'string'
      && isRecord(claim.category)
      && ['codeBehavior', 'repositoryContext', 'inference'].includes(String(claim.category.kind))
      && (claim.category.kind !== 'inference'
        || typeof claim.category.visibleLabel === 'string' && typeof claim.category.basis === 'string')
      && Array.isArray(claim.anchors)
      && claim.anchors.every(isAnchor)
      && Array.isArray(claim.dependencies)
      && claim.dependencies.every(isDependency))
    && Array.isArray(section.dependencies)
    && section.dependencies.every(isDependency)
    && Array.isArray(section.hotspots)
    && section.hotspots.every((hotspot) => isRecord(hotspot)
      && typeof hotspot.hotspotId === 'string'
      && typeof hotspot.label === 'string'
      && isRecord(hotspot.destination)
      && typeof hotspot.destination.kind === 'string'));
}

/** Read only the current checkpoint shape. Unsupported/corrupt bytes stay in session storage. */
export function readChapterStudioDraft(
  serialized: string | null,
  projectId: string,
  snapshotId: string,
  routeChapterId: string | null,
  routeRevisionId: string | null,
): ChapterStudioDraftLoad {
  if (serialized === null) return { status: 'empty' };
  let value: unknown;
  try { value = JSON.parse(serialized); }
  catch { return { status: 'preserved', reason: 'unreadable' }; }
  if (!isRecord(value)) return { status: 'preserved', reason: 'invalid' };
  if (value.version !== 1) return { status: 'preserved', reason: 'unsupported' };
  if (isRecord(value.candidate) && value.candidate.schemaVersion !== CHAPTER_SCHEMA_VERSION) {
    return { status: 'preserved', reason: 'unsupported' };
  }
  if (value.projectId !== projectId || value.snapshotId !== snapshotId
    || typeof value.targetKey !== 'string' || !value.targetKey
    || typeof value.baseRevisionId !== 'string' || !value.baseRevisionId
    || !isRestorableCandidate(value.candidate, projectId, snapshotId)) {
    return { status: 'preserved', reason: 'invalid' };
  }
  const checkpoint = value as unknown as ChapterStudioDraftCheckpoint;
  const expectedTargetKey = checkpoint.candidate.target.scope.kind === 'overview'
    ? 'overview' : checkpoint.candidate.target.targetId;
  if (checkpoint.candidate.revisionId !== checkpoint.baseRevisionId
    || checkpoint.targetKey !== expectedTargetKey) return { status: 'preserved', reason: 'invalid' };
  return chapterStudioDraftMatchesRoute(checkpoint, routeChapterId, routeRevisionId)
    ? { status: 'restored', checkpoint }
    : { status: 'pending', checkpoint };
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
