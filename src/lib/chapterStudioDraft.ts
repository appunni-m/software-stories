import { CHAPTER_SCHEMA_VERSION } from './chapterSchema.ts';
import { isCurrentChapterGenerationOrigin } from './chapterOrigin.ts';
import { inspectInteractiveVisual } from './visuals.ts';
import type { ChapterRevision, ChapterSection } from './commands.ts';

const MAX_CHAPTER_STUDIO_CHECKPOINT_BYTES = 4 * 1024 * 1024;

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

function hasExactKeys(value: unknown, required: readonly string[], optional: readonly string[] = []): value is Record<string, unknown> {
  if (!isRecord(value)) return false;
  const allowed = [...required, ...optional];
  const keys = Object.keys(value);
  return required.every((key) => Object.hasOwn(value, key)) && keys.every((key) => allowed.includes(key));
}

function isAnchor(value: unknown): boolean {
  return hasExactKeys(value, ['anchorId','path','symbolId','snapshotId','sourceSha256','startByte','endByteInclusive','startLine','endLine','startColumn','endColumn','excerpt'])
    && typeof value.anchorId === 'string'
    && typeof value.path === 'string'
    && (value.symbolId === null || typeof value.symbolId === 'string')
    && typeof value.snapshotId === 'string'
    && typeof value.sourceSha256 === 'string'
    && typeof value.excerpt === 'string'
    && Number.isSafeInteger(value.startLine)
    && Number.isSafeInteger(value.endLine)
    && Number.isSafeInteger(value.startByte)
    && Number.isSafeInteger(value.endByteInclusive);
}

function isDependency(value: unknown): boolean {
  if (!isRecord(value)) return false;
  const common = { path: typeof value.path === 'string', hash: typeof value.sourceSha256 === 'string' };
  if (value.kind === 'sourceFile') return hasExactKeys(value, ['kind','path','sourceSha256']) && common.path && common.hash;
  if (value.kind === 'symbol') return hasExactKeys(value, ['kind','path','sourceSha256','symbolId'])
    && common.path && common.hash && typeof value.symbolId === 'string';
  if (value.kind === 'relationship') return hasExactKeys(value, ['kind','path','sourceSha256','relationshipId'])
    && common.path && common.hash && typeof value.relationshipId === 'string';
  return false;
}

function isTargetScope(value: unknown): boolean {
  if (!isRecord(value)) return false;
  if (value.kind === 'overview') return hasExactKeys(value, ['kind']);
  if (value.kind === 'file') return hasExactKeys(value, ['kind','path']) && typeof value.path === 'string';
  return value.kind === 'group' && hasExactKeys(value, ['kind','paths'])
    && Array.isArray(value.paths) && value.paths.every((path) => typeof path === 'string');
}

function isHotspot(value: unknown): boolean {
  if (!hasExactKeys(value, ['hotspotId','label','destination']) || !isRecord(value.destination)) return false;
  const destination = value.destination;
  if (typeof value.hotspotId !== 'string' || typeof value.label !== 'string') return false;
  if (destination.kind === 'section') return hasExactKeys(destination, ['kind','sectionId']) && typeof destination.sectionId === 'string';
  if (destination.kind === 'codeLens') return hasExactKeys(destination, ['kind','anchorId']) && typeof destination.anchorId === 'string';
  if (destination.kind === 'atlasFile') return hasExactKeys(destination, ['kind','path']) && typeof destination.path === 'string';
  if (destination.kind === 'atlasSymbol') return hasExactKeys(destination, ['kind','path','symbolId'])
    && typeof destination.path === 'string' && typeof destination.symbolId === 'string';
  return destination.kind === 'relationship' && hasExactKeys(destination, ['kind','relationshipId'])
    && typeof destination.relationshipId === 'string';
}

