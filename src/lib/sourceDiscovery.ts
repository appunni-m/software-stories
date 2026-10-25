import { invoke } from '@tauri-apps/api/core';
export interface SourceDiscoveryScope { projectId: string; snapshotId: string; question: string; targetPath?: string | null }
export interface DocumentationNotice { path: string; status: 'declaredDeprecated' | 'unverified'; reason: string }
export type SourceDiscoveryIndexStatus = 'complete' | 'partial' | 'cancelled' | 'missing' | 'unreadable';
export interface SourceDiscoveryReview {
  reviewId: string; fingerprint: string; modelId: string; effort: string; question: string;
  metadata: string; candidateCount: number; omittedCount: number; indexStatus: SourceDiscoveryIndexStatus;
  documentation: DocumentationNotice[];
}
export interface SourceDiscoveryResult { selectedFiles: { path: string; reason: string }[]; limitation: string }
export const prepareSourceDiscovery = (input: SourceDiscoveryScope & { modelId: string; effort: string; workspaceGrant?: string }) =>
  invoke<SourceDiscoveryReview>('prepare_source_discovery', { input: { projectId: input.projectId, snapshotId: input.snapshotId, question: input.question, targetPath: input.targetPath ?? null, modelId: input.modelId, effort: input.effort, workspaceGrant: input.workspaceGrant ?? null } });
export const sendSourceDiscovery = (review: SourceDiscoveryReview) =>
  invoke<SourceDiscoveryResult>('send_source_discovery', { input: { reviewId: review.reviewId, fingerprint: review.fingerprint, consent: true } });
export const discardSourceDiscovery = (reviewId: string) => invoke<void>('discard_source_discovery', { reviewId });
export const getDocumentationNotices = (projectId: string, snapshotId: string) =>
  invoke<DocumentationNotice[]>('get_documentation_notices', { projectId, snapshotId });
