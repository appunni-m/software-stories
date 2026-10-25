import type { ChapterDraftPreview, ChapterRevision, PendingChapterPreviewSummary, ProviderRequestReview } from './commands';
import { samePlannedTarget } from './chapterOrigin';
import type { PlannedChapterScope } from './plannedChapters';

export interface ReviewedBatchStep { scope: PlannedChapterScope; review: ProviderRequestReview }
export interface ChapterBatchCommands {
  build: (scope: PlannedChapterScope) => Promise<ProviderRequestReview>;
  matches: (review: ProviderRequestReview, scope: PlannedChapterScope) => boolean;
  send: (review: ProviderRequestReview) => Promise<ChapterDraftPreview>;
  save: (generationId: string) => Promise<ChapterRevision>;
  list: () => Promise<ChapterRevision[]>;
  findPending?: (scope: PlannedChapterScope) => Promise<PendingChapterPreviewSummary | null>;
  discardReview: (reviewId: string) => Promise<void>;
  discardDraft: (generationId: string) => Promise<void>;
}

/** Do not regenerate a route step when a matching or unattributable result may still need review. */
export function pendingPreviewForStep(
  previews: readonly PendingChapterPreviewSummary[],
  scope: PlannedChapterScope,
  projectId: string,
): PendingChapterPreviewSummary | null {
  return previews.find((preview) => {
    if (preview.unavailableReason) {
      return preview.targetPath === scope.primaryPath
        && (preview.projectId === null || preview.projectId === projectId)
        && (preview.snapshotId === null || preview.snapshotId === scope.snapshotId);
    }
    if (preview.projectId !== projectId || preview.snapshotId !== scope.snapshotId
      || preview.targetPath !== scope.primaryPath) return false;
    if (!preview.generationOrigin) return true;
    const origin = preview.generationOrigin;
    if (origin.projectId !== projectId || origin.snapshotId !== scope.snapshotId || origin.targetPath !== scope.primaryPath) return false;
    return Boolean(origin.plannedChapter && samePlannedTarget(origin.plannedChapter.target, scope.target));
  }) ?? null;
}

/** Resume only from a saved revision with this exact accepted learning step. */
export function chapterForStep(chapters: ChapterRevision[], scope: PlannedChapterScope, projectId: string) {
  return chapters.find((chapter) => {
    const origin = chapter.generationOrigin;
    const planned = origin?.plannedChapter;
    return chapter.lifecycle !== 'rejected' && !!origin && !!planned && chapter.projectId === projectId && origin.projectId === projectId
        && chapter.snapshotId === scope.snapshotId && origin.snapshotId === scope.snapshotId
        && origin.targetPath === scope.primaryPath && planned.snapshotId === scope.snapshotId
        && planned.primaryPath === scope.primaryPath && planned.question === scope.question
        && planned.target.revisionId === scope.target.revisionId
        && planned.target.recordSha256 === scope.target.recordSha256 && planned.target.stepId === scope.target.stepId;
  });
}

/** One explicit authorization covers only these exact locally reviewed requests.
 * Rebuild each request just before dispatch; never carry an expired review into
 * the next chapter, widen source scope, switch accounts, or retry inference. */
export async function runChapterBatch(steps: ReviewedBatchStep[], commands: ChapterBatchCommands, options: {
  cancelled: () => boolean;
  active: (reviewId: string | null) => void;
  progress: (index: number, phase: 'generating' | 'saved' | 'skipped' | 'review', revision?: ChapterRevision, pending?: PendingChapterPreviewSummary) => void;
}) {
  if (!steps.length || steps.length > 24 || new Set(steps.map((step) => step.scope.target.stepId)).size !== steps.length) throw Error('Review a valid route before building chapters.');
  const saved: ChapterRevision[] = [];
  for (const [index, step] of steps.entries()) {
    if (options.cancelled()) break;
    const existing = chapterForStep(await commands.list(), step.scope, step.review.projectId);
    if (options.cancelled()) break;
    if (existing) { saved.push(existing); options.progress(index, 'skipped', existing); continue; }
    const pending = await commands.findPending?.(step.scope) ?? null;
    if (options.cancelled()) break;
    if (pending) { options.progress(index, 'review', undefined, pending); continue; }
    const fresh = await commands.build(step.scope);
    try {
      if (options.cancelled()) break;
      if (!commands.matches(fresh, step.scope) || fresh.fingerprint !== step.review.fingerprint
        || fresh.requestBodySha256 !== step.review.requestBodySha256 || fresh.modelId !== step.review.modelId
        || fresh.projectId !== step.review.projectId || fresh.snapshotId !== step.review.snapshotId) {
        throw Error('The source, account, runtime or settings changed. Review the remaining chapters again.');
      }
      options.active(fresh.reviewId);
      options.progress(index, 'generating');
      const preview = await commands.send(fresh);
      options.active(null);
      if (options.cancelled()) { await commands.discardDraft(preview.generationId); break; }
      if (preview.validation.status !== 'valid' || !chapterForStep([preview.revision], step.scope, step.review.projectId)) {
        await commands.discardDraft(preview.generationId);
        throw Error('The returned chapter did not match its validated reading step. No draft was saved.');
      }
      let revision: ChapterRevision;
      try { revision = await commands.save(preview.generationId); }
      catch {
        // A lost reply may follow a successful immutable write. Reconcile the
        // exact ID before reporting failure; never regenerate automatically.
        const found = (await commands.list()).find((item) => item.revisionId === preview.revision.revisionId);
        if (!found) throw Error('The draft save could not be confirmed. Check Edit and pending previews before continuing.');
        revision = found;
        await commands.discardDraft(preview.generationId).catch(() => {});
      }
      if (revision.revisionId !== preview.revision.revisionId || !chapterForStep([revision], step.scope, step.review.projectId)) throw Error('The saved chapter identity changed. Check Edit before continuing.');
      saved.push(revision); options.progress(index, 'saved', revision);
    } finally {
      options.active(null);
      await commands.discardReview(fresh.reviewId).catch(() => {});
    }
  }
  return saved;
}
