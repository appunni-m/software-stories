import { invoke } from '@tauri-apps/api/core';

export interface ApplicationStatus {
  productName: string;
  version: string;
  native: boolean;
}

export interface RecentProject {
  projectId: string;
  displayName: string;
  selectedPath: string;
  repositoryRoot: string;
  branch: string | null;
  head: string | null;
  accessStatus: 'available';
  lastOpenedUnixMs: number;
}

export interface InterruptedOperation {
  operationId: string;
  kind: 'capture' | 'providerGeneration';
  projectId: string;
  snapshotId: string;
  targetPath: string | null;
  startedAtUnixMs: number;
  providerRequest?: {
    providerId: string;
    purpose: 'fileChapter' | 'repositoryOverview' | 'sectionUpdate' | 'codeLensExplanation' | 'prGuide' | 'prChangeExplanation';
    selectedPaths: string[];
    sectionUpdate: SectionUpdateReviewTarget | null;
  } | null;
}

export type ProjectOpenStatus =
  | 'cancelled'
  | 'opened'
  | 'notGitRepository'
  | 'invalidDirectory'
  | 'permissionDenied'
  | 'gitUnavailable'
  | 'gitError'
  | 'storageError'
  | 'recentProjectUnavailable';

export interface ProjectOpenResult {
  status: ProjectOpenStatus;
  project: RecentProject | null;
  selectedPath: string | null;
  message: string | null;
}

export type CaptureStatus = 'running' | 'complete' | 'partial' | 'cancelled' | 'inconsistent' | 'failed';
export type CapturePhase = 'scanning' | 'checkingIgnoreRules' | 'readingFiles' | 'finalizing';
export type GitChangeState = 'clean' | 'modified';

export interface CaptureLimits {
  policyVersion: string;
  maxFileBytes: number;
  maxTotalBytes: number;
  maxPaths: number;
  maxPathBytes: number;
  maxEntriesPerDirectory: number;
  excludedDirectories: string[];
  otherExclusions: string[];
}

export interface SnapshotSummary {
  snapshotId: string;
  projectId: string;
  parentSnapshotId: string | null;
  createdAtUnixMs: number;
  branch: string | null;
  head: string | null;
  indexState: GitChangeState;
  worktreeState: GitChangeState;
  status: CaptureStatus;
  filesCaptured: number;
  pathsSkipped: number;
  totalBytes: number;
  pathsVisited: number;
  policyVersion: string;
}

export interface SnapshotProgress {
  projectId: string;
  snapshotId: string;
  parentSnapshotId: string | null;
  status: CaptureStatus;
  phase: CapturePhase;
  pathsVisited: number;
  candidateFiles: number;
  filesProcessed: number;
  filesCaptured: number;
  pathsSkipped: number;
  bytesRead: number;
  currentPath: string | null;
  message: string | null;
  limits: CaptureLimits;
  summary: SnapshotSummary | null;
}

export type GitComparisonStatus = 'ready' | 'chooseAgain' | 'limited' | 'error';
export type GitRefSide = 'base' | 'head';
export type GitRefProblemKind = 'invalid' | 'missing' | 'ambiguous' | 'notCommit';
export type GitChangeKind = 'added' | 'deleted' | 'modified' | 'renamed' | 'copied' | 'typeChanged' | 'other';
export type GitEvidenceClassification = 'text' | 'binary' | 'oversized' | 'unsupported' | 'parserUncertain';

export interface GitRefProblem {
  side: GitRefSide;
  requested: string;
  kind: GitRefProblemKind;
  message: string;
}

export interface ResolvedGitRef {
  requested: string;
  objectId: string;
}

export interface GitSourceAnchor {
  revisionId: string;
  path: string;
  blobId: string;
  startLine: number;
  endLine: number;
}

export type GitRelationshipImpactStatus = 'notNeeded' | 'complete' | 'partial' | 'unavailable';

export interface GitRelationshipImpact {
  status: GitRelationshipImpactStatus;
  message: string;
}

export interface GitRelationshipEvidence {
  kind: string;
  confidence: string;
  sourcePath: string;
  sourceName: string | null;
  targetPath: string;
  targetName: string | null;
  evidenceAnchor: GitSourceAnchor;
  targetAnchor: GitSourceAnchor | null;
}

export interface GitDiffHunk {
  baseAnchor: GitSourceAnchor | null;
  headAnchor: GitSourceAnchor | null;
  baseChangedRanges: GitChangedRange[];
  headChangedRanges: GitChangedRange[];
  baseSymbols: GitSymbolEvidence[];
  headSymbols: GitSymbolEvidence[];
  patchExcerpt: string;
}

export interface GitChangedRange {
  startLine: number;
  endLine: number;
}

export interface GitSymbolEvidence {
  kind: string;
  name: string;
  anchor: GitSourceAnchor;
}

export interface GitFileChange {
  kind: GitChangeKind;
  basePath: string | null;
  headPath: string | null;
  classification: GitEvidenceClassification;
  baseBlobId: string | null;
  headBlobId: string | null;
  baseSizeBytes: number | null;
  headSizeBytes: number | null;
  hunks: GitDiffHunk[];
  baseRelationships: GitRelationshipEvidence[];
  headRelationships: GitRelationshipEvidence[];
  skippedReason: string | null;
}

export interface GitComparisonResult {
  status: GitComparisonStatus;
  comparisonId: string | null;
  base: ResolvedGitRef | null;
  head: ResolvedGitRef | null;
  refProblem: GitRefProblem | null;
  changes: GitFileChange[];
  relationshipImpact: GitRelationshipImpact;
  omittedChangeCount: number;
  omittedReason: string | null;
  message: string | null;
}

export interface SelectedGitChangeIdentity {
  basePath: string | null;
  headPath: string | null;
}

