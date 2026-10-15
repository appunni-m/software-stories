import { invoke } from '@tauri-apps/api/core';
import type { CaptureStatus, ProviderRelationshipEvidence, ProviderRequestFile, ProviderSymbolEvidence } from './commands';

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
