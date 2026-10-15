import {
  buildClaudeChapterReview, buildClaudeCodeLensReview, buildClaudePrGuideReview, buildClaudePrChangeReview,
  type ClaudeGenerationSettings, type CodeLensExplanationTarget, type PrChangeExplanationTarget,
  type ProviderRequestReview, type SectionUpdateReviewTarget, type SelectedGitChangeIdentity,
} from './commands';
import { buildClaudeRepositoryPlanReview, type PrepareRepositoryPlanInput } from './repositoryPlans';
import { buildClaudePlannedChapterReview, matchesPlannedChapterReview, type PlannedChapterScope } from './plannedChapters';

export type ClaudeChapterScope = {
  kind: 'review'; projectId: string; snapshotId: string; targetPath: string; selectedPaths?: string[];
  plannedChapter?: PlannedChapterScope;
} | {
  kind: 'overview'; projectId: string; snapshotId: string; selectedPaths?: string[];
} | {
  kind: 'section-update'; projectId: string; snapshotId: string; targetPath: string;
  selectedPaths: string[]; sectionUpdate: SectionUpdateReviewTarget;
};

export type ClaudeOperationReviewMode = (ClaudeChapterScope | (PrepareRepositoryPlanInput & {
  kind: 'repository-plan';
}) | {
  kind: 'code-lens-range'; projectId: string; snapshotId: string; target: CodeLensExplanationTarget;
} | {
  kind: 'pr-guide'; projectId: string; baseRef: string; headRef: string;
  comparisonId: string; selectedChanges: SelectedGitChangeIdentity[];
} | {
  kind: 'pr-change'; projectId: string; target: PrChangeExplanationTarget;
}) & { recoveryOperationId?: string };

export function isClaudeChapterScope(mode: ClaudeOperationReviewMode | null): mode is ClaudeChapterScope & { recoveryOperationId?: string } {
  return mode !== null && (mode.kind === 'review' || mode.kind === 'overview' || mode.kind === 'section-update');
}

/** Remount/dispose pending review state for any exact evidence change. */
export function claudeReviewScopeKey(mode: ClaudeOperationReviewMode | 'settings' | null): string {
  if (mode === null) return 'none';
  if (mode === 'settings') return 'settings';
  switch (mode.kind) {
    case 'repository-plan': return JSON.stringify([mode.kind, mode.projectId, mode.snapshotId, mode.question, mode.selectedPaths, mode.recoveryOperationId ?? null]);
    case 'review': case 'overview': case 'section-update':
      return JSON.stringify([mode.kind, mode.projectId, mode.snapshotId,
        mode.kind === 'overview' ? null : mode.targetPath, mode.selectedPaths ?? [],
        mode.kind === 'section-update' ? mode.sectionUpdate : null, mode.kind === 'review' ? mode.plannedChapter ?? null : null, mode.recoveryOperationId ?? null]);
    case 'code-lens-range': return JSON.stringify([mode.kind, mode.projectId, mode.snapshotId, mode.target, mode.recoveryOperationId ?? null]);
    case 'pr-guide': return JSON.stringify([mode.kind, mode.projectId, mode.baseRef, mode.headRef, mode.comparisonId, mode.selectedChanges, mode.recoveryOperationId ?? null]);
    case 'pr-change': return JSON.stringify([mode.kind, mode.projectId, mode.target, mode.recoveryOperationId ?? null]);
  }
}

export function claudeOperationLabel(mode: ClaudeOperationReviewMode): string {
  switch (mode.kind) {
    case 'repository-plan': return 'Repository learning plan';
    case 'review': return mode.plannedChapter ? 'Accepted-step chapter' : 'File chapter';
    case 'overview': return 'Selected-source overview';
    case 'section-update': return 'One-section repair';
    case 'code-lens-range': return 'Selected code range';
    case 'pr-guide': return 'PR Guide';
    case 'pr-change': return 'Single diff hunk';
  }
}

