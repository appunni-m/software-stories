import { invoke } from '@tauri-apps/api/core';
import type {
  CaptureStatus, ChapterClaim, ClaudeGenerationSettings, ProviderCommandResult, ProviderRelationshipEvidence,
  ProviderRequestFile, ProviderRequestReview, ProviderSymbolEvidence,
} from './commands';

/** One bounded planning unit; this does not request AI inference or cover the whole repository. */
export interface PrepareRepositoryPlanInput {
  projectId: string;
  snapshotId: string;
  question: string;
  selectedPaths: string[];
}

export interface PlanSourceCoverage {
  selectedFileCount: number;
  eligibleCapturedFileCount: number;
  unselectedEligibleFileCount: number;
  excludedCapturedPathCount: number;
  skippedPathCount: number;
}

/** Native source/prompt observation, not an agent review, connection or consent. */
export interface PreparedRepositoryPlanContext {
  projectId: string;
  snapshotId: string;
  question: string;
  promptVersion: 'grounded-repository-plan-v1';
  captureStatus: CaptureStatus;
  inventorySha256: string;
  contextSha256: string;
  coverage: PlanSourceCoverage;
  selectedFiles: ProviderRequestFile[];
  selectedSymbols: ProviderSymbolEvidence[];
  selectedRelationships: ProviderRelationshipEvidence[];
  indexStatus: 'complete' | 'partial' | 'cancelled' | 'missing' | 'unreadable';
  indexEvidenceTruncated: boolean;
  sourceBytes: number;
  systemInstruction: string;
  userPrompt: string;
  outputSchema: Record<string, unknown>;
  requestBodyBytes: number;
  requestBodySha256: string;
  estimatedInputTokens: number;
}

/** Prepare exact immutable evidence locally. No API credential or agent login is involved. */
export function prepareRepositoryPlanContext(input: PrepareRepositoryPlanInput): Promise<PreparedRepositoryPlanContext> {
  return invoke('prepare_repository_plan_context', { input });
}

/** Display metadata from a native agent review; it grants no inference admission. */
export interface RepositoryPlanReviewScope {
  schemaVersion: 1;
  question: string;
  inventorySha256: string;
  contextSha256: string;
  planningContractSha256: string;
  captureStatus: CaptureStatus;
  coverage: PlanSourceCoverage;
  indexStatus: PreparedRepositoryPlanContext['indexStatus'];
}

export interface CodexRepositoryPlanInput extends PrepareRepositoryPlanInput {
  modelId: string;
  effort: string;
  approvalPolicy: 'never';
}

export interface ClaudeRepositoryPlanInput extends PrepareRepositoryPlanInput {
  settings: ClaudeGenerationSettings;
}

/** Native validated plan data. This type alone carries no acceptance or persistence. */
export interface RepositoryPlanDraft {
  schemaVersion: 1;
  draftId: string;
  projectId: string;
  snapshotId: string;
  question: string;
  title: string;
  captureStatus: CaptureStatus;
  inventorySha256: string;
  contextSha256: string;
  createdAtUnixMs: number;
  coverage: PlanSourceCoverage;
  plannedSelectedPaths: string[];
  citedSelectedPaths: string[];
  omittedSelectedPaths: string[];
  steps: {
    stepId: string;
    question: string;
    primaryPath: string;
    prerequisiteStepIds: string[];
    claim: ChapterClaim;
  }[];
  unknowns: ChapterClaim[];
}

/** Native validated draft. Persistence does not accept a plan or explain a file. */
export interface RepositoryPlanPreview {
  reviewId: string;
  recoveryOperationId: string;
  fingerprint: string;
  providerId: 'codex' | 'claude-agent';
  modelId: string;
  modelVersion: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  savedLocally: boolean;
  plan: RepositoryPlanDraft;
}

export type SavedPlanAgentSettings =
  | { backend: 'codex'; effort: string; approvalPolicy: 'never' }
  | { backend: 'claude-agent'; effort: ClaudeGenerationSettings['effort']; approvalPolicy: 'dontAsk'; maxTurns: number };

/** Metadata only. A successful listing does not validate source evidence. */
export interface RepositoryPlanDraftSummary {
  draftId: string;
  title: string;
  question: string | null;
  snapshotId: string | null;
  reviewId: string | null;
  providerId: 'codex' | 'claude-agent' | null;
  modelId: string | null;
  createdAtUnixMs: number | null;
  unavailableReason: string | null;
}

/** Revalidated exact proposal; recorded settings do not prove account identity. */
export interface RepositoryPlanDraftDetail {
  preview: RepositoryPlanPreview;
  scope: RepositoryPlanReviewScope;
  selectedPaths: string[];
  settings: SavedPlanAgentSettings;
}

export function listRepositoryPlanDrafts(projectId: string): Promise<RepositoryPlanDraftSummary[]> {
  return invoke('list_repository_plan_drafts', { projectId });
}

export function loadRepositoryPlanDraft(projectId: string, draftId: string): Promise<RepositoryPlanDraftDetail> {
  return invoke('load_repository_plan_draft', { projectId, draftId });
}

/** Discard does not resolve any separate interruption notice or accepted history. */
export function discardRepositoryPlanDraft(projectId: string, draftId: string): Promise<void> {
  return invoke('discard_repository_plan_draft', { projectId, draftId });
}

/** Local preparation only; installation identity comes from native Codex state. */
export function buildCodexRepositoryPlanReview(input: CodexRepositoryPlanInput): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_codex_repository_plan_review', { input });
}

/** Local preparation only; the opaque native connection supplies account/runtime identity. */
export function buildClaudeRepositoryPlanReview(connectionId: string, input: ClaudeRepositoryPlanInput): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_claude_repository_plan_review', { connectionId, input });
}

/** Native account-backed execution; model/settings/source stay bound to the review. */
export function sendCodexRepositoryPlanRequest(reviewId: string, fingerprint: string, consent: boolean): Promise<ProviderCommandResult<RepositoryPlanPreview>> {
  return invoke('send_codex_repository_plan_request', { input: { reviewId, fingerprint, consent } });
}

/** Native release qualification and the original observed connection remain required. */
export function sendClaudeRepositoryPlanRequest(reviewId: string, fingerprint: string, consent: boolean): Promise<ProviderCommandResult<RepositoryPlanPreview>> {
  return invoke('send_claude_repository_plan_request', { input: { reviewId, fingerprint, consent } });
}