export interface PrGuideComparisonContext {
  baseRef: string;
  headRef: string;
  comparisonId: string;
  base: ResolvedGitRef;
  head: ResolvedGitRef;
  selectedChanges: GitFileChange[];
}

export type PrGuideLifecycle = 'draft' | 'accepted' | 'rejected';
export type PrGuideClaimCategory = 'codeBehavior' | 'repositoryContext' | 'inference';
export type PrGuideClaimAspect = 'codeChange' | 'tests' | 'dependency' | 'compatibility' | 'risk';
export type GitEvidenceSide = 'base' | 'head';

export interface PrGuideCitation {
  changeIndex: number;
  hunkIndex: number;
  side: GitEvidenceSide;
  anchor: GitSourceAnchor;
  quote: string;
}

export interface PrGuideClaim {
  claimId: string;
  category: PrGuideClaimCategory;
  aspect: PrGuideClaimAspect;
  prose: string;
  inferenceBasis: string | null;
  citations: PrGuideCitation[];
}

export interface PrGuideSection {
  sectionId: string;
  title: string;
  claims: PrGuideClaim[];
}

export interface PrGuideRevision {
  schemaVersion: number;
  revisionId: string;
  basedOnRevisionId: string | null;
  revisionNumber: number;
  projectId: string;
  comparison: PrGuideComparisonContext;
  lifecycle: PrGuideLifecycle;
  decisionNote: string | null;
  createdAtUnixMs: number;
  title: string;
  sections: PrGuideSection[];
}

