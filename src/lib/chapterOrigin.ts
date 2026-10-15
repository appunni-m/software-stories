import type { SectionUpdateReviewTarget } from './commands';
import type { PlannedChapterScope, PlannedChapterTarget } from './plannedChapters';

export type RecordedAgentSettings = { backend: 'codex'; modelId: string; effort: string; approvalPolicy: 'never' }
  | { backend: 'claude-agent'; modelId: string; effort: string | null; approvalPolicy: 'dontAsk'; maxTurns: number };

/** Local dispatch lineage. This is not a cryptographic account attestation. */
export interface ChapterGenerationOrigin {
  schemaVersion: 1;
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
  contentSha256: string;
}

export function samePlannedTarget(left: PlannedChapterTarget | null | undefined, right: PlannedChapterTarget): boolean {
  return !!left && left.revisionId === right.revisionId && left.recordSha256 === right.recordSha256 && left.stepId === right.stepId;
}

export function recordedRecoveryLabel(origin: ChapterGenerationOrigin | null | undefined,
  requestId: string | undefined, backend: 'codex' | 'claude-agent', target?: PlannedChapterTarget,
): string {
  if (!origin) return 'Legacy or unrecorded request · possible file match';
  const stepMatches = target ? samePlannedTarget(origin.plannedChapter?.target, target) : !origin.plannedChapter;
  if (origin.settings.backend === backend && origin.requestId === requestId && stepMatches) return 'Recorded request match';
  if (target && origin.settings.backend === backend && stepMatches) return 'Same recorded learning step · another request';
  return 'Another recorded request for this file';
}