function isClaim(value: unknown): boolean {
  if (!hasExactKeys(value, ['claimId','category','prose','anchors','dependencies']) || !isRecord(value.category)) return false;
  const category = value.category;
  const categoryValid = category.kind === 'inference'
    ? hasExactKeys(category, ['kind','visibleLabel','basis']) && typeof category.visibleLabel === 'string' && typeof category.basis === 'string'
    : (category.kind === 'codeBehavior' || category.kind === 'repositoryContext') && hasExactKeys(category, ['kind']);
  return typeof value.claimId === 'string' && typeof value.prose === 'string' && categoryValid
    && Array.isArray(value.anchors) && value.anchors.every(isAnchor)
    && Array.isArray(value.dependencies) && value.dependencies.every(isDependency);
}

function isSection(value: unknown): value is ChapterSection {
  return hasExactKeys(value, ['sectionId','sectionRevisionId','order','title','prose','spokenText','claims','dependencies','visual','hotspots'])
    && typeof value.sectionId === 'string' && typeof value.sectionRevisionId === 'string'
    && Number.isSafeInteger(value.order) && Number(value.order) >= 0
    && typeof value.title === 'string' && typeof value.prose === 'string' && typeof value.spokenText === 'string'
    && Array.isArray(value.claims) && value.claims.every(isClaim)
    && Array.isArray(value.dependencies) && value.dependencies.every(isDependency)
    && (value.visual === null || isRecord(value.visual))
    && Array.isArray(value.hotspots) && value.hotspots.every(isHotspot);
}

function isRestorableCandidate(value: unknown, projectId: string, snapshotId: string): value is ChapterRevision {
  const required = ['schemaVersion','chapterId','revisionId','revisionNumber','projectId','snapshotId','target','title','lifecycle','validationStatus','createdAtUnixMs','sections'];
  const optional = ['basedOnRevisionId','decisionNote','generationOrigin'];
  if (!hasExactKeys(value, required, optional) || value.schemaVersion !== CHAPTER_SCHEMA_VERSION
    || value.projectId !== projectId || value.snapshotId !== snapshotId
    || typeof value.chapterId !== 'string' || typeof value.revisionId !== 'string'
    || !Number.isSafeInteger(value.revisionNumber) || Number(value.revisionNumber) < 1
    || typeof value.title !== 'string' || !hasExactKeys(value.target, ['targetId','scope'])
    || typeof value.target.targetId !== 'string' || !isTargetScope(value.target.scope)
    || !['draft','accepted','rejected'].includes(String(value.lifecycle))
    || !['unvalidated','valid','invalid'].includes(String(value.validationStatus))
    || !Number.isSafeInteger(value.createdAtUnixMs) || Number(value.createdAtUnixMs) <= 0
    || Object.hasOwn(value, 'basedOnRevisionId') && value.basedOnRevisionId !== null && typeof value.basedOnRevisionId !== 'string'
    || Object.hasOwn(value, 'decisionNote') && value.decisionNote !== null && typeof value.decisionNote !== 'string'
    || Object.hasOwn(value, 'generationOrigin') && !isCurrentChapterGenerationOrigin(value.generationOrigin)
    || !Array.isArray(value.sections) || value.sections.length === 0 || !value.sections.every(isSection)) return false;
  const candidate = value as unknown as ChapterRevision;
  return candidate.sections.every((section) => section.visual === null
    || inspectInteractiveVisual(candidate, section).status === 'available');
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
  if (new TextEncoder().encode(serialized).byteLength > MAX_CHAPTER_STUDIO_CHECKPOINT_BYTES) {
    return { status: 'preserved', reason: 'invalid' };
  }
  let value: unknown;
  try { value = JSON.parse(serialized); }
  catch { return { status: 'preserved', reason: 'unreadable' }; }
  if (!isRecord(value)) return { status: 'preserved', reason: 'invalid' };
  if (value.version !== 1) return { status: 'preserved', reason: 'unsupported' };
  if (isRecord(value.candidate) && value.candidate.schemaVersion !== CHAPTER_SCHEMA_VERSION) {
    return { status: 'preserved', reason: 'unsupported' };
  }
  if (isRecord(value.candidate) && isRecord(value.candidate.generationOrigin)
    && value.candidate.generationOrigin.schemaVersion !== 6) return { status: 'preserved', reason: 'unsupported' };
  if (!hasExactKeys(value, ['version','projectId','snapshotId','targetKey','baseRevisionId','candidate'])) {
    return { status: 'preserved', reason: 'invalid' };
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