/** Offline review dispatch only. No agent process, model turn or consent is
 * started here; Git preparation may read existing local commit objects. */
export function prepareClaudeOperationReview(
  connectionId: string, mode: ClaudeOperationReviewMode, settings: ClaudeGenerationSettings, selectedPaths: string[],
) {
  switch (mode.kind) {
    case 'repository-plan': return buildClaudeRepositoryPlanReview(connectionId, {
      projectId: mode.projectId, snapshotId: mode.snapshotId, question: mode.question,
      selectedPaths: mode.selectedPaths, settings,
    });
    case 'review': case 'overview': case 'section-update':
      if (mode.kind === 'review' && mode.plannedChapter) return buildClaudePlannedChapterReview(connectionId, {
        projectId: mode.projectId, target: mode.plannedChapter.target, settings,
      });
      return buildClaudeChapterReview(connectionId, {
        projectId: mode.projectId, snapshotId: mode.snapshotId,
        targetPath: mode.kind === 'overview' ? null : mode.targetPath,
        selectedPaths, settings, sectionUpdate: mode.kind === 'section-update' ? mode.sectionUpdate : null,
      });
    case 'code-lens-range': return buildClaudeCodeLensReview(connectionId, {
      projectId: mode.projectId, snapshotId: mode.snapshotId, target: mode.target, settings,
    });
    case 'pr-guide': return buildClaudePrGuideReview(connectionId, {
      projectId: mode.projectId, baseRef: mode.baseRef, headRef: mode.headRef,
      comparisonId: mode.comparisonId, selectedChanges: mode.selectedChanges, settings,
    });
    case 'pr-change': return buildClaudePrChangeReview(connectionId, { projectId: mode.projectId, target: mode.target, settings });
  }
}

export function gitChangeLabel(change: SelectedGitChangeIdentity): string {
  const { basePath, headPath } = change;
  return basePath && headPath && basePath !== headPath ? `${basePath} → ${headPath}` : headPath ?? basePath ?? '';
}

function sameChanges(left: SelectedGitChangeIdentity[], right: SelectedGitChangeIdentity[]): boolean {
  const identity = (change: SelectedGitChangeIdentity) => JSON.stringify([change.basePath, change.headPath]);
  const expected = new Set(right.map(identity));
  return expected.size === right.length && left.length === right.length
    && new Set(left.map(identity)).size === left.length && left.every((change) => expected.has(identity(change)));
}

function sameRange(left: CodeLensExplanationTarget | null, right: CodeLensExplanationTarget): boolean {
  return !!left && left.path === right.path && left.sourceSha256 === right.sourceSha256
    && left.startLine === right.startLine && left.endLine === right.endLine
    && left.startByte === right.startByte && left.endByteExclusive === right.endByteExclusive;
}

function sameHunk(left: PrChangeExplanationTarget | null, right: PrChangeExplanationTarget): boolean {
  return !!left && left.baseRef === right.baseRef && left.headRef === right.headRef
    && left.comparisonId === right.comparisonId && left.hunkIndex === right.hunkIndex
    && sameChanges([left.selectedChange], [right.selectedChange]);
}

/** UI identity check, in addition to authoritative native revalidation. It
 * cannot qualify runtime/account access or validate model output. */
