import type { SectionUpdateReviewTarget } from './commands';
import type { PlannedChapterScope, PlannedChapterTarget } from './plannedChapters';

export type RecordedAgentSettings = { backend: 'codex'; modelId: string; effort: string; approvalPolicy: 'never' }
  | { backend: 'claude-agent'; modelId: string; effort: string | null; approvalPolicy: 'dontAsk'; maxTurns: number };

type ParserVersion = {
  engine: string; engineVersion: string; engineLanguageAbi: number;
  grammar: string; grammarVersion: string; grammarAbi: number;
};

type SymbolContext = {
  symbolId: string; name: string; qualifiedName: string; startByte: number; endByte: number;
};

type RecordedIndexIdentity =
  | { status: 'available'; schemaVersion: number; indexerVersion: string }
  | { status: 'unavailable' };

/** Local dispatch lineage. This is not a cryptographic account attestation. */
export interface ChapterGenerationOrigin {
  schemaVersion: 6;
  projectId: string;
  snapshotId: string;
  targetPath: string | null;
  purpose: 'fileChapter' | 'repositoryOverview' | 'sectionUpdate';
  requestId: string;
  requestFingerprint: string;
  requestBodySha256: string;
  contractVersion: string;
  settings: RecordedAgentSettings;
  sourceFiles: { path: string; sha256: string; sizeBytes: number }[];
  sectionUpdate: SectionUpdateReviewTarget | null;
  plannedChapter: PlannedChapterScope | null;
  retrieval?: { policy: string; ledgerSha256: string; indexIdentity: RecordedIndexIdentity; ranges: {
    snapshotId: string; path: string; sourceSha256: string; startByte: number; endByte: number;
    startLine: number; endLine: number; chunkSha256: string; enclosing: SymbolContext | null;
    parent: SymbolContext | null; parser: ParserVersion | null;
  }[] };
  contentSha256: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function exactKeys(value: unknown, required: readonly string[], optional: readonly string[] = []): value is Record<string, unknown> {
  if (!isRecord(value)) return false;
  const allowed = [...required, ...optional];
  return required.every((key) => Object.hasOwn(value, key)) && Object.keys(value).every((key) => allowed.includes(key));
}

function boundedText(value: unknown, maxBytes: number): value is string {
  return typeof value === 'string' && value.trim().length > 0
    && new TextEncoder().encode(value).byteLength <= maxBytes
    && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

function digest(value: unknown): value is string {
  return typeof value === 'string' && /^[a-f0-9]{64}$/u.test(value);
}

function count(value: unknown): value is number {
  return Number.isSafeInteger(value) && Number(value) >= 0;
}

function validRepositoryPlanScope(value: unknown): boolean {
  if (!exactKeys(value, ['schemaVersion','question','inventorySha256','contextSha256','planningContractSha256','captureStatus','coverage','indexStatus'])
    || value.schemaVersion !== 1 || !boundedText(value.question, 720)
    || !digest(value.inventorySha256) || !digest(value.contextSha256) || !digest(value.planningContractSha256)
    || !['running','complete','partial','cancelled','inconsistent','failed'].includes(String(value.captureStatus))
    || !['complete','partial','cancelled','missing','unreadable'].includes(String(value.indexStatus))) return false;
  const coverage = value.coverage;
  if (!exactKeys(coverage, ['selectedFileCount','eligibleCapturedFileCount','unselectedEligibleFileCount','excludedCapturedPathCount','skippedPathCount'])
    || !['selectedFileCount','eligibleCapturedFileCount','unselectedEligibleFileCount','excludedCapturedPathCount','skippedPathCount'].every((key) => count(coverage[key]))) return false;
  return Number(coverage.selectedFileCount) >= 1 && Number(coverage.selectedFileCount) <= 12
    && Number(coverage.eligibleCapturedFileCount) >= Number(coverage.selectedFileCount)
    && Number(coverage.unselectedEligibleFileCount)
      === Number(coverage.eligibleCapturedFileCount) - Number(coverage.selectedFileCount);
}

function validPlannedChapterScope(value: unknown): boolean {
  const required = ['schemaVersion','target','snapshotId','primaryPath','selectedPaths','planQuestion','question','prerequisites','planContext'];
  if (!exactKeys(value, required) || value.schemaVersion !== 1 || !boundedText(value.snapshotId, 256)
    || !boundedText(value.primaryPath, 4096) || !Array.isArray(value.selectedPaths)
    || value.selectedPaths.length < 1 || value.selectedPaths.length > 12
    || !value.selectedPaths.every((path) => boundedText(path, 4096))
    || new Set(value.selectedPaths).size !== value.selectedPaths.length || !value.selectedPaths.includes(value.primaryPath)
    || !boundedText(value.planQuestion, 720) || !boundedText(value.question, 720)
    || !Array.isArray(value.prerequisites) || value.prerequisites.length > 23 || !validRepositoryPlanScope(value.planContext)) return false;
  const target = value.target;
  if (!exactKeys(target, ['revisionId','recordSha256','stepId']) || !boundedText(target.revisionId, 256)
    || !digest(target.recordSha256) || !boundedText(target.stepId, 256)
    || value.planQuestion !== (value.planContext as Record<string, unknown>).question) return false;
  const prerequisiteIds = value.prerequisites.map((step) => isRecord(step) ? step.stepId : undefined);
  return new Set(prerequisiteIds).size === prerequisiteIds.length
    && value.prerequisites.every((step) => exactKeys(step, ['stepId','question'])
      && boundedText(step.stepId, 256) && step.stepId !== target.stepId && boundedText(step.question, 720));
}

function validParserVersion(value: unknown): boolean {
  return exactKeys(value, ['engine','engineVersion','engineLanguageAbi','grammar','grammarVersion','grammarAbi'])
    && boundedText(value.engine, 128) && boundedText(value.engineVersion, 128) && count(value.engineLanguageAbi)
    && boundedText(value.grammar, 128) && boundedText(value.grammarVersion, 128) && count(value.grammarAbi);
}

function validSymbolContext(value: unknown): boolean {
  return exactKeys(value, ['symbolId','name','qualifiedName','startByte','endByte'])
    && boundedText(value.symbolId, 256) && boundedText(value.name, 512)
    && boundedText(value.qualifiedName, 1024) && count(value.startByte) && count(value.endByte)
    && Number(value.endByte) >= Number(value.startByte);
}

function validRetrieval(value: unknown): boolean {
  if (!exactKeys(value, ['policy','ledgerSha256','indexIdentity','ranges'])
    || !boundedText(value.policy, 128) || !digest(value.ledgerSha256) || !Array.isArray(value.ranges)
    || value.ranges.length < 1 || value.ranges.length > 512) return false;
  const indexIdentity = value.indexIdentity;
  const validIndex = exactKeys(indexIdentity, ['status','schemaVersion','indexerVersion'])
    ? indexIdentity.status === 'available' && Number.isSafeInteger(indexIdentity.schemaVersion)
      && Number(indexIdentity.schemaVersion) > 0 && boundedText(indexIdentity.indexerVersion, 128)
    : exactKeys(indexIdentity, ['status']) && indexIdentity.status === 'unavailable';
  if (!validIndex) return false;
  return value.ranges.every((range) => {
    if (!exactKeys(range, ['snapshotId','path','sourceSha256','startByte','endByte','startLine','endLine','chunkSha256','enclosing','parent','parser'])
      || !boundedText(range.snapshotId, 256) || !boundedText(range.path, 4096)
      || !digest(range.sourceSha256) || !count(range.startByte) || !count(range.endByte)
      || !count(range.startLine) || !count(range.endLine) || !digest(range.chunkSha256)
      || Number(range.endByte) <= Number(range.startByte) || Number(range.startLine) < 1
      || Number(range.endByte) - Number(range.startByte) > 32768
      || Number(range.endLine) < Number(range.startLine)) return false;
    return (range.enclosing === null || validSymbolContext(range.enclosing))
      && (range.parent === null || validSymbolContext(range.parent))
      && (range.parser === null || validParserVersion(range.parser));
  });
}

/** Exact structural check for persisted chapter lineage. Native storage still
 * verifies that its digests, source scope and settings match the saved chapter. */
export function isCurrentChapterGenerationOrigin(value: unknown): value is ChapterGenerationOrigin {
  const required = ['schemaVersion','projectId','snapshotId','targetPath','purpose','requestId','requestFingerprint','requestBodySha256','contractVersion','settings','sourceFiles','sectionUpdate','plannedChapter','contentSha256'];
  if (!exactKeys(value, required, ['retrieval']) || value.schemaVersion !== 6
    || !boundedText(value.projectId, 4096) || !boundedText(value.snapshotId, 256)
    || !(value.targetPath === null || boundedText(value.targetPath, 4096))
    || !['fileChapter','repositoryOverview','sectionUpdate'].includes(String(value.purpose))
    || !boundedText(value.requestId, 256) || !digest(value.requestFingerprint) || !digest(value.requestBodySha256)
    || !boundedText(value.contractVersion, 128) || !digest(value.contentSha256)
    || !Array.isArray(value.sourceFiles) || value.sourceFiles.length < 1 || value.sourceFiles.length > 12
    || !value.sourceFiles.every((file) => exactKeys(file, ['path','sha256','sizeBytes'])
      && boundedText(file.path, 4096) && digest(file.sha256) && count(file.sizeBytes))) return false;
  const paths = value.sourceFiles.map((file) => (file as Record<string, unknown>).path as string);
  if (new Set(paths).size !== paths.length) return false;
  const settings = value.settings;
  const validSettings = exactKeys(settings, ['backend','modelId','effort','approvalPolicy'])
    ? settings.backend === 'codex' && boundedText(settings.modelId, 512) && boundedText(settings.effort, 32) && settings.approvalPolicy === 'never'
    : exactKeys(settings, ['backend','modelId','effort','approvalPolicy','maxTurns'])
      && settings.backend === 'claude-agent' && boundedText(settings.modelId, 512)
      && (settings.effort === null || boundedText(settings.effort, 32)) && settings.approvalPolicy === 'dontAsk'
      && Number.isSafeInteger(settings.maxTurns) && Number(settings.maxTurns) >= 1 && Number(settings.maxTurns) <= 8;
  if (!validSettings) return false;
  const sectionUpdate = value.sectionUpdate === null || exactKeys(value.sectionUpdate, ['chapterRevisionId','sectionId','sectionRevisionId'])
    && boundedText(value.sectionUpdate.chapterRevisionId, 256) && boundedText(value.sectionUpdate.sectionId, 256)
    && boundedText(value.sectionUpdate.sectionRevisionId, 256);
  const planned = value.plannedChapter === null || validPlannedChapterScope(value.plannedChapter);
  return sectionUpdate && planned && (!Object.hasOwn(value, 'retrieval') || validRetrieval(value.retrieval));
}

export function samePlannedTarget(left: PlannedChapterTarget | null | undefined, right: PlannedChapterTarget): boolean {
  return !!left && left.revisionId === right.revisionId && left.recordSha256 === right.recordSha256 && left.stepId === right.stepId;
}

export function recordedRecoveryLabel(origin: ChapterGenerationOrigin | null | undefined,
  requestId: string | undefined, backend: 'codex' | 'claude-agent', target?: PlannedChapterTarget,
): string {
  if (!origin) return 'Unrecorded request · possible file match';
  const stepMatches = target ? samePlannedTarget(origin.plannedChapter?.target, target) : !origin.plannedChapter;
  if (origin.settings.backend === backend && origin.requestId === requestId && stepMatches) return 'Recorded request match';
  if (target && origin.settings.backend === backend && stepMatches) return 'Same recorded learning step · another request';
  return 'Another recorded request for this file';
}
