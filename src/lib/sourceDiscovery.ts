import { invoke } from '@tauri-apps/api/core';
export interface SourceDiscoveryScope { projectId: string; snapshotId: string; question: string; targetPath?: string | null }
export interface DocumentationNotice { path: string; status: 'declaredDeprecated' | 'unverified'; reason: string }
export type DiscoveryCoverageGapCategory = 'largeFiles' | 'unreadableSource' | 'unsupportedLanguage' | 'indexLimit' | 'partialParse' | 'unavailableSource' | 'incompleteIndex' | 'unindexedFiles';
export interface DiscoveryCoverageGap { category: DiscoveryCoverageGapCategory; count: number; examples: string[]; message: string }
export const SOURCE_DISCOVERY_COVERAGE_LABELS: Record<DiscoveryCoverageGapCategory, string> = {
  largeFiles: 'large files',
  unreadableSource: 'captured code paths with non-text contents',
  unsupportedLanguage: 'files without a local parser',
  indexLimit: 'files beyond local index limits',
  partialParse: 'partially mapped files',
  unavailableSource: 'files the local code map could not read',
  incompleteIndex: 'files left unmapped when indexing stopped',
  unindexedFiles: 'files absent from the saved code map',
};
export type SourceDiscoveryIndexStatus = 'complete' | 'partial' | 'cancelled' | 'missing' | 'unreadable';
export interface SourceDiscoveryReview {
  reviewId: string; fingerprint: string; modelId: string; effort: string; question: string;
  metadata: string; candidateCount: number; omittedCount: number; indexStatus: SourceDiscoveryIndexStatus;
  documentation: DocumentationNotice[]; coverageGaps: DiscoveryCoverageGap[];
}
export interface SourceDiscoveryResult {
  selectedFiles: { path: string; reason: string }[];
  /** One large implementation to explore through bounded source-range tools. */
  rangeTargets?: { path: string; reason: string }[];
  limitation: string;
}

/** Preserve bounded discovery omissions with a large-file answer so readers
 * can see its evidence boundary after leaving the dialog or reopening the app. */
export function sourceDiscoveryLimitNotes(
  result: Pick<SourceDiscoveryResult, 'limitation'>,
  review?: Pick<SourceDiscoveryReview, 'omittedCount' | 'coverageGaps'>,
): string[] {
  const clean = (value: string, limit: number) => value
    .replace(/[\u0000-\u001f\u007f-\u009f]/gu, ' ')
    .replace(/\s+/gu, ' ')
    .trim()
    .slice(0, limit);
  const notes: string[] = [];
  const coverage: string[] = [];
  if (review && review.omittedCount > 0) {
    coverage.push(`${review.omittedCount.toLocaleString()} candidate paths did not fit the bounded discovery review.`);
  }
  const gaps = (review?.coverageGaps ?? []).map((gap) => {
    const examples = gap.examples.slice(0, 1).map((path) => clean(path, 120)).filter(Boolean);
    return `${gap.count.toLocaleString()} ${SOURCE_DISCOVERY_COVERAGE_LABELS[gap.category]}${examples.length ? ` (example: ${examples[0]})` : ''}`;
  });
  if (gaps.length) coverage.push(`Repository coverage gaps: ${gaps.join('; ')}.`);
  if (coverage.length) notes.push(clean(`Discovery coverage: ${coverage.join(' ')}`, 1_500));
  const limitation = clean(result.limitation, 1_480);
  if (limitation) notes.push(`Discovery note: ${limitation}`);
  return notes;
}
export interface LargeFileDiscoveryHandoff {
  projectId: string;
  snapshotId: string;
  targetPath: string;
  supportingPaths: string[];
  question: string;
  discoveryNotes: string[];
}
export const prepareSourceDiscovery = (input: SourceDiscoveryScope & { modelId: string; effort: string; workspaceGrant?: string }) =>
  invoke<SourceDiscoveryReview>('prepare_source_discovery', { input: { projectId: input.projectId, snapshotId: input.snapshotId, question: input.question, targetPath: input.targetPath ?? null, modelId: input.modelId, effort: input.effort, workspaceGrant: input.workspaceGrant ?? null } });
export const sendSourceDiscovery = (review: SourceDiscoveryReview) =>
  invoke<SourceDiscoveryResult>('send_source_discovery', { input: { reviewId: review.reviewId, fingerprint: review.fingerprint, consent: true } });
export const discardSourceDiscovery = (reviewId: string) => invoke<void>('discard_source_discovery', { reviewId });
