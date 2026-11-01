import type {
  ChapterDraftPreview, ChapterRevision, PendingChapterPreviewSummary, ProviderRequestReview,
} from './commands.ts';
import { isCurrentChapterGenerationOrigin, samePlannedTarget } from './chapterOrigin.ts';
import { matchesPlannedChapterReview, type PlannedChapterScope } from './plannedChapters.ts';

export function exactSavedPlannedChapter(revision: ChapterRevision, scope: PlannedChapterScope): boolean {
  if (revision.lifecycle !== 'draft' && revision.lifecycle !== 'accepted') return false;
  const origin = revision.generationOrigin;
  return isCurrentChapterGenerationOrigin(origin)
    && samePlannedTarget(origin.plannedChapter?.target, scope.target)
    && origin.projectId === revision.projectId && origin.snapshotId === revision.snapshotId;
}

export function sameReviewedPlannedChapterRequest(
  reviewed: ProviderRequestReview,
  current: ProviderRequestReview,
  scope: PlannedChapterScope,
  selection: { modelId: string; effort: string },
): boolean {
  if (!matchesPlannedChapterReview(current, scope)
    || current.providerId !== 'codex' || current.projectId !== reviewed.projectId
    || current.snapshotId !== reviewed.snapshotId || current.purpose !== reviewed.purpose
    || current.modelId !== selection.modelId || reviewed.modelId !== selection.modelId
    || current.generationConfig.effort !== selection.effort
    || reviewed.generationConfig.effort !== selection.effort
    || current.generationConfig.approvalPolicy !== 'never'
    || reviewed.generationConfig.approvalPolicy !== 'never'
    || current.fingerprint !== reviewed.fingerprint
    || current.requestBodySha256 !== reviewed.requestBodySha256
    || current.requestBodyBytes !== reviewed.requestBodyBytes
    || current.selectedFiles.length !== reviewed.selectedFiles.length) return false;
  return current.selectedFiles.every((file, index) => {
    const previous = reviewed.selectedFiles[index];
    return file.path === previous.path && file.sha256 === previous.sha256
      && file.sizeBytes === previous.sizeBytes && file.contents === previous.contents;
  });
}

export function matchingPlannedChapterPendingPreview(
  previews: PendingChapterPreviewSummary[], projectId: string, snapshotId: string,
  scopes: PlannedChapterScope[],
): PendingChapterPreviewSummary | null {
  for (const preview of previews) {
    if (preview.projectId !== projectId || preview.snapshotId !== snapshotId) continue;
    const raw = preview.generationOrigin?.plannedChapter?.target;
    if (!raw || !scopes.some((scope) => samePlannedTarget(raw, scope.target))) continue;
    return preview;
  }
  return null;
}

export function matchesPlannedChapterPreview(
  preview: ChapterDraftPreview, review: ProviderRequestReview, scope: PlannedChapterScope,
): boolean {
  const revision = preview.revision;
  const origin = revision.generationOrigin;
  if (preview.validation.status !== 'valid' || revision.projectId !== review.projectId
    || revision.snapshotId !== scope.snapshotId || revision.lifecycle !== 'draft'
    || !isCurrentChapterGenerationOrigin(origin) || !origin.plannedChapter
    || !samePlannedTarget(origin.plannedChapter.target, scope.target)
    || origin.projectId !== review.projectId || origin.snapshotId !== scope.snapshotId
    || origin.targetPath !== scope.primaryPath || origin.purpose !== 'fileChapter'
    || origin.requestId !== review.reviewId || origin.requestFingerprint !== review.fingerprint
    || origin.requestBodySha256 !== review.requestBodySha256 || origin.contentSha256.length !== 64
    || origin.settings.backend !== 'codex' || origin.settings.modelId !== review.modelId
    || origin.contractVersion !== review.generationConfig.contractVersion
    || origin.settings.effort !== review.generationConfig.effort || origin.settings.approvalPolicy !== 'never'
    || origin.sourceFiles.length !== review.selectedFiles.length) return false;
  return origin.sourceFiles.every((source, index) => {
    const selected = review.selectedFiles[index];
    return source.path === selected.path && source.sha256 === selected.sha256
      && source.sizeBytes === selected.sizeBytes;
  });
}
