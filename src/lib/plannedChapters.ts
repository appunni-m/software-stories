import { invoke } from '@tauri-apps/api/core';
import type { ClaudeGenerationSettings, ProviderCommandResult, ProviderRequestReview } from './commands';
import type { RepositoryPlanReviewScope } from './repositoryPlans';

/** Exact accepted revision and step; never a renderer-authored prompt. */
export interface PlannedChapterTarget {
  revisionId: string;
  recordSha256: string;
  stepId: string;
}

export interface PlannedChapterScope {
  schemaVersion: 1;
  target: PlannedChapterTarget;
  snapshotId: string;
  primaryPath: string;
  selectedPaths: string[];
  planQuestion: string;
  question: string;
  prerequisites: { stepId: string; question: string }[];
  planContext: RepositoryPlanReviewScope;
}

export function preparePlannedChapterScope(projectId: string, target: PlannedChapterTarget): Promise<PlannedChapterScope> {
  return invoke('prepare_planned_chapter_scope', { projectId, target });
}

export function buildCodexPlannedChapterReview(input: {
  projectId: string; target: PlannedChapterTarget; modelId: string; effort: string; approvalPolicy: 'never';
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_codex_planned_chapter_review', { input });
}

export function buildClaudePlannedChapterReview(connectionId: string, input: {
  projectId: string; target: PlannedChapterTarget; settings: ClaudeGenerationSettings;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_claude_planned_chapter_review', { connectionId, input });
}

/** UI guard only. Native preparation, preflight and result validation each
 * reconstruct this scope from accepted history and original captured source. */
export function matchesPlannedChapterReview(review: ProviderRequestReview, expected: PlannedChapterScope): boolean {
  const raw = review.generationConfig.plannedChapter;
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return false;
  const actual = raw as unknown as PlannedChapterScope;
  const a = actual.planContext;
  const b = expected.planContext;
  const coverage = a?.coverage;
  const target = actual.target;
  const paths = new Set(expected.selectedPaths);
  const contract = review.providerId === 'codex' ? 'codex-planned-chapter-v1'
    : review.providerId === 'claude-agent' ? 'claude-planned-chapter-v1' : null;
  return contract !== null && review.generationConfig.contractVersion === contract
    && review.purpose === 'fileChapter' && review.targetPath === expected.primaryPath
    && review.snapshotId === expected.snapshotId && actual.schemaVersion === 1
    && review.sectionUpdateContext === null && review.codeLensContext === null
    && review.prGuideContext === null && review.prChangeContext === null
    && Number.isFinite(review.expiresAtUnixMs) && review.expiresAtUnixMs > Date.now()
    && target?.revisionId === expected.target.revisionId && target.recordSha256 === expected.target.recordSha256
    && target.stepId === expected.target.stepId && actual.snapshotId === expected.snapshotId
    && actual.primaryPath === expected.primaryPath && actual.question === expected.question && actual.planQuestion === expected.planQuestion
    && Array.isArray(actual.selectedPaths) && actual.selectedPaths.length === expected.selectedPaths.length
    && actual.selectedPaths.every((path, index) => path === expected.selectedPaths[index])
    && paths.size > 0 && paths.size <= 12 && paths.size === expected.selectedPaths.length
    && review.selectedFiles.length === paths.size && new Set(review.selectedFiles.map((file) => file.path)).size === paths.size
    && review.selectedFiles.every((file) => paths.has(file.path))
    && Array.isArray(actual.prerequisites) && actual.prerequisites.length === expected.prerequisites.length
    && actual.prerequisites.every((step, index) => step?.stepId === expected.prerequisites[index].stepId && step.question === expected.prerequisites[index].question)
    && a?.schemaVersion === b.schemaVersion && a.question === b.question && a.inventorySha256 === b.inventorySha256
    && a.contextSha256 === b.contextSha256 && a.planningContractSha256 === b.planningContractSha256
    && a.captureStatus === b.captureStatus && a.indexStatus === b.indexStatus
    && coverage?.selectedFileCount === b.coverage.selectedFileCount && coverage.eligibleCapturedFileCount === b.coverage.eligibleCapturedFileCount
    && coverage.unselectedEligibleFileCount === b.coverage.unselectedEligibleFileCount
    && coverage.excludedCapturedPathCount === b.coverage.excludedCapturedPathCount && coverage.skippedPathCount === b.coverage.skippedPathCount;
}