export function matchesClaudeOperationReview(
  review: ProviderRequestReview, mode: ClaudeOperationReviewMode, settings: ClaudeGenerationSettings,
  connectionId: string, selectedPaths: string[],
): boolean {
  const config = review.generationConfig;
  if (review.providerId !== 'claude-agent' || review.endpointUrl !== 'claude-agent-cli://stdio'
    || review.projectId !== mode.projectId || review.modelId !== settings.modelId
    || config.connectionId !== connectionId || config.effort !== settings.effort
    || config.approvalPolicy !== settings.approvalPolicy || config.maxTurns !== settings.maxTurns) return false;
  const noGit = review.prGuideContext === null && review.prChangeContext === null;
  const noSection = review.sectionUpdateContext === null;
  const noRange = review.codeLensContext === null;
  switch (mode.kind) {
    case 'repository-plan': {
      const planning = config.repositoryPlan;
      const paths = new Set(mode.selectedPaths);
      return noGit && noSection && noRange && review.targetPath === null
        && review.purpose === 'repositoryPlan' && review.snapshotId === mode.snapshotId
        && config.contractVersion === 'claude-repository-plan-v1'
        && !!planning && typeof planning === 'object' && !Array.isArray(planning)
        && 'question' in planning && planning.question === mode.question && 'schemaVersion' in planning && planning.schemaVersion === 1
        && paths.size === mode.selectedPaths.length && review.selectedFiles.length === paths.size
        && new Set(review.selectedFiles.map((file) => file.path)).size === paths.size
        && review.selectedFiles.every((file) => paths.has(file.path));
    }
    case 'review': case 'overview': case 'section-update': {
      if (mode.kind === 'review' && mode.plannedChapter) return matchesPlannedChapterReview(review, mode.plannedChapter)
        && review.snapshotId === mode.snapshotId && review.targetPath === mode.targetPath;
      const section = mode.kind === 'section-update' ? mode.sectionUpdate : null;
      const returned = review.sectionUpdateContext;
      const sectionMatches = section ? returned?.chapterRevisionId === section.chapterRevisionId
        && returned?.sectionId === section.sectionId && returned?.sectionRevisionId === section.sectionRevisionId : noSection;
      const paths = new Set(selectedPaths);
      return noGit && noRange && sectionMatches && config.contractVersion === 'claude-chapter-v1'
        && review.snapshotId === mode.snapshotId && review.targetPath === (mode.kind === 'overview' ? null : mode.targetPath)
        && review.purpose === (mode.kind === 'overview' ? 'repositoryOverview' : section ? 'sectionUpdate' : 'fileChapter')
        && paths.size === selectedPaths.length && review.selectedFiles.length === paths.size
        && new Set(review.selectedFiles.map((file) => file.path)).size === paths.size
        && review.selectedFiles.every((file) => paths.has(file.path));
    }
    case 'code-lens-range': return noGit && noSection && config.contractVersion === 'claude-code-lens-v1'
      && review.purpose === 'codeLensExplanation' && review.snapshotId === mode.snapshotId
      && review.targetPath === mode.target.path && sameRange(review.codeLensContext, mode.target)
      && review.selectedFiles.length === 1 && review.selectedFiles[0].path === mode.target.path
      && review.selectedFiles[0].sha256 === mode.target.sourceSha256;
    case 'pr-guide': {
      const context = review.prGuideContext;
      const labels = new Set(mode.selectedChanges.map(gitChangeLabel));
      return noSection && noRange && review.prChangeContext === null && config.contractVersion === 'claude-pr-guide-v1'
        && review.purpose === 'prGuide' && review.snapshotId === mode.comparisonId && review.targetPath === null
        && !!context && context.baseRef === mode.baseRef && context.headRef === mode.headRef && context.comparisonId === mode.comparisonId
        && sameChanges(context.selectedChanges, mode.selectedChanges) && review.selectedFiles.length === mode.selectedChanges.length
        && labels.size === mode.selectedChanges.length && review.selectedFiles.every((file) => labels.has(file.path))
        && new Set(review.selectedFiles.map((file) => file.path)).size === labels.size;
    }
    case 'pr-change': return noSection && noRange && review.prGuideContext === null && config.contractVersion === 'claude-pr-change-v1'
      && review.purpose === 'prChangeExplanation' && review.snapshotId === mode.target.comparisonId
      && review.targetPath === gitChangeLabel(mode.target.selectedChange) && sameHunk(review.prChangeContext, mode.target)
      && review.selectedFiles.length === 1 && review.selectedFiles[0].path === review.targetPath;
  }
}