export interface PrGuidePreview {
  revision: PrGuideRevision;
  modelId: string;
  modelVersion: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface SnapshotStart {
  snapshotId: string;
  parentSnapshotId: string | null;
  limits: CaptureLimits;
}

export interface CapturedFile {
  path: string;
  kind: 'file' | 'directory' | 'symlink' | 'other' | 'unknown';
  sizeBytes: number;
  sha256: string;
  blobPath: string;
  contentKind: 'text' | 'binary' | 'invalidEncoding' | 'unknown';
  secretLike: boolean;
}

export interface SkippedPath {
  path: string;
  kind: 'file' | 'directory' | 'symlink' | 'other' | 'unknown';
  sizeBytes: number | null;
  reason: string;
  details: string;
}

export interface InventoryDirectory {
  path: string;
  kind: 'directory';
  sizeBytes: number | null;
}

export interface SnapshotManifest {
  schemaVersion: number;
  summary: SnapshotSummary;
  repositoryRoot: string;
  selectedPath: string;
  captureLimits: CaptureLimits;
  directories: InventoryDirectory[];
  files: CapturedFile[];
  skipped: SkippedPath[];
}

export interface SnapshotSource {
  snapshotId: string;
  path: string;
  sha256: string;
  sizeBytes: number;
  contents: string;
}

export type SnapshotIndexStatus = 'complete' | 'partial' | 'cancelled';
export type IndexFileStatus =
  | 'indexed' | 'partialParse' | 'parseError' | 'unsupportedLanguage' | 'binary'
  | 'invalidEncoding' | 'sourceUnavailable' | 'fileTooLarge' | 'indexLimitExceeded'
  | 'analysisLimitExceeded' | 'parserUnavailable' | 'cancelled';
export type IndexJobStatus = 'running' | 'complete' | 'partial' | 'cancelled' | 'failed';
export type SymbolKind =
  | 'function' | 'method' | 'class' | 'struct' | 'enum' | 'trait' | 'interface'
  | 'typeAlias' | 'module' | 'constant' | 'variable' | 'macro';
export type RelationshipKind = 'defines' | 'exports' | 'imports' | 'calls' | 'testReference';

export interface ParserVersion {
  engine: string;
  engineVersion: string;
  engineLanguageAbi: number;
  grammar: string;
  grammarVersion: string;
  grammarAbi: number;
}

export interface SourceAnchor {
  path: string;
  sourceSha256: string;
  startByte: number;
  endByte: number;
  startLine: number;
  endLine: number;
  startColumn: number;
  endColumn: number;
}

export interface IndexedFile {
  path: string;
  fileId: string;
  language: string | null;
  status: IndexFileStatus;
  parser: ParserVersion | null;
  sourceSha256: string;
  sourceSizeBytes: number;
  parseErrorCount: number;
  symbolCount: number;
  relationshipCount: number;
  message: string | null;
}

export interface IndexedSymbol {
  symbolId: string;
  path: string;
  language: string;
  kind: SymbolKind;
  name: string;
  qualifiedName: string;
  declarationIdentity: string;
  signatureSha256: string;
  isExported: boolean;
  isTest: boolean;
  anchor: SourceAnchor;
}

export interface IndexedRelationship {
  relationshipId: string;
  kind: RelationshipKind;
  sourceId: string;
  targetId: string | null;
  targetPath: string | null;
  targetLabel: string | null;
  confidence: 'high' | 'unresolved';
  unresolvedReason: string | null;
  evidence: SourceAnchor;
}

export interface SnapshotIndex {
  schemaVersion: number;
  indexerVersion: string;
  projectId: string;
  snapshotId: string;
  status: SnapshotIndexStatus;
  parsers: ParserVersion[];
  files: IndexedFile[];
  symbols: IndexedSymbol[];
  relationships: IndexedRelationship[];
}

export type ChapterRevisionLifecycle = 'draft' | 'accepted' | 'rejected';
export type ChapterValidationStatus = 'unvalidated' | 'valid' | 'invalid';
export type ClaimCategory =
  | { kind: 'codeBehavior' }
  | { kind: 'repositoryContext' }
  | { kind: 'inference'; visibleLabel: string; basis: string };
export type ChapterTargetScope =
  | { kind: 'overview' }
  | { kind: 'file'; path: string }
  | { kind: 'group'; paths: string[] };
export type SectionDependency =
  | { kind: 'sourceFile'; path: string; sourceSha256: string }
  | { kind: 'symbol'; path: string; sourceSha256: string; symbolId: string }
  | { kind: 'relationship'; path: string; sourceSha256: string; relationshipId: string };
export type HotspotDestination =
  | { kind: 'section'; sectionId: string }
  | { kind: 'codeLens'; anchorId: string }
  | { kind: 'atlasFile'; path: string }
  | { kind: 'atlasSymbol'; path: string; symbolId: string }
  | { kind: 'relationship'; relationshipId: string };

/** Byte, line, and byte-column endpoints are inclusive and pinned to one snapshot. */
export interface ChapterSourceAnchor {
  anchorId: string;
  path: string;
  symbolId: string | null;
  snapshotId: string;
  sourceSha256: string;
  startByte: number;
  endByteInclusive: number;
  startLine: number;
  endLine: number;
  startColumn: number;
  endColumn: number;
  excerpt: string;
}

export interface ChapterHotspot {
  hotspotId: string;
  label: string;
  destination: HotspotDestination;
}

export type VisualLayoutDirection = 'auto' | 'leftToRight' | 'topToBottom';

export interface VisualLayoutHint {
  direction: VisualLayoutDirection;
}

export interface LegacyVisualSpec {
  schemaVersion: 1;
  kind: 'dataFlow' | 'sequence' | 'hierarchy';
  title: string;
  nodes: Array<{
    nodeId: string;
    label: string;
    detail: string | null;
    hotspotId: string | null;
  }>;
  edges: Array<{
    edgeId: string;
    fromNodeId: string;
    toNodeId: string;
    label: string;
  }>;
}

export interface LinkedVisualNode {
  nodeId: string;
  label: string;
  detail: string | null;
  hotspotId: string | null;
  evidenceAnchorIds: string[];
}

export interface LinkedVisualEdge {
  edgeId: string;
  fromNodeId: string;
  toNodeId: string;
  label: string;
  evidenceAnchorIds: string[];
}

export interface EvidenceLinkedVisualSpec {
  schemaVersion: 2;
  kind: 'dataFlow' | 'sequence' | 'hierarchy';
  title: string;
  layoutHint: VisualLayoutHint;
  nodes: LinkedVisualNode[];
  edges: LinkedVisualEdge[];
}

/** Accepted v1 diagrams remain decodable and immutable; only v2 is interactive. */
export type VisualSpec = LegacyVisualSpec | EvidenceLinkedVisualSpec;

export interface ChapterClaim {
  claimId: string;
  category: ClaimCategory;
  prose: string;
  anchors: ChapterSourceAnchor[];
  dependencies: SectionDependency[];
}

export interface ChapterSection {
  sectionId: string;
  sectionRevisionId: string;
  order: number;
  title: string;
  prose: string;
  spokenText: string;
  claims: ChapterClaim[];
  dependencies: SectionDependency[];
  visual: VisualSpec | null;
  hotspots: ChapterHotspot[];
}

export interface ChapterRevision {
  schemaVersion: number;
  chapterId: string;
  revisionId: string;
  revisionNumber: number;
  projectId: string;
  snapshotId: string;
  target: { targetId: string; scope: ChapterTargetScope };
  title: string;
  lifecycle: ChapterRevisionLifecycle;
  validationStatus: ChapterValidationStatus;
  createdAtUnixMs: number;
  basedOnRevisionId?: string | null;
  decisionNote?: string | null;
  sections: ChapterSection[];
}

export interface SavedChapterReview {
  revision: ChapterRevision;
  validation: ChapterValidationReport;
}

export type ChapterIssueCode =
  | 'unsupportedSchemaVersion' | 'invalidId' | 'invalidRevision' | 'invalidTarget'
  | 'invalidSectionOrder' | 'duplicateId' | 'invalidClaim' | 'wrongSnapshot'
  | 'missingPath' | 'rangeMismatch' | 'hashMismatch' | 'excerptMismatch'
  | 'ambiguousRemap' | 'missingSymbol' | 'invalidDependency' | 'invalidDestination' | 'invalidVisual'
  | 'indexUnavailable' | 'sourceUnavailable';

export interface ChapterValidationIssue {
  code: ChapterIssueCode;
  message: string;
  path: string | null;
  sectionId: string | null;
  claimId: string | null;
  anchorId: string | null;
  hotspotId: string | null;
  visualItemId: string | null;
}

export interface ChapterValidationReport {
  schemaVersion: number;
  chapterId: string;
  revisionId: string;
  snapshotId: string;
  status: 'valid' | 'invalid';
  issues: ChapterValidationIssue[];
}

export interface AnchorRemapResult {
  status: 'remapped' | 'unavailable';
  anchor: ChapterSourceAnchor | null;
  issues: ChapterValidationIssue[];
}

export interface ChapterFreshnessLineRange {
  /** Source path for this side of the change; it can differ from the change's summary path. */
  path: string;
  /** Immutable snapshot containing the captured bytes for this range. */
  snapshotId: string;
  /** Captured file hash required to verify this range in Code Lens. */
  sourceSha256: string;
  startLine: number;
  endLine: number;
}

export interface ChapterFreshnessReason {
  code: string;
  message: string;
  path: string | null;
}

export interface ChapterFreshnessSourceChange {
  path: string;
  kind: string;
  oldRange: ChapterFreshnessLineRange | null;
  newRange: ChapterFreshnessLineRange | null;
}

export type ChapterFreshnessStatus = 'current' | 'safeToCarry' | 'stale' | 'needsReview';
export type ChapterFreshnessEvidenceState = 'complete' | 'partial' | 'unavailable';
export type ChapterFreshnessIndexStatus = 'complete' | 'partial' | 'cancelled' | 'unavailable';

export interface ChapterFreshnessIndexSummary {
  status: ChapterFreshnessIndexStatus;
  indexedFiles: number | null;
  totalFiles: number | null;
}

export interface ChapterSectionFreshness {
  sectionId: string;
  sectionRevisionId: string;
  status: ChapterFreshnessStatus;
  evidenceState: ChapterFreshnessEvidenceState;
  reasons: ChapterFreshnessReason[];
  sourceChanges: ChapterFreshnessSourceChange[];
  narrationScriptSha256: string | null;
}

export interface ChapterFreshnessReverseDependency {
  evidenceKey: string;
  path: string;
  dependentSectionIds: string[];
  assetIds: string[];
}

/** Read-only comparison of one immutable chapter revision with another captured snapshot. */
export interface ChapterFreshnessReport {
  chapterRevisionId: string;
  fromSnapshotId: string;
  toSnapshotId: string;
  indexes: {
    from: ChapterFreshnessIndexSummary;
    to: ChapterFreshnessIndexSummary;
  };
  sections: ChapterSectionFreshness[];
  reverseDependencies: ChapterFreshnessReverseDependency[];
}

export interface IndexProgress {
  jobId: string;
  projectId: string;
  snapshotId: string;
  status: IndexJobStatus;
  filesTotal: number;
  filesProcessed: number;
  filesIndexed: number;
  symbolsFound: number;
  relationshipsFound: number;
  currentPath: string | null;
  message: string | null;
}

export interface IndexStart {
  jobId: string;
  projectId: string;
  snapshotId: string;
  status: IndexJobStatus;
}

export interface ProviderModel {
  modelId: string;
  displayName: string;
  intendedUse: string;
  freeTierStatus: string;
  paidInputUsdPerMillionTokens: string;
  paidOutputUsdPerMillionTokens: string;
  paidPriceChangeDate: string | null;
  supportsStructuredOutput: boolean;
  supportsStreaming: boolean;
}

export interface ProviderDisclosure {
  providerId: string;
  displayName: string;
  version: string;
  checkedDate: string;
  endpointUrl: string;
  requestPathTemplate: string;
  pricingUrl: string;
  termsUrl: string;
  regionsUrl: string;
  apiKeyUrl: string;
  apiKeySecurityNotice: string;
  models: ProviderModel[];
  dataUseTerms: string;
  regionRequirements: string;
  eligibilityRequirements: string;
  billingUncertainty: string;
}

export interface ProviderCredentialStatus {
  keychainAvailable: boolean;
  apiKeyConfigured: boolean;
  explanation: string;
}

export type ProviderFailureKind =
  | 'keychainUnavailable' | 'keychainOperationFailed' | 'missingApiKey' | 'invalidApiKey'
  | 'consentRequired' | 'reviewExpired' | 'reviewChanged' | 'reviewUnavailable' | 'localStorageUnavailable' | 'invalidReview'
  | 'contextTooLarge' | 'secretLikeContext' | 'unavailableModel' | 'billingRequired' | 'quotaExceeded'
  | 'regionUnavailable' | 'networkUnavailable' | 'requestTimedOut' | 'requestCancelled' | 'providerRejectedRequest' | 'providerResponseTooLarge'
  | 'malformedProviderResponse';

export interface ProviderFailure {
  kind: ProviderFailureKind;
  message: string;
}

export type ProviderCommandResult<T> =
  | { status: 'ok'; data: T }
  | { status: 'error'; error: ProviderFailure };

export interface ProviderConsent {
  privacyAndDataUseAcknowledged: boolean;
  billingUncertaintyAcknowledged: boolean;
  adultSupportedRegionAcknowledged: boolean;
}

export interface NarrationDisclosure {
  version: string;
  checkedDate: string;
  provider: string;
  modelId: string;
  modelName: string;
  freeTierTerms: string;
  paidTierTerms: string;
  dataUseTerms: string;
  paidDataUseTerms: string;
  regionalDataUseTerms: string;
  eligibilityTerms: string;
  billingUncertainty: string;
  termsUrl: string;
  pricingUrl: string;
  regionsUrl: string;
  apiKeyUrl: string;
  requestScope: string;
  voices: string[];
}

export interface NarrationPronunciationEntry {
  name: string;
  sayAs: string;
  include: boolean;
}

export interface NarrationRequestReview {
  reviewId: string;
  exactText: string;
  requestSegments: string[];
  spokenTextSha256: string;
  disclosureVersion: string;
  modelId: string;
  voice: string;
  style: string;
  format: string;
  fingerprint: string;
  cacheKey: string;
  cachedAudioAvailable: boolean;
  expiresAtUnixMs: number;
}

export interface NarrationAudio {
  fingerprint: string;
  mimeType: string;
  audioBase64: string;
  cacheHit: boolean;
}

export interface NarrationConsent {
  termsAndRequestAcknowledged: boolean;
}

export interface ProviderRequestFile {
  path: string;
  sha256: string;
  sizeBytes: number;
  contents: string;
}

export interface ProviderSymbolEvidence {
  symbolId: string;
  path: string;
  kind: string;
  name: string;
  qualifiedName: string;
  isExported: boolean;
  isTest: boolean;
  startLine: number;
  endLine: number;
}

export interface ProviderRelationshipEvidence {
  relationshipId: string;
  kind: string;
  sourcePath: string;
  sourceName: string;
  targetPath: string | null;
  targetName: string | null;
  confidence: 'high' | 'unresolved';
  unresolvedReason: string | null;
  evidenceStartLine: number;
  evidenceEndLine: number;
  evidenceExcerpt: string;
}

export interface ProviderRequestReview {
  reviewId: string;
  projectId: string;
  snapshotId: string;
  purpose: string;
  targetPath: string | null;
  providerId: string;
  modelId: string;
  endpointUrl: string;
  disclosureVersion: string;
  selectedFiles: ProviderRequestFile[];
  selectedSymbols: ProviderSymbolEvidence[];
  selectedRelationships: ProviderRelationshipEvidence[];
  indexEvidenceTruncated: boolean;
  prGuideContext: PrGuideComparisonContext | null;
  sectionUpdateContext: SectionUpdateReviewTarget | null;
  codeLensContext: CodeLensExplanationTarget | null;
  prChangeContext: PrChangeExplanationTarget | null;
  systemInstruction: string;
  userPrompt: string;
  generationConfig: Record<string, unknown>;
  sourceBytes: number;
  estimatedInputTokens: number;
  requestBodyBytes: number;
  requestBodySha256: string;
  requestBodyPreview: Record<string, unknown>;
  fingerprint: string;
  expiresAtUnixMs: number;
}

export interface SectionUpdateReviewTarget {
  chapterRevisionId: string;
  sectionId: string;
  sectionRevisionId: string;
}

export interface CodeLensExplanationTarget {
  path: string;
  sourceSha256: string;
  startLine: number;
  endLine: number;
  startByte: number | null;
  endByteExclusive: number | null;
}

export interface PrChangeExplanationTarget {
  baseRef: string;
  headRef: string;
  comparisonId: string;
  selectedChange: SelectedGitChangeIdentity;
  hunkIndex: number;
}

export interface PrChangeExplanationCitation {
  side: GitEvidenceSide;
  anchor: GitSourceAnchor;
  hunkIndex: number;
  quote: string;
}

export interface PrChangeExplanationClaim {
  claimId: string;
  category: PrGuideClaimCategory;
  prose: string;
  inferenceBasis: string | null;
  citation: PrChangeExplanationCitation;
}

export interface PrChangeExplanationPreview {
  providerId: string;
  modelId: string;
  modelVersion: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  title: string;
  claims: PrChangeExplanationClaim[];
}

export interface CodeLensExplanationCitation {
  path: string;
  snapshotId: string;
  sourceSha256: string;
  startLine: number;
  endLine: number;
  startByte: number;
  endByteExclusive: number;
  quote: string;
}

export interface CodeLensExplanationClaim {
  claimId: string;
  prose: string;
  citation: CodeLensExplanationCitation;
}

export interface CodeLensExplanationPreview {
  providerId: string;
  modelId: string;
  modelVersion: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  claims: CodeLensExplanationClaim[];
}

export interface ChapterDraftPreview {
  generationId: string;
  revision: ChapterRevision;
  validation: ChapterValidationReport;
  modelId: string;
  modelVersion: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  updateNotes: string[];
}

export interface PendingChapterPreviewSummary {
  generationId: string;
  projectId: string | null;
  snapshotId: string | null;
  title: string;
  modelId: string | null;
  targetPath: string | null;
  unavailableReason: string | null;
}

export interface ProjectSnapshots {
  projectId: string;
  snapshots: SnapshotSummary[];
}

export function isNativeRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function getApplicationStatus(): Promise<ApplicationStatus> {
  if (!isNativeRuntime()) {
    return { productName: 'Code Atlas', version: '0.1.0', native: false };
  }
  return invoke<ApplicationStatus>('get_application_status');
}

export async function getCaptureLimits(): Promise<CaptureLimits> {
  if (!isNativeRuntime()) {
    return {
      policyVersion: 'capture-policy-v1', maxFileBytes: 2 * 1024 * 1024,
      maxTotalBytes: 100 * 1024 * 1024, maxPaths: 25_000, maxPathBytes: 1_024,
      maxEntriesPerDirectory: 1_000,
      excludedDirectories: ['node_modules', 'vendor', 'target', 'dist', 'build', 'coverage', 'out', 'generated', '.next', '.nuxt', '.turbo', '.cache', '.venv', 'venv'],
      otherExclusions: ['Git-ignored paths; tracked files remain eligible', 'Symlinks and their targets', 'The repository .git marker and its contents', 'Nested Git repositories', 'Non-regular filesystem objects'],
    };
  }
  return invoke<CaptureLimits>('get_capture_limits');
}

export async function listRecentProjects(): Promise<RecentProject[]> {
  if (!isNativeRuntime()) return [];
  return invoke<RecentProject[]>('list_recent_projects');
}

export async function listInterruptedOperations(): Promise<InterruptedOperation[]> {
  if (!isNativeRuntime()) return [];
  return invoke<InterruptedOperation[]>('list_interrupted_operations');
}

export async function discardInterruptedOperation(operationId: string): Promise<void> {
  if (!isNativeRuntime()) return;
  await invoke<void>('discard_interrupted_operation', { operationId });
}

export async function pickProjectFolder(): Promise<ProjectOpenResult> {
  return invoke<ProjectOpenResult>('pick_project_folder');
}

export async function openRecentProject(projectId: string): Promise<ProjectOpenResult> {
  return invoke<ProjectOpenResult>('open_recent_project', { projectId });
}

export async function compareLocalGitRevisions(
  projectId: string,
  baseRef: string,
  headRef: string,
): Promise<GitComparisonResult> {
  return invoke<GitComparisonResult>('compare_local_git_revisions', { projectId, baseRef, headRef });
}

export async function buildProviderPrGuideRequestReview(providerId: string, input: {
  projectId: string;
  baseRef: string;
  headRef: string;
  comparisonId: string;
  selectedChanges: SelectedGitChangeIdentity[];
  modelId: string;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke<ProviderCommandResult<ProviderRequestReview>>('build_provider_pr_guide_request_review', { providerId, input });
}

export async function buildProviderPrChangeExplanationRequestReview(providerId: string, input: {
  projectId: string;
  target: PrChangeExplanationTarget;
  modelId: string;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke<ProviderCommandResult<ProviderRequestReview>>('build_provider_pr_change_explanation_request_review', { providerId, input });
}

export async function sendProviderPrGuideRequest(input: {
  reviewId: string;
  fingerprint: string;
  consent: ProviderConsent;
}): Promise<ProviderCommandResult<PrGuidePreview>> {
  return invoke<ProviderCommandResult<PrGuidePreview>>('send_provider_pr_guide_request', input);
}

export async function sendProviderPrChangeExplanationRequest(input: {
  reviewId: string;
  fingerprint: string;
  consent: ProviderConsent;
}): Promise<ProviderCommandResult<PrChangeExplanationPreview>> {
  return invoke<ProviderCommandResult<PrChangeExplanationPreview>>('send_provider_pr_change_explanation_request', input);
}

export async function listPrGuideRevisions(projectId: string, comparisonId: string): Promise<PrGuideRevision[]> {
  return invoke<PrGuideRevision[]>('list_pr_guide_revisions', { projectId, comparisonId });
}

export async function acceptPrGuideDraft(input: {
  projectId: string;
  comparisonId: string;
  revisionId: string;
  includedClaimIds: string[];
}): Promise<PrGuideRevision> {
  return invoke<PrGuideRevision>('accept_pr_guide_draft', input);
}

export async function rejectPrGuideDraft(input: {
  projectId: string;
  comparisonId: string;
  revisionId: string;
  reason: string;
}): Promise<PrGuideRevision> {
  return invoke<PrGuideRevision>('reject_pr_guide_draft', input);
}

export async function removeRecentProject(projectId: string): Promise<RecentProject[]> {
  return invoke<RecentProject[]>('remove_recent_project', { projectId });
}

export async function listProjectSnapshots(projectId: string): Promise<ProjectSnapshots> {
  return invoke<ProjectSnapshots>('list_project_snapshots', { projectId });
}

export async function loadSnapshotManifest(projectId: string, snapshotId: string): Promise<SnapshotManifest> {
  return invoke<SnapshotManifest>('load_snapshot_manifest', { projectId, snapshotId });
}

export async function readSnapshotSource(projectId: string, snapshotId: string, relativePath: string): Promise<SnapshotSource> {
  return invoke<SnapshotSource>('read_snapshot_source', { projectId, snapshotId, relativePath });
}

export async function startSnapshot(projectId: string): Promise<SnapshotStart> {
  return invoke<SnapshotStart>('start_snapshot', { projectId });
}

export async function cancelSnapshot(snapshotId: string): Promise<void> {
  return invoke<void>('cancel_snapshot', { snapshotId });
}

export async function getSnapshotProgress(snapshotId: string): Promise<SnapshotProgress | null> {
  return invoke<SnapshotProgress | null>('get_snapshot_progress', { snapshotId });
}

export async function startSnapshotIndexing(projectId: string, snapshotId: string): Promise<IndexStart> {
  return invoke<IndexStart>('start_snapshot_indexing', { projectId, snapshotId });
}

export async function cancelSnapshotIndexing(jobId: string): Promise<void> {
  return invoke<void>('cancel_snapshot_indexing', { jobId });
}

export async function getSnapshotIndexProgress(jobId: string): Promise<IndexProgress | null> {
  return invoke<IndexProgress | null>('get_snapshot_index_progress', { jobId });
}

export async function loadSnapshotIndex(projectId: string, snapshotId: string): Promise<SnapshotIndex | null> {
  return invoke<SnapshotIndex | null>('load_snapshot_index', { projectId, snapshotId });
}

export async function validateChapterRevision(
  projectId: string,
  revision: ChapterRevision,
): Promise<ChapterValidationReport> {
  return invoke<ChapterValidationReport>('validate_chapter_revision', {
    projectId,
    revisionJson: JSON.stringify(revision),
  });
}

export async function listChapterRevisions(projectId: string, snapshotId: string): Promise<ChapterRevision[]> {
  return invoke<ChapterRevision[]>('list_chapter_revisions', { projectId, snapshotId });
}

export async function listProjectChapterRevisions(projectId: string): Promise<ChapterRevision[]> {
  return invoke<ChapterRevision[]>('list_project_chapter_revisions', { projectId });
}

export async function saveChapterEdits(
  projectId: string,
  baseRevisionId: string,
  revision: ChapterRevision,
): Promise<SavedChapterReview> {
  return invoke<SavedChapterReview>('save_chapter_edits', {
    projectId,
    baseRevisionId,
    revisionJson: JSON.stringify(revision),
  });
}

export async function restoreChapterRevision(
  projectId: string,
  baseRevisionId: string,
  sourceRevisionId: string,
): Promise<SavedChapterReview> {
  return invoke<SavedChapterReview>('restore_chapter_revision', {
    projectId,
    baseRevisionId,
    sourceRevisionId,
  });
}

export async function acceptChapterDraft(projectId: string, revisionId: string): Promise<SavedChapterReview> {
  return invoke<SavedChapterReview>('accept_chapter_draft', { projectId, revisionId });
}

export async function rejectChapterDraft(projectId: string, revisionId: string, reason: string): Promise<ChapterRevision> {
  return invoke<ChapterRevision>('reject_chapter_draft', { projectId, revisionId, reason });
}

export async function saveGeneratedChapterDraft(generationId: string): Promise<ChapterRevision> {
  return invoke<ChapterRevision>('save_generated_chapter_draft', { generationId });
}

export async function listPendingGeneratedChapterPreviews(): Promise<PendingChapterPreviewSummary[]> {
  if (!isNativeRuntime()) return [];
  return invoke<PendingChapterPreviewSummary[]>('list_pending_generated_chapter_previews');
}

export async function loadPendingGeneratedChapterPreview(generationId: string): Promise<ChapterDraftPreview> {
  return invoke<ChapterDraftPreview>('load_pending_generated_chapter_preview', { generationId });
}

export async function discardGeneratedChapterDraft(generationId: string): Promise<void> {
  await invoke<void>('discard_generated_chapter_draft', { generationId });
}

export async function remapChapterAnchor(
  projectId: string,
  targetSnapshotId: string,
  anchor: ChapterSourceAnchor,
): Promise<AnchorRemapResult> {
  return invoke<AnchorRemapResult>('remap_chapter_anchor', {
    projectId,
    targetSnapshotId,
    anchorJson: JSON.stringify(anchor),
  });
}

export async function assessChapterFreshness(
  projectId: string,
  revisionId: string,
  targetSnapshotId: string,
): Promise<ChapterFreshnessReport> {
  return invoke<ChapterFreshnessReport>('assess_chapter_freshness', {
    projectId,
    revisionId,
    targetSnapshotId,
  });
}

export async function getProviderCatalog(): Promise<ProviderDisclosure[]> {
  return invoke<ProviderDisclosure[]>('get_provider_catalog');
}

export async function getProviderCredentialStatus(providerId: string): Promise<ProviderCommandResult<ProviderCredentialStatus>> {
  return invoke<ProviderCommandResult<ProviderCredentialStatus>>('get_provider_credential_status', { providerId });
}

export async function storeProviderApiKey(providerId: string, apiKey: string): Promise<ProviderCommandResult<ProviderCredentialStatus>> {
  return invoke<ProviderCommandResult<ProviderCredentialStatus>>('store_provider_api_key', { providerId, apiKey });
}

export async function removeProviderApiKey(providerId: string): Promise<ProviderCommandResult<ProviderCredentialStatus>> {
  return invoke<ProviderCommandResult<ProviderCredentialStatus>>('remove_provider_api_key', { providerId });
}

export async function buildProviderRequestReview(providerId: string, input: {
  projectId: string;
  snapshotId: string;
  targetPath: string | null;
  selectedPaths: string[];
  modelId: string;
  sectionUpdate?: SectionUpdateReviewTarget | null;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke<ProviderCommandResult<ProviderRequestReview>>('build_provider_request_review', { providerId, input });
}

export async function buildCodeLensProviderRequestReview(providerId: string, input: {
  projectId: string;
  snapshotId: string;
  target: CodeLensExplanationTarget;
  modelId: string;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke<ProviderCommandResult<ProviderRequestReview>>('build_code_lens_provider_request_review', { providerId, input });
}

export async function discardProviderRequestReview(reviewId: string): Promise<void> {
  await invoke<void>('discard_provider_request_review', { reviewId });
}

export async function cancelProviderRequest(reviewId: string): Promise<boolean> {
  return invoke<boolean>('cancel_provider_request', { reviewId });
}

export async function sendProviderRequest(input: {
  reviewId: string;
  fingerprint: string;
  consent: ProviderConsent;
}): Promise<ProviderCommandResult<ChapterDraftPreview>> {
  return invoke<ProviderCommandResult<ChapterDraftPreview>>('send_provider_request', input);
}

export async function sendCodeLensProviderRequest(input: {
  reviewId: string;
  fingerprint: string;
  consent: ProviderConsent;
}): Promise<ProviderCommandResult<CodeLensExplanationPreview>> {
  return invoke<ProviderCommandResult<CodeLensExplanationPreview>>('send_code_lens_provider_request', input);
}

export type NarrationBackend = 'pocket-tts' | 'gemini';

export interface PocketTtsStatus {
  phase: string;
  ready: boolean;
  busy: boolean;
  setupAvailable: boolean;
  supportedPlatform: boolean;
  packageVersion: string;
  pythonVersion: string;
  modelId: string;
  voice: string;
  message: string;
}

export async function getPocketTtsStatus(): Promise<PocketTtsStatus> {
  return invoke<PocketTtsStatus>('get_pocket_tts_status');
}

export async function setupPocketTts(requestId: string): Promise<ProviderCommandResult<PocketTtsStatus>> {
  return invoke<ProviderCommandResult<PocketTtsStatus>>('setup_pocket_tts', { requestId });
}

export async function getNarrationDisclosure(backend: NarrationBackend = 'pocket-tts'): Promise<NarrationDisclosure> {
  return invoke<NarrationDisclosure>(backend === 'pocket-tts' ? 'get_local_narration_disclosure' : 'get_narration_disclosure');
}

export async function buildNarrationRequestReview(input: {
  backend?: NarrationBackend;
  spokenText: string;
  voice: string;
  style: string;
  lexicon: NarrationPronunciationEntry[];
}): Promise<ProviderCommandResult<NarrationRequestReview>> {
  const { backend = 'pocket-tts', ...request } = input;
  return invoke<ProviderCommandResult<NarrationRequestReview>>(
    backend === 'pocket-tts' ? 'build_local_narration_request_review' : 'build_narration_request_review', { input: request });
}

export async function discardNarrationRequestReview(reviewId: string): Promise<void> {
  await invoke<void>('discard_narration_request_review', { reviewId });
}

export async function loadReviewedNarrationAudio(input: {
  reviewId: string;
  fingerprint: string;
}): Promise<ProviderCommandResult<NarrationAudio>> {
  return invoke<ProviderCommandResult<NarrationAudio>>('load_reviewed_narration_audio', input);
}

export async function synthesizeReviewedNarrationAudio(input: {
  reviewId: string;
  fingerprint: string;
  consent: NarrationConsent;
}): Promise<ProviderCommandResult<NarrationAudio>> {
  return invoke<ProviderCommandResult<NarrationAudio>>('synthesize_reviewed_narration_audio', input);
}

export interface CodexAgentStatus {
  executablePath: string | null;
  runtimeRunning: boolean;
  authenticated: boolean;
  accountEmail: string | null;
  planType: string | null;
  pendingLoginId: string | null;
  activeRunId: string | null;
  capabilityChecked: boolean;
  message: string;
}

export interface CodexLoginStart { loginId: string; authUrl: string }
export interface CodexModel {
  modelId: string;
  displayName: string;
  defaultReasoningEffort: string | null;
  supportedReasoningEfforts: string[];
  isDefault: boolean;
}
export interface CodexProgress {
  runId: string;
  phase: string;
  threadId: string | null;
  turnId: string | null;
  message: string;
}
export async function getCodexAgentStatus(): Promise<CodexAgentStatus> {
  return invoke<CodexAgentStatus>('get_codex_agent_status');
}
export async function configureCodexAgent(executablePath: string): Promise<CodexAgentStatus> {
  return invoke<CodexAgentStatus>('configure_codex_agent', { executablePath });
}
export async function startCodexLogin(): Promise<CodexLoginStart> {
  return invoke<CodexLoginStart>('start_codex_login');
}
export async function cancelCodexLogin(loginId: string): Promise<void> {
  return invoke('cancel_codex_login', { loginId });
}
export async function logoutCodexAgent(): Promise<void> {
  return invoke('logout_codex_agent');
}
export async function listCodexModels(): Promise<CodexModel[]> {
  return invoke<CodexModel[]>('list_codex_models');
}
export async function buildCodexChapterReview(input: {
  projectId: string; snapshotId: string; targetPath: string | null; selectedPaths: string[];
  modelId: string; effort: string; approvalPolicy: 'never';
  sectionUpdate?: SectionUpdateReviewTarget | null;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_codex_chapter_review', { input });
}
export async function buildCodexCodeLensReview(input: {
  projectId: string; snapshotId: string; target: CodeLensExplanationTarget;
  modelId: string; effort: string; approvalPolicy: 'never';
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_codex_code_lens_review', { input });
}
export async function sendCodexCodeLensRequest(reviewId: string, fingerprint: string, consent: boolean): Promise<ProviderCommandResult<CodeLensExplanationPreview>> {
  return invoke('send_codex_code_lens_request', { reviewId, fingerprint, consent });
}
export async function sendCodexChapterRequest(reviewId: string, fingerprint: string, consent: boolean): Promise<ProviderCommandResult<ChapterDraftPreview>> {
  return invoke('send_codex_chapter_request', { reviewId, fingerprint, consent });
}
export async function cancelCodexChapterRequest(runId: string): Promise<boolean> {
  return invoke('cancel_codex_chapter_request', { runId });
}

export type ClaudeInspectionFailureKind = 'busy' | 'cancelled' | 'timedOut' | 'runtimeChanged' | 'reviewChanged'
  | 'preparationFailed' | 'processFailed' | 'protocolRejected' | 'outputLimit' | 'cleanupFailed' | 'unsupportedPlatform';
export interface ClaudeConnectionObservation {
  executablePath: string;
  executableSha256: string;
  runtimeVersion: string;
  candidateVersionMatches: boolean;
  automaticGenerationAvailable: false;
  account: {
    loggedIn: boolean;
    authMethod: 'none' | 'claudeAccount' | 'oauthToken' | 'apiKey' | 'apiKeyHelper' | 'thirdParty' | 'unrecognized';
    firstPartyProvider: boolean;
    accountEmail: string | null;
    subscriptionType: string | null;
    configDirectory: string | null;
    accountIdentitySha256: string | null;
  } | null;
  message: string;
}
export async function inspectClaudeAgentConnection(executablePath: string, runId: string): Promise<ClaudeConnectionObservation> {
  return invoke('inspect_claude_agent_connection', { executablePath, runId });
}
export interface ClaudeModelDiscoveryObservation {
  reviewConnection: { connectionId: string; expiresAtUnixMs: number } | null;
  connection: ClaudeConnectionObservation;
  catalog: {
    sdkVersion: string;
    runtimeVersion: string;
    models: {
      modelId: string;
      displayName: string;
      aliases: string[];
      effortSupportAdvertised: boolean;
      supportedEfforts: string[];
    }[];
    omittedRows: number;
  };
  nodeExecutablePath: string;
  nodeExecutableSha256: string;
  nodeVersion: string;
  sdkModulePath: string;
  sdkModuleSha256: string;
}
export async function inspectClaudeAgentModelCatalog(
  executablePath: string, nodeExecutablePath: string, sdkModulePath: string, runId: string,
): Promise<ClaudeModelDiscoveryObservation> {
  return invoke('inspect_claude_agent_model_catalog', { executablePath, nodeExecutablePath, sdkModulePath, runId });
}
export async function cancelClaudeAgentOperation(runId: string): Promise<boolean> {
  return invoke('cancel_claude_agent_operation', { runId });
}
export async function forgetClaudeAgentConnection(connectionId: string): Promise<boolean> {
  return invoke('forget_claude_agent_connection', { connectionId });
}
export interface ClaudeGenerationSettings {
  modelId: string;
  effort: 'low' | 'medium' | 'high' | 'xhigh' | 'max' | null;
  approvalPolicy: 'dontAsk';
  maxTurns: number;
}
/** Offline preparation only. Native identity/capabilities come from the opaque
 * current connection; this does not grant consent or dispatch generation. */
export async function buildClaudeChapterReview(connectionId: string, input: {
  projectId: string; snapshotId: string; targetPath: string | null;
  selectedPaths: string[]; settings: ClaudeGenerationSettings;
  sectionUpdate?: SectionUpdateReviewTarget | null;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_claude_chapter_review', { connectionId, input });
}

export type HandoffAgent = 'claude-code' | 'codex' | 'antigravity' | 'other';
export async function buildAgentHandoffReview(input: {
  projectId: string; snapshotId: string; targetPath: string | null;
  selectedPaths: string[]; agentId: HandoffAgent;
}): Promise<ProviderCommandResult<ProviderRequestReview>> {
  return invoke('build_agent_handoff_review', { input });
}
export async function exportAgentHandoff(reviewId: string, fingerprint: string, consent: boolean): Promise<ProviderCommandResult<string>> {
  return invoke('export_agent_handoff', { reviewId, fingerprint, consent });
}
export async function importAgentHandoffResult(reviewId: string, fingerprint: string, responseText: string): Promise<ProviderCommandResult<ChapterDraftPreview>> {
  return invoke('import_agent_handoff_result', { reviewId, fingerprint, responseText });
}
