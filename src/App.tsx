import { updateWorkspace, useWorkspaceOutcomes } from './workspace/store';
import { markQuestionOutcome, openWorkspaceQuestion } from './workspace/outcomes';
import { WorkspaceFrame } from './workspace/Workspace';
import { workspaceReadiness, type SnapshotHistoryLoad, type SnapshotManifestLoad } from './lib/workspaceReadiness';
import { readFileLocation, rememberFileLocation } from './lib/workspacePreferences';
import { GenerationRecord } from './GenerationRecord';
import { FileFlow } from './FileFlow';
import { readGenerationBackend, rememberGenerationBackend, type GenerationBackend } from './lib/generationPreferences';
import type { AgentGitRecovery } from './lib/agentRecovery';
import { lazy, Suspense, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState, useSyncExternalStore, type FormEvent, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type RefObject } from 'react';
import { listen } from '@tauri-apps/api/event';
import { type CodexChapterDialogMode } from './CodexChapterDialog';
import { CodexContextDialog } from './CodexContextDialog';
import { AgentHandoffDialog } from './AgentHandoffDialog';
import { ClaudeConnectionDialog } from './ClaudeConnectionDialog';
import { RepositoryPlanningDialog, type RepositoryPlanningScope } from './RepositoryPlanningDialog';
import { RepositoryPlans } from './RepositoryPlans';
import { ConnectionHeader } from './ConnectionHeader';
import { ConnectionsDialog } from './ConnectionsDialog';
import { NotificationCenter, type WorkspaceNotification } from './NotificationCenter';
import { ThemeControl } from './ThemeControl';
import type { PrepareRepositoryPlanInput } from './lib/repositoryPlans';
import type { PlannedChapterScope } from './lib/plannedChapters';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Check,
  ChevronDown,
  CircleHelp,
  Code2,
  Command,
  FileCode2,
  FolderOpen,
  GitBranch,
  HardDrive,
  Layers3,
  ListChecks,
  MoveDown,
  MoveUp,
  Network,
  Pencil,
  Play,
  Search,
  Save,
  Settings2,
  Sparkles,
  Trash2,
  TriangleAlert,
  Workflow,
  X,
} from 'lucide-react';
import {
  compareLocalGitRevisions,
  cancelLocalGitComparison,
  getApplicationStatus,
  getCaptureLimits,
  assessChapterFreshness,
  getSnapshotIndexProgress,
  isNativeRuntime,
  loadSnapshotIndex,
  listRecentProjects,
  listInterruptedOperations,
  loadInterruptedGenerationScope,
  listPendingGeneratedChapterPreviews,
  loadPendingGeneratedChapterPreview,
  pickProjectFolder,
  isProjectOpeningBusy,
  subscribeProjectOpening,
  stopProjectOpening,
  openRecentProject,
  discardInterruptedOperation,
  removeRecentProject,
  type ApplicationStatus,
  type ProjectOpenResult,
  type ProjectOpenStatus,
  type RecentProject,
  type InterruptedOperation,
  type PendingChapterPreviewSummary,
  type CaptureLimits,
  type SnapshotProgress,
  type SnapshotSummary,
  type SnapshotManifest,
  type SnapshotSource,
  type ChapterClaim,
  type ChapterSection,
  type ChapterSourceAnchor,
  type HotspotDestination,
  type ChapterValidationReport,
  type ChapterFreshnessReport,
  type ChapterFreshnessSourceChange,
  type SectionUpdateReviewTarget,
  type CodeLensExplanationTarget,
  type CodeLensExplanationCitation,
  type CodeLensExplanationPreview,
  type SnapshotIndex,
  type IndexProgress,
  type RelationshipKind,
  type GitComparisonResult,
} from './lib/commands';
import type { ReaderRouteContext } from './Walkthrough';
import { scrollBehaviorForMotionPreference } from './lib/motion';
import { errorMessageOr } from './lib/errorMessage';
const WalkthroughScreen = lazy(() => import('./Walkthrough').then(({ WalkthroughScreen }) => ({ default: WalkthroughScreen })));
const GitChangesScreen = lazy(() => import('./GitChanges').then(({ GitChangesScreen }) => ({ default: GitChangesScreen })));
const RepositoryJourneys = lazy(() => import('./RepositoryJourneys').then(({ RepositoryJourneys }) => ({ default: RepositoryJourneys })));
import {
  buildCodeLensProviderRequestReview,
  buildProviderRequestReview,
  cancelProviderRequest,
  discardProviderRequestReview,
  discardGeneratedChapterDraft,
  getProviderCredentialStatus,
  getProviderCatalog,
  removeProviderApiKey,
  sendProviderRequest,
  sendCodeLensProviderRequest,
  saveGeneratedChapterDraft,
  listChapterRevisions,
  listProjectChapterRevisions,
  validateChapterRevision,
  saveChapterEdits,
  restoreChapterRevision,
  acceptChapterDraft,
  rejectChapterDraft,
  storeProviderApiKey,
  type ProviderCommandResult,
  type ProviderConsent,
  type ProviderCredentialStatus,
  type ProviderDisclosure,
  type ProviderFailure,
  type ProviderRequestReview,
  type ChapterDraftPreview,
  type ChapterRevision,
} from './lib/commands';
import {
  cancelSnapshot,
  getSnapshotProgress,
  listProjectSnapshots,
  loadSnapshotManifest,
  readSnapshotSource,
  startSnapshot,
  startSnapshotIndexing,
} from './lib/commands';
import {
  buildInventory,
  filterInventory,
  inventoryReasonLabel,
  languageSupportLabel,
  languageSupportNote,
  languageSupportTier,
  languageForPath,
  type CaptureDisposition,
  type InventoryCategory,
  type InventoryRow,
  type ParserReadiness,
} from './lib/inventory';
import { ATLAS_MAX_ZOOM, ATLAS_MIN_ZOOM, ATLAS_ZOOM_STEP, atlasRowIndexStatus, atlasRowIndexStatusLabel, atlasTreeKeyboardAction, buildInventoryTree, filterInventoryTreeRows, filterProvenRelationshipEdges, fitAtlasViewport, flattenInventoryTree, normalizeAtlasViewport, panAtlasViewport, projectProvenRelationshipGraph, summarizeAtlasPathIndexCounts, virtualRowWindow, zoomAtlasViewport, type AtlasViewport, type ProvenGraphEdge } from './lib/atlas';
import { classifyCodeLensUnavailableState, codeLensRangeFromChapterAnchor, codeLensRangeFromIndexedAnchor, codeLensRelationshipLabel, highlightSourceLines, resolveCodeLensCitation, sourceLineTextRanges, splitCapturedSourceLines, splitSourceTextAtRange } from './lib/codeLens';
import { loadShellStartupData, workspaceHeading } from './lib/shell';
import { compareChapterRevisions, type RevisionChange } from './lib/revisionHistory';
import {
  addCoverageGroup,
  buildCoverageRows,
  COVERAGE_STATUSES,
  excludeCoveragePath,
  filterCoverageRows,
  loadCoveragePlan,
  paginateCoverage,
  removeCoverageGroup,
  restoreCoveragePath,
  saveCoveragePlan,
  summarizeCoverage,
  type CoveragePlan,
  type CoverageRow,
  type CoverageStatus,
} from './lib/coverage';
import {
  canNavigateBack,
  canNavigateForward,
  initialNavigationState,
  navigateBack,
  navigateForward,
  navigateToRoute,
  navigateToWorkspace,
  replaceCurrentRoute,
  type NavigationState,
  type SourceLineRange,
  type Workspace,
  type WorkspaceRoute,
} from './lib/navigation';

type Notice = { title: string; message: string; actionLabel?: string; action?: () => void; tone?: 'info' | 'warning' | 'error' } | null;
type ProviderDialogMode = (
  | { kind: 'settings'; backend?: 'api' | 'codex'; section?: 'explanations' | 'voice'; recoveryOperationId?: string }
  | { kind: 'review'; projectId: string; snapshotId: string; targetPath: string; selectedPaths?: string[]; recoveryOperationId?: string }
  | { kind: 'code-lens-range'; projectId: string; snapshotId: string; target: CodeLensExplanationTarget; recoveryOperationId?: string }
  | { kind: 'section-update'; projectId: string; snapshotId: string; targetPath: string; selectedPaths: string[]; baseRevision: ChapterRevision; sectionUpdate: SectionUpdateReviewTarget; recoveryOperationId?: string }
  | { kind: 'overview'; projectId: string; snapshotId: string; selectedPaths?: string[]; recoveryOperationId?: string }
) & { recoveryBackend?: 'codex' | 'claude-agent' };


const projectResultTitles: Record<ProjectOpenStatus, string> = {
  cancelled: 'Folder selection cancelled',
  opened: 'Repository selected',
  notGitRepository: 'This folder is not a Git repository',
  invalidDirectory: 'This folder is unavailable',
  permissionDenied: 'Folder access was denied',
  gitUnavailable: 'Git is not available',
  gitError: 'Could not inspect Git metadata',
  storageError: 'Local project storage is unavailable',
  recentProjectUnavailable: 'This recent project is unavailable',
};


function App() {
  const [navigation, setNavigation] = useState<NavigationState>(initialNavigationState);
  const navigationRef = useRef(navigation);
  navigationRef.current = navigation;
  const interruptedReviewToken = useRef(0);
  const interruptionLookupVersion = useRef(0);
  const [repositoryContextAnnouncement, setRepositoryContextAnnouncement] = useState('');
  const announcedRepositoryContextRef = useRef<string | null>(null);
  const [runtime, setRuntime] = useState<ApplicationStatus | null>(null);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice>(null);
  useEffect(() => {
    const notify = (event: Event) => { const detail=(event as CustomEvent<{title:string;message:string;projectId?:string;snapshotId?:string;explorationId?:string}>).detail;
      if(detail) setNotice({...detail,...(detail.projectId&&detail.snapshotId&&detail.explorationId?{actionLabel:'Open explanation',action:()=>{
        updateWorkspace(detail.projectId!,detail.snapshotId!,(s)=>({...s,currentId:s.explorations.some((e)=>e.id===detail.explorationId)?detail.explorationId!:'root'}));
        setNavigation((s)=>navigateToRoute(s,{workspace:'guide-plan',projectId:detail.projectId!,snapshotId:detail.snapshotId!}));
      }}:{})}); };
    window.addEventListener('code-atlas:workspace-notice',notify);return()=>window.removeEventListener('code-atlas:workspace-notice',notify);
  }, []);
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [toolsOpen, setToolsOpen] = useState(false);
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const [projectMetadataBusy, setProjectMetadataBusy] = useState(false);
  const projectOpening = useSyncExternalStore(subscribeProjectOpening, isProjectOpeningBusy);
  const projectBusy = projectOpening || projectMetadataBusy;
  useEffect(() => () => stopProjectOpening(), []);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const [recentsPending, setRecentsPending] = useState(true);
  const automaticRestoreAllowed = useRef(true);
  const [knownProjects, setKnownProjects] = useState<Record<string, RecentProject>>({});
  const [snapshotsByProject, setSnapshotsByProject] = useState<Record<string, SnapshotSummary[]>>({});
  const [captureByProject, setCaptureByProject] = useState<Record<string, SnapshotProgress>>({});
  const [snapshotManifest, setSnapshotManifest] = useState<SnapshotManifest | null>(null);
  const [snapshotHistoryLoad, setSnapshotHistoryLoad] = useState<SnapshotHistoryLoad | null>(null);
  const [snapshotManifestLoad, setSnapshotManifestLoad] = useState<SnapshotManifestLoad | null>(null);
  const [workspaceReload, setWorkspaceReload] = useState(0);
  const [snapshotMenuOpen, setSnapshotMenuOpen] = useState(false);
  const [snapshotBusy, setSnapshotBusy] = useState(false);
  const [captureLimits, setCaptureLimits] = useState<CaptureLimits | null>(null);
  const [interruptedOperations, setInterruptedOperations] = useState<InterruptedOperation[]>([]);
  const [interruptedOperationsUnavailable, setInterruptedOperationsUnavailable] = useState(false);
  const [pendingChapterPreviews, setPendingChapterPreviews] = useState<PendingChapterPreviewSummary[]>([]);
  const [pendingChapterPreviewsUnavailable, setPendingChapterPreviewsUnavailable] = useState(false);
  const [gitComparisonsByProject, setGitComparisonsByProject] = useState<Record<string, {
    baseRef: string;
    headRef: string;
    result: GitComparisonResult;
  }>>({});
  const [recoveredPreviewId, setRecoveredPreviewId] = useState<string | null>(null);
  const [recoveryBusyId, setRecoveryBusyId] = useState<string | null>(null);
  const [recoveredGitRequest, setRecoveredGitRequest] = useState<AgentGitRecovery | null>(null);
  const [sourceDocument, setSourceDocument] = useState<SnapshotSource | null>(null);
  const [sourceLoading, setSourceLoading] = useState(false);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [providerDialog, setProviderDialog] = useState<ProviderDialogMode | null>(null);
  const [planningDialog, setPlanningDialog] = useState<RepositoryPlanningScope | null>(null);
  const planningReturnFocus = useRef<HTMLElement | null>(null);
  const closePlanningDialog = useCallback(() => {
    setPlanningDialog(null);
    const previous = planningReturnFocus.current;
    if (previous?.isConnected) previous.focus();
  }, []);
  const openPlanningDialog = useCallback((scope: RepositoryPlanningScope) => {
    planningReturnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setPlanningDialog(scope);
  }, []);
  useEffect(() => {
    if (planningDialog && (navigation.current.workspace !== 'guide-plan'
      || navigation.current.projectId !== planningDialog.projectId || navigation.current.snapshotId !== planningDialog.snapshotId)) {
      // Unmount the existing agent owner; it cancels work and preserves any
      // already published unaccepted result for later native discovery.
      setPlanningDialog(null);
    }
  }, [planningDialog, navigation.current.workspace, navigation.current.projectId, navigation.current.snapshotId]);
  const providerReturnFocusRef = useRef<HTMLElement | null>(null);
  const recoveredPreviewReturnFocusRef = useRef<HTMLButtonElement | null>(null);
  const projectButtonRef = useRef<HTMLButtonElement>(null);
  const pickerReturnFocusRef = useRef<string | undefined>(undefined);
  const currentWorkspace = navigation.current.workspace;
  useEffect(()=>{if(navigation.current.projectId){try{localStorage.setItem('code-atlas:last-project',navigation.current.projectId);}catch{/* Optional preference. */}}},[navigation.current.projectId]);
  const currentProject = navigation.current.projectId
    ? knownProjects[navigation.current.projectId] ?? recentProjects.find((project) => project.projectId === navigation.current.projectId) ?? null
    : null;
  useEffect(() => {
    const projectId = currentProject?.projectId;
    return () => { if (projectId) cancelLocalGitComparison(projectId); };
  }, [currentProject?.projectId]);
  const currentSnapshots = currentProject ? snapshotsByProject[currentProject.projectId] ?? [] : [];
  const currentCapture = currentProject ? captureByProject[currentProject.projectId] ?? null : null;
  const selectedSnapshot = currentSnapshots.find((snapshot) => snapshot.snapshotId === navigation.current.snapshotId) ?? null;
  const questionUpdates = useWorkspaceOutcomes(currentProject?.projectId, selectedSnapshot?.snapshotId);
  const readiness = workspaceReadiness({ recentsPending, projectOpening,
    projectId: navigation.current.projectId, projectAvailable: !!currentProject, snapshotId: navigation.current.snapshotId,
    history: snapshotHistoryLoad, manifest: snapshotManifestLoad });
  const workspaceUnavailable = readiness === 'project-error' || readiness === 'history-error' || readiness === 'snapshot-error';
  const copy = readiness === 'ready'
    ? workspaceHeading(currentWorkspace, !!snapshotManifest && snapshotManifest.summary.snapshotId === selectedSnapshot?.snapshotId)
    : { eyebrow: 'WORKSPACE', title: workspaceUnavailable ? 'Your saved workspace could not open.' : 'Opening your workspace.', subtitle: '' };

  useEffect(() => {
    if (readiness !== 'ready') return;
    if (navigation.current.projectId && !currentProject) return;
    if (navigation.current.snapshotId && !selectedSnapshot) return;

    const projectId = navigation.current.projectId ?? '';
    const snapshotId = navigation.current.snapshotId ?? '';
    const contextKey = `${projectId}\u0000${snapshotId}`;
    if (announcedRepositoryContextRef.current === contextKey) return;
    announcedRepositoryContextRef.current = contextKey;

    if (!currentProject) {
      setRepositoryContextAnnouncement('No repository selected.');
      return;
    }
    if (!selectedSnapshot) {
      setRepositoryContextAnnouncement(`Repository ${currentProject.displayName}. No snapshot selected.`);
      return;
    }

    const snapshotRef = selectedSnapshot.head?.slice(0, 10) ?? 'unborn HEAD';
    const completeness = selectedSnapshot.status === 'partial' ? 'partial capture' : 'complete under policy';
    setRepositoryContextAnnouncement(
      `Repository ${currentProject.displayName}. Selected snapshot ${selectedSnapshot.branch ?? 'detached'}, commit ${snapshotRef}, ${completeness}, created ${formatSnapshotDate(selectedSnapshot.createdAtUnixMs)}.`,
    );
  }, [
    currentProject?.displayName,
    currentProject?.projectId,
    navigation.current.projectId,
    navigation.current.snapshotId,
    selectedSnapshot,
    readiness,
  ]);

  const navigationActions = useMemo(
    () => ({
      back: () => setNavigation((state) => navigateBack(state)),
      forward: () => setNavigation((state) => navigateForward(state)),
      workspace: (workspace: Workspace) => {
        const activeElement = document.activeElement;
        const returnFocusTarget = activeElement instanceof HTMLElement
          ? activeElement.dataset.focusKey
          : undefined;
        setNavigation((state) => navigateToWorkspace(state, workspace, returnFocusTarget));
      },
    }),
    [],
  );

  const loadRuntime = () => {
    setRuntimeError(null);
    void getApplicationStatus()
      .then(setRuntime)
      .catch(() => setRuntimeError('The desktop bridge did not respond. The workspace is still available.'));
  };

  async function retryRecentProjectsLoad() {
    try {
      const recent = await listRecentProjects();
      setRecentProjects(recent);
      setKnownProjects(Object.fromEntries(recent.map((project) => [project.projectId, project])));
      setNotice(null);
    } catch (error) {
      setNotice({
        title: 'Recent projects could not be loaded',
        message: errorMessageOr(error, 'The local project list is unavailable.'),
        actionLabel: 'Retry',
        action: () => { void retryRecentProjectsLoad(); },
        tone: 'error',
      });
    }
  }

  const refreshInterruptedOperations = useCallback(async () => {
    const version = ++interruptionLookupVersion.current;
    try {
      const operations = await listInterruptedOperations();
      if (version !== interruptionLookupVersion.current) return;
      setInterruptedOperations(operations);
      setInterruptedOperationsUnavailable(false);
    } catch {
      if (version === interruptionLookupVersion.current) setInterruptedOperationsUnavailable(true);
    }
  }, []);

  const completeRecoveredProviderReview = useCallback(async (operationId: string) => {
    try {
      await discardInterruptedOperation(operationId);
      setInterruptedOperations((current) => current.filter((operation) => operation.operationId !== operationId));
      void refreshInterruptedOperations();
      setRecoveredGitRequest((current) => current?.operationId === operationId ? null : current);
    } catch (error) {
      setNotice({
        title: 'Recovery notice could not be cleared',
        message: errorMessageOr(error, 'The old request notice remains. The new request still requires fresh review and consent.'),
        tone: 'error',
      });
    }
  }, [refreshInterruptedOperations]);

  const refreshPendingChapterPreviews = useCallback(async () => {
    try {
      setPendingChapterPreviews(await listPendingGeneratedChapterPreviews());
      setPendingChapterPreviewsUnavailable(false);
    } catch {
      setPendingChapterPreviewsUnavailable(true);
    }
  }, []);

  const discardPendingPreview = async (summary: PendingChapterPreviewSummary) => {
    try {
      await discardGeneratedChapterDraft(summary.generationId);
      setPendingChapterPreviews((current) => current.filter((item) => item.generationId !== summary.generationId));
      if (recoveredPreviewId === summary.generationId) setRecoveredPreviewId(null);
    } catch (error) {
      setNotice({
        title: 'Saved preview could not be discarded',
        message: errorMessageOr(error, 'The local recovery copy is still available.'),
        tone: 'error',
      });
    }
  };

  const discardRecovery = async (operation: InterruptedOperation) => {
    setRecoveryBusyId(operation.operationId);
    try {
      await discardInterruptedOperation(operation.operationId);
      setInterruptedOperations((current) => current.filter((item) => item.operationId !== operation.operationId));
      void refreshInterruptedOperations();
      setRecoveredGitRequest((current) => current?.operationId === operation.operationId ? null : current);
    } catch (error) {
      setNotice({
        title: 'Recovery notice could not be cleared',
        message: errorMessageOr(error, 'The local recovery record could not be updated.'),
        tone: 'error',
      });
    } finally {
      setRecoveryBusyId(null);
    }
  };

  const retryInterruptedCapture = async (operation: InterruptedOperation) => {
    if (operation.kind !== 'capture') return;
    setRecoveryBusyId(operation.operationId);
    try {
      const opened = await openRecentProject(operation.projectId);
      if (!opened.project || (opened.status !== 'opened' && opened.status !== 'storageError')) {
        handleProjectOpenResult(opened);
        return;
      }
      handleProjectOpenResult(opened);
      const start = await startSnapshot(operation.projectId);
      const progress = await getSnapshotProgress(start.snapshotId).catch(() => null);
      if (progress) setCaptureByProject((current) => ({ ...current, [operation.projectId]: progress }));
      try {
        await discardInterruptedOperation(operation.operationId);
        setInterruptedOperations((current) => current.filter((item) => item.operationId !== operation.operationId));
        void refreshInterruptedOperations();
        setNotice({
          title: 'A fresh capture has started',
          message: 'The interrupted capture was not resumed. Existing snapshots remain available while this new capture runs.',
        });
      } catch (error) {
        if (error instanceof DOMException && error.name === 'AbortError') return;
        setNotice({
          title: 'A fresh capture started; old recovery data remains',
          message: errorMessageOr(error, 'The old staging bytes could not be cleared. The interrupted item remains available to discard.'),
          tone: 'warning',
        });
      }
    } catch (error) {
      setNotice({
        title: 'Capture retry could not start',
        message: errorMessageOr(error, 'Open the repository and try a new capture again.'),
        tone: 'error',
      });
    } finally {
      setRecoveryBusyId(null);
    }
  };

  const reviewInterruptedGeneration = async (operation: InterruptedOperation) => {
    if (operation.kind !== 'providerGeneration') return;
    const token = ++interruptedReviewToken.current;
    const originRoute = navigationRef.current.current;
    const current = () => interruptedReviewToken.current === token && navigationRef.current.current === originRoute;
    setRecoveryBusyId(operation.operationId);
    setRecoveredGitRequest(null);
    try {
      const recordedRequest = operation.providerRequest;
      if (!recordedRequest) throw new Error('This older interruption record does not identify the request scope. Reopen the project and select the original file, section, source range or change before preparing a new request. The app will not assume it was a whole-file explanation.');
      if (!['codex', 'claude-agent', 'gemini', 'openai', 'anthropic'].includes(recordedRequest.providerId)) throw new Error('The original backend is not connected to interruption recovery. Its scope stays in this notice; no API request was opened.');
      const opened = await openRecentProject(operation.projectId);
      if (!current()) return;
      if (!opened.project || (opened.status !== 'opened' && opened.status !== 'storageError')) {
        handleProjectOpenResult(opened);
        return;
      }
      const recoveryBackend = recordedRequest.providerId === 'codex' ? 'codex' : recordedRequest.providerId === 'claude-agent' ? 'claude-agent' : undefined;
      // Both agents restore native journal-owned evidence. This reads local
      // source/Git only; it restores neither runtime settings nor consent.
      const recovered = recoveryBackend ? await loadInterruptedGenerationScope(operation.operationId) : null;
      if (!current()) return;
      const request = recovered ? recovered.operation.providerRequest : recordedRequest;
      if (!request) throw new Error('The interrupted request scope is unavailable. Select the original evidence again.');
      if (recovered) {
        const original = recovered.operation;
        const exact = original.providerRequest;
        if (!exact || original.operationId !== operation.operationId || original.projectId !== operation.projectId
          || original.snapshotId !== operation.snapshotId || original.targetPath !== operation.targetPath
          || original.startedAtUnixMs !== operation.startedAtUnixMs
          || exact.providerId !== recoveryBackend || exact.purpose !== recordedRequest.purpose) {
          throw new Error('The interrupted request changed during recovery. Review the current notice again.');
        }
      }
      if (request.purpose === 'repositoryPlan') {
        if (!recovered || !recoveryBackend || !request.repositoryPlan) throw new Error('The original planning question or native source scope is unavailable. The recovery notice was preserved.');
        handleProjectOpenResult(opened);
        setNavigation((state) => navigateToRoute(state, { workspace: 'guide-plan', projectId: operation.projectId,
          snapshotId: operation.snapshotId, focusTarget: 'heading:guide-plan' }));
        openPlanningDialog({ projectId: operation.projectId, snapshotId: operation.snapshotId,
          question: request.repositoryPlan.question, selectedPaths: request.selectedPaths,
          recoveryOperationId: operation.operationId, recoveryBackend });
        return;
      }
      if (request.plannedChapter) {
        if (!recovered || !recoveryBackend || request.purpose !== 'fileChapter') throw new Error('The accepted reading step cannot be restored with this backend. Its recovery notice was preserved.');
        const scope = request.plannedChapter;
        if (scope.snapshotId !== operation.snapshotId || scope.primaryPath !== operation.targetPath) throw new Error('The accepted step differs from the interrupted chapter. No broader request was opened.');
        handleProjectOpenResult(opened);
        setNavigation((state) => navigateToRoute(state, { workspace: 'guide-plan', projectId: operation.projectId,
          snapshotId: operation.snapshotId, focusTarget: 'heading:guide-plan' }));
        openPlanningDialog({ kind: 'planned-chapter', projectId: operation.projectId, snapshotId: scope.snapshotId,
          question: scope.question, selectedPaths: scope.selectedPaths, plannedChapter: scope,
          recoveryOperationId: operation.operationId, recoveryBackend });
        return;
      }
      const sectionTarget = request.sectionUpdate;
      let sectionBase: ChapterRevision | null = null;
      if (request.purpose === 'sectionUpdate') {
        if (!sectionTarget || !operation.targetPath) throw new Error('The interrupted section identity is incomplete. Select the original stale section in Edit and review it again.');
        const revisions = await listProjectChapterRevisions(operation.projectId);
        if (!current()) return;
        const base = revisions.find((revision) => revision.revisionId === sectionTarget.chapterRevisionId);
        const section = base?.sections.find((item) => item.sectionId === sectionTarget.sectionId && item.sectionRevisionId === sectionTarget.sectionRevisionId);
        if (!base || base.lifecycle !== 'accepted' || base.target.scope.kind !== 'file' || base.target.scope.path !== operation.targetPath || !section) {
          throw new Error('The exact accepted chapter or section is unavailable. Reassess freshness in Edit and select the intended repair; no broader request was opened.');
        }
        sectionBase = base;
      }
      let rangeTarget: CodeLensExplanationTarget | null = null;
      let gitRecovery: AgentGitRecovery | null = null;
      let recoveredComparison: GitComparisonResult | null = null;
      if (recovered && recoveryBackend && ['codeLensExplanation', 'prGuide', 'prChangeExplanation'].includes(request.purpose)) {
        if (request.purpose === 'codeLensExplanation') {
          if (!request.codeLens) throw new Error('This older notice has no exact range. Select the original range again.');
          rangeTarget = request.codeLens;
        } else {
          const git = request.git;
          if (!git || !recovered.comparison || recovered.comparison.comparisonId !== git.comparisonId) {
            throw new Error('This notice has no recoverable Git selection. Select the original comparison again.');
          }
          recoveredComparison = recovered.comparison;
          if (request.purpose === 'prGuide') {
            gitRecovery = { operationId: operation.operationId, backend: recoveryBackend,
              mode: { kind: 'pr-guide', projectId: operation.projectId,
                baseRef: git.baseRef, headRef: git.headRef, comparisonId: git.comparisonId,
                selectedChanges: git.selectedChanges, recoveryOperationId: operation.operationId } };
          } else {
            if (git.hunkIndex === null || git.selectedChanges.length !== 1) throw new Error('The exact interrupted hunk is unavailable. Select it again.');
            gitRecovery = { operationId: operation.operationId, backend: recoveryBackend,
              mode: { kind: 'pr-change', projectId: operation.projectId,
                target: { baseRef: git.baseRef, headRef: git.headRef, comparisonId: git.comparisonId,
                  selectedChange: git.selectedChanges[0], hunkIndex: git.hunkIndex }, recoveryOperationId: operation.operationId } };
          }
        }
      }
      handleProjectOpenResult(opened);
      const chapterRequest = ['fileChapter', 'repositoryOverview', 'sectionUpdate'].includes(request.purpose);
      const recoveredRefs = !gitRecovery ? null : gitRecovery.mode.kind === 'pr-guide' ? gitRecovery.mode : gitRecovery.mode.target;
      if (recoveredRefs && recoveredComparison) {
        const result = recoveredComparison;
        setGitComparisonsByProject((state) => ({ ...state, [operation.projectId]: {
          baseRef: recoveredRefs.baseRef, headRef: recoveredRefs.headRef, result,
        } }));
      }
      setNavigation((state) => navigateToRoute(state, {
        workspace: chapterRequest ? 'chapter-studio' : request.purpose === 'codeLensExplanation' ? 'atlas' : 'changes',
        projectId: operation.projectId,
        snapshotId: request.purpose === 'prGuide' || request.purpose === 'prChangeExplanation' ? null : operation.snapshotId,
        baseRef: recoveredRefs?.baseRef, headRef: recoveredRefs?.headRef,
        relativePath: rangeTarget?.path ?? (request.purpose === 'codeLensExplanation' ? operation.targetPath ?? undefined : undefined),
        sourceRange: rangeTarget ? { startLine: rangeTarget.startLine, endLine: rangeTarget.endLine, sourceSha256: rangeTarget.sourceSha256,
          ...(rangeTarget.startByte === null || rangeTarget.endByteExclusive === null ? {} : { startByte: rangeTarget.startByte, endByteExclusive: rangeTarget.endByteExclusive }) } : undefined,
        focusTarget: chapterRequest ? 'heading:chapter-studio' : request.purpose === 'codeLensExplanation' ? 'heading:atlas' : 'heading:changes',
      }));
      if (gitRecovery) {
        setRecoveredGitRequest(gitRecovery);
      } else if (rangeTarget) {
        setProviderDialog({ kind: 'code-lens-range', projectId: operation.projectId, snapshotId: operation.snapshotId,
          target: rangeTarget, recoveryOperationId: operation.operationId, recoveryBackend });
      } else if (request.purpose === 'sectionUpdate' && sectionBase && sectionTarget && operation.targetPath) {
        setProviderDialog({ kind: 'section-update', projectId: operation.projectId, snapshotId: operation.snapshotId,
          targetPath: operation.targetPath, selectedPaths: request.selectedPaths, baseRevision: sectionBase, sectionUpdate: sectionTarget,
          recoveryOperationId: operation.operationId, recoveryBackend });
      } else if (request.purpose === 'fileChapter' && operation.targetPath) {
        setProviderDialog({ kind: 'review', projectId: operation.projectId, snapshotId: operation.snapshotId,
          targetPath: operation.targetPath, selectedPaths: request.selectedPaths,
          recoveryOperationId: operation.operationId, recoveryBackend });
      } else if (request.purpose === 'repositoryOverview') {
        setProviderDialog({ kind: 'overview', projectId: operation.projectId, snapshotId: operation.snapshotId,
          selectedPaths: request.selectedPaths, recoveryOperationId: operation.operationId, recoveryBackend });
      } else {
        setNotice({ title: 'Select the original evidence again', message: request.purpose === 'codeLensExplanation'
          ? 'This was a source-range explanation. Select its exact range in Files and review a fresh request; a whole-file request was not opened.'
          : 'This was a Git change explanation. Reopen the original local comparison and selected files or hunk before reviewing a fresh request. No chapter request was opened.', tone: 'warning' });
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      if (current()) setNotice({
        title: 'A fresh request review could not open',
        message: errorMessageOr(error, 'Reopen the repository and review a new exact-context request.'),
        tone: 'error',
      });
    } finally {
      if (interruptedReviewToken.current === token) setRecoveryBusyId(null);
    }
  };

  const activateProject = (project: RecentProject) => {
    setKnownProjects((current) => ({ ...current, [project.projectId]: project }));
    setNavigation((state) => {
      const returnFocus = pickerReturnFocusRef.current;
      const stateWithOriginFocus = returnFocus
        ? { ...state, current: { ...state.current, focusTarget: returnFocus } }
        : state;
      return navigateToRoute(stateWithOriginFocus, {
        workspace: 'atlas',
        projectId: project.projectId,
        snapshotId: null,
        focusTarget: 'heading:atlas',
      });
    });
    setProjectMenuOpen(false);
    setToolsOpen(false);
    setSnapshotMenuOpen(false);
  };

  const handleProjectOpenResult = (result: ProjectOpenResult) => {
    if (result.status === 'opened' && result.project) {
      const project = result.project;
      setRecentProjects((current) => [project, ...current.filter((recent) => recent.projectId !== project.projectId)]);
      activateProject(project);
      setNotice({
        title: 'Repository selected',
        message: `Git root: ${project.repositoryRoot}. ${project.branch ? `Branch ${project.branch}. ` : ''}${project.head ? `HEAD ${project.head.slice(0, 10)}. ` : 'No commit is available yet. '}No source files have been read.`,
      });
      return;
    }

    if (result.status === 'storageError' && result.project) {
      activateProject(result.project);
      setNotice({
        title: 'Repository ready for this session',
        message: `${result.message ?? 'Code Atlas could not save this path to Recents.'} You can continue now; it will not appear after relaunch unless you open it again.`,
        tone: 'warning',
      });
      return;
    }

    if (result.status === 'cancelled') return;

    setNotice({
      title: result.status === 'opened' ? 'The project response was incomplete' : projectResultTitles[result.status],
      message: `${result.message ?? 'Choose another folder to continue.'} Your current project selection has not changed.`,
      actionLabel: 'Choose another folder',
      action: () => { void openLocalRepository(); },
      tone: 'error',
    });
  };

  const openLocalRepository = async () => {
    automaticRestoreAllowed.current = false;
    const activeElement = document.activeElement;
    const activeFocus = activeElement instanceof HTMLElement ? activeElement.dataset.focusKey : undefined;
    pickerReturnFocusRef.current = activeElement instanceof HTMLElement && activeElement.closest('.project-menu')
      ? 'project-button'
      : activeFocus;
    setProjectMenuOpen(false);
    setToolsOpen(false);

    if (!isNativeRuntime()) {
      setNotice({
        title: 'Open the Code Atlas Mac app to choose a repository',
        message: 'The desktop preview cannot show the native folder picker or read local folder permissions.',
        tone: 'info',
      });
      return;
    }

    try {
      const result = await pickProjectFolder();
      if (result.status === 'cancelled') {
        const returnFocus = pickerReturnFocusRef.current;
        requestAnimationFrame(() => {
          if (returnFocus) {
            document.querySelector<HTMLElement>(`[data-focus-key="${CSS.escape(returnFocus)}"]`)?.focus();
          }
        });
        return;
      }
      handleProjectOpenResult(result);
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      setNotice({
        title: 'Could not open the folder picker',
        message: errorMessageOr(error, 'The native folder picker did not respond.'),
        actionLabel: 'Try again',
        action: () => { void openLocalRepository(); },
        tone: 'error',
      });
    }
  };

  const openRecent = async (projectId: string) => {
    automaticRestoreAllowed.current = false;
    pickerReturnFocusRef.current = 'project-button';
    setProjectMenuOpen(false);
    try {
      handleProjectOpenResult(await openRecentProject(projectId));
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      setNotice({
        title: 'Could not open this recent project',
        message: errorMessageOr(error, 'Local project metadata could not be read.'),
        actionLabel: 'Choose a folder',
        action: () => { void openLocalRepository(); },
        tone: 'error',
      });
    }
  };

  const forgetRecent = async (project: RecentProject) => {
    setProjectMenuOpen(false);
    setProjectMetadataBusy(true);
    try {
      setRecentProjects(await removeRecentProject(project.projectId));
      setNotice({
        title: 'Removed from Recents',
        message: `${project.displayName} was removed from Code Atlas. Its repository folder and files were left untouched.`,
      });
      projectButtonRef.current?.focus();
    } catch (error) {
      setNotice({
        title: 'Could not update Recents',
        message: errorMessageOr(error, 'The local project list could not be updated.'),
        tone: 'error',
      });
    } finally {
      setProjectMetadataBusy(false);
    }
  };

  const createSnapshot = async () => {
    if (!currentProject) return;
    if (!isNativeRuntime()) {
      setNotice({
        title: 'Capture from the Code Atlas Mac app',
        message: 'The browser preview cannot read a repository or create a local snapshot.',
      });
      return;
    }
    setSnapshotBusy(true);
    setNotice(null);
    try {
      const start = await startSnapshot(currentProject.projectId);
      const progress = await getSnapshotProgress(start.snapshotId);
      if (progress) {
        setCaptureByProject((current) => ({ ...current, [currentProject.projectId]: progress }));
      }
    } catch (error) {
      setNotice({
        title: 'Snapshot could not start',
        message: errorMessageOr(error, 'The local capture service did not respond.'),
        tone: 'error',
      });
    } finally {
      setSnapshotBusy(false);
    }
  };

  const stopSnapshot = async () => {
    if (!currentCapture || currentCapture.status !== 'running') return;
    try {
      await cancelSnapshot(currentCapture.snapshotId);
    } catch (error) {
      setNotice({
        title: 'Capture could not be cancelled',
        message: errorMessageOr(error, 'The capture may have already finished.'),
        tone: 'error',
      });
    }
  };

  useEffect(() => {
    if (navigation.current.projectId && navigation.current.snapshotId && currentWorkspace === 'atlas') {
      rememberFileLocation(navigation.current.projectId, {
        snapshotId: navigation.current.snapshotId, path: navigation.current.relativePath,
      });
    }
  }, [navigation.current.projectId, navigation.current.snapshotId, navigation.current.relativePath, currentWorkspace]);

  const selectSnapshot = (summary: SnapshotSummary) => {
    if (!currentProject) return;
    setNavigation((state) => navigateToRoute(state, {
      workspace: state.current.workspace,
      projectId: currentProject.projectId,
      snapshotId: summary.snapshotId,
      focusTarget: `heading:${state.current.workspace}`,
    }));
    setSnapshotMenuOpen(false);
    setNotice((current) => current?.title === 'Repository selected' ? null : current);
  };

  useEffect(() => {
    let active = true;
    const recoveryVersion = interruptionLookupVersion.current;
    void loadShellStartupData({ getApplicationStatus, getCaptureLimits, listRecentProjects, listInterruptedOperations, listPendingGeneratedChapterPreviews }, (projects) => {
      if (!active) return;
      if (projects === null) {
        setNotice({
          title: 'Recent projects could not be loaded',
          message: 'The workspace is still available. Retry the local Recents lookup or choose a repository to add it again.',
          actionLabel: 'Retry', action: () => { void retryRecentProjectsLoad(); }, tone: 'error',
        });
      } else {
        // Preserve a repository opened by the user while the startup lookup ran.
        setRecentProjects((current) => [...current, ...projects.filter((project) => !current.some((item) => item.projectId === project.projectId))]);
        setKnownProjects((current) => ({ ...Object.fromEntries(projects.map((project) => [project.projectId, project])), ...current }));
        try {
          const previous = localStorage.getItem('code-atlas:last-project');
          if (automaticRestoreAllowed.current && previous && !navigationRef.current.current.projectId
            && !isProjectOpeningBusy() && projects.some((project) => project.projectId === previous)) void openRecent(previous);
        } catch { /* Starting without an optional preference remains usable. */ }
      }
      setRecentsPending(false);
    }).then((startup) => {
      if (!active) return;
      setRuntime(startup.applicationStatus);
      setRuntimeError(startup.applicationStatusUnavailable ? 'The desktop bridge did not respond. The workspace is still available.' : null);
      setCaptureLimits(startup.captureLimits);
      if (recoveryVersion === interruptionLookupVersion.current) {
        setInterruptedOperations(startup.interruptedOperations ?? []);
        setInterruptedOperationsUnavailable(startup.interruptedOperationsUnavailable);
      }
      setPendingChapterPreviews(startup.pendingChapterPreviews ?? []);
      setPendingChapterPreviewsUnavailable(startup.pendingChapterPreviewsUnavailable);
    });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    const refresh = () => { void refreshPendingChapterPreviews(); };
    const refreshPlans = () => { void refreshInterruptedOperations(); };
    window.addEventListener('code-atlas:pending-chapter-previews-changed', refresh);
    window.addEventListener('code-atlas:repository-plans-changed', refreshPlans);
    window.addEventListener('code-atlas:agent-recovery-changed', refreshPlans);
    return () => {
      window.removeEventListener('code-atlas:pending-chapter-previews-changed', refresh);
      window.removeEventListener('code-atlas:repository-plans-changed', refreshPlans);
      window.removeEventListener('code-atlas:agent-recovery-changed', refreshPlans);
      ++interruptionLookupVersion.current;
    };
  }, [refreshInterruptedOperations, refreshPendingChapterPreviews]);

  useEffect(() => {
    if (!currentProject) {
      setSnapshotManifest(null);
      return;
    }
    let active = true;
    setSnapshotHistoryLoad({ projectId: currentProject.projectId, status: 'loading' });
    void listProjectSnapshots(currentProject.projectId)
      .then(({ snapshots }) => {
        if (!active) return;
        const sorted = newestSnapshotsFirst(snapshots);
        setSnapshotsByProject((current) => ({ ...current, [currentProject.projectId]: sorted }));
        setSnapshotHistoryLoad({ projectId: currentProject.projectId, status: 'ready' });
        const saved = readFileLocation(currentProject.projectId);
        const snapshot = sorted.find((item) => item.snapshotId === saved?.snapshotId) ?? sorted[0];
        if (snapshot) {
          setNavigation((state) => state.current.projectId !== currentProject.projectId || state.current.snapshotId
            ? state : replaceCurrentRoute(state, { ...state.current, snapshotId: snapshot.snapshotId,
              relativePath: saved?.snapshotId === snapshot.snapshotId ? saved.path : undefined }));
          setNotice((current) => current?.title === 'Repository selected' ? null : current);
        }
      })
      .catch((error) => {
        if (active) setSnapshotHistoryLoad({ projectId: currentProject.projectId, status: 'error' });
        if (active && isNativeRuntime()) {
          setNotice({ title: 'Snapshot history could not be loaded', message: errorMessageOr(error, 'Local snapshot history is unavailable.'), tone: 'error' });
        }
      });
    return () => { active = false; };
  }, [currentProject?.projectId, workspaceReload]);

  useEffect(() => {
    if (!currentProject || !navigation.current.snapshotId) {
      setSnapshotManifest(null);
      return;
    }
    setSnapshotManifest(null);
    let active = true;
    const scope = { projectId: currentProject.projectId, snapshotId: navigation.current.snapshotId };
    setSnapshotManifestLoad({ ...scope, status: 'loading' });
    void loadSnapshotManifest(currentProject.projectId, navigation.current.snapshotId)
      .then((manifest) => { if (active) { setSnapshotManifest(manifest); setSnapshotManifestLoad({ ...scope, status: 'ready' }); } })
      .catch((error) => {
        if (active) {
          setSnapshotManifestLoad({ ...scope, status: 'error' });
          setNotice({ title: 'Snapshot details could not be opened', message: errorMessageOr(error, 'The immutable snapshot manifest is unavailable.'), tone: 'error' });
        }
      });
    return () => { active = false; };
  }, [currentProject?.projectId, navigation.current.snapshotId, workspaceReload]);

  useEffect(() => {
    const route = navigation.current;
    if ((route.workspace !== 'atlas' && route.workspace !== 'code-lens') || !route.projectId || !route.snapshotId || !route.relativePath) {
      setSourceDocument(null);
      setSourceError(null);
      setSourceLoading(false);
      return;
    }
    if (!isNativeRuntime()) {
      setSourceDocument(null);
      setSourceError(null);
      setSourceLoading(false);
      return;
    }
    let active = true;
    setSourceDocument(null);
    setSourceError(null);
    setSourceLoading(true);
    void readSnapshotSource(route.projectId, route.snapshotId, route.relativePath)
      .then((source) => { if (active) setSourceDocument(source); })
      .catch((error) => {
        if (active) setSourceError(errorMessageOr(error, 'Snapshot source could not be opened.'));
      })
      .finally(() => { if (active) setSourceLoading(false); });
    return () => { active = false; };
  }, [navigation.current.workspace, navigation.current.projectId, navigation.current.snapshotId, navigation.current.relativePath]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listen<SnapshotProgress>('snapshot-progress', ({ payload }) => {
      setCaptureByProject((current) => ({ ...current, [payload.projectId]: payload }));
      if (payload.summary) {
        setSnapshotsByProject((current) => {
          const previous = current[payload.projectId] ?? [];
          return {
            ...current,
            [payload.projectId]: [payload.summary!, ...previous.filter((item) => item.snapshotId !== payload.summary!.snapshotId)],
          };
        });
        void listProjectSnapshots(payload.projectId)
          .then(({ snapshots }) => setSnapshotsByProject((current) => ({ ...current, [payload.projectId]: newestSnapshotsFirst(snapshots) })))
          .catch(() => undefined);
        setNavigation((state) => {
          if (payload.status === 'complete' || payload.status === 'partial') {
            if (state.current.projectId === payload.projectId && !state.current.snapshotId && payload.summary) {
              return navigateToRoute(state, { ...state.current, snapshotId: payload.summary.snapshotId, focusTarget: `heading:${state.current.workspace}` });
            }
          }
          return state;
        });
      }
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });
    return () => { active = false; unlisten?.(); };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        if (providerDialog || planningDialog) {
          event.preventDefault();
          return;
        } else if (snapshotMenuOpen) {
          event.preventDefault();
          setSnapshotMenuOpen(false);
          document.querySelector<HTMLElement>('[data-focus-key="snapshot-button"]')?.focus();
        } else if (projectMenuOpen) {
          event.preventDefault();
          setProjectMenuOpen(false);
          projectButtonRef.current?.focus();
        } else if (toolsOpen) {
          event.preventDefault();
          setToolsOpen(false);
          document.querySelector<HTMLElement>('[data-focus-key="tools-button"]')?.focus();
        } else if (notice) {
          event.preventDefault();
          setNotice(null);
        }
        return;
      }

      if ((toolsOpen || projectMenuOpen || snapshotMenuOpen) && ['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
        const menuItems = Array.from(document.querySelectorAll<HTMLElement>('.tools-menu [role="menuitem"], .project-menu [role="menuitem"], .snapshot-menu [role="menuitem"]'));
        const current = menuItems.indexOf(document.activeElement as HTMLElement);
        const nextIndex = event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? menuItems.length - 1
            : current < 0
              ? (event.key === 'ArrowDown' ? 0 : menuItems.length - 1)
              : (current + (event.key === 'ArrowDown' ? 1 : -1) + menuItems.length) % menuItems.length;
        if (menuItems.length > 0) {
          event.preventDefault();
          menuItems[nextIndex]?.focus();
        }
        return;
      }

      if (!(event.metaKey || event.ctrlKey)) return;
      const key = event.key.toLowerCase();
      if (key === '1') {
        event.preventDefault();
        navigationActions.workspace('guide-plan');
      } else if (key === '2') {
        event.preventDefault();
        navigationActions.workspace('atlas');
      } else if (key === '3') {
        event.preventDefault();
        navigationActions.workspace('changes');
      } else if (key === '[') {
        event.preventDefault();
        navigationActions.back();
      } else if (key === ']') {
        event.preventDefault();
        navigationActions.forward();
      } else if (key === 'o') {
        event.preventDefault();
        void openLocalRepository();
      } else if (key === 'k' && navigation.current.snapshotId) {
        event.preventDefault();
        setNavigation((state) => navigateToRoute(state, { ...state.current, workspace: 'atlas', focusTarget: 'atlas-tree-search' }));
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [navigation.current.snapshotId, navigationActions, notice, openLocalRepository, projectMenuOpen, providerDialog, planningDialog, snapshotMenuOpen, toolsOpen]);

  const showComingSoon = (title: string, message: string) => {
    setNotice({ title, message });
    setNotificationsOpen(true);
    setToolsOpen(false);
    setProjectMenuOpen(false);
    setSnapshotMenuOpen(false);
  };

  useEffect(() => {
    const open = () => {
      providerReturnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      setToolsOpen(false); setProjectMenuOpen(false); setSnapshotMenuOpen(false);
      setProviderDialog({ kind: 'settings', section: 'voice' });
    };
    window.addEventListener('code-atlas:voice-connections', open);
    return () => window.removeEventListener('code-atlas:voice-connections', open);
  }, []);

  const openProviderSettings = () => {
    providerReturnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setToolsOpen(false);
    setProjectMenuOpen(false);
    setSnapshotMenuOpen(false);
    setProviderDialog({ kind: 'settings', backend: 'codex' });
  };

  const reviewFileWithProvider = (targetPath: string, selectedPaths?: string[]) => {
    if (!currentProject || !selectedSnapshot) {
      setNotice({ title: 'A snapshot is required', message: 'Capture a repository snapshot before preparing an exact-context request.' });
      return;
    }
    providerReturnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setToolsOpen(false);
    setProviderDialog({
      kind: 'review',
      projectId: currentProject.projectId,
      snapshotId: selectedSnapshot.snapshotId,
      targetPath,
      selectedPaths,
    });
  };

  const explainCodeLensRangeWithProvider = (targetPath: string, range: SourceLineRange) => {
    if (!currentProject || !sourceDocument || !range.sourceSha256) {
      setNotice({ title: 'A verified source range is required', message: 'Open an exact source citation in Files from the selected immutable snapshot before requesting a range explanation.' });
      return;
    }
    providerReturnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setToolsOpen(false);
    setProviderDialog({
      kind: 'code-lens-range',
      projectId: currentProject.projectId,
      snapshotId: sourceDocument.snapshotId,
      target: {
        path: targetPath,
        sourceSha256: range.sourceSha256,
        startLine: range.startLine,
        endLine: range.endLine,
        startByte: range.startByte ?? null,
        endByteExclusive: range.endByteExclusive ?? null,
      },
    });
  };

  const closeProviderDialog = () => {
    const returnFocus = providerReturnFocusRef.current;
    setProviderDialog(null);
    requestAnimationFrame(() => returnFocus?.focus());
  };

  const closeRecoveredPreview = () => {
    const returnFocus = recoveredPreviewReturnFocusRef.current;
    setRecoveredPreviewId(null);
    requestAnimationFrame(() => {
      (returnFocus?.isConnected ? returnFocus : projectButtonRef.current)?.focus();
    });
  };

  const chooseWorkspace = (workspace: Workspace) => {
    navigationActions.workspace(workspace);
    setNotice(null);
    setToolsOpen(false);
    setProjectMenuOpen(false);
  };

  const openCapturedFile = (relativePath: string, sourceSnapshotId?: string, sourceRange?: SourceLineRange) => {
    if (!isNativeRuntime()) {
      setNotice({
        title: 'Open source in the Code Atlas Mac app',
        message: 'The browser preview cannot read local snapshots. Reopen Code Atlas in the Mac app to view this file.',
      });
      return;
    }
    const targetSnapshotId = sourceSnapshotId ?? selectedSnapshot?.snapshotId;
    if (!currentProject || !targetSnapshotId) return;
    const activeElement = document.activeElement;
    const returnFocusTarget = activeElement instanceof HTMLElement
      ? activeElement.dataset.focusKey ?? activeElement.closest<HTMLElement>('[data-focus-key]')?.dataset.focusKey
      : undefined;
    setNavigation((state) => {
      const origin = returnFocusTarget
        ? replaceCurrentRoute(state, { ...state.current, focusTarget: returnFocusTarget })
        : state;
      const destination: WorkspaceRoute = {
        ...origin.current,
        workspace: 'atlas',
        projectId: currentProject.projectId,
        snapshotId: targetSnapshotId,
        relativePath,
        sourceRange,
        sourceRangeIssue: undefined,
        symbolId: undefined,
        atlasSelection: undefined,
        treeSelection: relativePath,
        chapterId: undefined,
        chapterRevisionId: undefined,
        sectionId: undefined,
        playbackPosition: undefined,
        transcriptPosition: undefined,
        readerSnapshotBaselineId: undefined,
        focusTarget: 'heading:atlas',
      };
      return origin.current.workspace === 'atlas'
        && origin.current.snapshotId === targetSnapshotId
        && origin.current.relativePath === relativePath
        ? replaceCurrentRoute(origin, destination)
        : navigateToRoute(origin, destination);
    });
    setNotice(null);
  };

  const openWalkthroughSource = (anchor: ChapterSourceAnchor, context: ReaderRouteContext) => {
    setNavigation((state) => {
      const readerOrigin = {
        ...state.current,
        workspace: 'walkthrough' as const,
        projectId: context.projectId,
        snapshotId: context.snapshotId,
        relativePath: undefined,
        sourceRange: undefined,
        symbolId: undefined,
        chapterId: context.chapterId,
        chapterRevisionId: context.chapterRevisionId,
        sectionId: context.sectionId,
        playbackPosition: context.playbackPosition,
        transcriptPosition: context.transcriptPosition,
        focusTarget: context.focusTarget,
      };
      return navigateToRoute(replaceCurrentRoute(state, readerOrigin), {
        workspace: 'atlas',
        projectId: context.projectId,
        snapshotId: anchor.snapshotId,
        relativePath: anchor.path,
        sourceRange: codeLensRangeFromChapterAnchor(anchor),
        atlasSelection: undefined,
        treeSelection: anchor.path,
        focusTarget: 'heading:atlas',
      });
    });
    setNotice(null);
  };

  const openWalkthroughAtlas = (destination: HotspotDestination, context: ReaderRouteContext) => {
    const atlasSelection = destination.kind === 'atlasFile'
      ? { kind: 'file' as const, path: destination.path }
      : destination.kind === 'atlasSymbol'
        ? { kind: 'symbol' as const, path: destination.path, symbolId: destination.symbolId }
        : destination.kind === 'relationship'
          ? { kind: 'relationship' as const, relationshipId: destination.relationshipId }
          : null;
    if (!atlasSelection) return;
    setNavigation((state) => {
      const readerOrigin = {
        ...state.current,
        workspace: 'walkthrough' as const,
        projectId: context.projectId,
        snapshotId: context.snapshotId,
        chapterId: context.chapterId,
        chapterRevisionId: context.chapterRevisionId,
        sectionId: context.sectionId,
        playbackPosition: context.playbackPosition,
        transcriptPosition: context.transcriptPosition,
        focusTarget: context.focusTarget,
      };
      return navigateToRoute(replaceCurrentRoute(state, readerOrigin), {
        workspace: 'atlas',
        projectId: context.projectId,
        snapshotId: context.snapshotId,
        atlasSelection,
        relativePath: atlasSelection.kind === 'file' || atlasSelection.kind === 'symbol' ? atlasSelection.path : undefined,
        symbolId: atlasSelection.kind === 'symbol' ? atlasSelection.symbolId : undefined,
        treeSelection: atlasSelection.kind === 'relationship'
          ? `relationship:${atlasSelection.relationshipId}`
          : atlasSelection.kind === 'symbol'
            ? `${atlasSelection.path}#${atlasSelection.symbolId}`
            : atlasSelection.path,
        focusTarget: atlasSelection.kind === 'file' ? 'heading:atlas' : 'atlas-destination',
      });
    });
  };

  const selectStudioRevision = (revision: ChapterRevision, sectionId?: string) => {
    setNavigation((state) => navigateToRoute(state, {
      ...state.current,
      chapterId: revision.chapterId,
      chapterRevisionId: revision.revisionId,
      sectionId,
      focusTarget: sectionId ? `chapter-section:${sectionId}` : 'heading:chapter-studio',
    }));
  };

  const openGuideRevision = (revision: ChapterRevision) => {
    const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement.dataset.focusKey : undefined;
    setNavigation((state) => {
      const origin = returnFocus ? replaceCurrentRoute(state, { ...state.current, focusTarget: returnFocus }) : state;
      return navigateToRoute(origin, {
        ...origin.current,
        workspace: 'walkthrough',
        projectId: revision.projectId,
        snapshotId: revision.snapshotId,
        relativePath: undefined,
        sourceRange: undefined,
        chapterId: revision.chapterId,
        chapterRevisionId: revision.revisionId,
        sectionId: undefined,
        playbackPosition: 0,
        transcriptPosition: 0,
        focusTarget: 'heading:walkthrough',
      });
    });
  };

  const reviewGuideRevision = (revision: ChapterRevision) => {
    const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement.dataset.focusKey : undefined;
    setNavigation((state) => {
      const origin = returnFocus ? replaceCurrentRoute(state, { ...state.current, focusTarget: returnFocus }) : state;
      return navigateToRoute(origin, {
        ...origin.current, workspace: 'chapter-studio', projectId: revision.projectId, snapshotId: revision.snapshotId,
        chapterId: revision.chapterId, chapterRevisionId: revision.revisionId, sectionId: undefined, focusTarget: 'heading:chapter-studio',
      });
    });
  };

  const clearStudioRevisionRoute = () => {
    setNavigation((state) => navigateToRoute(state, {
      ...state.current,
      chapterId: undefined,
      chapterRevisionId: undefined,
      sectionId: undefined,
      focusTarget: 'heading:chapter-studio',
    }));
  };

  useEffect(() => {
    const focusTarget = navigation.current.focusTarget;
    if (!focusTarget) return;
    let observer: MutationObserver | null = null;
    let expiry: number | null = null;
    const focusWhenAvailable = (): boolean => {
      const target = Array.from(document.querySelectorAll<HTMLElement>('[data-focus-key]'))
        .find((element) => element.dataset.focusKey === focusTarget);
      if (!target) return false;
      target.focus({ preventScroll: false });
      observer?.disconnect();
      if (expiry !== null) window.clearTimeout(expiry);
      return true;
    };
    if (!focusWhenAvailable()) {
      observer = new MutationObserver(() => { focusWhenAvailable(); });
      observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-focus-key'] });
      expiry = window.setTimeout(() => observer?.disconnect(), 15_000);
    }
    return () => {
      observer?.disconnect();
      if (expiry !== null) window.clearTimeout(expiry);
    };
  }, [navigation.current.focusTarget, navigation.current.workspace, navigation.current.chapterRevisionId, navigation.current.sectionId]);

  const workspaceNotifications: WorkspaceNotification[] = [
    ...questionUpdates.map((outcome): WorkspaceNotification => ({
      id: `question:${outcome.turnId}`, title: outcome.title,
      message: `${currentProject?.displayName ?? 'Repository'} · ${outcome.question}`,
      read: outcome.read, tone: outcome.tone,
      actions: [{ label: 'Open question', movesFocus: true, run: () => {
        if (!currentProject || !selectedSnapshot) return;
        updateWorkspace(currentProject.projectId,selectedSnapshot.snapshotId,(s)=>openWorkspaceQuestion(s,outcome.explorationId,outcome.turnId));
        setNavigation((s)=>navigateToRoute(s,{workspace:'guide-plan',projectId:currentProject.projectId,snapshotId:selectedSnapshot.snapshotId}));
      } }, { label: 'Dismiss', run: () => {
        if (currentProject && selectedSnapshot) updateWorkspace(currentProject.projectId,selectedSnapshot.snapshotId,(s)=>markQuestionOutcome(s,outcome.explorationId,outcome.turnId,outcome.key,'dismissed'));
      } }],
    })),
    ...(runtimeError ? [{ id: 'runtime', title: 'Some app features are unavailable', message: 'Try connecting to the local app again.', tone: 'error' as const,
      actions: [{ label: 'Retry', run: loadRuntime }] }] : []),
    ...(notice ? [{ id: 'workspace-notice', title: notice.title, message: notice.message, tone: notice.tone,
      actions: [...(notice.action ? [{ label: notice.actionLabel ?? 'Continue', run: notice.action }] : []), { label: 'Dismiss', run: () => setNotice(null) }] }] : []),
    ...(interruptedOperationsUnavailable ? [{ id: 'recovery-unavailable', title: 'Recent work could not be loaded', message: 'Your saved explanations are still available.', tone: 'error' as const,
      actions: [{ label: 'Try again', run: () => { void refreshInterruptedOperations(); } }] }] : []),
    ...interruptedOperations.map((operation): WorkspaceNotification => ({
      id: `interrupted:${operation.operationId}`, tone: 'warning',
      title: operation.kind === 'capture' ? 'Repository update interrupted' : 'Explanation interrupted',
      message: operation.kind === 'capture' ? 'You can keep reading the previous version or try the update again.'
        : operation.providerRequest?.repositoryPlan?.question ?? 'Your saved work is preserved. Open it to continue.',
      actions: [{ label: operation.kind === 'capture' ? 'Try update again' : 'Open work', disabled: recoveryBusyId !== null,
        run: () => { void (operation.kind === 'capture' ? retryInterruptedCapture(operation) : reviewInterruptedGeneration(operation)); } },
        { label: 'Dismiss', disabled: recoveryBusyId !== null, run: () => { void discardRecovery(operation); } }],
    })),
    ...(pendingChapterPreviewsUnavailable ? [{ id: 'previews-unavailable', title: 'Saved previews could not be loaded', message: 'Try again to find your unfinished explanations.', tone: 'error' as const,
      actions: [{ label: 'Try again', run: () => { void refreshPendingChapterPreviews(); } }] }] : []),
    ...pendingChapterPreviews.map((preview): WorkspaceNotification => ({
      id: `preview:${preview.generationId}`, title: preview.unavailableReason ? 'An explanation needs attention' : 'An explanation is ready',
      message: `${preview.title}${preview.targetPath ? ` · ${preview.targetPath}` : ''}`, tone: preview.unavailableReason ? 'warning' : 'info',
      actions: [{ label: 'Open explanation', disabled: Boolean(preview.unavailableReason), opensDialog: true, run: () => {
        recoveredPreviewReturnFocusRef.current = document.querySelector<HTMLButtonElement>('[data-focus-key="notifications"]');
        setRecoveredPreviewId(preview.generationId);
      } }, { label: 'Discard preview', run: () => { void discardPendingPreview(preview); } }],
    })),
  ];

  return (
    <div className={`app-window${selectedSnapshot ? ' has-snapshot' : ''}${readiness !== 'ready' ? ' is-restoring' : ''}`}>
      <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">
        {repositoryContextAnnouncement}
      </div>
      <header className="topbar">
        <div className="brand-lockup">
          <div className="brand-mark"><Layers3 size={19} strokeWidth={2.1} /></div>
          <span className="brand-name">code<span>atlas</span></span>
        </div>

        <div className="project-controls">
          <button
            ref={projectButtonRef}
            className="project-button"
            data-focus-key="project-button"
            onClick={() => { setToolsOpen(false); setSnapshotMenuOpen(false); setProjectMenuOpen((open) => !open); }}
            aria-label={currentProject ? `Switch repository. Current repository: ${currentProject.displayName}` : 'Open the repository picker'}
            aria-expanded={projectMenuOpen}
            aria-haspopup="menu"
          >
            <FolderOpen size={16} />
            <span>{projectOpening ? 'Opening repository…' : projectMetadataBusy ? 'Updating repositories…' : currentProject?.displayName ?? (recentsPending ? 'Opening workspace…' : 'Open a local repository')}</span>
            <ChevronDown size={14} className="muted-icon" />
          </button>
          {projectOpening && <button className="icon-button" aria-label="Stop opening repository" title="Stop opening repository" onClick={stopProjectOpening}><X size={16}/></button>}
          {projectMenuOpen && (
            <div className="project-menu" role="menu" aria-label="Local repositories">
              <div className="menu-label">LOCAL REPOSITORIES</div>
              <button
                role="menuitem"
                data-focus-key="action:pick-repository"
                disabled={projectBusy}
                onClick={() => { void openLocalRepository(); }}
              >
                <FolderOpen size={16} /> <span>Choose a local repository…</span>
              </button>
              <div className="menu-rule" />
              <div className="menu-label">RECENT PROJECTS</div>
              {recentProjects.length === 0 ? (
                <p className="recent-empty">Repositories you open on this Mac appear here.</p>
              ) : (
                <div className="recent-menu-list">
                  {recentProjects.map((project) => (
                    <div className="recent-menu-row" key={project.projectId}>
                      <button
                        role="menuitem"
                        data-focus-key={`recent:${project.projectId}`}
                        disabled={projectBusy}
                        onClick={() => { void openRecent(project.projectId); }}
                        aria-label={`Open ${project.displayName} at ${project.repositoryRoot}`}
                      >
                        <GitBranch size={15} />
                        <span className="recent-menu-copy">
                          <strong>{project.displayName}</strong>
                          <small>{project.repositoryRoot}</small>
                          <small>{project.branch ?? 'Detached or unborn HEAD'} · {project.head?.slice(0, 10) ?? 'No commit yet'}</small>
                        </span>
                      </button>
                      <button
                        className="recent-remove-button"
                        role="menuitem"
                        disabled={projectBusy}
                        onClick={() => { void forgetRecent(project); }}
                        aria-label={`Remove ${project.displayName} from Recents; repository files will not be deleted`}
                        title="Remove from Recents"
                      >
                        <X size={14} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
          <span className="header-divider" />
          <div className="snapshot-picker-wrap">
            <button
              className="snapshot-button"
              data-focus-key="snapshot-button"
              disabled={!currentProject || currentSnapshots.length === 0}
              onClick={() => { setProjectMenuOpen(false); setToolsOpen(false); setSnapshotMenuOpen((open) => !open); }}
              aria-label={selectedSnapshot ? `Choose snapshot. Selected ${formatSnapshotDate(selectedSnapshot.createdAtUnixMs)}` : 'No snapshot selected'}
              aria-expanded={snapshotMenuOpen}
              aria-haspopup="menu"
            >
              <GitBranch size={14} />
              <span>{selectedSnapshot ? `${selectedSnapshot.branch ?? 'Detached'} · ${selectedSnapshot.head?.slice(0, 7) ?? selectedSnapshot.snapshotId.slice(-7)}` : currentSnapshots.length ? 'Choose snapshot' : 'No snapshot'}</span>
              {currentSnapshots.length > 0 && <ChevronDown size={13} className="muted-icon" />}
            </button>
            {snapshotMenuOpen && (
              <div className="snapshot-menu" role="menu" aria-label="Repository snapshots">
                <div className="menu-label">IMMUTABLE SNAPSHOTS · {currentSnapshots.length}</div>
                {currentSnapshots.map((summary) => (
                  <button role="menuitem" key={summary.snapshotId} onClick={() => selectSnapshot(summary)} aria-current={summary.snapshotId === navigation.current.snapshotId ? 'true' : undefined}>
                    <HardDrive size={15} />
                    <span className="snapshot-menu-copy">
                      <strong>{formatSnapshotDate(summary.createdAtUnixMs)} · {summary.status === 'partial' ? 'Partial' : 'Complete under policy'}</strong>
                      <small>{summary.branch ?? 'Detached'} · {summary.head?.slice(0, 10) ?? 'Unborn HEAD'}</small>
                      <small>{summary.filesCaptured.toLocaleString()} files · {formatBytes(summary.totalBytes)} · {summary.pathsSkipped.toLocaleString()} skipped</small>
                    </span>
                    {summary.snapshotId === navigation.current.snapshotId && <Check size={14} className="snapshot-menu-check" />}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>


        <div className="topbar-actions">
          <ConnectionHeader onOpen={openProviderSettings} />
          <ThemeControl />
          <NotificationCenter items={workspaceNotifications} open={notificationsOpen} onOpenChange={setNotificationsOpen} />
          <button
            className="search-control"
            disabled={!navigation.current.snapshotId}
            onClick={() => window.dispatchEvent(new Event('code-atlas:search'))}
            aria-label="Search repository files, Command K"
          >
            <Search size={15} />
            <span>Search repository</span>
            <kbd><Command size={10} /> K</kbd>
          </button>
          <div className="tools-wrap">
            <button
              className={`more-tools-button ${toolsOpen ? 'selected' : ''}`}
              onClick={() => { setProjectMenuOpen(false); setSnapshotMenuOpen(false); setToolsOpen((open) => !open); }}
              data-focus-key="tools-button"
              aria-label="Open workspace settings"
              title="Workspace settings"
              aria-expanded={toolsOpen}
              aria-haspopup="menu"
            >
              <Settings2 size={16} />
            </button>
            {toolsOpen && (
              <div className="tools-menu" role="menu" aria-label="More tools">
                <div className="menu-label">WORKSPACE</div>
                <button role="menuitem" onClick={openProviderSettings}><Settings2 size={16} /> <span>Connections</span></button>
                <button role="menuitem" data-focus-key="tools-button" onClick={() => chooseWorkspace('walkthrough')}>
                  <Play size={16} /> <span>Browse accepted guides</span>
                </button>
                <button role="menuitem" data-focus-key="tools-button" onClick={() => chooseWorkspace('chapter-studio')}>
                  <BookOpen size={16} /> <span>Review a draft</span>
                </button>
                <button role="menuitem" onClick={() => { setToolsOpen(false); showComingSoon('Keyboard shortcuts', '⌘1 Guide · ⌘2 Flow · ⌘3 Chat history · ⌘P / ⌘K Search · ⌘L Ask · ⌘I Explain selected code · ⌥← Parent exploration · ⌘= zoom in · ⌘− zoom out (20% steps)'); }}><CircleHelp size={16} /><span>Keyboard shortcuts</span></button>
              </div>
            )}
          </div>
        </div>
      </header>

      {projectMenuOpen && <button className="menu-scrim" aria-label="Close local repositories menu" onClick={() => { setProjectMenuOpen(false); projectButtonRef.current?.focus(); }} />}
      {toolsOpen && <button className="menu-scrim" aria-label="Close project tools" onClick={() => { setToolsOpen(false); document.querySelector<HTMLElement>('[data-focus-key="tools-button"]')?.focus(); }} />}
      {snapshotMenuOpen && <button className="menu-scrim" aria-label="Close snapshot history" onClick={() => { setSnapshotMenuOpen(false); document.querySelector<HTMLElement>('[data-focus-key="snapshot-button"]')?.focus(); }} />}

      <div className="app-body">
        <main className="workspace" id="main-content">
          <div className="workspace-toolbar">
            <div className="breadcrumb"><span>Workspace</span><span className="breadcrumb-slash">/</span><strong>{currentWorkspace === 'atlas' || currentWorkspace === 'code-lens' ? 'Files' : currentWorkspace === 'changes' ? 'Changes' : 'Guide'}</strong></div>
            <div className="toolbar-right">
              <button className="history-button" disabled={!canNavigateBack(navigation)} onClick={navigationActions.back} aria-label="Go back" title="Back · ⌘[">
                <ArrowLeft size={15} />
              </button>
              <button className="history-button" disabled={!canNavigateForward(navigation)} onClick={navigationActions.forward} aria-label="Go forward" title="Forward · ⌘]">
                <ArrowRight size={15} />
              </button>
              <span className="toolbar-separator" />
              <span className="offline-indicator"><span /> Local</span>
            </div>
          </div>

          <section className={`workspace-content view-${currentWorkspace}${readiness !== 'ready' ? ' workspace-restoring' : ''}`} aria-labelledby="workspace-title">
            <div className="workspace-heading">
              <div className="eyebrow"><span className="eyebrow-mark" />{copy.eyebrow}</div>
              <h1 id="workspace-title" tabIndex={-1} data-focus-key={`heading:${currentWorkspace}`}>{copy.title}</h1>
              <p>{copy.subtitle}</p>
            </div>

            {readiness !== 'ready' ? (
              <div className="workspace-restore-state">
                {workspaceUnavailable ? <>
                  <h2>Your saved workspace could not open.</h2>
                  <p>Your saved explanations are still on this Mac.</p>
                  <div className="workspace-restore-actions">
                    {readiness !== 'project-error' && <button className="secondary-button" onClick={() => { setSnapshotHistoryLoad(null); setSnapshotManifestLoad(null); setWorkspaceReload((value) => value + 1); }}>Try again</button>}
                    <button className="secondary-button" onClick={() => { void openLocalRepository(); }}>Open a repository</button>
                  </div>
                </> : <p role="status">{readiness === 'restoring' ? 'Opening your workspace…' : `Opening ${currentProject?.displayName ?? 'your workspace'}…`}</p>}
              </div>
            ) : <WorkspaceFrame manifest={snapshotManifest?.summary.snapshotId === selectedSnapshot?.snapshotId ? snapshotManifest : null}
              projectName={currentProject?.displayName ?? 'Repository'} workspace={currentWorkspace}
              routePath={navigation.current.relativePath} routeRange={navigation.current.sourceRange} routeSelection={navigation.current.atlasSelection}
              onWorkspace={chooseWorkspace} onSettings={openProviderSettings} onReview={reviewGuideRevision} onRead={openGuideRevision}
              onRememberFile={(relativePath) => { if (currentProject && selectedSnapshot) rememberFileLocation(currentProject.projectId,{snapshotId:selectedSnapshot.snapshotId,path:relativePath}); }}
              onCompare={(baseRef,headRef) => {
                setNavigation((state)=>navigateToRoute(state,{...state.current,workspace:'changes',baseRef,headRef,selectedChangePath:undefined}));
                if(currentProject){const projectId=currentProject.projectId;void compareLocalGitRevisions(projectId,baseRef,headRef).then((result)=>setGitComparisonsByProject((s)=>({...s,[projectId]:{baseRef,headRef,result}}))).catch((error: unknown)=>{if(error instanceof Error && error.name==='AbortError')return;setNotice({title:'Comparison unavailable',message:'Choose two local commits and try again.'});});}
              }}>
            {currentWorkspace === 'atlas' && selectedSnapshot && snapshotManifest ? (
              <>
                {currentCapture && currentCapture.snapshotId !== selectedSnapshot.snapshotId && (
                  <SnapshotCaptureBanner
                    progress={currentCapture}
                    onCancel={currentCapture.status === 'running' ? () => { void stopSnapshot(); } : undefined}
                    onRetry={currentCapture.status === 'failed' || currentCapture.status === 'inconsistent' || currentCapture.status === 'cancelled' ? () => { void createSnapshot(); } : undefined}
                    onView={currentCapture.summary ? () => selectSnapshot(currentCapture.summary!) : undefined}
                  />
                )}
                <SnapshotReadyState key={snapshotManifest.summary.snapshotId} manifest={snapshotManifest} atlasSelection={navigation.current.atlasSelection} mapViewport={navigation.current.mapViewport} selectedPath={navigation.current.relativePath ?? null} source={sourceDocument} sourceRange={navigation.current.sourceRange ?? null} sourceRangeIssue={navigation.current.sourceRangeIssue ?? null} sourceLoading={sourceLoading} sourceError={sourceError} onCaptureAgain={() => { void createSnapshot(); }} onOpenFile={openCapturedFile} onRequestExplanation={reviewFileWithProvider} onRequestRangeExplanation={explainCodeLensRangeWithProvider} onReadRevision={openGuideRevision} onReviewRevision={(revision) => setNavigation((state) => navigateToRoute(state, {
                  ...state.current,
                  workspace: 'chapter-studio',
                  projectId: revision.projectId,
                  snapshotId: revision.snapshotId,
                  chapterId: revision.chapterId,
                  chapterRevisionId: revision.revisionId,
                  sectionId: undefined,
                  focusTarget: 'heading:chapter-studio',
                }))} onAtlasSelection={(selection, treeSelection, mapViewport) => {
                  setNavigation((state) => replaceCurrentRoute(state, {
                    ...state.current,
                    atlasSelection: undefined,
                    relativePath: selection && (selection.kind === 'file' || selection.kind === 'symbol') ? selection.path : undefined,
                    sourceRange: undefined,
                    sourceRangeIssue: undefined,
                    symbolId: selection?.kind === 'symbol' ? selection.symbolId : undefined,
                    treeSelection,
                    mapViewport,
                  }));
                }} onAtlasViewportChange={(mapViewport) => {
                  setNavigation((state) => replaceCurrentRoute(state, { ...state.current, mapViewport }));
                }} />
              </>
            ) : currentWorkspace === 'atlas' && selectedSnapshot ? (
              <div className="empty-workspace snapshot-loading"><HardDrive size={18} /><span>Opening the selected immutable snapshot…</span></div>
            ) : currentWorkspace === 'atlas' && currentCapture?.status === 'running' ? (
              <SnapshotCaptureState progress={currentCapture} onCancel={() => { void stopSnapshot(); }} />
            ) : currentWorkspace === 'atlas' && (currentCapture?.status === 'failed' || currentCapture?.status === 'inconsistent' || currentCapture?.status === 'cancelled') ? (
              <SnapshotCaptureResult progress={currentCapture} onRetry={() => { void createSnapshot(); }} />
            ) : currentWorkspace === 'atlas' ? (
              <AtlasEmptyState
                project={currentProject}
                onOpen={() => { void openLocalRepository(); }}
                onCreateSnapshot={() => { void createSnapshot(); }}
                busy={snapshotBusy}
                limits={currentCapture?.limits ?? captureLimits}
              />
            ) : currentWorkspace === 'code-lens' ? (
              <CodeLensEmptyState
                projectId={navigation.current.projectId}
                snapshotId={navigation.current.snapshotId}
                manifest={snapshotManifest?.summary.snapshotId === navigation.current.snapshotId ? snapshotManifest : null}
                onGoAtlas={() => navigation.back.at(-1)?.workspace === 'atlas' ? navigationActions.back() : chooseWorkspace('atlas')}
                onGoWalkthrough={navigation.back.at(-1)?.workspace === 'walkthrough' ? navigationActions.back : undefined}
                onOpenFile={openCapturedFile}
                onRequestExplanation={reviewFileWithProvider}
                onRequestRangeExplanation={explainCodeLensRangeWithProvider}
                source={sourceDocument}
                sourceRange={navigation.current.sourceRange ?? null}
                sourceRangeIssue={navigation.current.sourceRangeIssue ?? null}
                loading={sourceLoading}
                error={sourceError}
                selectedPath={navigation.current.relativePath ?? null}
              />
            ) : currentWorkspace === 'walkthrough' ? (
              <Suspense fallback={<div className="empty-workspace snapshot-loading" role="status"><HardDrive size={18} /><span>Loading Walkthrough…</span></div>}>
                {currentProject ? (
                  <WalkthroughScreen
                    key={currentProject.projectId}
                    projectId={currentProject.projectId}
                    selectedSnapshotId={navigation.current.snapshotId}
                    route={navigation.current}
                    onReplaceRoute={(route) => setNavigation((state) => replaceCurrentRoute(state, route))}
                    onNavigateRoute={(route) => setNavigation((state) => navigateToRoute(state, route))}
                    onOpenSource={openWalkthroughSource}
                    onOpenAtlas={openWalkthroughAtlas}
                    onGoPlan={() => chooseWorkspace('guide-plan')}
                    onOpenProviderSettings={openProviderSettings}
                  />
                ) : <WalkthroughEmptyState onGoPlan={() => chooseWorkspace('guide-plan')} />}
              </Suspense>
            ) : currentWorkspace === 'guide-plan' && currentProject && selectedSnapshot && snapshotManifest ? (
              <GuidePlanScreen
                key={`${currentProject.projectId}:${selectedSnapshot.snapshotId}`}
                projectId={currentProject.projectId}
                projectName={currentProject.displayName}
                manifest={snapshotManifest}
                rows={buildInventory(snapshotManifest)}
                returnFocusTarget={navigation.current.focusTarget}
                onPlan={openPlanningDialog}
                onGeneratePlannedChapter={(scope) => openPlanningDialog({ kind: 'planned-chapter',
                  projectId: currentProject.projectId, snapshotId: scope.snapshotId,
                  question: scope.question, selectedPaths: scope.selectedPaths, plannedChapter: scope })}
                onGoAtlas={() => chooseWorkspace('atlas')}
                onGoChanges={() => chooseWorkspace('changes')}
                onGoStudio={() => chooseWorkspace('chapter-studio')}
                onGenerateFile={reviewFileWithProvider}
                onGenerateOverview={() => setProviderDialog({
                  kind: 'overview',
                  projectId: currentProject.projectId,
                  snapshotId: selectedSnapshot.snapshotId,
                })}
                onReadRevision={openGuideRevision}
                onReviewRevision={reviewGuideRevision}
                onOpenFile={openCapturedFile}
              />
            ) : currentWorkspace === 'guide-plan' && selectedSnapshot ? (
              <div className="empty-workspace snapshot-loading" role="status"><HardDrive size={18} /><span>Opening the selected snapshot for Guide Plan…</span></div>
            ) : currentWorkspace === 'guide-plan' ? (
              <WorkflowEmptyState kind="plan" onGoAtlas={() => chooseWorkspace('atlas')} />
            ) : currentWorkspace === 'chapter-studio' && currentProject && selectedSnapshot && snapshotManifest ? (
              <ChapterStudioScreen
                key={`${currentProject.projectId}:${selectedSnapshot.snapshotId}`}
                projectId={currentProject.projectId}
                manifest={snapshotManifest}
                rows={buildInventory(snapshotManifest)}
                routeChapterId={navigation.current.chapterId ?? null}
                routeRevisionId={navigation.current.chapterRevisionId ?? null}
                routeSectionId={navigation.current.sectionId ?? null}
                onSelectRevision={selectStudioRevision}
                onClearRevisionRoute={clearStudioRevisionRoute}
                onOpenFile={openCapturedFile}
                onGoPlan={() => chooseWorkspace('guide-plan')}
                onRegenerateSection={(revision, section, selectedPaths) => {
                  const targetPath = revision.target.scope.kind === 'file' ? revision.target.scope.path : null;
                  if (!targetPath || !currentProject) return;
                  setProviderDialog({
                    kind: 'section-update',
                    projectId: currentProject.projectId,
                    snapshotId: selectedSnapshot.snapshotId,
                    targetPath,
                    selectedPaths,
                    baseRevision: revision,
                    sectionUpdate: {
                      chapterRevisionId: revision.revisionId,
                      sectionId: section.sectionId,
                      sectionRevisionId: section.sectionRevisionId,
                    },
                  });
                }}
                onGenerateOverview={() => setProviderDialog({
                  kind: 'overview',
                  projectId: currentProject.projectId,
                  snapshotId: selectedSnapshot.snapshotId,
                })}
                onGenerate={(targetPath) => setProviderDialog({
                  kind: 'review',
                  projectId: currentProject.projectId,
                  snapshotId: selectedSnapshot.snapshotId,
                  targetPath,
                })}
              />
            ) : currentWorkspace === 'chapter-studio' && selectedSnapshot ? (
              <div className="empty-workspace snapshot-loading" role="status"><HardDrive size={18} /><span>Opening the selected snapshot for Chapter Studio…</span></div>
            ) : currentWorkspace === 'changes' ? (
              <Suspense fallback={<div className="empty-workspace snapshot-loading" role="status"><HardDrive size={18} /><span>Loading Changes…</span></div>}>
                <GitChangesScreen compact
                  projectId={currentProject?.projectId ?? null}
                  baseRef={navigation.current.baseRef ?? ''}
                  headRef={navigation.current.headRef ?? ''}
                  selectedChangePath={navigation.current.selectedChangePath}
                  comparison={currentProject ? gitComparisonsByProject[currentProject.projectId] ?? null : null}
                  onRefsChange={(baseRef, headRef) => setNavigation((state) => replaceCurrentRoute(state, {
                    ...state.current,
                    baseRef,
                    headRef,
                    selectedChangePath: undefined,
                    focusTarget: undefined,
                  }))}
                  onSelectChange={(selectedChangePath) => setNavigation((state) => replaceCurrentRoute(state, {
                    ...state.current,
                    selectedChangePath,
                  }))}
                  onComparison={(projectId, baseRef, headRef, result) => setGitComparisonsByProject((state) => ({
                    ...state,
                    [projectId]: { baseRef, headRef, result },
                  }))}
                  onOpenRepository={() => { void openLocalRepository(); }}
                  onOpenProviderSettings={openProviderSettings}
                  recovery={recoveredGitRequest}
                  onRecoveryReviewReady={completeRecoveredProviderReview}
                />
              </Suspense>
            ) : (
              <WorkflowEmptyState kind="studio" onGoPlan={() => chooseWorkspace('guide-plan')} />
            )}

            </WorkspaceFrame>}
            <footer className="workspace-footer">
              <span>{currentProject?.displayName ?? 'Code Atlas'}</span>
              <span>{currentWorkspace === 'changes' ? 'Local changes' : selectedSnapshot?.branch ?? ''}</span>
            </footer>
          </section>
        </main>
      </div>
      {planningDialog && <RepositoryPlanningDialog
        key={JSON.stringify([planningDialog.kind, planningDialog.projectId, planningDialog.snapshotId, planningDialog.question, planningDialog.selectedPaths, planningDialog.kind === 'planned-chapter' ? planningDialog.plannedChapter.target : null, planningDialog.recoveryOperationId])}
        scope={planningDialog} native={runtime?.native ?? isNativeRuntime()} onClose={closePlanningDialog}
        onRecoveryResultReady={completeRecoveredProviderReview}
        onOpenPendingPreview={(generationId) => { closePlanningDialog(); setRecoveredPreviewId(generationId); }} />}
      {providerDialog && (
        <ProviderDialog
          key={providerDialog.kind === 'settings' ? 'settings' : `${providerDialog.kind}:${providerDialog.projectId}:${providerDialog.snapshotId}:${providerDialog.kind === 'review' ? providerDialog.targetPath : providerDialog.kind === 'section-update' ? providerDialog.sectionUpdate.sectionId : providerDialog.kind === 'code-lens-range' ? `${providerDialog.target.path}:${providerDialog.target.startLine}:${providerDialog.target.endLine}:${providerDialog.target.startByte ?? ''}:${providerDialog.target.endByteExclusive ?? ''}` : ''}:${'recoveryOperationId' in providerDialog ? providerDialog.recoveryOperationId ?? '' : ''}`}
          mode={providerDialog}
          native={runtime?.native ?? isNativeRuntime()}
          onClose={closeProviderDialog}
          onRecoveryReviewReady={completeRecoveredProviderReview}
          onOpenPendingPreview={(generationId) => { closeProviderDialog(); setRecoveredPreviewId(generationId); }}
          onOpenCodeLensCitation={(citation) => {
            closeProviderDialog();
            openCapturedFile(citation.path, citation.snapshotId, {
              startLine: citation.startLine,
              endLine: citation.endLine,
              sourceSha256: citation.sourceSha256,
              startByte: citation.startByte,
              endByteExclusive: citation.endByteExclusive,
            });
          }}
        />
      )}
      {recoveredPreviewId && (
        <RecoveredGeneratedPreviewDialog
          key={recoveredPreviewId}
          generationId={recoveredPreviewId}
          onClose={closeRecoveredPreview}
          onSaved={(revision) => {
            setPendingChapterPreviews((current) => current.filter((preview) => preview.generationId !== recoveredPreviewId));
            closeRecoveredPreview();
            window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision } }));
            window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
          }}
          onDiscarded={() => {
            setPendingChapterPreviews((current) => current.filter((preview) => preview.generationId !== recoveredPreviewId));
            closeRecoveredPreview();
            window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
          }}
          onRecoveryChanged={() => { void refreshPendingChapterPreviews(); }}
        />
      )}
    </div>
  );
}

const noProviderConsent: ProviderConsent = {
  privacyAndDataUseAcknowledged: false,
  billingUncertaintyAcknowledged: false,
  adultSupportedRegionAcknowledged: false,
};

interface ProviderDialogProps {
  mode: ProviderDialogMode;
  native: boolean;
  onClose: () => void;
  onRecoveryReviewReady: (operationId: string) => void;
  onOpenCodeLensCitation: (citation: CodeLensExplanationCitation) => void;
  onOpenPendingPreview: (generationId: string) => void;
}

function ProviderDialog(props: ProviderDialogProps) {
  const agentOperation = props.mode.kind === 'settings' || props.mode.kind === 'review' || props.mode.kind === 'overview' || props.mode.kind === 'section-update' || props.mode.kind === 'code-lens-range';
  const handoffOperation = props.mode.kind === 'settings' || props.mode.kind === 'review' || props.mode.kind === 'overview';
  const [backend, setBackend] = useState<'choose' | GenerationBackend>(() => {
    if (props.mode.recoveryBackend === 'codex' && agentOperation) return 'codex';
    if (props.mode.recoveryBackend === 'claude-agent' && agentOperation) return 'claude-setup';
    if (props.mode.kind === 'settings' && props.mode.backend) return props.mode.backend;
    const saved = readGenerationBackend();
    return saved === 'api' || agentOperation && saved !== 'handoff' || handoffOperation ? saved : 'choose';
  });
  const [handoffAgent, setHandoffAgent] = useState<'claude-code' | 'antigravity'>(() => { try { return localStorage.getItem('code-atlas:handoff-agent') === 'antigravity' ? 'antigravity' : 'claude-code'; } catch { return 'claude-code'; } });
  useEffect(() => { try { localStorage.setItem('code-atlas:handoff-agent', handoffAgent); } catch { /* Optional. */ } }, [handoffAgent]);
  const chooserRef = useRef<HTMLElement>(null);
  useModalFocusTrap(chooserRef);
  useEffect(() => {
    if (backend !== 'choose') return;
    chooserRef.current?.focus();
    const close = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); props.onClose(); }
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [backend, props.onClose]);
  const choose = (next: GenerationBackend) => { rememberGenerationBackend(next); setBackend(next); };
  if (props.mode.kind === 'settings' && props.mode.backend !== 'api') return <ConnectionsDialog native={props.native} initialSection={props.mode.section} onClose={props.onClose}/>;
  if (backend === 'api') return <DirectApiDialog {...props} onChooseBackend={() => setBackend('choose')} />;
  if (backend === 'claude-setup') return <ClaudeConnectionDialog native={props.native}
    reviewMode={props.mode.kind === 'settings' ? 'settings' : props.mode}
    onClose={props.onClose} onChooseBackend={() => setBackend('choose')}
    onRecoveryResultReady={props.onRecoveryReviewReady}
    onOpenPendingPreview={props.onOpenPendingPreview}
    onOpenCodeLensCitation={props.onOpenCodeLensCitation} />;
  if (backend === 'handoff' && (props.mode.kind === 'settings' || props.mode.kind === 'review' || props.mode.kind === 'overview')) {
    return <AgentHandoffDialog mode={props.mode.kind === 'settings' ? 'settings' : props.mode} native={props.native}
      initialAgent={handoffAgent} onClose={props.onClose} onChooseBackend={() => setBackend('choose')} />;
  }
  if (backend === 'codex' && (props.mode.kind === 'settings' || props.mode.kind === 'review' || props.mode.kind === 'overview' || props.mode.kind === 'section-update' || props.mode.kind === 'code-lens-range')) {
    const selectedMode = props.mode;
    const mode: CodexChapterDialogMode = selectedMode.kind === 'settings' ? 'settings' : selectedMode.kind === 'section-update'
      ? { ...selectedMode, sectionTitle: selectedMode.baseRevision.sections.find((section) => section.sectionId === selectedMode.sectionUpdate.sectionId)?.title }
      : selectedMode;
    return <CodexContextDialog mode={mode}
      native={props.native} onClose={props.onClose} onChooseBackend={() => setBackend('choose')}
      onOpenPendingPreview={props.onOpenPendingPreview}
      onRecoveryReviewReady={props.onRecoveryReviewReady} onOpenCodeLensCitation={props.onOpenCodeLensCitation} />;
  }
  return <div className="provider-overlay">
    <section className="provider-dialog" ref={chooserRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="backend-choice-title">
      <header className="provider-dialog-header">
        <div><div className="step-pill"><span>GENERATION</span> YOUR CHOICE</div>
          <h2 id="backend-choice-title">Choose how to generate</h2>
          <p>Use a coding agent you already have. Direct APIs are an optional alternative. Review the selected source before generation.</p></div>
        <button className="icon-button" onClick={props.onClose} aria-label="Close backend chooser"><X size={17} /></button>
      </header>
      <div className="provider-dialog-content">
        <section className="provider-review-card"><h3>Codex · ChatGPT account</h3>
          <p>File chapters, selected-source overviews, one-section repairs and exact code selections using the official Codex runtime. Choose a model and reasoning level from its catalog. Your plan limits apply; this path needs no API key.</p>
          <button className="primary-button" disabled={!agentOperation} onClick={() => choose('codex')}>Use Codex</button>
          {!agentOperation && <p role="status">Codex support for this operation is pending. The current source adapter covers file chapters, overviews, one-section repairs and exact code selections.</p>}
        </section>
        <section className="provider-review-card"><h3>Claude Agent · setup in progress</h3>
          <p>Check an installed Claude CLI and its catalog, configure supported settings, and preview the selected request locally. Automatic generation is still being implemented.</p>
          <button className="secondary-button" onClick={() => choose('claude-setup')}>Check Claude installation</button>
          <h4>Manual Claude Code handoff</h4>
          <p>Copy a reviewed task into Claude Code, then import its JSON explanation for local source validation. Configure model, reasoning and approvals in Claude Code. Code Atlas needs no API key for this workflow.</p>
          <button className="primary-button" disabled={!handoffOperation} onClick={() => { setHandoffAgent('claude-code'); choose('handoff'); }}>Use Claude Code handoff</button>
          <p className="provider-small-note">Automatic Claude Agent integration is a separate pending task; no Claude login or automatic generation is offered here.</p>
        </section>
        <section className="provider-review-card"><h3>Antigravity or another agent · manual handoff</h3>
          <p>Use the same reviewed task and validated import with your chosen agent. Its own account rules and permissions apply. Code Atlas does not manage or verify that external run.</p>
          <button className="secondary-button" disabled={!handoffOperation} onClick={() => { setHandoffAgent('antigravity'); choose('handoff'); }}>Choose a manual agent handoff</button>
        </section>
        {!handoffOperation && <p role="status">Manual agent handoffs currently cover file chapters and selected-source overviews. Automatic Claude integration remains pending.</p>}
        <section className="provider-review-card"><h3>Direct API</h3>
          <p>Google Gemini, OpenAI or Anthropic with your own API key. Review the selected model’s terms and cost before sending.</p>
          <button className="secondary-button" onClick={() => choose('api')}>Choose a direct API</button>
        </section>
      </div>
    </section>
  </div>;
}

function DirectApiDialog({ mode, native, onClose, onRecoveryReviewReady, onOpenCodeLensCitation, onChooseBackend }: ProviderDialogProps & { onChooseBackend: () => void }) {
  const [providerCatalog, setProviderCatalog] = useState<ProviderDisclosure[]>([]);
  const [providerId, setProviderId] = useState('');
  const [disclosure, setDisclosure] = useState<ProviderDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCommandResult<ProviderCredentialStatus> | null>(null);
  const [informationLoading, setInformationLoading] = useState(native);
  const [informationError, setInformationError] = useState<string | null>(null);
  const [keyDraft, setKeyDraft] = useState('');
  const [keyBusy, setKeyBusy] = useState(false);
  const [keyMessage, setKeyMessage] = useState<string | null>(null);
  const [modelId, setModelId] = useState('');
  const [requestReview, setRequestReview] = useState<ProviderRequestReview | null>(null);
  const [snapshotManifest, setSnapshotManifest] = useState<SnapshotManifest | null>(null);
  const [snapshotManifestError, setSnapshotManifestError] = useState<string | null>(null);
  const [snapshotManifestLoading, setSnapshotManifestLoading] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<string[]>(mode.kind === 'review'
    ? [...new Set([mode.targetPath, ...(mode.selectedPaths ?? [])])]
    : mode.kind === 'section-update' ? mode.selectedPaths : mode.kind === 'overview' ? mode.selectedPaths ?? [] : []);
  const [contextQuery, setContextQuery] = useState('');
  const [requestBuilding, setRequestBuilding] = useState(false);
  const [requestBuildError, setRequestBuildError] = useState<ProviderFailure | null>(null);
  const [requestError, setRequestError] = useState<ProviderFailure | null>(null);
  const [sendResult, setSendResult] = useState<ChapterDraftPreview | null>(null);
  const [codeLensResult, setCodeLensResult] = useState<CodeLensExplanationPreview | null>(null);
  const [saveBusy, setSaveBusy] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savedRevision, setSavedRevision] = useState<ChapterRevision | null>(null);
  const [consent, setConsent] = useState<ProviderConsent>(noProviderConsent);
  const [sending, setSending] = useState(false);
  const [cancelConfirmation, setCancelConfirmation] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [cancelStatus, setCancelStatus] = useState<string | null>(null);
  const cancelRequested = useRef(false);
  const dialogMounted = useRef(true);
  const providerDialogRef = useRef<HTMLElement>(null);
  const requestSummaryRef = useRef<HTMLDivElement>(null);
  useModalFocusTrap(providerDialogRef);
  const [reviewGeneration, setReviewGeneration] = useState(0);
  const [unavailableModel, setUnavailableModel] = useState<string | null>(null);
  const credentialInfo = credential?.status === 'ok' ? credential.data : null;
  const credentialError = credential?.status === 'error' ? credential.error.message : null;
  const hasApiKey = credentialInfo?.apiKeyConfigured ?? false;
  const reviewMode = mode.kind !== 'settings';
  const overviewMode = mode.kind === 'overview';
  const sectionUpdateMode = mode.kind === 'section-update';
  const codeLensRangeMode = mode.kind === 'code-lens-range';
  const recoveryOperationId = !mode.recoveryBackend && 'recoveryOperationId' in mode ? mode.recoveryOperationId : undefined;
  const availableContextFiles = useMemo(() => (snapshotManifest?.files ?? [])
    .filter((file) => file.kind === 'file' && (file.contentKind === 'text' || file.contentKind === 'unknown')
      && !file.secretLike && file.sizeBytes <= 250_000)
    .filter((file) => file.path.toLowerCase().includes(contextQuery.trim().toLowerCase()))
    .sort((left, right) => left.path.localeCompare(right.path)), [contextQuery, snapshotManifest]);

  useEffect(() => {
    dialogMounted.current = true;
    providerDialogRef.current?.focus();
    return () => { dialogMounted.current = false; };
  }, []);

  useEffect(() => {
    if (requestReview) requestSummaryRef.current?.focus();
  }, [requestReview]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      if (saveBusy || keyBusy) return;
      if (sending) {
        setCancelConfirmation(true);
        return;
      }
      if (requestReview) void discardProviderRequestReview(requestReview.reviewId);
      onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [keyBusy, onClose, requestReview, saveBusy, sending]);

  useEffect(() => {
    if (!reviewMode || !native) return;
    let active = true;
    setSnapshotManifestLoading(true);
    setSnapshotManifestError(null);
    void loadSnapshotManifest(mode.projectId, mode.snapshotId)
      .then((manifest) => { if (active) setSnapshotManifest(manifest); })
      .catch((error) => { if (active) setSnapshotManifestError(errorMessageOr(error, 'The captured file list could not be loaded. Select source files again after reopening the snapshot.')); })
      .finally(() => { if (active) setSnapshotManifestLoading(false); });
    return () => { active = false; };
  }, [mode, native, reviewMode]);

  useEffect(() => {
    if (!native) {
      setInformationLoading(false);
      return;
    }
    let active = true;
    setInformationLoading(true);
    setInformationError(null);
    getProviderCatalog()
      .then((catalog) => {
        if (!active) return;
        setProviderCatalog(catalog);
      })
      .catch(() => {
        if (active) setInformationError('Code Atlas could not load local provider settings. No model request was sent.');
      })
      .finally(() => { if (active) setInformationLoading(false); });
    return () => { active = false; };
  }, [native]);

  useEffect(() => {
    if (!native || !providerId) {
      setDisclosure(null);
      setCredential(null);
      return;
    }
    const selected = providerCatalog.find((provider) => provider.providerId === providerId) ?? null;
    setDisclosure(selected);
    // Invalidate any earlier status read while Keychain is being changed.
    if (keyBusy) return;
    setCredential(null);
    if (!selected) return;
    let active = true;
    void getProviderCredentialStatus(providerId)
      .then((status) => { if (active) setCredential(status); })
      .catch(() => { if (active) setCredential(null); });
    return () => { active = false; };
  }, [keyBusy, native, providerCatalog, providerId]);

  useEffect(() => {
    if (!reviewMode || !native || !disclosure || !modelId) return;
    if (mode.kind === 'overview' && selectedPaths.length === 0) {
      setRequestReview(null);
      setRequestBuildError(null);
      setRequestBuilding(false);
      setConsent(noProviderConsent);
      return;
    }
    let active = true;
    let createdReviewId: string | null = null;
    setRequestBuilding(true);
    setRequestBuildError(null);
    setRequestReview(null);
    setConsent(noProviderConsent);
    setSendResult(null);
    setCodeLensResult(null);
    setSavedRevision(null);
    setSaveError(null);
    const requestBuild = mode.kind === 'code-lens-range'
      ? buildCodeLensProviderRequestReview(providerId, {
        projectId: mode.projectId,
        snapshotId: mode.snapshotId,
        target: mode.target,
        modelId,
      })
      : buildProviderRequestReview(providerId, {
        projectId: mode.projectId,
        snapshotId: mode.snapshotId,
        targetPath: mode.kind === 'review' || mode.kind === 'section-update' ? mode.targetPath : null,
        selectedPaths,
        modelId,
        sectionUpdate: mode.kind === 'section-update' ? mode.sectionUpdate : null,
      });
    void requestBuild.then((result) => {
      if (result.status === 'error') {
        if (active) setRequestBuildError(result.error);
        return;
      }
      createdReviewId = result.data.reviewId;
      if (active) {
        setRequestReview(result.data);
        if (recoveryOperationId) onRecoveryReviewReady(recoveryOperationId);
      }
      else void discardProviderRequestReview(result.data.reviewId);
    }).catch(() => {
      if (active) setRequestBuildError({
        kind: 'invalidReview',
        message: 'Code Atlas could not build a local request preview. No model request was sent.',
      });
    }).finally(() => { if (active) setRequestBuilding(false); });
    return () => {
      active = false;
      if (createdReviewId) void discardProviderRequestReview(createdReviewId);
    };
  }, [disclosure, mode, modelId, native, onRecoveryReviewReady, providerId, reviewGeneration, reviewMode, selectedPaths]);

  const updateConsent = (key: keyof ProviderConsent, checked: boolean) => {
    setConsent((current) => ({ ...current, [key]: checked }));
    setRequestError(null);
  };

  const handleSaveKey = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!providerId || keyBusy) return;
    setKeyMessage(null);
    setKeyBusy(true);
    try {
      const result = await storeProviderApiKey(providerId, keyDraft);
      setCredential(result);
      window.dispatchEvent(new Event('code-atlas:provider-credentials-changed'));
      if (result.status === 'ok' && result.data.apiKeyConfigured) {
        setKeyDraft('');
        setKeyMessage('Key saved to macOS Keychain. Code Atlas does not retain it in app settings.');
      } else if (result.status === 'error') {
        setKeyMessage(result.error.message);
      }
    } catch {
      setKeyMessage('Code Atlas could not save the key to macOS Keychain. It was not written to a file.');
    } finally {
      setKeyBusy(false);
    }
  };

  const handleRemoveKey = async () => {
    if (!providerId || keyBusy) return;
    setKeyMessage(null);
    setKeyBusy(true);
    try {
      const result = await removeProviderApiKey(providerId);
      setCredential(result);
      window.dispatchEvent(new Event('code-atlas:provider-credentials-changed'));
      setKeyMessage(result.status === 'ok' ? result.data.explanation : result.error.message);
    } catch {
      setKeyMessage('Code Atlas could not remove the key from macOS Keychain.');
    } finally {
      setKeyBusy(false);
    }
  };

  const handleModelChange = (nextModelId: string) => {
    if (sending || saveBusy || sendResult || codeLensResult) return;
    if (requestReview) void discardProviderRequestReview(requestReview.reviewId);
    setConsent(noProviderConsent);
    setRequestReview(null);
    setSendResult(null);
    setCodeLensResult(null);
    setUnavailableModel(null);
    setRequestError(null);
    setModelId(nextModelId);
  };

  const handleProviderChange = (nextProviderId: string) => {
    if (keyBusy || sending || saveBusy || sendResult || codeLensResult) return;
    if (requestReview) void discardProviderRequestReview(requestReview.reviewId);
    setConsent(noProviderConsent);
    setRequestReview(null);
    setSendResult(null);
    setCodeLensResult(null);
    setUnavailableModel(null);
    setRequestError(null);
    setKeyDraft('');
    setKeyMessage(null);
    setCredential(null);
    setModelId('');
    setProviderId(nextProviderId);
  };

  const handleContextFileChange = (path: string, checked: boolean) => {
    if (sending || saveBusy || sendResult || codeLensResult) return;
    setRequestError(null);
    setSelectedPaths((current) => {
      if (checked) {
        if (current.includes(path) || current.length >= 12) return current;
        return [...current, path];
      }
      if (mode.kind === 'review' && path === mode.targetPath) return current;
      return current.filter((selected) => selected !== path);
    });
  };

  const handleSend = async () => {
    if (!providerId || !modelId || !requestReview || sending || saveBusy || sendResult || codeLensResult || !hasApiKey || !consent.privacyAndDataUseAcknowledged
      || !consent.billingUncertaintyAcknowledged || !consent.adultSupportedRegionAcknowledged
      || unavailableModel === modelId) return;
    setSending(true);
    cancelRequested.current = false;
    setCancelConfirmation(false);
    setCancelStatus(null);
    setRequestError(null);
    try {
      if (mode.kind === 'code-lens-range') {
        const result = await sendCodeLensProviderRequest({
          reviewId: requestReview.reviewId,
          fingerprint: requestReview.fingerprint,
          consent,
        });
        if (result.status === 'ok') {
          if (!dialogMounted.current) return;
          if (cancelRequested.current) {
            setRequestError({
              kind: 'requestCancelled',
              message: `Cancellation was requested after ${disclosure?.displayName ?? 'the selected provider'} had already returned. The transient explanation was discarded. The provider has already received the selected source.`,
            });
            setRequestReview(null);
            setConsent(noProviderConsent);
            setReviewGeneration((generation) => generation + 1);
            return;
          }
          setCodeLensResult(result.data);
          return;
        }
        setRequestError(result.error);
        if (result.error.kind === 'unavailableModel') setUnavailableModel(modelId);
        if (!['missingApiKey', 'keychainUnavailable', 'keychainOperationFailed', 'consentRequired'].includes(result.error.kind)) {
          setRequestReview(null);
          setConsent(noProviderConsent);
          setReviewGeneration((generation) => generation + 1);
        }
        return;
      }
      const result = await sendProviderRequest({
        reviewId: requestReview.reviewId,
        fingerprint: requestReview.fingerprint,
        consent,
      });
      if (result.status === 'ok') {
        if (!dialogMounted.current) {
          await discardGeneratedChapterDraft(result.data.generationId);
          return;
        }
        if (cancelRequested.current) {
          await discardGeneratedChapterDraft(result.data.generationId);
          setRequestError({
            kind: 'requestCancelled',
            message: `Cancellation was requested after ${disclosure?.displayName ?? 'the selected provider'} had already returned. The response was discarded and not accepted. The provider has already received the selected source.`,
          });
          setRequestReview(null);
          setConsent(noProviderConsent);
          setReviewGeneration((generation) => generation + 1);
          return;
        }
        setSendResult(result.data);
        window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
        return;
      }
      setRequestError(result.error);
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
      if (result.error.kind === 'unavailableModel') setUnavailableModel(modelId);
      if (!['missingApiKey', 'keychainUnavailable', 'keychainOperationFailed', 'consentRequired'].includes(result.error.kind)) {
        setRequestReview(null);
        setConsent(noProviderConsent);
        setReviewGeneration((generation) => generation + 1);
      }
    } catch {
      setRequestError({
        kind: 'networkUnavailable',
        message: 'The send did not return a result. Treat it as used, inspect the refreshed fingerprint, and confirm a new request before retrying.',
      });
      setRequestReview(null);
      setConsent(noProviderConsent);
      setReviewGeneration((generation) => generation + 1);
    } finally {
      setSending(false);
      setCancelConfirmation(false);
      setCancelBusy(false);
    }
  };

  const handleConfirmCancel = async () => {
    if (!requestReview || !sending || cancelBusy) return;
    cancelRequested.current = true;
    setCancelBusy(true);
    setCancelConfirmation(false);
    setCancelStatus(null);
    try {
      const stopped = await cancelProviderRequest(requestReview.reviewId);
      setCancelStatus(stopped
        ? `Cancellation reached the local request. ${disclosure?.displayName ?? 'The provider'} may already have received the selected source; no response will be accepted.`
        : 'The request had already finished locally. Its response will be discarded because you chose Cancel.');
    } catch {
      setCancelStatus(`The app could not confirm a local abort. ${disclosure?.displayName ?? 'The provider'} may already have received the selected source; any returned response will be discarded.`);
    } finally {
      setCancelBusy(false);
    }
  };

  const handleClose = () => {
    if (keyBusy || sending || saveBusy) return;
    if (requestReview) void discardProviderRequestReview(requestReview.reviewId);
    onClose();
  };

  const handleSaveDraft = async () => {
    if (!sendResult || saveBusy || savedRevision) return;
    setSaveBusy(true);
    setSaveError(null);
    try {
      const revision = await saveGeneratedChapterDraft(sendResult.generationId);
      setSavedRevision(revision);
      window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision } }));
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    } catch (error) {
      // A lost command reply can occur after the immutable revision was written.
      const existing = await listChapterRevisions(sendResult.revision.projectId, sendResult.revision.snapshotId)
        .then((revisions) => revisions.find((revision) => revision.revisionId === sendResult.revision.revisionId))
        .catch(() => undefined);
      if (existing) {
        await discardGeneratedChapterDraft(sendResult.generationId).catch(() => {});
        setSavedRevision(existing);
        window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision: existing } }));
        window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
      } else {
        setSaveError(errorMessageOr(error, 'The draft could not be saved. Existing revisions were preserved.'));
      }
    } finally {
      setSaveBusy(false);
    }
  };

  const handleDiscardPreview = async () => {
    if (saveBusy || !sendResult) return;
    try {
      await discardGeneratedChapterDraft(sendResult.generationId);
    } catch (error) {
      setSaveError(errorMessageOr(error, 'The local recovery copy could not be discarded.'));
      return;
    }
    if (requestReview) void discardProviderRequestReview(requestReview.reviewId);
    window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    setSendResult(null);
    setSavedRevision(null);
    setSaveError(null);
    setRequestReview(null);
    setConsent(noProviderConsent);
    setReviewGeneration((generation) => generation + 1);
  };

  const requestReady = Boolean(
    providerId && modelId && requestReview && hasApiKey && consent.privacyAndDataUseAcknowledged
      && consent.billingUncertaintyAcknowledged && consent.adultSupportedRegionAcknowledged
      && unavailableModel !== modelId,
  );

  return (
    <div className="provider-overlay">
      <section ref={providerDialogRef} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="provider-dialog-title" tabIndex={-1}>
        <header className="provider-dialog-header">
          <div>
            <div className="step-pill"><span>{reviewMode ? 'EXACT CONTEXT' : 'OPTIONAL'}</span> {disclosure?.displayName ?? 'DIRECT API'}</div>
            <h2 id="provider-dialog-title">{reviewMode ? (codeLensRangeMode ? 'Explain cited source range' : sectionUpdateMode ? 'Regenerate one section' : overviewMode ? 'Map the repository' : 'Review before sending') : 'AI and narration setup'}</h2>
            <p>{reviewMode ? (codeLensRangeMode ? `Explain only ${mode.target.path}:${mode.target.startLine}–${mode.target.endLine} from this immutable snapshot. The result is transient and will not create a chapter.` : sectionUpdateMode ? `Only “${mode.baseRevision.sections.find((section) => section.sectionId === mode.sectionUpdate.sectionId)?.title ?? mode.sectionUpdate.sectionId}” and its declared evidence are included. Other sections stay separate.` : 'Choose a direct API, then inspect the exact snapshot context and disclosures. Nothing is sent until you confirm below.') : 'Local browsing works without a model API. Provider keys are optional and stay in macOS Keychain.'}</p>
          </div>
          <button className="icon-button" onClick={handleClose} disabled={keyBusy || sending || saveBusy} aria-label="Close provider dialog"><X size={17} /></button>
        </header>

        <div className="provider-key-actions">
          <button className="secondary-button" disabled={keyBusy || sending || saveBusy || requestBuilding || Boolean(sendResult || codeLensResult)} onClick={() => {
            if (requestReview) void discardProviderRequestReview(requestReview.reviewId);
            onChooseBackend();
          }}>Change generation backend</button>
        </div>

        {!native ? (
          <div className="provider-preview-only" role="status">
            <HardDrive size={18} />
            <div><strong>Direct model requests run in the macOS app.</strong><p>The desktop preview does not read Keychain or send repository content. No API key or source was requested here.</p></div>
          </div>
        ) : informationLoading ? (
          <div className="provider-loading" role="status"><Sparkles size={17} /> Loading direct API catalog…</div>
        ) : informationError ? (
          <div className="provider-error" role="alert"><TriangleAlert size={16} /><span>{informationError}</span></div>
        ) : (
          <div className="provider-dialog-content">
            <label className="provider-model-select">Direct API provider
              <select value={providerId} onChange={(event) => handleProviderChange(event.target.value)} disabled={keyBusy || sending || saveBusy || Boolean(sendResult || codeLensResult)}>
                <option value="">Choose a provider…</option>
                {providerCatalog.map((provider) => <option key={provider.providerId} value={provider.providerId}>{provider.displayName}</option>)}
              </select>
            </label>
            {!providerId && <p className="provider-small-note" role="status">Choose a provider explicitly for this task. Provider choice does not send a request or select a model automatically.</p>}
            {disclosure && <details className="provider-key-card" open={!hasApiKey || !reviewMode}>
              <summary>{disclosure.displayName} API key · {credentialInfo?.keychainAvailable ? (hasApiKey ? 'stored in Keychain' : 'Keychain ready') : 'Keychain unavailable'}</summary>
              <div className="provider-key-content">
                <p>{credentialInfo?.explanation ?? credentialError ?? 'Code Atlas checks Keychain only after you open provider setup.'}</p>
                <form className="provider-key-form" onSubmit={(event) => { void handleSaveKey(event); }}>
                  <label htmlFor="provider-api-key">Your {disclosure.displayName} API key</label>
                  <input
                    id="provider-api-key"
                    type="password"
                    value={keyDraft}
                    onChange={(event) => setKeyDraft(event.target.value)}
                    autoComplete="new-password"
                    spellCheck={false}
                    placeholder="Paste a provider API key"
                    disabled={!credentialInfo?.keychainAvailable || keyBusy}
                  />
                  <div className="provider-key-actions">
                    <button className="primary-button" type="submit" disabled={!credentialInfo?.keychainAvailable || keyDraft.length === 0 || keyBusy}>{keyBusy ? 'Saving…' : 'Save to Keychain'}</button>
                    {hasApiKey && <button className="secondary-button" type="button" onClick={() => { void handleRemoveKey(); }} disabled={keyBusy}>Remove saved key</button>}
                    <a href={disclosure.apiKeyUrl} target="_blank" rel="noreferrer">Get an API key ↗</a>
                  </div>
                </form>
                {keyMessage && <p className="provider-status-note" role="status">{keyMessage}</p>}
                {credentialError && <p className="provider-error-text" role="alert">{credentialError}</p>}
                {disclosure && <p className="provider-small-note">{disclosure.apiKeySecurityNotice}</p>}
                <p className="provider-small-note">The key is sent only in an HTTPS request header from Rust. It is not returned to the WebView or written to project files, settings, or logs.</p>
              </div>
            </details>}

            {disclosure && (
              <section className="provider-disclosure-card" aria-labelledby="provider-model-title">
                <div className="provider-section-heading">
                  <div><span>{disclosure.displayName.toUpperCase()} MODEL ALLOWLIST · CHECKED {disclosure.checkedDate}</span><h3 id="provider-model-title">Versioned models and current terms</h3></div>
                  <code>{disclosure.version}</code>
                </div>
                {reviewMode && (
                  <label className="provider-model-select">Model
                    <select value={modelId} onChange={(event) => handleModelChange(event.target.value)} disabled={sending || saveBusy || Boolean(sendResult || codeLensResult) || disclosure.models.length === 0}>
                      <option value="">Choose a model…</option>
                      {disclosure.models.map((model) => <option key={model.modelId} value={model.modelId}>{model.displayName} · {model.modelId}</option>)}
                    </select>
                  </label>
                )}
                <div className="provider-model-list">
                  {disclosure.models.map((model) => (
                    <article className={`provider-model ${model.modelId === modelId ? 'selected' : ''}`} key={model.modelId}>
                      <div><strong>{model.displayName}</strong><code>{model.modelId}</code></div>
                      <p>{model.intendedUse}</p>
                      <p>{model.freeTierStatus}</p>
                      <small>Published paid Standard rate: input {model.paidInputUsdPerMillionTokens}/1M tokens · output {model.paidOutputUsdPerMillionTokens}/1M tokens.</small>
                      <small>{model.supportsStructuredOutput ? 'Structured JSON output supported' : 'Structured output unavailable'} · {model.supportsStreaming ? 'provider streaming available; Code Atlas sends one non-streaming request' : 'provider streaming unavailable'}</small>
                    </article>
                  ))}
                </div>
                <div className="provider-source-links">
                  <a href={disclosure.pricingUrl} target="_blank" rel="noreferrer">{disclosure.displayName} pricing ↗</a>
                  <a href={disclosure.termsUrl} target="_blank" rel="noreferrer">Provider API terms ↗</a>
                  <a href={disclosure.regionsUrl} target="_blank" rel="noreferrer">Availability and regions ↗</a>
                </div>
                <p className="provider-small-note">The model list is shipped with the app; Code Atlas does not discover models, change a model automatically, or use a paid fallback. Pricing and availability can change after the checked date.</p>
              </section>
            )}

            {reviewMode && (
              <section className="provider-review-card" aria-labelledby="provider-review-title">
                  <div className="provider-section-heading">
                  <div><span>LOCAL PREVIEW · {codeLensRangeMode ? 'CITED CODE RANGE' : sectionUpdateMode ? 'ONE SECTION UPDATE' : overviewMode ? 'REPOSITORY OVERVIEW' : 'FILE CHAPTER DRAFT'}</span><h3 id="provider-review-title">Exact request context</h3></div>
                  {requestReview && <span className="provider-expiry">Expires {new Intl.DateTimeFormat(undefined, { timeStyle: 'short' }).format(new Date(requestReview.expiresAtUnixMs))}</span>}
                </div>
                {requestBuilding && <div className="provider-loading" role="status"><HardDrive size={16} /> Building a local preview from the selected snapshot. No model request is sent.</div>}
                {requestBuildError && <div className="provider-error" role="alert"><TriangleAlert size={16} /><span>{requestBuildError.message}</span></div>}
                {cancelConfirmation && sending && (
                  <div className="provider-cancel-confirm" role="group" aria-labelledby="provider-cancel-title" aria-describedby="provider-cancel-description">
                    <strong id="provider-cancel-title">Cancel this request?</strong>
                    <p id="provider-cancel-description">Code Atlas will try to stop its local HTTPS request. {disclosure?.displayName ?? 'The provider'} may already have received the selected source. Any returned response will be discarded.</p>
                    <div><button className="secondary-button" onClick={() => setCancelConfirmation(false)}>Keep waiting</button><button className="danger-button" onClick={() => { void handleConfirmCancel(); }} disabled={cancelBusy}>{cancelBusy ? 'Stopping…' : 'Confirm cancel'}</button></div>
                  </div>
                )}
                {cancelStatus && <p className="provider-small-note" role="status">{cancelStatus}</p>}
                {sectionUpdateMode ? <div className="provider-context-picker">
                  <div className="provider-context-heading"><strong>Selected section evidence</strong><span>{selectedPaths.length} declared files</span></div>
                  <p>This request regenerates one section only. Code Atlas selected the old section record and the exact declared files captured in snapshot {mode.snapshotId}; the selection cannot be broadened from this dialog.</p>
                  <ul className="provider-section-path-list">{selectedPaths.map((path) => <li key={path}><code>{path}</code>{path === mode.targetPath && <small>chapter target</small>}</li>)}</ul>
                  <p className="provider-small-note">Changing the model rebuilds the exact prompt and clears consent. No other section is rewritten or sent as chapter content.</p>
                </div> : mode.kind === 'code-lens-range' ? <div className="provider-context-picker">
                  <div className="provider-context-heading"><strong>Fixed source range</strong><span>{mode.target.path}</span></div>
                  <p>Only this hash-verified range from snapshot {mode.snapshotId} is sent. It cannot be expanded to neighboring lines or other files.</p>
                  <code>{mode.target.path}:{mode.target.startLine}–{mode.target.endLine}</code>
                  <p className="provider-small-note">The structured result will cite exact quotes inside this range and remain a temporary source-range explanation. It is not added to chapter coverage or revision history.</p>
                </div> : <div className="provider-context-picker">
                  <div className="provider-context-heading"><strong>Choose source files</strong><span>{selectedPaths.length} of 12 selected</span></div>
                  {selectedPaths.length > 12 && <p className="provider-error-text" role="status">This selection exceeds 12 files. Remove supporting files before preparing the request; the app keeps the full requested selection.</p>}
                  <p>{overviewMode ? 'Choose the files that should ground this optional repository map. Changing the selection rebuilds the preview and clears consent.' : 'The target stays selected. Add up to eleven supporting files from this snapshot; changing the selection rebuilds the preview and clears consent.'}</p>
                  {overviewMode && selectedPaths.length === 0 && <p className="provider-small-note" role="status">Select at least one captured text file to build the overview request preview.</p>}
                  {snapshotManifestLoading && <div className="provider-loading" role="status"><HardDrive size={15} /> Loading captured file names locally…</div>}
                  {snapshotManifestError && <p className="provider-small-note" role="status">{snapshotManifestError}</p>}
                  {snapshotManifest && (
                    <>
                      <label className="provider-context-search">Filter captured files<input value={contextQuery} onChange={(event) => setContextQuery(event.target.value)} placeholder="Search snapshot paths" /></label>
                      <div className="provider-context-files">
                        {availableContextFiles.map((file) => {
                          const checked = selectedPaths.includes(file.path);
                          const isTarget = mode.kind === 'review' && file.path === mode.targetPath;
                          return (
                            <label key={file.path} className={checked ? 'selected' : ''}>
                              <input type="checkbox" checked={checked} disabled={sending || saveBusy || Boolean(sendResult) || isTarget || (!checked && selectedPaths.length >= 12)} onChange={(event) => handleContextFileChange(file.path, event.target.checked)} />
                              <code>{file.path}</code><small>{formatBytes(file.sizeBytes)}{isTarget ? ' · target' : ''}</small>
                            </label>
                          );
                        })}
                        {availableContextFiles.length === 0 && <p className="provider-small-note">No eligible captured text files match this filter.</p>}
                      </div>
                      <p className="provider-small-note">Only captured text up to 250 KB per file is listed. Selected source is limited to 250 KB total and the encoded request to 512 KB. Secret-like, binary, invalid-encoding, skipped, and uncaptured paths cannot be selected.</p>
                    </>
                  )}
                </div>}
                {requestReview && (
                  <>
                    <div ref={requestSummaryRef} className="provider-request-summary" role="group" tabIndex={-1} aria-label="Reviewed request summary">
                      {sectionUpdateMode && <div><span>Update target</span><strong>One section · {mode.sectionUpdate.sectionId}</strong></div>}
                      {sectionUpdateMode && <div><span>Accepted source revision</span><code>{mode.baseRevision.revisionId} · snapshot {mode.baseRevision.snapshotId}</code></div>}
                      <div><span>{requestReview.targetPath ? 'Target file' : 'Chapter scope'}</span><code>{requestReview.targetPath ?? 'Repository overview'}</code></div>
                      <div><span>Provider and model</span><strong>{requestReview.providerId} · {requestReview.modelId}</strong></div>
                      <div><span>Endpoint</span><code>{requestReview.endpointUrl}</code></div>
                      <div><span>Source snapshot</span><code>{requestReview.snapshotId}</code></div>
                      <div><span>{codeLensRangeMode ? 'Range sent' : 'Source selected'}</span><strong>{codeLensRangeMode ? `${formatBytes(requestReview.sourceBytes)} from one fixed range` : `${requestReview.selectedFiles.length} ${requestReview.selectedFiles.length === 1 ? 'file' : 'files'} · ${formatBytes(requestReview.sourceBytes)}`}</strong></div>
                      <div><span>Input estimate</span><strong>About {requestReview.estimatedInputTokens.toLocaleString()} tokens · not a bill</strong></div>
                      <div><span>Request body</span><strong>{formatBytes(requestReview.requestBodyBytes)}</strong></div>
                      {!codeLensRangeMode && <div><span>Local index evidence</span><strong>{requestReview.selectedSymbols.length} declarations · {requestReview.selectedRelationships.length} relationships{requestReview.indexEvidenceTruncated ? ' · incomplete context' : ''}</strong></div>}
                    </div>
                    <p className="provider-warning-note"><TriangleAlert size={14} /> Review code for credentials and confidential or personal data before sending. The local secret-like scan catches common names and patterns, not every secret.</p>
                    {requestReview.selectedFiles.map((file) => (
                      <details className="provider-source-file" open key={`${file.path}:${file.sha256}`}>
                        <summary><code>{file.path}</code><span>{codeLensRangeMode ? `Sent excerpt ${formatBytes(file.contents.length)} · file SHA-256 ${file.sha256.slice(0, 16)}…` : `${formatBytes(file.sizeBytes)} · SHA-256 ${file.sha256.slice(0, 16)}…`}</span></summary>
                        <pre><code>{file.contents}</code></pre>
                      </details>
                    ))}
                    {!codeLensRangeMode && <details className="provider-index-preview">
                      <summary>Inspect selected declarations and relationships</summary>
                      <p>These entries come from the saved local index. Every entry refers to selected source; unresolved relationships remain uncertain.</p>
                      {requestReview.indexEvidenceTruncated && <p>The index context is incomplete or reached its bound. Omitted entries are not sent or available as chapter evidence.</p>}
                      <ul>{requestReview.selectedSymbols.map((symbol) => <li key={symbol.symbolId}><code>{symbol.path}:{symbol.startLine}–{symbol.endLine}</code><strong>{symbol.kind} · {symbol.qualifiedName ?? symbol.name}</strong><small>{symbol.isExported ? 'exported' : 'local declaration'}{symbol.isTest ? ' · test' : ''}</small></li>)}</ul>
                      <ul>{requestReview.selectedRelationships.map((relationship) => <li key={relationship.relationshipId}><strong>{relationship.kind} · {relationship.confidence} confidence</strong><code>{relationship.sourcePath} → {relationship.targetPath ?? 'unresolved'}</code><small>{relationship.unresolvedReason ?? `${relationship.sourceName ?? 'file'} → ${relationship.targetName ?? 'file'}`}</small><pre>{relationship.evidenceExcerpt}</pre></li>)}</ul>
                      {requestReview.selectedSymbols.length === 0 && requestReview.selectedRelationships.length === 0 && <p>No saved structural evidence is available for this selection. The draft must use exact source text and identify its limits.</p>}
                    </details>}
                    <details className="provider-prompt-preview">
                      <summary>Inspect exact prompt and encoded provider request</summary>
                      <div><strong>System instruction</strong><pre>{requestReview.systemInstruction}</pre><strong>User prompt</strong><pre>{requestReview.userPrompt}</pre><strong>Generation settings</strong><pre>{JSON.stringify(requestReview.generationConfig, null, 2)}</pre><strong>Exact JSON request body · SHA-256 {requestReview.requestBodySha256}</strong><pre>{JSON.stringify(requestReview.requestBodyPreview, null, 2)}</pre></div>
                    </details>
                    <div className="provider-fingerprint"><span>Consent applies to this request fingerprint</span><code>{requestReview.fingerprint}</code><small>Provider, model, endpoint, exact encoded body hash, prompt purpose, source paths and hashes, prompt version, disclosure version, and generation settings are included.</small></div>
                  </>
                )}
                {requestError && <div className="provider-error" role="alert"><TriangleAlert size={16} /><div><strong>{providerFailureTitle(requestError.kind)}</strong><span>{requestError.message}</span></div></div>}
                {unavailableModel === modelId && <p className="provider-error-text" role="alert">This model was rejected by {disclosure?.displayName ?? 'the provider'}. Choose another listed model explicitly; Code Atlas will not switch it for you.</p>}
                {sendResult && (
                  <section className="provider-response" aria-labelledby="provider-response-title">
                    <div><span>{savedRevision ? 'SAVED · UNACCEPTED DRAFT' : 'VALIDATED · UNSAVED PREVIEW'}</span><h4 id="provider-response-title">{sendResult.revision.title}</h4></div>
                    <p>{savedRevision ? `Draft revision ${savedRevision.revisionNumber} is saved locally. Guide Plan counts it as a draft. Acceptance requires a separate review.` : sectionUpdateMode ? 'Only the selected section received new claims. Other section revision IDs remain unchanged after their citations verify in the current snapshot. Assets with changed evidence are listed below for review.' : 'The schema and exact source citations passed local validation. Review what the claims mean, then save an unaccepted draft. Closing keeps this preview in local recovery until you save or discard it.'}</p>
                    {sectionUpdateMode && <div className="section-update-preview-summary">
                      <strong>Section-by-section revision check</strong>
                      {sendResult.revision.sections.map((section) => {
                        const previous = mode.baseRevision.sections.find((item) => item.sectionId === section.sectionId);
                        const changed = section.sectionId === mode.sectionUpdate.sectionId;
                        return <div key={section.sectionId}><span>{changed ? 'Regenerated' : previous?.sectionRevisionId === section.sectionRevisionId ? 'Carried unchanged' : 'Changed'}</span><code>{section.title}</code><small>{section.sectionRevisionId}</small></div>;
                      })}
                      {(sendResult.updateNotes ?? []).map((note) => <p className="provider-small-note" key={note}>{note}</p>)}
                    </div>}
                    <GeneratedChapterPreview revision={sendResult.revision} />
                    {saveError && <p className="provider-error-text" role="alert">{saveError}</p>}
                    <small>{sendResult.inputTokens ?? '—'} input tokens · {sendResult.outputTokens ?? '—'} output tokens · one request · no fallback</small>
                    <small>{sendResult.modelVersion ?? sendResult.modelId}{savedRevision ? ` · ${savedRevision.revisionId}` : ''}</small>
                  </section>
                )}
                {codeLensResult && (
                  <section className="provider-response" aria-labelledby="code-lens-explanation-title">
                    <div><span>TRANSIENT · RANGE GROUNDED</span><h4 id="code-lens-explanation-title">What this code does</h4></div>
                    <p>Each claim is checked against an exact, unique quote inside the selected source range. Nothing was saved to chapter history.</p>
                    <ol className="code-lens-explanation-claims">{codeLensResult.claims.map((claim) => (
                      <li key={claim.claimId}>
                        <p>{claim.prose}</p>
                        <button className="text-button" onClick={() => onOpenCodeLensCitation(claim.citation)}>
                          <FileCode2 size={13} /> Open {claim.citation.path}:{claim.citation.startLine}–{claim.citation.endLine}
                        </button>
                        <details className="chapter-evidence"><summary>Inspect cited code</summary><pre><code>{claim.citation.quote}</code></pre></details>
                      </li>
                    ))}</ol>
                    <small>{codeLensResult.modelVersion ?? codeLensResult.modelId} · {codeLensResult.inputTokens ?? '—'} input tokens · {codeLensResult.outputTokens ?? '—'} output tokens</small>
                  </section>
                )}
                {disclosure && (
                  <div className="provider-consent-list">
                    <label><input type="checkbox" checked={consent.privacyAndDataUseAcknowledged} onChange={(event) => updateConsent('privacyAndDataUseAcknowledged', event.target.checked)} disabled={sending || Boolean(sendResult || codeLensResult)} /><span><strong>Privacy and data use</strong>{disclosure.dataUseTerms} {disclosure.regionRequirements}</span></label>
                    <label><input type="checkbox" checked={consent.billingUncertaintyAcknowledged} onChange={(event) => updateConsent('billingUncertaintyAcknowledged', event.target.checked)} disabled={sending || Boolean(sendResult || codeLensResult)} /><span><strong>Billing uncertainty</strong>{disclosure.billingUncertainty}</span></label>
                    <label><input type="checkbox" checked={consent.adultSupportedRegionAcknowledged} onChange={(event) => updateConsent('adultSupportedRegionAcknowledged', event.target.checked)} disabled={sending || Boolean(sendResult || codeLensResult)} /><span><strong>Eligibility and region</strong>{disclosure.eligibilityRequirements}</span></label>
                  </div>
                )}
              </section>
            )}
          </div>
        )}

        <footer className="provider-dialog-footer">
          <button className="secondary-button" onClick={handleClose} disabled={sending || saveBusy}>{sending ? 'Provider request in progress…' : savedRevision || codeLensResult ? 'Done' : 'Close'}</button>
          {reviewMode && (
            sending ? (
              <button className="secondary-button" onClick={() => setCancelConfirmation(true)} disabled={cancelBusy || cancelRequested.current}>
                {cancelBusy || cancelRequested.current ? 'Cancelling…' : 'Cancel request'}
              </button>
            ) : codeLensResult ? (
              <span className="provider-saved-status" role="status"><Check size={15} /> Range explanation ready</span>
            ) : sendResult ? (
              savedRevision ? <span className="provider-saved-status" role="status"><Check size={15} /> Draft saved locally</span> : <div className="provider-draft-actions"><button className="secondary-button" onClick={() => { void handleDiscardPreview(); }} disabled={saveBusy}>Discard preview</button><button className="primary-button" onClick={() => { void handleSaveDraft(); }} disabled={saveBusy}><Check size={15} /> {saveBusy ? 'Saving draft…' : sectionUpdateMode ? 'Save section update as draft' : 'Save unaccepted draft'}</button></div>
            ) : (
              <button className="primary-button" onClick={() => { void handleSend(); }} disabled={!requestReady || requestBuilding || informationLoading || Boolean(sendResult || codeLensResult)}>
                <Sparkles size={15} /> Send this request
              </button>
            )
          )}
        </footer>
      </section>
    </div>
  );
}

function RecoveredGeneratedPreviewDialog({ generationId, onClose, onSaved, onDiscarded, onRecoveryChanged }: {
  generationId: string;
  onClose: () => void;
  onSaved: (revision: ChapterRevision) => void;
  onDiscarded: () => void;
  onRecoveryChanged: () => void;
}) {
  const [preview, setPreview] = useState<ChapterDraftPreview | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLElement>(null);
  useModalFocusTrap(dialogRef);

  useEffect(() => {
    dialogRef.current?.focus();
  }, []);

  useEffect(() => {
    let active = true;
    setLoading(true);
    void loadPendingGeneratedChapterPreview(generationId)
      .then((value) => { if (active) setPreview(value); })
      .catch((failure) => {
        if (!active) return;
        setError(errorMessageOr(failure, 'The saved preview could not be revalidated against its local snapshot.'));
        onRecoveryChanged();
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [generationId, onRecoveryChanged]);

  const save = async () => {
    if (!preview || busy) return;
    setBusy(true);
    setError(null);
    try {
      const revision = await saveGeneratedChapterDraft(generationId);
      onSaved(revision);
    } catch (failure) {
      const existing = await listChapterRevisions(preview.revision.projectId, preview.revision.snapshotId)
        .then((revisions) => revisions.find((revision) => revision.revisionId === preview.revision.revisionId))
        .catch(() => undefined);
      if (existing) {
        await discardGeneratedChapterDraft(generationId).catch(() => {});
        onSaved(existing);
      } else {
        setError(errorMessageOr(failure, 'The draft could not be saved. Existing revisions were preserved.'));
      }
    } finally {
      setBusy(false);
    }
  };

  const discard = async () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await discardGeneratedChapterDraft(generationId);
      onDiscarded();
    } catch (failure) {
      setError(errorMessageOr(failure, 'The local recovery copy could not be discarded.'));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="provider-overlay">
      <section ref={dialogRef} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="recovered-preview-title" tabIndex={-1}>
        <header className="provider-dialog-header">
          <div><div className="step-pill"><span>LOCAL RECOVERY</span> UNACCEPTED</div><h2 id="recovered-preview-title">Review saved chapter preview</h2></div>
          <button className="icon-button" aria-label="Close saved preview" onClick={onClose}><X size={17} /></button>
        </header>
        <div className="provider-dialog-body">
          {loading ? <div className="empty-workspace snapshot-loading" role="status"><HardDrive size={18} /><span>Checking the saved preview against its exact local snapshot…</span></div> : preview ? (
            <section className="provider-response" aria-label="Recovered generated chapter">
              <div><span>VALIDATED · UNSAVED PREVIEW</span><h4>{preview.revision.title}</h4></div>
              <p>This is saved local work. No provider request will be sent when you review or save it. Prior consent and request prompts were not stored; sending again requires a new exact-context review and fresh consent.</p>
              <GenerationRecord origin={preview.revision.generationOrigin} />
              <GeneratedChapterPreview revision={preview.revision} />
              <small>{preview.modelVersion ?? preview.modelId} · {preview.inputTokens ?? '—'} input tokens · {preview.outputTokens ?? '—'} output tokens</small>
            </section>
          ) : <div className="provider-error" role="alert"><TriangleAlert size={16} /><span>{error ?? 'The saved preview is unavailable.'}</span></div>}
          {error && preview && <p className="provider-error-text" role="alert">{error}</p>}
        </div>
        <footer className="provider-dialog-footer">
          <button className="secondary-button" onClick={onClose} disabled={busy}>Close</button>
          {preview ? (
            <div className="provider-draft-actions">
              <button className="secondary-button" onClick={() => { void discard(); }} disabled={busy}>{busy ? 'Working…' : 'Discard preview'}</button>
              <button className="primary-button" onClick={() => { void save(); }} disabled={busy}><Check size={15} /> {busy ? 'Saving draft…' : 'Save unaccepted draft'}</button>
            </div>
          ) : <button className="secondary-button" onClick={() => { void discard(); }} disabled={busy}>{busy ? 'Discarding…' : 'Discard preview'}</button>}
        </footer>
      </section>
    </div>
  );
}

function GeneratedChapterPreview({ revision }: { revision: ChapterRevision }) {
  return <div className="generated-chapter-sections">
    {revision.sections.map((section) => <article className="generated-chapter-section" key={section.sectionId}>
      <h5>{section.order + 1}. {section.title}</h5>
      <p>{section.prose}</p>
      <ul>{section.claims.map((claim) => <li key={claim.claimId}>
        <span className="chapter-claim-category">{claim.category.kind === 'inference' ? claim.category.visibleLabel : claim.category.kind === 'codeBehavior' ? 'Code behavior' : 'Repository context'}</span>
        <p>{claim.prose}</p>
        {claim.category.kind === 'inference' && <p className="chapter-inference-basis">Basis: {claim.category.basis}</p>}
        {claim.anchors.map((anchor) => <details className="chapter-evidence" key={anchor.anchorId}>
          <summary><FileCode2 size={12} /><code>{anchor.path}:{anchor.startLine}–{anchor.endLine}</code><span>Open exact evidence</span></summary>
          <pre><code>{anchor.excerpt}</code></pre>
          <small>Snapshot {anchor.snapshotId} · SHA-256 {anchor.sourceSha256.slice(0, 16)}…</small>
        </details>)}
      </li>)}</ul>
      <details className="chapter-spoken-preview"><summary>Narration script</summary><p>{section.spokenText}</p></details>
    </article>)}
  </div>;
}

function providerFailureTitle(kind: ProviderFailure['kind']): string {
  const titles: Record<ProviderFailure['kind'], string> = {
    keychainUnavailable: 'Keychain unavailable',
    keychainOperationFailed: 'Keychain operation failed',
    missingApiKey: 'Provider API key required',
    invalidApiKey: 'Provider API key rejected',
    consentRequired: 'Review and confirm every disclosure',
    reviewExpired: 'Request review expired',
    reviewChanged: 'Request context changed',
    reviewUnavailable: 'Request review unavailable',
    localStorageUnavailable: 'Local storage unavailable',
    invalidReview: 'Request could not be validated',
    contextTooLarge: 'Selected context is too large',
    secretLikeContext: 'Secret-like path blocked',
    unavailableModel: 'Model unavailable',
    billingRequired: 'Provider requires billing for this key',
    quotaExceeded: 'Provider quota or rate limit reached',
    regionUnavailable: 'Provider is unavailable in this region',
    networkUnavailable: 'Provider could not be reached',
    requestTimedOut: 'Provider request timed out',
    requestCancelled: 'Request cancelled',
    providerRejectedRequest: 'Provider rejected this request',
    providerResponseTooLarge: 'Response exceeded the size limit',
    malformedProviderResponse: 'Provider returned an unreadable response',
  };
  return titles[kind];
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(bytes < 10 * 1024 ? 1 : 0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(bytes < 10 * 1024 * 1024 ? 1 : 0)} MB`;
}

function formatSnapshotDate(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(timestamp));
}

function newestSnapshotsFirst(snapshots: SnapshotSummary[]): SnapshotSummary[] {
  return [...snapshots].sort((left, right) => right.createdAtUnixMs - left.createdAtUnixMs || right.snapshotId.localeCompare(left.snapshotId));
}

function CapturePolicy({ limits }: { limits: CaptureLimits | null }) {
  if (!limits) return null;
  return (
    <details className="capture-policy">
      <summary>What this snapshot reads · {limits.policyVersion}</summary>
      <div className="capture-policy-content">
        <div className="capture-policy-bounds">
          <span>Per file <strong>{formatBytes(limits.maxFileBytes)}</strong></span>
          <span>Total bytes <strong>{formatBytes(limits.maxTotalBytes)}</strong></span>
          <span>Paths <strong>{limits.maxPaths.toLocaleString()}</strong></span>
          <span>Path length <strong>{limits.maxPathBytes.toLocaleString()} bytes</strong></span>
          <span>Entries per folder <strong>{limits.maxEntriesPerDirectory.toLocaleString()}</strong></span>
        </div>
        <p><strong>Excluded folders:</strong> {limits.excludedDirectories.join(', ')}.</p>
        <p>{limits.otherExclusions.join(' · ')}. Git-tracked files remain eligible even if an ignore rule matches.</p>
      </div>
    </details>
  );
}

function SnapshotCaptureState({ progress, onCancel }: { progress: SnapshotProgress; onCancel: () => void }) {
  const phase = progress.phase === 'scanning'
    ? 'Scanning eligible paths'
    : progress.phase === 'checkingIgnoreRules'
      ? 'Checking Git ignore rules'
      : progress.phase === 'readingFiles'
        ? 'Capturing file bytes'
        : 'Saving immutable snapshot';
  const hasReadProgress = progress.phase === 'readingFiles' && progress.candidateFiles > 0;
  const percent = hasReadProgress ? Math.min(100, Math.round(progress.filesProcessed * 100 / progress.candidateFiles)) : undefined;
  return (
    <div className="empty-workspace snapshot-state">
      <div className="snapshot-state-card" aria-live="polite">
        <div className="snapshot-state-icon spinning"><HardDrive size={22} /></div>
        <div className="snapshot-state-copy">
          <div className="step-pill"><span>LOCAL</span> IMMUTABLE CAPTURE</div>
          <h2>{phase}</h2>
          <p>{progress.currentPath ? <><span className="mono">{progress.currentPath}</span><br /></> : null}{progress.message ?? 'Repository data stays on this Mac. The original working tree is opened read-only.'}</p>
        </div>
        {hasReadProgress ? (
          <div className="capture-progress-detail">
            <div className="capture-progress-line"><strong>{progress.filesCaptured.toLocaleString()} captured · {progress.filesProcessed.toLocaleString()} / {progress.candidateFiles.toLocaleString()} checked</strong><span>{percent}%</span></div>
            <progress value={progress.filesProcessed} max={progress.candidateFiles} aria-label="Candidate files checked" />
          </div>
        ) : <progress className="capture-indeterminate" aria-label={phase} />}
        <div className="capture-live-stats">
          <span>{progress.pathsVisited.toLocaleString()} paths checked</span>
          <span>{formatBytes(progress.bytesRead)} read</span>
          <span>{progress.pathsSkipped.toLocaleString()} skipped</span>
        </div>
        <div className="capture-state-actions"><button className="secondary-button" onClick={onCancel}>Cancel capture</button></div>
        <CapturePolicy limits={progress.limits} />
      </div>
    </div>
  );
}

function SnapshotCaptureBanner({ progress, onCancel, onRetry, onView }: { progress: SnapshotProgress; onCancel?: () => void; onRetry?: () => void; onView?: () => void }) {
  const label = progress.status === 'running'
    ? `Capturing locally · ${progress.filesCaptured.toLocaleString()} files · ${formatBytes(progress.bytesRead)}`
    : progress.status === 'cancelled'
      ? 'New capture cancelled · selected snapshot is unchanged'
      : progress.status === 'inconsistent'
        ? 'Repository changed during capture · no new snapshot was added'
        : progress.status === 'complete'
          ? 'New snapshot saved · selected revision is unchanged'
          : progress.status === 'partial'
            ? 'New partial snapshot saved · selected revision is unchanged'
            : 'Capture failed · selected snapshot is unchanged';
  return (
    <div className={`snapshot-capture-banner ${progress.status === 'running' ? 'is-running' : 'is-terminal'}`} role={progress.status === 'running' ? 'status' : 'alert'}>
      <HardDrive size={15} />
      <span><strong>{label}</strong>{progress.currentPath && <small title={progress.currentPath}>{progress.currentPath}</small>}{progress.message && progress.status !== 'running' && <small>{progress.message}</small>}</span>
      {onCancel && <button onClick={onCancel}>Cancel capture</button>}
      {onRetry && <button onClick={onRetry}>Try again</button>}
      {onView && <button onClick={onView}>View new snapshot</button>}
    </div>
  );
}

function SnapshotCaptureResult({ progress, onRetry }: { progress: SnapshotProgress; onRetry: () => void }) {
  const heading = progress.status === 'cancelled'
    ? 'Capture cancelled'
    : progress.status === 'inconsistent'
      ? 'Repository changed during capture'
      : 'Snapshot could not be saved';
  return (
    <div className="empty-workspace snapshot-state">
      <div className={`snapshot-state-card terminal-${progress.status}`} role={progress.status === 'failed' || progress.status === 'inconsistent' ? 'alert' : 'status'}>
        <div className="snapshot-state-icon"><TriangleAlert size={22} /></div>
        <div className="snapshot-state-copy">
          <div className="step-pill"><span>{progress.status.toUpperCase()}</span> SNAPSHOT RESULT</div>
          <h2>{heading}</h2>
          <p>{progress.message ?? 'No new snapshot was added to history. Previous snapshots remain unchanged.'}</p>
        </div>
        <div className="capture-state-actions"><button className="primary-button" onClick={onRetry}>Try capture again</button></div>
        <CapturePolicy limits={progress.limits} />
      </div>
    </div>
  );
}

function SnapshotReadyState({ manifest, atlasSelection, mapViewport, selectedPath, source, sourceRange, sourceRangeIssue, sourceLoading, sourceError, onCaptureAgain, onOpenFile, onAtlasSelection, onAtlasViewportChange, onRequestExplanation, onRequestRangeExplanation, onReviewRevision, onReadRevision }: {
  manifest: SnapshotManifest;
  atlasSelection?: WorkspaceRoute['atlasSelection'];
  mapViewport?: AtlasViewport;
  selectedPath: string | null;
  source: SnapshotSource | null;
  sourceRange: SourceLineRange | null;
  sourceRangeIssue: WorkspaceRoute['sourceRangeIssue'] | null;
  sourceLoading: boolean;
  sourceError: string | null;
  onCaptureAgain: () => void;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onAtlasSelection: (selection: WorkspaceRoute['atlasSelection'], treeSelection: string, mapViewport?: AtlasViewport) => void;
  onAtlasViewportChange: (mapViewport: AtlasViewport) => void;
  onRequestExplanation: (path: string) => void;
  onRequestRangeExplanation: (path: string, range: SourceLineRange) => void;
  onReviewRevision: (revision: ChapterRevision) => void;
  onReadRevision: (revision: ChapterRevision) => void;
}) {
  const { summary } = manifest;
  const partial = summary.status === 'partial';
  const inventory = useMemo(() => buildInventory(manifest), [manifest]);
  const omittedRows = inventory.filter((row) => row.disposition !== 'captured').length;
  const destinationRef = useRef<HTMLDivElement>(null);
  const atlasSelectionKey = atlasSelection ? JSON.stringify(atlasSelection) : '';

  useEffect(() => {
    if (!atlasSelection || atlasSelection.kind === 'file') return;
    const frame = requestAnimationFrame(() => destinationRef.current?.focus({ preventScroll: false }));
    return () => cancelAnimationFrame(frame);
  }, [atlasSelectionKey, summary.snapshotId]);

  return (
    <div className="empty-workspace snapshot-ready-state">
      <section className="snapshot-ready-card" aria-label="Selected repository snapshot and files">
        <div className="files-capture-strip">
          <details className="snapshot-technical-details">
            <summary>{summary.filesCaptured.toLocaleString()} files · {partial ? `${summary.pathsSkipped} skipped` : 'snapshot saved'} <ChevronDown size={12} /></summary>
          <div className="snapshot-metrics">
            <div><strong>{inventory.length.toLocaleString()}</strong><span>inventory paths</span></div>
            <div><strong>{summary.filesCaptured.toLocaleString()}</strong><span>files captured</span></div>
            <div><strong>{formatBytes(summary.totalBytes)}</strong><span>source bytes</span></div>
            <div><strong>{omittedRows.toLocaleString()}</strong><span>excluded or skipped rows</span></div>
          </div>
          <div className="snapshot-identity-row"><span>Snapshot ID</span><code>{summary.snapshotId}</code><span>Parent</span><code>{summary.parentSnapshotId ?? 'First capture'}</code></div>
          <div className="snapshot-dirty-state"><GitBranch size={15} /><span>Git index: <strong>{summary.indexState}</strong></span><span>Working tree: <strong>{summary.worktreeState}</strong></span><span>HEAD: <code>{summary.head?.slice(0, 12) ?? 'unborn'}</code></span></div>
          <CapturePolicy limits={manifest.captureLimits} />
          <button className="secondary-button" onClick={onCaptureAgain}>Capture current code</button>
          </details>
          <span>Read-only source · saved on this Mac</span>
        </div>
        {atlasSelection && atlasSelection.kind !== 'file' && <AtlasDestinationCard refElement={destinationRef} manifest={manifest} selection={atlasSelection} onOpenFile={onOpenFile} />}
        <AtlasWorkbench
          key={`${summary.projectId}:${summary.snapshotId}`}
          manifest={manifest}
          rows={inventory}
          routeSelection={atlasSelection}
          selectedPath={selectedPath}
          routeViewport={mapViewport}
          source={source}
          sourceRange={sourceRange}
          sourceRangeIssue={sourceRangeIssue}
          sourceLoading={sourceLoading}
          sourceError={sourceError}
          onOpenFile={onOpenFile}
          onAtlasSelection={onAtlasSelection}
          onViewportChange={onAtlasViewportChange}
          onRequestExplanation={onRequestExplanation}
          onRequestRangeExplanation={onRequestRangeExplanation}
          onReviewRevision={onReviewRevision}
          onReadRevision={onReadRevision}
        />
      </section>
    </div>
  );
}

function AtlasDestinationCard({ refElement, manifest, selection, onOpenFile }: {
  refElement: RefObject<HTMLDivElement | null>;
  manifest: SnapshotManifest;
  selection: NonNullable<WorkspaceRoute['atlasSelection']>;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
}) {
  const [index, setIndex] = useState<SnapshotIndex | null>(null);
  const [indexLoading, setIndexLoading] = useState(selection.kind !== 'file');
  const [indexError, setIndexError] = useState<string | null>(null);
  const snapshotId = manifest.summary.snapshotId;
  const projectId = manifest.summary.projectId;

  useEffect(() => {
    if (selection.kind === 'file') {
      setIndex(null);
      setIndexLoading(false);
      setIndexError(null);
      return;
    }
    if (!isNativeRuntime()) {
      setIndex(null);
      setIndexLoading(false);
      setIndexError('The local source index is available in the Mac app.');
      return;
    }
    let active = true;
    setIndexLoading(true);
    setIndexError(null);
    void loadSnapshotIndex(projectId, snapshotId)
      .then((loaded) => {
        if (!active) return;
        setIndex(loaded?.projectId === projectId && loaded.snapshotId === snapshotId ? loaded : null);
      })
      .catch((error) => {
        if (active) setIndexError(errorMessageOr(error, 'The local source index could not be opened.'));
      })
      .finally(() => { if (active) setIndexLoading(false); });
    return () => { active = false; };
  }, [projectId, selection.kind, snapshotId]);

  const symbol = selection.kind === 'symbol'
    ? index?.symbols.find((item) => item.symbolId === selection.symbolId) ?? null
    : null;
  const relationship = selection.kind === 'relationship'
    ? index?.relationships.find((item) => item.relationshipId === selection.relationshipId) ?? null
    : null;
  const sourceSymbol = relationship ? index?.symbols.find((item) => item.symbolId === relationship.sourceId) ?? null : null;
  const targetSymbol = relationship?.targetId ? index?.symbols.find((item) => item.symbolId === relationship.targetId) ?? null : null;
  const selectedPath = selection.kind === 'file' || selection.kind === 'symbol' ? selection.path : relationship?.evidence.path ?? null;
  const selectedRange = symbol
    ? codeLensRangeFromIndexedAnchor(symbol.anchor)
    : relationship
      ? codeLensRangeFromIndexedAnchor(relationship.evidence)
      : undefined;

  return (
    <section className="atlas-destination-card" aria-labelledby="atlas-destination-title">
      <div className="atlas-destination-header">
        <div className="atlas-destination-icon"><Workflow size={15} /></div>
        <div><span className="step-pill"><span>FROM WALKTHROUGH</span> ATLAS DESTINATION</span><h3 id="atlas-destination-title">Linked {selection.kind}</h3></div>
      </div>
      <div className="atlas-destination-body" ref={refElement} tabIndex={-1} data-focus-key="atlas-destination">
        {selection.kind === 'file' && <><code>{selection.path}</code><p>This file is selected in the snapshot inventory below.</p></>}
        {selection.kind === 'symbol' && <>
          <code>{selection.path}</code>
          {symbol ? <><strong>{symbol.qualifiedName}</strong><p>{symbol.kind} · lines {symbol.anchor.startLine}–{symbol.anchor.endLine}{symbol.isExported ? ' · exported' : ''}</p></>
            : indexLoading ? <p>Resolving symbol from the snapshot index…</p>
              : <p>The exact symbol is not present in this snapshot index.{indexError ? ` ${indexError}` : ''} The file remains selected below.</p>}
        </>}
        {selection.kind === 'relationship' && <>
          <code>{selection.relationshipId}</code>
          {relationship ? <><strong>{sourceSymbol?.qualifiedName ?? relationship.sourceId} {relationship.kind} {targetSymbol?.qualifiedName ?? relationship.targetLabel ?? relationship.targetPath ?? 'an unresolved target'}</strong><p>{relationship.confidence === 'unresolved' ? relationship.unresolvedReason ?? 'The relationship target could not be resolved.' : `Evidence at ${relationship.evidence.path}:${relationship.evidence.startLine}-${relationship.evidence.endLine}`}</p></>
            : indexLoading ? <p>Resolving relationship from the snapshot index…</p>
              : <p>The relationship is not present in this snapshot index.{indexError ? ` ${indexError}` : ''}</p>}
        </>}
        {selectedPath && <button className="atlas-destination-open" data-focus-key="atlas-destination-open" onClick={() => onOpenFile(selectedPath, snapshotId, selectedRange)}><FileCode2 size={14} /> {selectedRange ? 'Open exact source evidence' : 'Open this file'} <ArrowRight size={13} /></button>}
      </div>
    </section>
  );
}

interface AtlasWorkbenchViewState {
  treeQuery: string;
  mapQuery: string;
  kind: 'all' | RelationshipKind;
  language: string;
  collapsed: string[];
  selectedPath: string | null;
  selectedFolder: string | null;
  selectedRelationship: string | null;
  selectedSymbol: string | null;
  treeTop: number;
  canvasTop: number;
  viewport: AtlasViewport;
}

const ATLAS_TREE_ROW_HEIGHT = 29;
const ATLAS_TREE_OVERSCAN = 10;
const ATLAS_TREE_DEFAULT_VIEWPORT_HEIGHT = 510;

function emptyAtlasWorkbenchState(): AtlasWorkbenchViewState {
  return { treeQuery: '', mapQuery: '', kind: 'all', language: 'all', collapsed: [], selectedPath: null, selectedFolder: null, selectedRelationship: null, selectedSymbol: null, treeTop: 0, canvasTop: 0, viewport: { x: 0, y: 0, scale: 1 } };
}

function loadAtlasWorkbenchState(key: string): AtlasWorkbenchViewState {
  try {
    const value = sessionStorage.getItem(key);
    if (!value) return emptyAtlasWorkbenchState();
    const parsed = JSON.parse(value);
    return { ...emptyAtlasWorkbenchState(), ...parsed, viewport: normalizeAtlasViewport(parsed?.viewport) };
  } catch { return emptyAtlasWorkbenchState(); }
}

function AtlasWorkbench({ manifest, rows, routeSelection, selectedPath, routeViewport, source, sourceRange, sourceRangeIssue, sourceLoading, sourceError, onOpenFile, onAtlasSelection, onViewportChange, onRequestExplanation, onRequestRangeExplanation, onReviewRevision, onReadRevision }: {
  manifest: SnapshotManifest;
  rows: InventoryRow[];
  routeSelection?: WorkspaceRoute['atlasSelection'];
  selectedPath: string | null;
  routeViewport?: AtlasViewport;
  source: SnapshotSource | null;
  sourceRange: SourceLineRange | null;
  sourceRangeIssue: WorkspaceRoute['sourceRangeIssue'] | null;
  sourceLoading: boolean;
  sourceError: string | null;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onAtlasSelection: (selection: WorkspaceRoute['atlasSelection'], treeSelection: string, mapViewport?: AtlasViewport) => void;
  onViewportChange: (viewport: AtlasViewport) => void;
  onRequestExplanation: (path: string) => void;
  onRequestRangeExplanation: (path: string, range: SourceLineRange) => void;
  onReviewRevision: (revision: ChapterRevision) => void;
  onReadRevision: (revision: ChapterRevision) => void;
}) {
  const snapshotId = manifest.summary.snapshotId;
  const projectId = manifest.summary.projectId;
  const storageKey = `atlas-view:${projectId}:${snapshotId}`;
  const [view, setView] = useState(() => {
    const restored = loadAtlasWorkbenchState(storageKey);
    return routeViewport ? { ...restored, viewport: normalizeAtlasViewport(routeViewport) } : restored;
  });
  const [treeViewport, setTreeViewport] = useState(() => ({
    scrollTop: view.treeTop,
    height: ATLAS_TREE_DEFAULT_VIEWPORT_HEIGHT,
  }));
  const [showSkipped, setShowSkipped] = useState(false);
  const [treeRowHeight, setTreeRowHeight] = useState(ATLAS_TREE_ROW_HEIGHT);
  const [focusedTreeIndex, setFocusedTreeIndex] = useState<number | null>(null);
  const [index, setIndex] = useState<SnapshotIndex | null>(null);
  const [indexState, setIndexState] = useState<'loading' | 'pending' | 'ready' | 'error'>('loading');
  const [indexError, setIndexError] = useState<string | null>(null);
  const [indexJobId, setIndexJobId] = useState<string | null>(null);
  const [indexProgress, setIndexProgress] = useState<IndexProgress | null>(null);
  const [startingIndex, setStartingIndex] = useState(false);
  const treeRef = useRef<HTMLDivElement>(null);
  const treeTopRef = useRef(view.treeTop);
  const treePersistTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const canvasRef = useRef<HTMLDivElement>(null);
  const canvasContentRef = useRef<HTMLDivElement>(null);
  const panGestureRef = useRef<{ pointerId: number; startX: number; startY: number; startViewport: AtlasViewport; pendingViewport: AtlasViewport } | null>(null);
  const [dragViewport, setDragViewport] = useState<AtlasViewport | null>(null);
  const routeKey = routeSelection ? JSON.stringify(routeSelection) : '';

  useEffect(() => {
    try { sessionStorage.setItem(storageKey, JSON.stringify(view)); } catch { /* Session view persistence is optional. */ }
  }, [storageKey, view]);
  useEffect(() => {
    const tree = treeRef.current;
    if (!tree) return;
    const refreshViewport = () => setTreeViewport((current) => {
      const next = { scrollTop: tree.scrollTop, height: tree.clientHeight || ATLAS_TREE_DEFAULT_VIEWPORT_HEIGHT };
      return current.scrollTop === next.scrollTop && current.height === next.height ? current : next;
    });
    tree.scrollTop = treeTopRef.current;
    refreshViewport();
    const observer = new ResizeObserver(refreshViewport);
    observer.observe(tree);
    return () => observer.disconnect();
  }, []);
  useEffect(() => () => {
    if (treePersistTimerRef.current) clearTimeout(treePersistTimerRef.current);
  }, []);
  useEffect(() => {
    if (routeSelection) {
      setView((current) => ({
        ...current,
        selectedPath: routeSelection.kind === 'file' || routeSelection.kind === 'symbol' ? routeSelection.path : null,
        selectedSymbol: routeSelection.kind === 'symbol' ? routeSelection.symbolId : null,
        selectedRelationship: routeSelection.kind === 'relationship' ? routeSelection.relationshipId : null,
        selectedFolder: null,
      }));
    } else if (selectedPath) {
      setView((current) => current.selectedPath === selectedPath && !current.selectedFolder
        ? current
        : { ...current, selectedPath, selectedFolder: null, selectedSymbol: null, selectedRelationship: null });
    }
  }, [routeKey, routeSelection, selectedPath]);
  useEffect(() => {
    let active = true;
    setIndexState('loading');
    setIndex(null);
    if (!isNativeRuntime()) {
      setIndexState('pending');
      setIndexError('Local snapshot indexing is available in the Mac app.');
      return () => { active = false; };
    }
    void loadSnapshotIndex(projectId, snapshotId).then((loaded) => {
      if (!active) return;
      if (loaded?.projectId === projectId && loaded.snapshotId === snapshotId) {
        setIndex(loaded);
        setIndexState('ready');
      } else setIndexState('pending');
    }).catch((error) => {
      if (!active) return;
      setIndexError(errorMessageOr(error, 'The local snapshot index could not be loaded.'));
      setIndexState('error');
    });
    return () => { active = false; };
  }, [projectId, snapshotId]);
  useEffect(() => {
    if (!indexJobId) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const progress = await getSnapshotIndexProgress(indexJobId);
        if (!active) return;
        if (progress) setIndexProgress(progress);
        if (progress?.status === 'running') { timer = setTimeout(poll, 900); return; }
        const loaded = await loadSnapshotIndex(projectId, snapshotId);
        if (!active) return;
        if (loaded?.projectId === projectId && loaded.snapshotId === snapshotId) { setIndex(loaded); setIndexState('ready'); }
        else setIndexState('pending');
        setIndexJobId(null);
      } catch (error) {
        if (!active) return;
        setIndexError(errorMessageOr(error, 'Index progress could not be read.'));
        setIndexState('error');
        setIndexJobId(null);
      }
    };
    void poll();
    return () => { active = false; clearTimeout(timer); };
  }, [indexJobId, projectId, snapshotId]);
  const capturedPaths = useMemo(() => new Set(rows.filter((row) => row.disposition === 'captured' && row.sha256).map((row) => row.path)), [rows]);
  const graph = useMemo(() => index ? projectProvenRelationshipGraph(index, manifest, capturedPaths) : null, [capturedPaths, index, manifest]);
  const stalePaths = useMemo(() => {
    if (!index) return [];
    const hashes = new Map(manifest.files.map((file) => [file.path, file.sha256]));
    const stale = new Set(index.files.filter((file) => hashes.get(file.path) !== file.sourceSha256).map((file) => file.path));
    for (const symbol of index.symbols) {
      if (symbol.anchor.path !== symbol.path || hashes.get(symbol.path) !== symbol.anchor.sourceSha256) stale.add(symbol.path);
    }
    for (const relationship of index.relationships) {
      if (hashes.get(relationship.evidence.path) !== relationship.evidence.sourceSha256) stale.add(relationship.evidence.path);
    }
    return [...stale];
  }, [index, manifest.files]);
  const stalePathSet = useMemo(() => new Set(stalePaths), [stalePaths]);
  const deferredTreeQuery = useDeferredValue(view.treeQuery);
  const treeFilterPending = deferredTreeQuery !== view.treeQuery;
  const explorerRows = useMemo(() => {
    if (showSkipped) return rows;
    const parents = new Set<string>(['.']);
    for (const row of rows) {
      if (row.disposition !== 'captured') continue;
      const parts = row.path.split('/');
      for (let length = 1; length < parts.length; length += 1) parents.add(parts.slice(0, length).join('/'));
    }
    return rows.filter((row) => row.disposition === 'captured' || row.kind === 'directory' && parents.has(row.path));
  }, [rows, showSkipped]);
  const matchingRows = useMemo(
    () => filterInventoryTreeRows(explorerRows, view.selectedFolder, deferredTreeQuery),
    [explorerRows, view.selectedFolder, deferredTreeQuery],
  );
  const flattened = useMemo(() => flattenInventoryTree(buildInventoryTree(matchingRows), new Set(view.collapsed)), [matchingRows, view.collapsed]);
  const treeWindow = virtualRowWindow(flattened.length, treeViewport.scrollTop, treeViewport.height, treeRowHeight, ATLAS_TREE_OVERSCAN);
  const visibleTreeRows = flattened.slice(treeWindow.startIndex, treeWindow.endIndex);
  const selectedTreeIndex = flattened.findIndex((item) => item.path === view.selectedPath);
  const selectedTreeIndexIsVisible = selectedTreeIndex >= treeWindow.startIndex && selectedTreeIndex < treeWindow.endIndex;
  const initialTreeIndex = selectedTreeIndexIsVisible ? selectedTreeIndex : treeWindow.startIndex;
  const focusedTreeIndexIsVisible = focusedTreeIndex !== null
    && focusedTreeIndex >= treeWindow.startIndex
    && focusedTreeIndex < treeWindow.endIndex;
  const activeTreeIndex = focusedTreeIndexIsVisible ? focusedTreeIndex : initialTreeIndex;
  useEffect(() => {
    const row = treeRef.current?.querySelector<HTMLElement>('.atlas-tree-row');
    if (!row) return;
    const measureRowHeight = () => {
      const measured = row.getBoundingClientRect().height;
      if (measured > 0) setTreeRowHeight((current) => Math.abs(current - measured) < 0.5 ? current : measured);
    };
    measureRowHeight();
    const observer = new ResizeObserver(measureRowHeight);
    observer.observe(row);
    return () => observer.disconnect();
  }, [treeWindow.startIndex, treeWindow.endIndex]);
  useEffect(() => {
    if (focusedTreeIndex === null) return;
    if (focusedTreeIndex >= flattened.length) setFocusedTreeIndex(flattened.length > 0 ? flattened.length - 1 : null);
  }, [focusedTreeIndex, flattened.length]);
  useEffect(() => {
    if (focusedTreeIndex === null) return;
    treeRef.current?.querySelector<HTMLElement>(`[data-tree-index="${focusedTreeIndex}"]`)?.focus({ preventScroll: true });
  }, [focusedTreeIndex, treeWindow.startIndex, treeWindow.endIndex]);
  const provenEdges = graph?.edges ?? [];
  const languages = useMemo(() => Array.from(new Set(index?.symbols.map((symbol) => symbol.language) ?? [])).sort(), [index]);
  const shownEdges = useMemo(() => index
    ? filterProvenRelationshipEdges(provenEdges, index.symbols, {
      folderPath: view.selectedFolder,
      relationshipKind: view.kind,
      language: view.language,
      query: view.mapQuery,
    })
    : [], [index, provenEdges, view.kind, view.language, view.mapQuery, view.selectedFolder]);
  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      if (canvasRef.current) canvasRef.current.scrollTop = view.canvasTop;
    });
    return () => cancelAnimationFrame(frame);
  }, [indexState, routeKey, shownEdges.length]);
  const selectedRow = rows.find((row) => row.path === view.selectedPath) ?? null;
  const selectedIndexCounts = selectedRow && index
    ? summarizeAtlasPathIndexCounts(index.files, selectedRow.path, selectedRow.kind === 'directory')
    : null;
  const selectedSymbol = index?.symbols.find((symbol) => symbol.symbolId === view.selectedSymbol) ?? null;
  const selectedEdge = graph?.edges.find((edge) => edge.relationship.relationshipId === view.selectedRelationship) ?? null;
  const rawRelationship = index?.relationships.find((edge) => edge.relationshipId === view.selectedRelationship) ?? null;
  const unresolvedCount = index?.relationships.filter((edge) => edge.confidence !== 'high' || edge.unresolvedReason !== null || !edge.targetId).length ?? 0;
  const unsupportedCount = rows.filter((row) => row.parserReadiness === 'unsupported').length;

  const choosePath = (row: InventoryRow) => {
    const folder = row.kind === 'directory' ? row.path : null;
    if (folder !== null) {
      treeTopRef.current = 0;
      if (treeRef.current) treeRef.current.scrollTop = 0;
      setTreeViewport((current) => ({ ...current, scrollTop: 0 }));
      setFocusedTreeIndex(null);
    }
    const nextView = { ...view, selectedPath: row.path, selectedFolder: folder, selectedSymbol: null, selectedRelationship: null, treeTop: treeTopRef.current };
    setView(nextView);
    try { sessionStorage.setItem(storageKey, JSON.stringify(nextView)); } catch { /* Session view persistence is optional. */ }
    onAtlasSelection(row.kind === 'directory' ? undefined : { kind: 'file', path: row.path }, row.path, view.viewport);
  };
  const focusTreeRow = (rowIndex: number) => {
    if (flattened.length === 0) return;
    const index = Math.max(0, Math.min(flattened.length - 1, rowIndex));
    const tree = treeRef.current;
    if (tree) {
      let scrollTop = tree.scrollTop;
      const rowTop = index * treeRowHeight;
      const rowBottom = rowTop + treeRowHeight;
      if (rowTop < scrollTop) scrollTop = rowTop;
      else if (rowBottom > scrollTop + tree.clientHeight) scrollTop = rowBottom - tree.clientHeight;
      if (scrollTop !== tree.scrollTop) tree.scrollTop = scrollTop;
      treeTopRef.current = scrollTop;
      setTreeViewport({ scrollTop, height: tree.clientHeight || ATLAS_TREE_DEFAULT_VIEWPORT_HEIGHT });
    }
    setFocusedTreeIndex(index);
  };
  const setTreeFolderExpanded = (path: string, expanded: boolean) => setView((current) => {
    const isExpanded = !current.collapsed.includes(path);
    if (isExpanded === expanded) return current;
    return {
      ...current,
      collapsed: expanded
        ? current.collapsed.filter((collapsedPath) => collapsedPath !== path)
        : [...current.collapsed, path],
    };
  });
  const chooseSymbol = (id: string, path: string) => {
    setView((current) => ({ ...current, selectedPath: path, selectedFolder: null, selectedSymbol: id, selectedRelationship: null }));
    onAtlasSelection({ kind: 'symbol', path, symbolId: id }, `${path}#${id}`, view.viewport);
  };
  const chooseEdge = (edge: ProvenGraphEdge) => {
    setView((current) => ({ ...current, selectedRelationship: edge.relationship.relationshipId, selectedSymbol: null }));
    onAtlasSelection({ kind: 'relationship', relationshipId: edge.relationship.relationshipId }, `relationship:${edge.relationship.relationshipId}`, view.viewport);
  };
  const openEdgeEvidence = (edge: ProvenGraphEdge) => onOpenFile(edge.evidence.path, snapshotId, {
    ...codeLensRangeFromIndexedAnchor({ ...edge.evidence, sourceSha256: edge.evidence.sha256 }),
  });
  const startIndex = async () => {
    if (!isNativeRuntime() || startingIndex || indexJobId) return;
    setStartingIndex(true); setIndexError(null);
    try {
      const started = await startSnapshotIndexing(projectId, snapshotId);
      setIndexProgress(null); setIndexJobId(started.jobId); setIndexState('loading');
    } catch (error) {
      setIndexError(errorMessageOr(error, 'Snapshot indexing could not start.'));
      setIndexState('error');
    } finally { setStartingIndex(false); }
  };
  const commitViewport = (viewport: AtlasViewport) => {
    const normalized = normalizeAtlasViewport(viewport);
    setView((current) => ({ ...current, viewport: normalized, canvasTop: 0 }));
    onViewportChange(normalized);
    if (canvasRef.current) {
      canvasRef.current.scrollLeft = 0;
      canvasRef.current.scrollTop = 0;
    }
  };
  const zoomAtCenter = (delta: number) => {
    const frame = canvasRef.current;
    if (!frame) return;
    const viewport = dragViewport ?? view.viewport;
    commitViewport(zoomAtlasViewport(viewport, delta, frame.clientWidth / 2, frame.clientHeight / 2));
  };
  const fitMap = () => {
    const frame = canvasRef.current;
    const content = canvasContentRef.current;
    if (!frame || !content) return;
    commitViewport(fitAtlasViewport(content.offsetWidth, content.offsetHeight, frame.clientWidth - 16, frame.clientHeight - 16));
  };
  const resetMap = () => commitViewport({ x: 0, y: 0, scale: 1 });
  const handleCanvasKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget && event.target !== canvasContentRef.current) return;
    const step = 32;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight' || event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      commitViewport(panAtlasViewport(view.viewport,
        event.key === 'ArrowLeft' ? -step : event.key === 'ArrowRight' ? step : 0,
        event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0));
    } else if (event.key === '+' || event.key === '=') {
      event.preventDefault(); zoomAtCenter(ATLAS_ZOOM_STEP);
    } else if (event.key === '-') {
      event.preventDefault(); zoomAtCenter(-ATLAS_ZOOM_STEP);
    } else if (event.key.toLowerCase() === 'f') {
      event.preventDefault(); fitMap();
    } else if (event.key === '0') {
      event.preventDefault(); resetMap();
    }
  };
  const handleCanvasPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || (event.target !== event.currentTarget && event.target !== canvasContentRef.current)) return;
    const startViewport = view.viewport;
    panGestureRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startViewport,
      pendingViewport: startViewport,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    event.currentTarget.focus();
  };
  const handleCanvasPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const gesture = panGestureRef.current;
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const next = panAtlasViewport(gesture.startViewport, event.clientX - gesture.startX, event.clientY - gesture.startY);
    gesture.pendingViewport = next;
    setDragViewport(next);
  };
  const finishCanvasPointer = (event: ReactPointerEvent<HTMLDivElement>) => {
    const gesture = panGestureRef.current;
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    panGestureRef.current = null;
    commitViewport(gesture.pendingViewport);
    setDragViewport(null);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };

  return <section className="atlas-workbench" aria-label="Repository files and relationships">
    <div className="atlas-columns">
      <section className="atlas-tree-panel" aria-labelledby="atlas-tree-title">
        <header><div><strong id="atlas-tree-title">Files</strong><small aria-live="polite">{rows.length.toLocaleString()} paths · {flattened.length.toLocaleString()} shown</small></div></header>
        <label className="atlas-filter-search"><Search size={13} /><input data-focus-key="atlas-tree-search" aria-label="Search repository files and paths" placeholder="Find a file or folder" value={view.treeQuery} onChange={(event) => {
          const query = event.target.value;
          treeTopRef.current = 0;
          if (treeRef.current) treeRef.current.scrollTop = 0;
          setTreeViewport((current) => ({ ...current, scrollTop: 0 }));
          setFocusedTreeIndex(null);
          setView((s) => ({ ...s, treeQuery: query, treeTop: 0 }));
        }} /></label>
        {treeFilterPending && <span className="sr-only" role="status" aria-live="polite">Updating filtered paths.</span>}
        {view.selectedFolder && <button className="atlas-scope-chip" onClick={() => { treeTopRef.current = 0; if (treeRef.current) treeRef.current.scrollTop = 0; setTreeViewport((current) => ({ ...current, scrollTop: 0 })); setFocusedTreeIndex(null); setView((s) => ({ ...s, selectedFolder: null, treeTop: 0 })); onAtlasSelection(undefined, view.selectedPath ?? '', view.viewport); }}>Folder scope: {view.selectedFolder} <span>×</span></button>}
        <span className="sr-only" id="atlas-tree-keyboard-help">Use Up and Down Arrow to move between paths. Use Right Arrow to expand a folder or move to its first child, and Left Arrow to collapse or move to its parent. Press Enter to open a file or select a folder. Press Space to select a path.</span>
        <div className="atlas-tree-scroll" ref={treeRef} role={flattened.length > 0 ? 'tree' : undefined} aria-label={flattened.length > 0 ? 'Snapshot inventory tree' : undefined} aria-describedby={flattened.length > 0 ? 'atlas-tree-keyboard-help' : undefined} aria-busy={treeFilterPending} onScroll={(event) => {
          const tree = event.currentTarget;
          const scrollTop = tree.scrollTop;
          setTreeViewport({ scrollTop, height: tree.clientHeight || ATLAS_TREE_DEFAULT_VIEWPORT_HEIGHT });
          treeTopRef.current = scrollTop;
          if (treePersistTimerRef.current) clearTimeout(treePersistTimerRef.current);
          treePersistTimerRef.current = setTimeout(() => {
            setView((s) => s.treeTop === treeTopRef.current ? s : { ...s, treeTop: treeTopRef.current });
            treePersistTimerRef.current = null;
          }, 180);
        }}>
          {flattened.length === 0 ? <div className="atlas-empty-note"><strong>No paths match this search.</strong><span>Clear the search or widen the selected folder scope to see matching files.</span><button className="text-button" onClick={() => { treeTopRef.current = 0; if (treeRef.current) treeRef.current.scrollTop = 0; setTreeViewport((current) => ({ ...current, scrollTop: 0 })); setFocusedTreeIndex(null); setView((current) => ({ ...current, treeQuery: '', selectedFolder: null, treeTop: 0 })); }}>Clear search and folder scope</button></div> : <>
            {treeWindow.topSpacerHeight > 0 && <div className="atlas-tree-window-spacer" aria-hidden="true" style={{ height: `${treeWindow.topSpacerHeight}px` }} />}
            {visibleTreeRows.map((item, visibleIndex) => {
            const rowOrdinal = treeWindow.startIndex + visibleIndex;
            const row = item.row;
            const selected = row.path === view.selectedPath && !view.selectedRelationship;
            const openable = row.disposition === 'captured' && (row.contentKind === 'text' || row.contentKind === 'unknown');
            const rowIndexLabel = atlasRowIndexStatusLabel(atlasRowIndexStatus(row, index, indexState, stalePathSet.has(row.path)));
            return <div className={`atlas-tree-row${selected ? ' selected' : ''}${row.kind !== 'directory' && row.disposition !== 'captured' ? ' omitted' : ''}`} role="treeitem" data-tree-index={rowOrdinal} data-focus-key={`atlas-tree:${row.path}`} tabIndex={rowOrdinal === activeTreeIndex ? 0 : -1} aria-label={`${row.path === '.' ? 'repository root' : row.path}, ${row.kind}${row.kind === 'directory' ? '' : `, ${row.disposition}`}${rowIndexLabel ? `, ${rowIndexLabel}` : ''}`} aria-level={item.depth + 1} aria-posinset={item.positionInSet} aria-setsize={item.setSize} aria-selected={selected} aria-expanded={item.hasChildren ? item.expanded : undefined} key={`${row.path}:${row.reason}:${rowOrdinal}`} style={{ paddingLeft: `${7 + item.depth * 13}px` }} title={row.details} onFocus={() => setFocusedTreeIndex(rowOrdinal)} onClick={(event) => {
              if ((event.target as HTMLElement).closest('[data-tree-action]')) return;
              if (document.activeElement !== event.currentTarget) event.currentTarget.focus();
              choosePath(row);
              if (row.kind === 'file' && openable) onOpenFile(row.path, snapshotId);
            }} onKeyDown={(event) => {
              const action = atlasTreeKeyboardAction(flattened, rowOrdinal, event.key);
              if (action.kind === 'none') return;
              event.preventDefault();
              if (action.kind === 'focus') focusTreeRow(action.index);
              else if (action.kind === 'expand') setTreeFolderExpanded(action.path, true);
              else if (action.kind === 'collapse') setTreeFolderExpanded(action.path, false);
              else if (action.kind === 'activate') {
                choosePath(row);
                if (openable) onOpenFile(row.path, snapshotId);
              } else if (action.kind === 'select') choosePath(row);
            }}>
              {item.hasChildren ? <span className="atlas-tree-toggle" data-tree-action aria-hidden="true" onClick={() => setTreeFolderExpanded(item.path, !item.expanded)}><ChevronDown size={12} className={item.expanded ? '' : 'collapsed'} /></span> : <span className="atlas-tree-spacer" />}
              <span className="atlas-tree-entry" aria-hidden="true">{row.kind !== 'directory' && row.disposition !== 'captured' && <span className={`atlas-state-mark ${row.disposition}`} />}{row.kind === 'directory' ? <FolderOpen size={13} /> : <FileCode2 size={13} />}<code>{row.path === '.' ? 'repository root' : row.path.split('/').at(-1)}</code>{row.kind !== 'directory' && row.disposition !== 'captured' && <small>{row.disposition}</small>}</span>
              {openable && <span className="atlas-tree-open" data-tree-action aria-hidden="true" title="Open source" onClick={() => { choosePath(row); onOpenFile(row.path, snapshotId); }}><ArrowRight size={12} /></span>}
            </div>;
            })}
            {treeWindow.bottomSpacerHeight > 0 && <div className="atlas-tree-window-spacer" aria-hidden="true" style={{ height: `${treeWindow.bottomSpacerHeight}px` }} />}
          </>}
        </div>
        <footer><button className="text-button" aria-pressed={showSkipped} onClick={() => { setShowSkipped((value) => !value); setFocusedTreeIndex(null); }}>{showSkipped ? 'Hide skipped and excluded paths' : 'Show skipped and excluded paths'}</button></footer>
      </section>

      <CodeLensEmptyState
        inline
        projectId={projectId}
        snapshotId={snapshotId}
        manifest={manifest}
        selectedPath={selectedPath}
        source={source}
        sourceRange={sourceRange}
        sourceRangeIssue={sourceRangeIssue}
        loading={sourceLoading}
        error={sourceError}
        onOpenFile={onOpenFile}
        onRequestExplanation={onRequestExplanation}
        onRequestRangeExplanation={onRequestRangeExplanation}
      />
      <FileExplanationPanel
        onEnsureIndex={() => { if (!index && !startingIndex && !indexJobId && indexState !== 'loading') void startIndex(); }}
        index={index} edges={provenEdges} indexLoading={indexState === 'loading'}
        projectId={projectId}
        snapshotId={snapshotId}
        selectedPath={selectedPath}
        onOpenFile={onOpenFile}
        onRequestExplanation={onRequestExplanation}
        onReviewRevision={onReviewRevision}
        onReadRevision={onReadRevision}
      />

      <details className="atlas-relationship-details">
        <summary>Explore code relationships <span>Optional · verified links from the snapshot index</span></summary>
        <div className="atlas-map-columns">
      <section className="atlas-map-panel" aria-labelledby="atlas-map-title">
        <header><div><strong id="atlas-map-title">Relationships</strong><small>{shownEdges.length} shown · {provenEdges.length} proven for snapshot</small></div><span className="atlas-snapshot-badge">{index?.status ?? (indexState === 'loading' ? 'indexing' : indexState)}</span></header>
        <div className="atlas-map-filters"><label className="atlas-filter-search"><Search size={13} /><input aria-label="Search relationships" placeholder="Search symbol, path, or evidence" value={view.mapQuery} onChange={(event) => setView((s) => ({ ...s, mapQuery: event.target.value }))} /></label><label><span>Type</span><select aria-label="Filter relationships by type" value={view.kind} onChange={(event) => setView((s) => ({ ...s, kind: event.target.value as AtlasWorkbenchViewState['kind'] }))}><option value="all">All types</option><option value="calls">Calls</option><option value="imports">Imports</option><option value="testReference">Test references</option><option value="defines">Defines</option><option value="exports">Exports</option></select></label><label><span>Language</span><select aria-label="Filter relationships by language" value={view.language} onChange={(event) => setView((s) => ({ ...s, language: event.target.value }))}><option value="all">All languages</option>{languages.map((language) => <option key={language} value={language}>{language}</option>)}</select></label></div>
        {(view.selectedFolder || view.kind !== 'all' || view.language !== 'all') && <div className="atlas-active-filter-chips" role="group" aria-label="Active relationship filters">
          {view.selectedFolder && <button className="atlas-active-filter-chip" onClick={() => setView((s) => ({ ...s, selectedFolder: null }))}>Path: {view.selectedFolder} <span aria-hidden="true">×</span></button>}
          {view.kind !== 'all' && <button className="atlas-active-filter-chip" onClick={() => setView((s) => ({ ...s, kind: 'all' }))}>Type: {view.kind} <span aria-hidden="true">×</span></button>}
          {view.language !== 'all' && <button className="atlas-active-filter-chip" onClick={() => setView((s) => ({ ...s, language: 'all' }))}>Language: {view.language} <span aria-hidden="true">×</span></button>}
          <button className="atlas-clear-filters" onClick={() => setView((s) => ({ ...s, selectedFolder: null, kind: 'all', language: 'all' }))}>Clear filters</button>
        </div>}
        {indexState === 'loading' && <div className="atlas-index-banner" role="status">{indexProgress ? `Indexing ${indexProgress.filesProcessed}/${indexProgress.filesTotal} files · ${indexProgress.relationshipsFound} relationships` : 'Loading the local snapshot index…'}</div>}
        {indexState === 'pending' && <div className="atlas-index-banner" role="status"><strong>Index pending</strong><span>{indexError ?? 'No saved index is available for this snapshot yet.'}</span>{!indexError && <button onClick={() => { void startIndex(); }} disabled={startingIndex}>{startingIndex ? 'Starting…' : 'Index this snapshot'}</button>}</div>}
        {indexState === 'error' && <div className="atlas-index-banner stale" role="alert"><strong>Index unavailable</strong><span>{indexError}</span><button onClick={() => { setIndexError(null); setIndexState('pending'); }}>Try indexing</button></div>}
        {indexState === 'ready' && stalePaths.length > 0 && <div className="atlas-index-banner stale" role="status"><strong>Stale snapshot evidence</strong><span>{stalePaths.length} indexed file hash{stalePaths.length === 1 ? '' : 'es'} differ from the selected snapshot; affected edges are withheld.</span>{isNativeRuntime() && <button onClick={() => { void startIndex(); }} disabled={startingIndex || Boolean(indexJobId)}>{startingIndex || indexJobId ? 'Re-indexing…' : 'Re-index snapshot'}</button>}</div>}
        {indexState === 'ready' && unsupportedCount > 0 && <div className="atlas-index-banner unsupported" role="status"><strong>{unsupportedCount} text-only path{unsupportedCount === 1 ? '' : 's'}</strong><span>Captured source stays available, but these files do not contribute local symbols or relationships.</span></div>}
        <div className="atlas-canvas-toolbar" role="group" aria-label="Relationship map viewport controls">
          <span>Zoom {Math.round((dragViewport ?? view.viewport).scale * 100)}% · drag empty canvas to pan</span>
          <button aria-label="Zoom out relationship map" title="Zoom out (−)" onClick={() => zoomAtCenter(-ATLAS_ZOOM_STEP)} disabled={(dragViewport ?? view.viewport).scale <= ATLAS_MIN_ZOOM}>−</button>
          <button aria-label="Zoom in relationship map" title="Zoom in (+)" onClick={() => zoomAtCenter(ATLAS_ZOOM_STEP)} disabled={(dragViewport ?? view.viewport).scale >= ATLAS_MAX_ZOOM}>+</button>
          <button onClick={fitMap}>Fit</button>
          <button onClick={resetMap}>Reset</button>
        </div>
        <div className={`atlas-relationship-canvas${dragViewport ? ' is-panning' : ''}`} ref={canvasRef} role="group" tabIndex={0} aria-label="Relationship map viewport. Use arrow keys to pan, plus and minus to zoom, F to fit, or 0 to reset." aria-keyshortcuts="ArrowUp ArrowDown ArrowLeft ArrowRight + - F 0" onKeyDown={handleCanvasKeyDown} onPointerDown={handleCanvasPointerDown} onPointerMove={handleCanvasPointerMove} onPointerUp={finishCanvasPointer} onPointerCancel={finishCanvasPointer}>
          <div className="atlas-canvas-content" ref={canvasContentRef} style={{ transform: `translate(${(dragViewport ?? view.viewport).x}px, ${(dragViewport ?? view.viewport).y}px) scale(${(dragViewport ?? view.viewport).scale})` }}>
          {indexState === 'ready' && shownEdges.length > 0 ? shownEdges.map((edge) => {
            const from = index?.symbols.find((symbol) => symbol.symbolId === edge.sourceId);
            const to = index?.symbols.find((symbol) => symbol.symbolId === edge.targetId);
            return <article className={`atlas-edge${view.selectedRelationship === edge.relationship.relationshipId ? ' selected' : ''}`} key={edge.relationship.relationshipId}>
              <button className="atlas-edge-node" onClick={() => from && chooseSymbol(from.symbolId, from.path)}><small>FROM · {from?.kind ?? 'symbol'}</small><strong>{from?.qualifiedName ?? edge.sourceId}</strong><code>{from?.path}</code></button>
              <button className="atlas-edge-link" onClick={() => chooseEdge(edge)}><span>{edge.relationship.kind}</span><i /><small>high confidence</small></button>
              <button className="atlas-edge-node" onClick={() => to && chooseSymbol(to.symbolId, to.path)}><small>TO · {to?.kind ?? 'symbol'}</small><strong>{to?.qualifiedName ?? edge.targetId}</strong><code>{to?.path}</code></button>
              <button className="atlas-edge-evidence" onClick={() => chooseEdge(edge)}>{edge.evidence.path}:{edge.evidence.startLine}–{edge.evidence.endLine}<span>Inspect evidence</span></button>
            </article>;
          }) : indexState === 'ready' ? <div className="atlas-empty-note"><strong>{provenEdges.length ? 'No relationships match these filters.' : 'No proven relationships in this snapshot.'}</strong><span>{provenEdges.length ? 'Clear search, select another type, or widen the folder scope.' : unresolvedCount ? `${unresolvedCount} unresolved or dynamic links were withheld from the proven map.` : 'The saved index contains no resolved relationships.'}</span></div> : null}
          </div>
        </div>
        <footer>{indexState === 'ready' ? `${unresolvedCount} unresolved/dynamic links withheld · ${provenEdges.length} verified edges` : 'Map contents are limited to the selected immutable snapshot.'}</footer>
      </section>

      <aside className="atlas-details-panel" aria-labelledby="atlas-details-title">
        <header><div><strong id="atlas-details-title">Evidence details</strong><small>Snapshot {snapshotId.slice(-12)}</small></div></header>
        {selectedEdge ? <div className="atlas-detail-content"><span className="atlas-detail-kicker">PROVEN · HIGH CONFIDENCE</span><h3>{selectedEdge.relationship.kind}</h3><code>{index?.symbols.find((symbol) => symbol.symbolId === selectedEdge.sourceId)?.qualifiedName} → {index?.symbols.find((symbol) => symbol.symbolId === selectedEdge.targetId)?.qualifiedName}</code><dl><div><dt>Evidence</dt><dd>{selectedEdge.evidence.path}:{selectedEdge.evidence.startLine}–{selectedEdge.evidence.endLine}</dd></div><div><dt>SHA-256</dt><dd><code>{selectedEdge.evidence.sha256}</code></dd></div></dl><button className="atlas-open-evidence" data-focus-key={`atlas-open-edge:${selectedEdge.relationship.relationshipId}`} onClick={() => openEdgeEvidence(selectedEdge)}><FileCode2 size={14} /> Open exact source evidence</button></div>
            : selectedSymbol ? <div className="atlas-detail-content"><span className="atlas-detail-kicker">INDEXED SYMBOL</span><h3>{selectedSymbol.qualifiedName}</h3><code>{selectedSymbol.path}</code><p>{selectedSymbol.kind} · {selectedSymbol.language} · lines {selectedSymbol.anchor.startLine}–{selectedSymbol.anchor.endLine}</p><dl><div><dt>SHA-256</dt><dd><code>{selectedSymbol.anchor.sourceSha256}</code></dd></div></dl><button className="atlas-open-evidence" data-focus-key={`atlas-open-symbol:${selectedSymbol.symbolId}`} onClick={() => onOpenFile(selectedSymbol.path, snapshotId, codeLensRangeFromIndexedAnchor(selectedSymbol.anchor))}><FileCode2 size={14} /> Open symbol source</button></div>
            : rawRelationship ? <div className="atlas-detail-content"><span className="atlas-detail-kicker warning">NOT PROVEN</span><h3>{rawRelationship.kind} · {rawRelationship.confidence}</h3><p>This relationship is unresolved or its exact snapshot evidence is unavailable, so it is withheld from the canvas.</p><code>{rawRelationship.evidence.path}</code><small>{rawRelationship.unresolvedReason ?? 'Relationship did not pass endpoint and hash verification.'}</small><button className="atlas-open-evidence" data-focus-key={`atlas-open-relationship:${rawRelationship.relationshipId}`} onClick={() => onOpenFile(rawRelationship.evidence.path, snapshotId, codeLensRangeFromIndexedAnchor(rawRelationship.evidence))}><FileCode2 size={14} /> Inspect indexed anchor</button></div>
                : selectedRow ? <div className="atlas-detail-content"><span className="atlas-detail-kicker">{selectedRow.kind} · {selectedRow.disposition}</span><h3>{selectedRow.path === '.' ? 'Repository root' : selectedRow.path.split('/').at(-1)}</h3><code>{selectedRow.path}</code><dl><div><dt>Language</dt><dd>{selectedRow.language}</dd></div><div><dt>Capture</dt><dd>{inventoryReasonLabel(selectedRow.reason)}</dd></div><div><dt>Language support</dt><dd>{languageSupportLabel(languageSupportTier(selectedRow))}</dd></div><div><dt>Local index</dt><dd>{atlasRowIndexStatusLabel(atlasRowIndexStatus(selectedRow, index, indexState, stalePathSet.has(selectedRow.path))) ?? 'not applicable'}</dd></div>{selectedIndexCounts && <><div><dt>Indexed files</dt><dd>{selectedIndexCounts.indexedFiles}</dd></div><div><dt>Symbols</dt><dd>{selectedIndexCounts.symbols}</dd></div><div><dt>Relationships</dt><dd>{selectedIndexCounts.relationships}</dd></div></>}{selectedRow.sizeBytes !== null && <div><dt>Size</dt><dd>{formatBytes(selectedRow.sizeBytes)}</dd></div>}{selectedRow.sha256 && <div><dt>SHA-256</dt><dd><code>{selectedRow.sha256}</code></dd></div>}</dl>{selectedRow.contentKind === 'text' && <p className="atlas-detail-note">{languageSupportNote(languageSupportTier(selectedRow), selectedRow.language)}</p>}{selectedRow.disposition !== 'captured' && <p className="atlas-detail-note warning">No source bytes were captured. {selectedRow.details}</p>}{selectedRow.kind === 'directory' && <p className="atlas-detail-note">Folder selection scopes the relationship canvas to descendants.</p>}{selectedRow.disposition === 'captured' && (selectedRow.contentKind === 'text' || selectedRow.contentKind === 'unknown') && <button className="atlas-open-evidence" data-focus-key={`atlas-open-file:${selectedRow.path}`} onClick={() => onOpenFile(selectedRow.path, snapshotId)}><FileCode2 size={14} /> Open file source</button>}</div>
                : <div className="atlas-detail-empty"><Network size={18} /><strong>Select a folder, file, symbol, or edge.</strong><span>Details retain exact snapshot path and source hash evidence.</span></div>}
        <footer>Dynamic and unresolved relationships are never presented as proven.</footer>
      </aside>
        </div>
      </details>
    </div>
  </section>;
}

function FileExplanationPanel({ projectId, snapshotId, selectedPath, index, edges, indexLoading, onEnsureIndex, onOpenFile, onRequestExplanation, onReviewRevision, onReadRevision }: {
  index: SnapshotIndex | null;
  edges: ProvenGraphEdge[];
  indexLoading: boolean;
  onEnsureIndex: () => void;
  projectId: string;
  snapshotId: string;
  selectedPath: string | null;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onRequestExplanation: (path: string) => void;
  onReviewRevision: (revision: ChapterRevision) => void;
  onReadRevision: (revision: ChapterRevision) => void;
}) {
  const [panel, setPanel] = useState<'explanation' | 'flow'>('explanation');
  const [revisions, setRevisions] = useState<ChapterRevision[]>([]);
  const [loading, setLoading] = useState(isNativeRuntime());
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const refresh = () => setRefreshKey((value) => value + 1);
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => window.removeEventListener('code-atlas:chapters-changed', refresh);
  }, []);

  useEffect(() => {
    if (!isNativeRuntime()) {
      setLoading(false);
      setError(null);
      return;
    }
    let active = true;
    setLoading(true);
    setError(null);
    void listProjectChapterRevisions(projectId)
      .then((saved) => { if (active) setRevisions(saved); })
      .catch((loadError) => { if (active) setError(errorMessageOr(loadError, 'This file’s saved explanation could not be loaded.')); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [projectId, snapshotId, refreshKey]);

  const pathRevisions = selectedPath ? revisions.filter((revision) => (
    revision.projectId === projectId
      && revision.target.scope.kind === 'file'
      && revision.target.scope.path === selectedPath
  )) : [];
  const currentRevisions = pathRevisions.filter((revision) => revision.snapshotId === snapshotId)
    .sort((left, right) => right.revisionNumber - left.revisionNumber || right.createdAtUnixMs - left.createdAtUnixMs);
  const latestCurrent = currentRevisions[0] ?? null;
  const accepted = latestCurrent?.lifecycle === 'accepted' ? latestCurrent : null;
  const currentDraft = latestCurrent && latestCurrent.lifecycle !== 'accepted' ? latestCurrent : null;
  const previousAccepted = pathRevisions
    .filter((revision) => revision.lifecycle === 'accepted')
    .sort((left, right) => right.revisionNumber - left.revisionNumber || right.createdAtUnixMs - left.createdAtUnixMs)[0] ?? null;

  return <aside className="file-explanation-panel" aria-labelledby="file-explanation-title">
    <div className="file-tool-tabs" role="group" aria-label="File tools">
      <button aria-pressed={panel === 'explanation'} onClick={() => setPanel('explanation')}><BookOpen size={14} /> Explanation</button>
      <button aria-pressed={panel === 'flow'} onClick={() => { setPanel('flow'); onEnsureIndex(); }}><Workflow size={14} /> Flow</button>
      <button disabled={!accepted} onClick={() => accepted && onReadRevision(accepted)} title={accepted ? 'Open the section player' : 'Accept an explanation to open its player'}><Play size={13} /> Player</button>
    </div>
    <div className="file-tool-content">
    <header>
      <span className="file-explanation-kicker">{panel === 'flow' ? 'CONNECTED CODE' : 'THIS FILE'}</span>
      <h2 id="file-explanation-title">{selectedPath ? selectedPath.split('/').at(-1) : 'Understand your code'}</h2>
    </header>
    {panel === 'flow' ? <FileFlow key={selectedPath} path={selectedPath} index={index} edges={edges} loading={indexLoading} onOpenFile={onOpenFile} />
      : !selectedPath ? <div className="file-explanation-empty"><BookOpen size={24} /><strong>Start with a file.</strong><span>Choose one in the explorer. Its source and explanation stay side by side.</span></div>
      : !isNativeRuntime() ? <p className="file-explanation-message">Open the Mac app to browse your code and generate explanations.</p>
        : loading ? <p className="file-explanation-message" role="status">Loading explanation…</p>
          : error ? <div className="file-explanation-message" role="alert"><span>{error}</span><button className="text-button" onClick={() => setRefreshKey((value) => value + 1)}>Retry</button></div>
            : accepted ? <>
              <div className="file-explanation-status"><span><Check size={12} /> Source-linked · saved explanation</span></div>
              <div className="file-explanation-sections">
                {accepted.sections.map((section, sectionIndex) => {
                  const anchors = Array.from(new Map(section.claims.flatMap((claim) => claim.anchors)
                    .map((anchor) => [`${anchor.path}:${anchor.startLine}-${anchor.endLine}:${anchor.sourceSha256}`, anchor] as const)).values());
                  const visibleAnchors = anchors.slice(0, 4);
                  const additionalAnchors = anchors.slice(4);
                  const citationLabel = (anchor: ChapterSourceAnchor) => anchor.path === selectedPath
                    ? `Lines ${anchor.startLine}–${anchor.endLine}` : `${anchor.path}:${anchor.startLine}–${anchor.endLine}`;
                  return <article key={section.sectionId}>
                    <h3><span className="file-step-number">{sectionIndex + 1}</span>{section.title}</h3>
                    <p>{section.prose}</p>
                    {visibleAnchors.length > 0 && <div className="file-explanation-citations" role="group" aria-label={`Evidence for ${section.title}`}>
                      {visibleAnchors.map((anchor) => <button key={anchor.anchorId} title={`${anchor.path}:${anchor.startLine}–${anchor.endLine}`} onClick={() => onOpenFile(anchor.path, anchor.snapshotId, codeLensRangeFromChapterAnchor(anchor))}>
                        <FileCode2 size={12} /><span>{citationLabel(anchor)}</span><ArrowRight size={12} />
                      </button>)}
                    </div>}
                    {additionalAnchors.length > 0 && <details className="file-explanation-more-citations">
                      <summary>{additionalAnchors.length} more source links</summary>
                      <div className="file-explanation-citations">
                        {additionalAnchors.map((anchor) => <button key={anchor.anchorId} onClick={() => onOpenFile(anchor.path, anchor.snapshotId, codeLensRangeFromChapterAnchor(anchor))}>
                          <FileCode2 size={12} /><span>{citationLabel(anchor)}</span>
                        </button>)}
                      </div>
                    </details>}
                  </article>;
                })}
              </div>
              <button className="file-explanation-review" onClick={() => onReviewRevision(accepted)}><Pencil size={13} /> Edit explanation</button>
            </> : currentDraft ? <>
              <div className="file-explanation-empty"><span className="coverage-status coverage-status-draft">{currentDraft.lifecycle === 'rejected' ? 'Needs revision' : 'Draft ready'}</span><strong>Review the explanation.</strong><span>Check the draft against its source before accepting it.</span></div>
              <button className="file-explanation-review" onClick={() => onReviewRevision(currentDraft)}><Pencil size={13} /> Review draft</button>
            </> : <>
              {previousAccepted && <p className="file-explanation-message">The saved explanation describes an older snapshot. Generate one for this version.</p>}
              <div className="file-explanation-empty"><Sparkles size={24} /><strong>What does this file do?</strong><span>Generate an explanation of its behavior, with links to the code that supports it.</span>
              <button className="primary-button" onClick={() => onRequestExplanation(selectedPath)}><Sparkles size={14} /> Explain this file</button>
              <small>Your backend and model are remembered. Review the source before sending.</small></div>
            </>}
    </div>
  </aside>;
}

function inventoryFilterKey(row: Pick<InventoryRow, 'path' | 'kind' | 'reason'>): string {
  return `${row.path}\u0000${row.kind}\u0000${row.reason}`;
}

function GuidePlanScreen({ projectId, projectName, manifest, rows, returnFocusTarget, onPlan, onGoAtlas, onGoChanges, onGoStudio, onGenerateOverview, onGenerateFile, onGeneratePlannedChapter, onReadRevision, onReviewRevision, onOpenFile }: {
  projectId: string;
  projectName: string;
  manifest: SnapshotManifest;
  rows: InventoryRow[];
  returnFocusTarget?: string;
  onPlan: (input: PrepareRepositoryPlanInput) => void;
  onGoAtlas: () => void;
  onGoChanges: () => void;
  onGoStudio: () => void;
  onGenerateOverview: () => void;
  onGenerateFile: (path: string, selectedPaths: string[]) => void;
  onGeneratePlannedChapter: (scope: PlannedChapterScope) => void;
  onReadRevision: (revision: ChapterRevision) => void;
  onReviewRevision: (revision: ChapterRevision) => void;
  onOpenFile: (path: string, snapshotId?: string, range?: SourceLineRange) => void;
}) {
  const snapshotId = manifest.summary.snapshotId;
  const [plan, setPlan] = useState<CoveragePlan>(() => loadCoveragePlan(projectId, snapshotId));
  const [query, setQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | CoverageStatus>('all');
  const [guideView, setGuideView] = useState<'needs-explanation' | 'explained' | 'all'>('needs-explanation');
  const [languageFilter, setLanguageFilter] = useState('all');
  const [captureFilter, setCaptureFilter] = useState<'all' | CaptureDisposition>('all');
  const [parserFilter, setParserFilter] = useState<'all' | ParserReadiness>('all');
  const [contentFilter, setContentFilter] = useState<'all' | InventoryRow['contentKind']>('all');
  const [categoryFilter, setCategoryFilter] = useState<'all' | InventoryCategory>('all');
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(50);
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [groupFormOpen, setGroupFormOpen] = useState(false);
  const [groupingOpen, setGroupingOpen] = useState(false);
  const [groupName, setGroupName] = useState('');
  const [groupRationale, setGroupRationale] = useState('');
  const [excludePath, setExcludePath] = useState<string | null>(null);
  const [excludeReason, setExcludeReason] = useState('');
  const [feedback, setFeedback] = useState<string | null>(null);
  const [storageAvailable, setStorageAvailable] = useState(true);
  const [revisions, setRevisions] = useState<ChapterRevision[]>([]);
  const [revisionsLoading, setRevisionsLoading] = useState(isNativeRuntime());
  const [revisionsError, setRevisionsError] = useState<string | null>(null);
  const [revisionRefresh, setRevisionRefresh] = useState(0);
  const coverageReady = !revisionsLoading && !revisionsError;
  const coverageRows = useMemo(() => buildCoverageRows(rows, plan, revisions)
    .map((row) => coverageReady ? row : { ...row, canSelectForGroup: false, canExclude: false }), [rows, plan, revisions, coverageReady]);
  const totals = useMemo(() => summarizeCoverage(coverageRows), [coverageRows]);
  const inventoryMatches = useMemo(() => new Set(filterInventory(rows, {
    query: '',
    language: languageFilter,
    disposition: captureFilter,
    parser: parserFilter,
    content: contentFilter,
    category: categoryFilter,
  }).map(inventoryFilterKey)), [captureFilter, categoryFilter, contentFilter, languageFilter, parserFilter, rows]);
  const guideRows = useMemo(() => coverageRows.filter((row) => guideView === 'all'
    || (guideView === 'explained' ? row.status === 'accepted' : ['missing', 'draft', 'rejected', 'stale'].includes(row.status) || (row.status === 'unsupported' && row.canOpenSource))), [coverageRows, guideView]);
  const filtered = useMemo(() => filterCoverageRows(guideRows, query, statusFilter)
    .filter((row) => inventoryMatches.has(inventoryFilterKey(row.inventory))), [guideRows, inventoryMatches, query, statusFilter]);
  const pageData = useMemo(() => paginateCoverage(filtered, page, pageSize), [filtered, page, pageSize]);
  const languages = useMemo(() => Array.from(new Set(rows.map((row) => row.language))).sort(), [rows]);

  useEffect(() => {
    setStorageAvailable(saveCoveragePlan(plan));
  }, [plan]);

  useEffect(() => {
    const refresh = () => setRevisionRefresh((value) => value + 1);
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => window.removeEventListener('code-atlas:chapters-changed', refresh);
  }, []);

  useEffect(() => {
    if (!isNativeRuntime()) return;
    let active = true;
    setRevisionsLoading(true);
    setRevisionsError(null);
    void listProjectChapterRevisions(projectId)
      .then((saved) => { if (active) setRevisions(saved); })
      .catch((error) => { if (active) setRevisionsError(errorMessageOr(error, 'Saved chapter revisions could not be loaded. Coverage counts are incomplete until you retry.')); })
      .finally(() => { if (active) setRevisionsLoading(false); });
    return () => { active = false; };
  }, [projectId, snapshotId, revisionRefresh]);

  const setQueryFilter = (value: string) => { setQuery(value); setPage(0); };
  const setStatus = (value: 'all' | CoverageStatus) => { setStatusFilter(value); setGuideView('all'); setPage(0); };
  const chooseGuideView = (value: 'needs-explanation' | 'explained' | 'all') => {
    setGuideView(value);
    setStatusFilter('all');
    setPage(0);
  };
  const openRevision = (revisionId: string, action: 'read' | 'review') => {
    const revision = revisions.find((item) => item.revisionId === revisionId);
    if (!revision) return;
    if (action === 'read') onReadRevision(revision);
    else onReviewRevision(revision);
  };
  const visibleEligible = pageData.rows.filter((row) => row.canSelectForGroup).length;
  const allVisibleSelected = visibleEligible > 0 && pageData.rows
    .filter((row) => row.canSelectForGroup)
    .every((row) => selectedPaths.includes(row.inventory.path));
  const toggleSelected = (path: string) => setSelectedPaths((current) => current.includes(path)
    ? current.filter((item) => item !== path)
    : [...current, path]);
  const toggleVisible = () => {
    const visible = pageData.rows.filter((row) => row.canSelectForGroup).map((row) => row.inventory.path);
    setSelectedPaths((current) => allVisibleSelected
      ? current.filter((path) => !visible.includes(path))
      : Array.from(new Set([...current, ...visible])));
  };
  const startGroup = () => {
    setFeedback(null);
    setGroupName('');
    setGroupRationale('');
    setGroupFormOpen(true);
  };
  const submitGroup = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!coverageReady) return;
    try {
      const next = addCoverageGroup(plan, {
        groupId: makeCoverageId(),
        name: groupName,
        rationale: groupRationale,
        paths: selectedPaths,
        createdAtUnixMs: Date.now(),
      }, rows, revisions);
      setPlan(next);
      setSelectedPaths([]);
      setGroupFormOpen(false);
      setFeedback('Group created. Each file remains listed and keeps its own source link.');
    } catch (error) {
      setFeedback(errorMessageOr(error, 'The group could not be created.'));
    }
  };
  const submitExclusion = (event: FormEvent<HTMLFormElement>, row: CoverageRow) => {
    event.preventDefault();
    if (!coverageReady) return;
    try {
      setPlan((current) => excludeCoveragePath(current, row, excludeReason));
      setSelectedPaths((current) => current.filter((path) => path !== row.inventory.path));
      setExcludePath(null);
      setExcludeReason('');
      setFeedback(`${row.inventory.path} is excluded from this guide plan. You can include it again at any time.`);
    } catch (error) {
      setFeedback(errorMessageOr(error, 'The file could not be excluded.'));
    }
  };
  const dissolveGroup = (groupId: string, name: string) => {
    setPlan((current) => removeCoverageGroup(current, groupId));
    setFeedback(`“${name}” was removed. Its files are independent chapters again.`);
  };
  const statusLabel: Record<CoverageStatus, string> = {
    missing: 'Needs explanation', draft: 'Draft', rejected: 'Rejected', stale: 'Older snapshot', accepted: 'Accepted',
    excluded: 'Excluded by you', unsupported: 'Text only', unavailable: 'Source unavailable',
  };
  const acceptedOverviews = revisions
    .filter((revision) => revision.lifecycle === 'accepted' && revision.target.scope.kind === 'overview')
    .sort((left, right) => right.createdAtUnixMs - left.createdAtUnixMs || right.revisionNumber - left.revisionNumber);
  const acceptedOverview = acceptedOverviews.find((revision) => revision.snapshotId === snapshotId) ?? null;
  const earlierOverview = acceptedOverviews.find((revision) => revision.snapshotId !== snapshotId) ?? null;
  const currentOverviewReview = revisions
    .filter((revision) => revision.snapshotId === snapshotId && revision.target.scope.kind === 'overview')
    .sort((left, right) => right.createdAtUnixMs - left.createdAtUnixMs || right.revisionNumber - left.revisionNumber)
    .find((revision) => revision.lifecycle !== 'accepted') ?? null;
  const nextFiles = coverageRows.filter((row) => row.canOpenSource
    && ['missing', 'draft', 'rejected', 'stale', 'unsupported'].includes(row.status)).slice(0, 3);
  const explainedFiles = coverageRows.filter((row) => row.status === 'accepted').slice(0, 3);
  const firstFileToOpen = nextFiles[0] ?? explainedFiles[0] ?? null;
  const canBuildOverview = rows.some((row) => row.disposition === 'captured' && row.kind === 'file'
    && (row.contentKind === 'text' || row.contentKind === 'unknown'));
  const guideEligibleTotal = coverageRows.filter((row) => row.canOpenSource && row.status !== 'excluded').length;
  const guideProgress = guideEligibleTotal > 0 ? Math.round((totals.accepted / guideEligibleTotal) * 100) : 0;

  return (
    <div className="empty-workspace guide-plan-screen">
      <section className="guide-question-panel" aria-label="Ask the code">
      <RepositoryPlans key={`${projectId}:${snapshotId}`} manifest={manifest} onPlan={onPlan} onOpenFile={onOpenFile} onGenerateFile={onGenerateFile} onGeneratePlannedChapter={onGeneratePlannedChapter}
        chapters={revisions} chaptersLoading={revisionsLoading} chaptersError={revisionsError} onReadChapter={onReadRevision} onReviewChapter={onReviewRevision} />
      </section>

      <section className="guide-home-dashboard" aria-labelledby="guide-home-title">
        <div className="guide-home-context">
          <span className="step-pill"><span>YOUR REPOSITORY GUIDE</span> {projectName}</span>
          <p>{manifest.summary.branch ?? 'Detached HEAD'} · {manifest.summary.filesCaptured.toLocaleString()} captured files · {languages.length} detected languages</p>
        </div>
        <div className="guide-home-grid">
          <section className="guide-tour-card" aria-labelledby="guide-home-title">
            <div className="guide-tour-icon"><Play size={19} /></div>
            {acceptedOverview ? <>
              <div className="guide-tour-copy">
                <span className="guide-home-kicker">{acceptedOverview.sections.length} SECTION{acceptedOverview.sections.length === 1 ? '' : 'S'} · ACCEPTED REPOSITORY GUIDE</span>
                <h2 id="guide-home-title">{acceptedOverview.title}</h2>
                <p>{acceptedOverview.sections[0]?.prose || 'Follow the repository explanation one section at a time. Open its cited code whenever you want to inspect the evidence.'}</p>
                <span className="guide-tour-pacing">Pause, skip, or open the exact source at any point.</span>
              </div>
              <div className="guide-tour-actions">
                <button className="primary-button" onClick={() => onReadRevision(acceptedOverview)}>Read the guide <ArrowRight size={15} /></button>
                {earlierOverview && <button className="text-button" onClick={() => onReadRevision(earlierOverview)}>Read guide for an earlier snapshot</button>}
              </div>
            </> : revisionsLoading ? <>
              <div className="guide-tour-copy">
                <span className="guide-home-kicker">CHECKING THIS SNAPSHOT</span>
                <h2 id="guide-home-title">Looking for a saved repository guide.</h2>
                <p>While the local guide index loads, you can browse captured files or compare revisions.</p>
              </div>
            </> : revisionsError ? <>
              <div className="guide-tour-copy">
                <span className="guide-home-kicker">GUIDE STATUS UNAVAILABLE</span>
                <h2 id="guide-home-title">Saved explanations could not be loaded.</h2>
                <p>{revisionsError}</p>
                <button className="secondary-button" onClick={() => setRevisionRefresh((value) => value + 1)}>Retry</button>
              </div>
            </> : currentOverviewReview ? <>
              <div className="guide-tour-copy">
                <span className="guide-home-kicker">REPOSITORY TOUR · {currentOverviewReview.lifecycle === 'rejected' ? 'NEEDS REVISION' : 'DRAFT SAVED'}</span>
                <h2 id="guide-home-title">Review the repository tour draft.</h2>
                <p>This draft is tied to the selected snapshot. Review its source links and accept it before it appears as the Guide tour.</p>
              </div>
              <div className="guide-tour-actions">
                <button className="primary-button" onClick={() => onReviewRevision(currentOverviewReview)}>Review tour draft <ArrowRight size={15} /></button>
                {canBuildOverview && <button className="secondary-button" onClick={onGenerateOverview}>Create another tour</button>}
                {earlierOverview && <button className="text-button" onClick={() => onReadRevision(earlierOverview)}>Read guide for an earlier snapshot</button>}
              </div>
            </> : <>
              <div className="guide-tour-copy">
                <span className="guide-home-kicker">OPTIONAL REPOSITORY TOUR</span>
                <h2 id="guide-home-title">Build a short guide through this codebase.</h2>
                <p>A tour connects a few evidenced steps. Every file still gets its own explanation and coverage status; review the selected source and request before anything is sent.</p>
              </div>
              <div className="guide-tour-actions">
                {canBuildOverview && <button className="primary-button" onClick={onGenerateOverview}><Play size={14} /> Create repository tour <ArrowRight size={15} /></button>}
                {firstFileToOpen && <button className={canBuildOverview ? 'secondary-button' : 'primary-button'} onClick={() => onOpenFile(firstFileToOpen.inventory.path)}><FileCode2 size={14} /> {firstFileToOpen.status === 'accepted' ? 'Read' : 'Explain'} {firstFileToOpen.inventory.path}</button>}
                {earlierOverview && <button className="text-button" onClick={() => onReadRevision(earlierOverview)}>Read guide for an earlier snapshot</button>}
                {!canBuildOverview && !firstFileToOpen && <button className="primary-button" onClick={onGoAtlas}><FileCode2 size={14} /> Browse captured files</button>}
              </div>
            </>}
          </section>
          <section className="guide-home-coverage" aria-label="File explanation progress">
            <div className="guide-home-coverage-heading"><strong>File explanations</strong><span>{revisionsError ? 'Unavailable' : revisionsLoading ? 'Loading…' : `${totals.accepted} / ${guideEligibleTotal}`}</span></div>
            <div className="guide-home-progress" role="progressbar" aria-label="Captured files with accepted explanations" aria-valuemin={0} aria-valuemax={guideEligibleTotal} aria-valuenow={totals.accepted}><span style={{ width: `${guideProgress}%` }} /></div>
            <p>{revisionsError ? 'Coverage is unavailable until saved revisions can be read.' : revisionsLoading ? 'Loading saved file explanations…' : `${totals.accepted.toLocaleString()} explained · ${(guideEligibleTotal - totals.accepted).toLocaleString()} still to explore`}</p>
            <div className="guide-home-file-list">
              {coverageReady && (nextFiles.length > 0 ? nextFiles : explainedFiles).slice(0, 3).map((row) => (
                <button key={row.inventory.path} onClick={() => onOpenFile(row.inventory.path)}>
                  <span className={`guide-file-dot ${row.status === 'accepted' ? 'is-explained' : ''}`} />
                  <code>{row.inventory.path}</code>
                  <small>{row.status === 'accepted' ? 'Read' : row.status === 'missing' ? 'Needs explanation' : statusLabel[row.status]}</small>
                  <ArrowRight size={12} />
                </button>
              ))}
              {coverageReady && nextFiles.length === 0 && explainedFiles.length === 0 && <span className="guide-home-no-files">No captured text files in this snapshot.</span>}
            </div>
            {revisionsLoading && <small className="guide-home-loading" role="status">Loading saved explanations…</small>}
          </section>
        </div>
        <div className="guide-home-actions">
          <button className="secondary-button" onClick={onGoAtlas}><FileCode2 size={14} /> Browse all files</button>
          <button className="secondary-button" onClick={onGoChanges}><GitBranch size={14} /> See what changed</button>
          {acceptedOverview && <span>Tour and per-file explanations are separate. Every source link opens its exact snapshot.</span>}
        </div>
      </section>

      <details className="guide-coverage-disclosure" open={returnFocusTarget?.startsWith('journey:')}><summary>Build a static call reading order</summary>
      <Suspense fallback={<p className="journey-note" role="status">Loading repository journeys…</p>}>
        <RepositoryJourneys manifest={manifest} revisions={revisions} revisionsReady={coverageReady} returnFocusTarget={returnFocusTarget}
          onOpenFile={onOpenFile} onGenerateFile={onGenerateFile} onReadRevision={onReadRevision} onReviewRevision={onReviewRevision} />
      </Suspense>
      </details>

      <details className="guide-coverage-disclosure">
        <summary><span>File-by-file coverage</span><small>{totals.accepted} explained · {Math.max(0, guideEligibleTotal - totals.accepted)} remaining · {totals.total} inventory paths</small></summary>
      <section className="guide-plan-card" aria-labelledby="guide-plan-title">
        <div className="guide-plan-topline">
          <div>
            <div className="step-pill"><span>GUIDE</span> FILE BY FILE</div>
            <h2 id="guide-plan-title">What still needs an explanation?</h2>
            <p>Choose a file to explain it, or open an accepted explanation to read it with its source links.</p>
          </div>
          <div className="chapter-studio-top-actions"><button className="secondary-button" onClick={onGoAtlas}><FileCode2 size={14} /> Browse files</button><button className="secondary-button" onClick={onGoStudio}><BookOpen size={14} /> Edit explanations</button></div>
        </div>

        {manifest.summary.status === 'partial' && <p className="coverage-storage-warning" role="status">This snapshot is partial. {manifest.summary.pathsSkipped.toLocaleString()} paths were skipped; open “All paths” to review them and why they were skipped.</p>}
        <details className="guide-source-details">
          <summary>Source snapshot details</summary>
          <div className="guide-plan-identity"><span>{manifest.summary.status === 'partial' ? 'Partial snapshot' : 'Immutable snapshot'}</span><code>{snapshotId}</code><span>{projectId}</span></div>
        </details>

        {revisionsLoading && <p className="coverage-feedback" role="status">Loading saved chapter revisions across snapshots…</p>}
        {revisionsError && <p className="coverage-storage-warning" role="alert">{revisionsError}<button className="text-button" onClick={() => setRevisionRefresh((value) => value + 1)}>Retry</button></p>}

        {coverageReady && <div className="coverage-total-strip" role="group" aria-label="Filter guide files">
          <button className={`coverage-total${guideView === 'needs-explanation' ? ' is-active' : ''}`} onClick={() => chooseGuideView('needs-explanation')} aria-pressed={guideView === 'needs-explanation'}><strong>{(totals.missing + totals.draft + totals.rejected + totals.stale + totals.unsupported).toLocaleString()}</strong><span>Needs explanation</span></button>
          <button className={`coverage-total${guideView === 'explained' ? ' is-active' : ''}`} onClick={() => chooseGuideView('explained')} aria-pressed={guideView === 'explained'}><strong>{totals.accepted.toLocaleString()}</strong><span>Explained</span></button>
          <button className={`coverage-total${guideView === 'all' ? ' is-active' : ''}`} onClick={() => chooseGuideView('all')} aria-pressed={guideView === 'all'}><strong>{totals.total.toLocaleString()}</strong><span>All paths</span></button>
        </div>}

        <div className="guide-list-toolbar">
          <label className="inventory-search"><Search size={14} /><input aria-label="Search guide files" placeholder="Find a file by name or path" value={query} onChange={(event) => setQueryFilter(event.target.value)} /></label>
          <button className={`secondary-button${groupingOpen ? ' selected' : ''}`} onClick={() => { if (groupingOpen) setGroupFormOpen(false); setGroupingOpen(!groupingOpen); setSelectedPaths([]); }} aria-expanded={groupingOpen}><Layers3 size={14} /> {groupingOpen ? 'Done grouping' : 'Organize files'}</button>
          <details className="coverage-advanced-filters">
            <summary>More filters</summary>
            <div className="coverage-filter-bar">
              <label><span>Status</span><select aria-label="Filter guide coverage by status" value={statusFilter} onChange={(event) => setStatus(event.target.value as 'all' | CoverageStatus)}><option value="all">All statuses ({totals.total})</option>{COVERAGE_STATUSES.map((status) => <option key={status} value={status}>{statusLabel[status]} ({totals[status]})</option>)}</select></label>
              <label><span>Language</span><select aria-label="Filter guide coverage by language" value={languageFilter} onChange={(event) => { setLanguageFilter(event.target.value); setPage(0); }}><option value="all">All languages</option>{languages.map((language) => <option key={language} value={language}>{language}</option>)}</select></label>
              <label><span>Capture outcome</span><select aria-label="Filter inventory by capture outcome" value={captureFilter} onChange={(event) => { setCaptureFilter(event.target.value as 'all' | CaptureDisposition); setPage(0); }}><option value="all">All capture outcomes</option><option value="captured">Captured</option><option value="skipped">Skipped</option><option value="excluded">Excluded</option></select></label>
              <label><span>Language support</span><select aria-label="Filter inventory by language support" value={parserFilter} onChange={(event) => { setParserFilter(event.target.value as 'all' | ParserReadiness); setPage(0); }}><option value="all">All support levels</option><option value="notReady">Partial structural</option><option value="unsupported">Text only</option><option value="notApplicable">Not analyzed</option></select></label>
              <label><span>File category</span><select aria-label="Filter inventory by file category" value={categoryFilter} onChange={(event) => { setCategoryFilter(event.target.value as 'all' | InventoryCategory); setPage(0); }}><option value="all">All categories</option><option value="config">Configuration</option><option value="test">Test</option><option value="generated">Generated</option><option value="secret">Secret-like</option></select></label>
              <label><span>Content</span><select aria-label="Filter inventory by content type" value={contentFilter} onChange={(event) => { setContentFilter(event.target.value as 'all' | InventoryRow['contentKind']); setPage(0); }}><option value="all">All content</option><option value="text">Text</option><option value="binary">Binary</option><option value="invalidEncoding">Invalid encoding</option><option value="unknown">Unknown</option><option value="notApplicable">Not applicable</option></select></label>
              <button className="inventory-clear" onClick={() => { setQueryFilter(''); setStatus('all'); setLanguageFilter('all'); setCaptureFilter('all'); setParserFilter('all'); setContentFilter('all'); setCategoryFilter('all'); setPage(0); }}>Reset filters</button>
            </div>
          </details>
          <span className="coverage-match-count" aria-live="polite">{pageData.total.toLocaleString()} files</span>
        </div>

        {groupingOpen && <section className="guide-grouping-panel" aria-label="Organize files into optional groups">
          <div className="guide-plan-actions">
            <div><strong>Group related files (optional)</strong><span>Groups organize files; every file keeps its own explanation and source links.</span></div>
            <div className="guide-plan-action-buttons"><span aria-live="polite">{selectedPaths.length} selected</span><button className="secondary-button" onClick={startGroup} disabled={selectedPaths.length < 2 || revisionsLoading || Boolean(revisionsError)}>Create group</button>{selectedPaths.length > 0 && <button className="text-button" onClick={() => setSelectedPaths([])}>Clear selection</button>}</div>
          </div>
          {plan.groups.length > 0 && <section className="coverage-groups" aria-labelledby="coverage-groups-heading">
            <div className="coverage-subheading"><div><strong id="coverage-groups-heading">Your groups</strong><span>Each member still has its own explanation.</span></div><span>{plan.groups.length} group{plan.groups.length === 1 ? '' : 's'}</span></div>
            {plan.groups.map((group) => <article className="coverage-group-card" key={group.groupId}><div><strong>{group.name}</strong><p>{group.rationale}</p><div className="coverage-group-members">{group.paths.map((path) => <button key={path} onClick={() => onOpenFile(path)}><FileCode2 size={12} /><code>{path}</code></button>)}</div></div><button className="text-button" onClick={() => dissolveGroup(group.groupId, group.name)}>Remove group</button></article>)}
          </section>}
        </section>}

        {groupFormOpen && (
          <form className="coverage-group-form" onSubmit={submitGroup}>
            <div><strong>Name this group</strong><p>All selected paths remain visible as separate rows and source citations.</p></div>
            <label><span>Group name</span><input autoFocus value={groupName} onChange={(event) => setGroupName(event.target.value)} placeholder="For example, request handling" required maxLength={100} /></label>
            <label><span>Why do these files belong together?</span><textarea value={groupRationale} onChange={(event) => setGroupRationale(event.target.value)} placeholder="Describe their shared responsibility. This rationale is shown beside every member." required maxLength={500} rows={3} /></label>
            <div className="coverage-form-footer"><span>{selectedPaths.length} paths · {selectedPaths.slice(0, 3).join(', ')}{selectedPaths.length > 3 ? ` and ${selectedPaths.length - 3} more` : ''}</span><div><button type="button" className="text-button" onClick={() => setGroupFormOpen(false)}>Cancel</button><button className="primary-button" type="submit">Create named group</button></div></div>
          </form>
        )}

        {coverageReady && <CoverageTable
          rows={pageData.rows}
          statusLabel={statusLabel}
          showGrouping={groupingOpen}
          selectedPaths={selectedPaths}
          allVisibleSelected={allVisibleSelected}
          visibleEligible={visibleEligible}
          excludePath={excludePath}
          excludeReason={excludeReason}
          setExcludePath={(path) => { setExcludePath(path); setExcludeReason(''); }}
          setExcludeReason={setExcludeReason}
          onToggleSelected={toggleSelected}
          onToggleVisible={toggleVisible}
          onOpenFile={onOpenFile}
          onReadRevision={(revisionId) => openRevision(revisionId, 'read')}
          onReviewRevision={(revisionId) => openRevision(revisionId, 'review')}
          onExclude={submitExclusion}
          onRestore={(path) => { setPlan((current) => restoreCoveragePath(current, path)); setFeedback(`${path} is back in the guide plan.`); }}
          onCancelExclude={() => setExcludePath(null)}
        />}
        <div className="coverage-pagination">
          <span>Showing {pageData.firstVisible.toLocaleString()}–{pageData.lastVisible.toLocaleString()} of {pageData.total.toLocaleString()} matches · {totals.total.toLocaleString()} snapshot paths</span>
          <label>Rows <select value={pageSize} onChange={(event) => { setPageSize(Number(event.target.value)); setPage(0); }} aria-label="Rows per coverage page"><option value={25}>25</option><option value={50}>50</option><option value={100}>100</option></select></label>
          <button onClick={() => setPage((current) => Math.max(0, current - 1))} disabled={pageData.page === 0}>Previous</button>
          <span>Page {pageData.page + 1} of {pageData.pageCount}</span>
          <button onClick={() => setPage((current) => Math.min(pageData.pageCount - 1, current + 1))} disabled={pageData.page >= pageData.pageCount - 1}>Next</button>
        </div>
        {feedback && <p className="coverage-feedback" role="status">{feedback}<button className="text-button" onClick={() => setFeedback(null)}>Dismiss</button></p>}
        {!storageAvailable && <p className="coverage-storage-warning" role="status">This session can’t save Guide Plan changes in local storage. Keep this window open to retain them.</p>}
        <p className="coverage-footnote">Draft and accepted counts reflect chapter revisions tied to this exact project and snapshot. Capture exclusions remain unavailable inventory rows; they are never counted as completed explanations.</p>
        {coverageReady && totals.draft === 0 && totals.accepted === 0 && <p className="coverage-no-revisions">No draft or accepted chapter revision is saved for this snapshot yet. Captured text files still appear as independent work items.</p>}
      </section>
      </details>
    </div>
  );
}

interface ChapterStudioTarget {
  key: string;
  targetId: string | null;
  paths: string[];
  overview?: boolean;
  revisions: ChapterRevision[];
  latestCurrent: ChapterRevision | null;
  latestAny: ChapterRevision | null;
  missing: boolean;
}

interface ChapterStudioDraftCheckpoint {
  version: 1;
  projectId: string;
  snapshotId: string;
  targetKey: string;
  baseRevisionId: string;
  candidate: ChapterRevision;
}

interface ChapterFreshnessState {
  key: string;
  report: ChapterFreshnessReport | null;
  error: string | null;
  loading: boolean;
}

interface RangedDiffLine {
  line: number;
  text: string;
}

interface DiffLine {
  kind: 'same' | 'removed' | 'added';
  oldLine?: number;
  newLine?: number;
  text: string;
}

function changedRangeLines(source: SnapshotSource, range: { startLine: number; endLine: number }): { lines: RangedDiffLine[]; truncated: boolean } {
  const allLines = source.contents.split(/\r?\n/);
  const start = Math.max(0, range.startLine - 1);
  const stop = Math.min(allLines.length, Math.max(start, range.endLine));
  const selected = allLines.slice(start, stop);
  const lines: RangedDiffLine[] = [];
  let bytes = 0;
  for (let index = 0; index < selected.length; index += 1) {
    const original = selected[index];
    const text = original.slice(0, 1_000);
    bytes += text.length;
    if (lines.length >= 120 || bytes > 16_000) return { lines, truncated: true };
    lines.push({ line: start + index + 1, text });
  }
  return { lines, truncated: stop < range.endLine || selected.some((line) => line.length > 1_000) };
}

function compareRangeLines(oldLines: RangedDiffLine[], newLines: RangedDiffLine[]): DiffLine[] {
  const oldCount = oldLines.length;
  const newCount = newLines.length;
  const stride = newCount + 1;
  const lcs = new Uint16Array((oldCount + 1) * (newCount + 1));
  for (let oldIndex = oldCount - 1; oldIndex >= 0; oldIndex -= 1) {
    for (let newIndex = newCount - 1; newIndex >= 0; newIndex -= 1) {
      const cell = oldIndex * stride + newIndex;
      lcs[cell] = oldLines[oldIndex].text === newLines[newIndex].text
        ? lcs[(oldIndex + 1) * stride + newIndex + 1] + 1
        : Math.max(lcs[(oldIndex + 1) * stride + newIndex], lcs[oldIndex * stride + newIndex + 1]);
    }
  }
  const diff: DiffLine[] = [];
  let oldIndex = 0;
  let newIndex = 0;
  while (oldIndex < oldCount || newIndex < newCount) {
    if (oldIndex < oldCount && newIndex < newCount && oldLines[oldIndex].text === newLines[newIndex].text) {
      diff.push({ kind: 'same', oldLine: oldLines[oldIndex].line, newLine: newLines[newIndex].line, text: oldLines[oldIndex].text });
      oldIndex += 1;
      newIndex += 1;
    } else if (oldIndex < oldCount && (newIndex >= newCount || lcs[(oldIndex + 1) * stride + newIndex] >= lcs[oldIndex * stride + newIndex + 1])) {
      diff.push({ kind: 'removed', oldLine: oldLines[oldIndex].line, text: oldLines[oldIndex].text });
      oldIndex += 1;
    } else {
      diff.push({ kind: 'added', newLine: newLines[newIndex].line, text: newLines[newIndex].text });
      newIndex += 1;
    }
  }
  return diff;
}

function ChapterSourceDiff({ projectId, change }: { projectId: string; change: ChapterFreshnessSourceChange }) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [diff, setDiff] = useState<{ lines: DiffLine[]; oldTruncated: boolean; newTruncated: boolean } | null>(null);
  const started = useRef(false);
  useEffect(() => {
    if (!open || diff || started.current) return;
    started.current = true;
    let active = true;
    setLoading(true);
    const readRange = (range: ChapterFreshnessSourceChange['oldRange']) => range
      ? readSnapshotSource(projectId, range.snapshotId, range.path).then((source) => changedRangeLines(source, range))
      : Promise.resolve({ lines: [] as RangedDiffLine[], truncated: false });
    void Promise.all([readRange(change.oldRange), readRange(change.newRange)])
      .then(([oldSide, newSide]) => { if (active) setDiff({ lines: compareRangeLines(oldSide.lines, newSide.lines), oldTruncated: oldSide.truncated, newTruncated: newSide.truncated }); })
      .catch(() => { if (active) setError('The saved source ranges could not be loaded for a local diff.'); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [change, diff, open, projectId]);
  return <details className="chapter-source-diff" onToggle={(event) => setOpen(event.currentTarget.open)}>
    <summary>Show exact source diff</summary>
    {loading && <p role="status">Loading both captured source ranges locally…</p>}
    {error && <p role="alert">{error}</p>}
    {diff && <>
      <div className="chapter-source-diff-labels">
        <span>Old {change.oldRange ? `${change.oldRange.path}:${change.oldRange.startLine}–${change.oldRange.endLine}` : 'source removed'}</span>
        <span>Current {change.newRange ? `${change.newRange.path}:${change.newRange.startLine}–${change.newRange.endLine}` : 'source removed'}</span>
      </div>
      {(diff.oldTruncated || diff.newTruncated) && <p className="provider-small-note">Large range: showing a bounded excerpt. Open the exact old or current range in Files for full source.</p>}
      <pre className="chapter-source-diff-lines" role="group" aria-label="Captured source diff">{diff.lines.map((line, index) => <span className={`diff-${line.kind}`} key={`${line.kind}:${line.oldLine}:${line.newLine}:${index}`}>{line.kind === 'removed' ? '−' : line.kind === 'added' ? '+' : ' '} {String(line.oldLine ?? line.newLine ?? '').padStart(4, ' ')} │ {line.text}{'\n'}</span>)}</pre>
    </>}
  </details>;
}

interface RevisionDiffValue {
  field: string;
  before: string;
  after: string;
  path?: string;
}

function revisionEntityLabel(entity: RevisionChange['entity']): string {
  switch (entity) {
    case 'chapter': return 'chapter';
    case 'section': return 'section';
    case 'claim': return 'claim';
    case 'anchor': return 'source reference';
    case 'dependency': return 'dependency';
    case 'visual': return 'diagram';
    case 'visualNode': return 'diagram node';
    case 'visualEdge': return 'diagram edge';
    case 'hotspot': return 'hotspot';
  }
  return String(entity);
}

function revisionFieldLabel(field: string): string {
  const labels: Record<string, string> = {
    startByte: 'start byte',
    endByteInclusive: 'end byte',
    startLine: 'start line',
    endLine: 'end line',
    startColumn: 'start column',
    endColumn: 'end column',
    sourceSha256: 'source hash',
    evidenceAnchorIds: 'source references',
    spokenText: 'spoken narration',
    targetScope: 'target',
  };
  return labels[field] ?? field.replace(/[A-Z]/g, (letter) => ` ${letter.toLowerCase()}`);
}

function revisionChangeRow(change: RevisionChange): RevisionDiffValue {
  const action = change.kind[0].toUpperCase() + change.kind.slice(1);
  const identity = change.id ? ` · ${change.id}` : '';
  const field = change.field ? ` · ${revisionFieldLabel(change.field)}` : '';
  return {
    field: `${action} ${revisionEntityLabel(change.entity)}${field}${identity}`,
    path: change.path,
    before: change.beforeSummary ?? '—',
    after: change.afterSummary ?? '—',
  };
}

function ChapterRevisionComparison({ before, after }: { before: ChapterRevision; after: ChapterRevision }) {
  const contentDiff = compareChapterRevisions(before, after);
  const chapterFields: RevisionDiffValue[] = [
    ...(before.title !== after.title ? [{ field: 'Chapter title', before: before.title, after: after.title }] : []),
    ...(before.snapshotId !== after.snapshotId ? [{ field: 'Source snapshot', before: before.snapshotId, after: after.snapshotId }] : []),
    ...(before.lifecycle !== after.lifecycle ? [{ field: 'Status', before: before.lifecycle, after: after.lifecycle }] : []),
    ...(before.basedOnRevisionId !== after.basedOnRevisionId ? [{ field: 'Parent revision', before: before.basedOnRevisionId ?? '—', after: after.basedOnRevisionId ?? '—' }] : []),
  ];
  const contentRows = contentDiff.changes.map(revisionChangeRow);
  if (chapterFields.length === 0 && contentRows.length === 0) {
    return <p className="chapter-revision-no-diff">No chapter content, claims, citations, dependencies, or assets changed between these revisions.</p>;
  }
  return <div className="chapter-revision-diff">
    {chapterFields.length > 0 && <div className="chapter-revision-diff-group"><h5>Chapter</h5>{chapterFields.map((field) => <RevisionDiffRow key={field.field} value={field} />)}</div>}
    {contentRows.length > 0 && <div className="chapter-revision-diff-group"><h5>Content · {contentDiff.counts.total} change{contentDiff.counts.total === 1 ? '' : 's'}</h5>{contentRows.map((field, index) => <RevisionDiffRow key={`${field.path}:${field.field}:${index}`} value={field} />)}</div>}
  </div>;
}

function RevisionDiffRow({ value }: { value: RevisionDiffValue }) {
  return <div className="chapter-revision-diff-row">
    <strong>{value.field}{value.path && <small>{value.path}</small>}</strong>
    <div><span>Before</span><pre>{value.before}</pre></div>
    <div><span>After</span><pre>{value.after}</pre></div>
  </div>;
}

function ChapterStudioScreen({ projectId, manifest, rows, routeChapterId, routeRevisionId, routeSectionId, onSelectRevision, onClearRevisionRoute, onOpenFile, onGoPlan, onGenerate, onRegenerateSection, onGenerateOverview }: {
  projectId: string;
  manifest: SnapshotManifest;
  rows: InventoryRow[];
  routeChapterId: string | null;
  routeRevisionId: string | null;
  routeSectionId: string | null;
  onSelectRevision: (revision: ChapterRevision, sectionId?: string) => void;
  onClearRevisionRoute: () => void;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onGoPlan: () => void;
  onGenerate: (path: string) => void;
  onRegenerateSection: (revision: ChapterRevision, section: ChapterSection, selectedPaths: string[]) => void;
  onGenerateOverview: () => void;
}) {
  const snapshotId = manifest.summary.snapshotId;
  const draftStorageKey = `code-atlas:chapter-studio-draft:${projectId}:${snapshotId}`;
  const [restoredDraft] = useState<ChapterStudioDraftCheckpoint | null>(() => {
    try {
      const value = JSON.parse(window.sessionStorage.getItem(draftStorageKey) ?? 'null') as ChapterStudioDraftCheckpoint | null;
      if (!value || value.version !== 1 || value.projectId !== projectId || value.snapshotId !== snapshotId
        || typeof value.targetKey !== 'string' || typeof value.baseRevisionId !== 'string'
        || !value.candidate || value.candidate.projectId !== projectId || value.candidate.snapshotId !== snapshotId
        || !Array.isArray(value.candidate.sections)) return null;
      if (routeRevisionId && value.baseRevisionId !== routeRevisionId) return null;
      if (routeChapterId && value.candidate.chapterId !== routeChapterId) return null;
      return value;
    } catch {
      return null;
    }
  });
  const studioMounted = useRef(false);
  useEffect(() => {
    studioMounted.current = true;
    return () => { studioMounted.current = false; };
  }, []);
  const [revisions, setRevisions] = useState<ChapterRevision[]>([]);
  const [loading, setLoading] = useState(isNativeRuntime());
  const [loadError, setLoadError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [selectedTargetKey, setSelectedTargetKey] = useState(restoredDraft?.targetKey ?? '');
  const [selectedRevisionId, setSelectedRevisionId] = useState<string | null>(routeRevisionId ?? restoredDraft?.baseRevisionId ?? null);
  const [query, setQuery] = useState('');
  const [editing, setEditing] = useState(Boolean(restoredDraft));
  const [candidate, setCandidate] = useState<ChapterRevision | null>(restoredDraft?.candidate ?? null);
  const [validation, setValidation] = useState<ChapterValidationReport | null>(null);
  const [rejecting, setRejecting] = useState(false);
  const [rejectReason, setRejectReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [freshnessState, setFreshnessState] = useState<ChapterFreshnessState | null>(null);
  const [feedback, setFeedback] = useState<{ message: string; tone: 'success' | 'error' | 'info' } | null>(restoredDraft ? {
    message: 'Restored your unsaved draft after opening source. Save it as a new revision or cancel the edit.',
    tone: 'info',
  } : null);

  useEffect(() => {
    const refresh = (event: Event) => {
      setRefreshKey((value) => value + 1);
      const revision = (event as CustomEvent<{ revision?: ChapterRevision }>).detail?.revision;
      if (!revision || revision.projectId !== projectId || revision.snapshotId !== snapshotId) return;
      setSelectedTargetKey(revision.target.scope.kind === 'overview' ? 'overview' : revision.target.targetId);
      setSelectedRevisionId(revision.revisionId);
      setEditing(false);
      setCandidate(null);
      setValidation(null);
      onSelectRevision(revision);
    };
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => window.removeEventListener('code-atlas:chapters-changed', refresh);
  }, [onSelectRevision, projectId, snapshotId]);

  useEffect(() => {
    if (!editing || !candidate || !selectedRevisionId || !selectedTargetKey) {
      try { window.sessionStorage.removeItem(draftStorageKey); } catch { /* session storage may be unavailable */ }
      return;
    }
    const checkpoint: ChapterStudioDraftCheckpoint = {
      version: 1,
      projectId,
      snapshotId,
      targetKey: selectedTargetKey,
      baseRevisionId: selectedRevisionId,
      candidate,
    };
    try {
      window.sessionStorage.setItem(draftStorageKey, JSON.stringify(checkpoint));
    } catch {
      setFeedback({ message: 'This unsaved draft could not be preserved for source navigation. Save or cancel before leaving Chapter Studio.', tone: 'error' });
    }
  }, [candidate, draftStorageKey, editing, projectId, selectedRevisionId, selectedTargetKey, snapshotId]);

  useEffect(() => {
    if (!isNativeRuntime()) return;
    let active = true;
    setLoading(true);
    setLoadError(null);
    void listProjectChapterRevisions(projectId)
      .then((saved) => { if (active) setRevisions(saved); })
      .catch((error) => { if (active) setLoadError(errorMessageOr(error, 'Saved chapter history could not be loaded.')); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [projectId, refreshKey]);

  const targets = useMemo<ChapterStudioTarget[]>(() => {
    const result: ChapterStudioTarget[] = [];
    const seen = new Set<string>();
    const chapterRevisions = (targetId: string) => revisions
      .filter((revision) => revision.target.targetId === targetId)
      .sort((left, right) => left.revisionNumber - right.revisionNumber);
    const addTarget = (key: string, targetId: string | null, paths: string[], history: ChapterRevision[], overview = false) => {
      if (seen.has(key)) return;
      seen.add(key);
      const latestCurrent = history
        .filter((revision) => revision.snapshotId === snapshotId)
        .sort((left, right) => right.revisionNumber - left.revisionNumber)[0] ?? null;
      const latestAny = [...history].sort((left, right) => right.revisionNumber - left.revisionNumber)[0] ?? null;
      result.push({ key, targetId, paths, overview, revisions: history, latestCurrent, latestAny, missing: history.length === 0 });
    };
    for (const row of rows) {
      if (row.disposition !== 'captured' || row.kind !== 'file' || !['text', 'unknown'].includes(row.contentKind)) continue;
      const matches = revisions.filter((revision) => revision.target.scope.kind === 'file' && revision.target.scope.path === row.path);
      if (matches.length > 0) {
        const ids = Array.from(new Set(matches.map((revision) => revision.target.targetId)));
        for (const targetId of ids) addTarget(targetId, targetId, [row.path], chapterRevisions(targetId));
      } else {
        addTarget(`file:${row.path}`, null, [row.path], []);
      }
    }
    // Retain chapter history when a file is deleted, excluded, or no longer text
    // in the current snapshot. Its citations still open the original snapshot.
    for (const revision of revisions) {
      if (revision.target.scope.kind === 'file') {
        addTarget(revision.target.targetId, revision.target.targetId,
          [revision.target.scope.path], chapterRevisions(revision.target.targetId));
      }
    }
    const groupIds = Array.from(new Set(revisions
      .filter((revision) => revision.target.scope.kind === 'group')
      .map((revision) => revision.target.targetId)));
    for (const targetId of groupIds) {
      const history = chapterRevisions(targetId);
      const scope = history[0]?.target.scope;
      if (scope?.kind === 'group') addTarget(targetId, targetId, scope.paths, history);
    }
    const overviewHistory = revisions
      .filter((revision) => revision.target.scope.kind === 'overview')
      .sort((left, right) => left.revisionNumber - right.revisionNumber);
    if (overviewHistory.length > 0) addTarget('overview', 'target:overview', [], overviewHistory, true);
    return result.sort((left, right) => {
      const a = left.overview ? '' : left.paths[0] ?? left.key;
      const b = right.overview ? '' : right.paths[0] ?? right.key;
      return a.localeCompare(b);
    });
  }, [manifest, revisions, rows, snapshotId]);

  useEffect(() => {
    if (routeChapterId) {
      const routed = targets.find((target) => target.revisions.some((revision) => revision.chapterId === routeChapterId));
      if (routed) {
        setSelectedTargetKey(routed.key);
        return;
      }
    }
    if (!selectedTargetKey || !targets.some((target) => target.key === selectedTargetKey)) {
      setSelectedTargetKey(targets[0]?.key ?? '');
    }
  }, [routeChapterId, selectedTargetKey, targets]);

  useEffect(() => {
    if (routeRevisionId) setSelectedRevisionId(routeRevisionId);
  }, [routeRevisionId]);

  const visibleTargets = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return targets.filter((target) => !normalized || (target.overview && 'repository overview'.includes(normalized)) || target.paths.some((path) => path.toLowerCase().includes(normalized))
      || target.latestAny?.title.toLowerCase().includes(normalized));
  }, [query, targets]);
  const selectedTarget = targets.find((target) => target.key === selectedTargetKey) ?? null;
  const history = selectedTarget?.revisions ?? [];
  const selectedRevision = history.find((revision) => revision.revisionId === selectedRevisionId)
    ?? selectedTarget?.latestCurrent
    ?? selectedTarget?.latestAny
    ?? null;
  const isLatestCurrent = Boolean(selectedRevision && selectedTarget?.latestCurrent?.revisionId === selectedRevision.revisionId);
  const previousRevision = selectedRevision
    ? history.filter((revision) => revision.revisionNumber < selectedRevision.revisionNumber)
      .sort((left, right) => right.revisionNumber - left.revisionNumber)[0] ?? null
    : null;
  const activeDocument = editing && candidate ? candidate : selectedRevision;
  const status = !selectedTarget || selectedTarget.missing
    ? 'missing'
    : selectedRevision
      ? selectedRevision.snapshotId === snapshotId ? selectedRevision.lifecycle : 'stale'
      : selectedTarget.latestCurrent?.lifecycle ?? 'stale';
  const statusLabel = status === 'missing' ? 'Needs explanation'
    : status === 'stale' ? 'Older snapshot'
      : status === 'draft' ? 'Draft'
        : status === 'rejected' ? 'Rejected'
          : 'Accepted';
  const freshnessKey = selectedRevision?.lifecycle === 'accepted' && selectedRevision.snapshotId !== snapshotId
    ? `${selectedRevision.revisionId}:${snapshotId}`
    : null;
  const selectedFreshness = freshnessKey && freshnessState?.key === freshnessKey ? freshnessState : null;

  useEffect(() => {
    if (!routeSectionId || !activeDocument?.sections.some((section) => section.sectionId === routeSectionId)) return;
    const frame = requestAnimationFrame(() => {
      const section = document.getElementById(`studio-section-${routeSectionId}`);
      section?.focus({ preventScroll: true });
      section?.scrollIntoView({
        block: 'start',
        behavior: scrollBehaviorForMotionPreference(window.matchMedia('(prefers-reduced-motion: reduce)').matches),
      });
    });
    return () => cancelAnimationFrame(frame);
  }, [activeDocument?.revisionId, routeSectionId]);

  const reload = async (selectId?: string) => {
    const saved = await listProjectChapterRevisions(projectId);
    if (!studioMounted.current) return;
    setRevisions(saved);
    if (selectId) setSelectedRevisionId(selectId);
  };
  const setSection = (sectionId: string, update: (section: ChapterSection) => ChapterSection) => {
    setCandidate((current) => current ? ({
      ...current,
      sections: current.sections.map((section) => section.sectionId === sectionId ? update(section) : section),
    }) : current);
    setValidation(null);
  };
  const beginEditing = () => {
    if (!selectedRevision || !isLatestCurrent) return;
    setCandidate(structuredClone(selectedRevision));
    setEditing(true);
    setValidation(null);
    setRejecting(false);
    setFeedback(null);
  };
  const cancelEditing = () => {
    setEditing(false);
    setCandidate(null);
    setValidation(null);
  };
  const validateCurrent = async () => {
    if (!activeDocument || busy) return;
    setBusy(true);
    setFeedback(null);
    try {
      const report = await validateChapterRevision(projectId, activeDocument);
      if (!studioMounted.current) return;
      setValidation(report);
      setFeedback({
        message: report.status === 'valid'
          ? 'Every cited range matches the captured source.'
          : `${report.issues.length} issue${report.issues.length === 1 ? '' : 's'} need review before acceptance.`,
        tone: report.status === 'valid' ? 'success' : 'error',
      });
    } catch (error) {
      if (!studioMounted.current) return;
      setFeedback({ message: errorMessageOr(error, 'The chapter could not be validated.'), tone: 'error' });
    } finally {
      if (studioMounted.current) setBusy(false);
    }
  };
  const assessFreshness = async () => {
    if (!selectedRevision || !freshnessKey || editing) return;
    const key = freshnessKey;
    setFreshnessState({ key, report: null, error: null, loading: true });
    try {
      const report = await assessChapterFreshness(projectId, selectedRevision.revisionId, snapshotId);
      if (!studioMounted.current) return;
      setFreshnessState({ key, report, error: null, loading: false });
    } catch (error) {
      if (!studioMounted.current) return;
      setFreshnessState({
        key,
        report: null,
        error: errorMessageOr(error, 'Freshness could not be assessed. Check indexing and try again.'),
        loading: false,
      });
    }
  };
  const declaredCurrentPaths = (section: ChapterSection, targetPath: string) => {
    const paths = new Set<string>([targetPath]);
    for (const claim of section.claims) {
      for (const anchor of claim.anchors) paths.add(anchor.path);
      for (const dependency of claim.dependencies) paths.add(dependency.path);
    }
    for (const dependency of section.dependencies) paths.add(dependency.path);
    return Array.from(paths)
      .filter((path) => manifest.files.some((file) => file.path === path))
      .sort((left, right) => left.localeCompare(right));
  };
  const saveEdits = async () => {
    if (!candidate || !selectedRevision || !isLatestCurrent || busy) return;
    setBusy(true);
    setFeedback(null);
    try {
      const saved = await saveChapterEdits(projectId, selectedRevision.revisionId, candidate);
      if (!studioMounted.current) return;
      setValidation(saved.validation);
      await reload(saved.revision.revisionId);
      if (!studioMounted.current) return;
      setCandidate(null);
      setEditing(false);
      setFeedback({
        message: saved.validation.status === 'valid'
          ? 'Draft saved as a new immutable revision.'
          : 'Draft saved as a new revision with evidence issues to repair.',
        tone: saved.validation.status === 'valid' ? 'success' : 'info',
      });
      onSelectRevision(saved.revision);
    } catch (error) {
      if (!studioMounted.current) return;
      setFeedback({ message: errorMessageOr(error, 'The draft could not be saved.'), tone: 'error' });
    } finally {
      if (studioMounted.current) setBusy(false);
    }
  };
  const restoreSelectedRevision = async () => {
    if (!selectedRevision || !selectedTarget?.latestCurrent || isLatestCurrent
      || selectedRevision.revisionNumber >= selectedTarget.latestCurrent.revisionNumber || editing || busy) return;
    setBusy(true);
    setFeedback(null);
    try {
      const restored = await restoreChapterRevision(
        projectId,
        selectedTarget.latestCurrent.revisionId,
        selectedRevision.revisionId,
      );
      if (!studioMounted.current) return;
      setValidation(restored.validation);
      setEditing(false);
      setCandidate(null);
      setRejecting(false);
      await reload(restored.revision.revisionId);
      if (!studioMounted.current) return;
      onSelectRevision(restored.revision);
      setFeedback({
        message: restored.validation.status === 'valid'
          ? `Revision r${selectedRevision.revisionNumber} was restored as a new draft and revalidated against snapshot ${restored.revision.snapshotId.slice(-12)}.`
          : `A new restore draft was created with ${restored.validation.issues.length} evidence issue${restored.validation.issues.length === 1 ? '' : 's'}. Repair the listed citations before accepting it.`,
        tone: restored.validation.status === 'valid' ? 'success' : 'info',
      });
    } catch (error) {
      if (!studioMounted.current) return;
      setFeedback({ message: errorMessageOr(error, 'The selected revision could not be restored.'), tone: 'error' });
    } finally {
      if (studioMounted.current) setBusy(false);
    }
  };
  const accept = async () => {
    if (!selectedRevision || !isLatestCurrent || selectedRevision.lifecycle !== 'draft' || busy) return;
    setBusy(true);
    setFeedback(null);
    try {
      const accepted = await acceptChapterDraft(projectId, selectedRevision.revisionId);
      if (!studioMounted.current) return;
      setValidation(accepted.validation);
      await reload(accepted.revision.revisionId);
      if (!studioMounted.current) return;
      onSelectRevision(accepted.revision);
      setFeedback({ message: 'Accepted. The draft and accepted revision remain in history.', tone: 'success' });
    } catch (error) {
      if (!studioMounted.current) return;
      setFeedback({ message: errorMessageOr(error, 'The chapter could not be accepted.'), tone: 'error' });
    } finally {
      if (studioMounted.current) setBusy(false);
    }
  };
  const reject = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedRevision || !isLatestCurrent || selectedRevision.lifecycle !== 'draft' || busy) return;
    setBusy(true);
    setFeedback(null);
    try {
      const rejected = await rejectChapterDraft(projectId, selectedRevision.revisionId, rejectReason);
      await reload(rejected.revisionId);
      if (!studioMounted.current) return;
      onSelectRevision(rejected);
      setRejecting(false);
      setRejectReason('');
      setFeedback({ message: 'Rejected with reason recorded. The draft payload is still available in history.', tone: 'success' });
    } catch (error) {
      if (!studioMounted.current) return;
      setFeedback({ message: errorMessageOr(error, 'The draft could not be rejected.'), tone: 'error' });
    } finally {
      if (studioMounted.current) setBusy(false);
    }
  };
  const moveSection = (sectionId: string, direction: -1 | 1) => {
    setCandidate((current) => {
      if (!current) return current;
      const from = current.sections.findIndex((section) => section.sectionId === sectionId);
      const to = from + direction;
      if (from < 0 || to < 0 || to >= current.sections.length) return current;
      const sections = [...current.sections];
      [sections[from], sections[to]] = [sections[to], sections[from]];
      return { ...current, sections: sections.map((section, order) => ({ ...section, order })) };
    });
    setValidation(null);
  };
  const removeClaim = (sectionId: string, claimId: string) => setSection(sectionId, (section) => ({
    ...section,
    claims: section.claims.filter((claim) => claim.claimId !== claimId),
  }));

  return (
    <div className="empty-workspace chapter-studio-screen">
      <section className="chapter-studio-card" aria-labelledby="chapter-studio-title">
        <div className="chapter-studio-topline">
          <div>
            <div className="step-pill"><span>GUIDE EDITOR</span> SOURCE FIRST · VERSIONED</div>
            <h2 id="chapter-studio-title" data-focus-key="heading:chapter-studio">Inspect what each file actually does.</h2>
            <p>Each file explanation stays separate. The optional repository overview never counts files as explained; changes are saved as local versions.</p>
          </div>
          <div className="chapter-studio-top-actions">
            <button className="primary-button" onClick={onGenerateOverview} disabled={editing} title={editing ? 'Save or cancel the current edit before creating an overview.' : 'Create a separate overview; this does not mark any file chapter complete.'}><Sparkles size={14} /> Optional overview</button>
            <button className="secondary-button" onClick={onGoPlan}><ListChecks size={14} /> Guide</button>
            <span className="chapter-studio-snapshot">Snapshot <code>{snapshotId.slice(-12)}</code></span>
          </div>
        </div>

        {loading && <p className="coverage-feedback" role="status">Loading repository chapters and revision history…</p>}
        {loadError && <p className="coverage-storage-warning" role="alert">{loadError}<button className="text-button" onClick={() => setRefreshKey((value) => value + 1)}>Retry</button></p>}
        {!isNativeRuntime() && <p className="coverage-feedback" role="status">Chapter editing is available in the Mac app, where revisions are stored locally.</p>}
        {editing && <p className="coverage-feedback" role="status">Unsaved changes are kept while you inspect citations. File and revision selection is paused; save or cancel to switch.</p>}

        <div className="chapter-studio-layout">
          <aside className="chapter-studio-sidebar" aria-label="Repository chapter targets">
            <label className="chapter-studio-search"><Search size={14} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find a file or chapter" aria-label="Find a file or chapter" /></label>
            <div className="chapter-studio-target-list">
              {visibleTargets.map((target) => {
                const latest = target.latestCurrent ?? target.latestAny;
                const rowStatus = target.missing ? 'missing' : target.latestCurrent?.lifecycle ?? 'stale';
                const label = target.overview ? 'Repository overview' : target.paths.length === 1 ? target.paths[0] : `${target.paths[0]} + ${target.paths.length - 1} files`;
                return <button
                  key={target.key}
                  className={`chapter-studio-target${selectedTargetKey === target.key ? ' is-selected' : ''}`}
                  disabled={editing || busy}
                  onClick={() => {
                    setSelectedTargetKey(target.key);
                    setSelectedRevisionId(target.latestCurrent?.revisionId ?? target.latestAny?.revisionId ?? null);
                    setEditing(false); setCandidate(null); setValidation(null); setFeedback(null);
                    if (latest) onSelectRevision(latest);
                    else onClearRevisionRoute();
                  }}
                  aria-current={selectedTargetKey === target.key ? 'true' : undefined}
                >
                  <span className={`chapter-studio-target-status status-${rowStatus}`}>{rowStatus}</span>
                  <code title={label}>{label}</code>
                  <small>{latest ? `${latest.title} · r${latest.revisionNumber}` : 'No chapter for this snapshot'}</small>
                </button>;
              })}
              {visibleTargets.length === 0 && <p className="chapter-studio-no-results">No chapter targets match that search.</p>}
            </div>
          </aside>

          <main className="chapter-studio-review">
            {!selectedTarget ? <div className="chapter-studio-empty"><BookOpen size={20} /><strong>No captured text files</strong><p>Create a snapshot with readable text files before reviewing chapters.</p></div> : (
              <>
                <div className="chapter-review-heading">
                  <div><span className={`chapter-review-status status-${status}`}>{statusLabel}</span><h3>{selectedRevision?.title ?? (selectedTarget.overview ? 'Repository overview' : selectedTarget.paths.length === 1 ? selectedTarget.paths[0] : 'Grouped chapter')}</h3>{!selectedTarget.overview && <code>{selectedTarget.paths.join(' · ')}</code>}</div>
                  {selectedTarget.missing || status === 'stale' ? <div className="chapter-review-actions">
                    {!selectedTarget.overview && <button className="secondary-button" onClick={() => onOpenFile(selectedTarget.paths[0], selectedRevision?.snapshotId ?? selectedTarget.latestAny?.snapshotId)}><FileCode2 size={14} /> Open source</button>}
                    {selectedTarget.overview
                      ? <button className="primary-button" onClick={onGenerateOverview}><Sparkles size={14} /> Generate current overview</button>
                      : selectedTarget.paths.length === 1 && !selectedTarget.latestCurrent
                        && rows.some((row) => row.path === selectedTarget.paths[0] && row.disposition === 'captured' && row.kind === 'file' && ['text', 'unknown'].includes(row.contentKind))
                        && <button className="primary-button" onClick={() => onGenerate(selectedTarget.paths[0])}><Sparkles size={14} /> Generate current snapshot</button>}
                  </div> : null}
                </div>

                <GenerationRecord origin={activeDocument?.generationOrigin} />
                {history.length > 0 && <div className="chapter-revision-history" role="group" aria-label="Immutable chapter revision history">
                  <span>Revision history</span>
                  {history.map((revision) => <button
                    key={revision.revisionId}
                    className={`chapter-history-item lifecycle-${revision.lifecycle}${selectedRevision?.revisionId === revision.revisionId ? ' is-current' : ''}`}
                    disabled={editing || busy}
                    onClick={() => { setSelectedRevisionId(revision.revisionId); setEditing(false); setCandidate(null); setValidation(null); setRejecting(false); setFeedback(null); onSelectRevision(revision); }}
                    aria-pressed={selectedRevision?.revisionId === revision.revisionId}
                    title={`${revision.lifecycle} · snapshot ${revision.snapshotId}`}
                  ><strong>r{revision.revisionNumber}</strong><span>{revision.lifecycle} · {new Date(revision.createdAtUnixMs).toLocaleDateString()}</span><small title={revision.basedOnRevisionId ? `Parent ${revision.basedOnRevisionId}` : 'No parent revision'}>snap {revision.snapshotId.slice(-8)} · {revision.basedOnRevisionId ? `from ${revision.basedOnRevisionId.slice(-8)}` : 'root'}</small></button>)}
                </div>}
                {previousRevision && selectedRevision && <details className="chapter-revision-comparison">
                  <summary>Compare r{previousRevision.revisionNumber} with r{selectedRevision.revisionNumber}</summary>
                  <ChapterRevisionComparison before={previousRevision} after={selectedRevision} />
                </details>}

                {selectedTarget.missing ? <div className="chapter-studio-empty"><BookOpen size={20} /><strong>{selectedTarget.overview ? 'No repository overview has been saved for this snapshot.' : 'This file has no chapter in the selected snapshot.'}</strong><p>Start with the captured source; the generated draft stays local until you choose to save it.</p>{selectedTarget.overview && <button className="primary-button" onClick={onGenerateOverview}><Sparkles size={14} /> Create optional overview</button>}</div>
                    : selectedRevision && activeDocument ? <>
                      {status === 'stale' && <div className="chapter-studio-stale"><TriangleAlert size={17} /><div><strong>This selected revision belongs to an older snapshot.</strong><p>Its citations stay pinned to snapshot {selectedRevision.snapshotId.slice(-12)}. Open a citation to inspect that exact captured code, or choose a current revision in the history above.</p></div></div>}
                      <div className="chapter-review-toolbar">
                        {editing ? <>
                          <span className="chapter-editing-label"><Pencil size={13} /> Editing a new draft from r{selectedRevision.revisionNumber}</span>
                          <div className="chapter-review-actions">
                            <button className="text-button" onClick={cancelEditing} disabled={busy}>Cancel</button>
                            <button className="secondary-button" onClick={() => { void validateCurrent(); }} disabled={busy}><Check size={14} /> Validate</button>
                            <button className="primary-button" onClick={() => { void saveEdits(); }} disabled={busy || !candidate}><Save size={14} /> Save as new draft</button>
                          </div>
                        </> : <>
                          <span className="chapter-validation-summary">{selectedRevision.lifecycle === 'accepted' ? 'Accepted revision is read-only.' : selectedRevision.lifecycle === 'rejected' ? `Rejected · ${selectedRevision.decisionNote ?? 'Reason not recorded'}` : `Evidence status: ${selectedRevision.validationStatus}`}</span>
                          <div className="chapter-review-actions">
                            <button className="secondary-button" onClick={() => { void validateCurrent(); }} disabled={busy}><Check size={14} /> Validate</button>
                            {freshnessKey && isNativeRuntime() && <button className="secondary-button" onClick={() => { void assessFreshness(); }} disabled={selectedFreshness?.loading} title="Compare this accepted revision with the selected snapshot without changing either one.">
                              {selectedFreshness?.loading ? 'Assessing…' : selectedFreshness?.report ? 'Reassess freshness' : 'Assess freshness'}
                            </button>}
                            {selectedTarget.latestCurrent && !isLatestCurrent && selectedRevision.revisionNumber < selectedTarget.latestCurrent.revisionNumber && <button className="secondary-button" onClick={() => { void restoreSelectedRevision(); }} disabled={busy || editing} title="Create a new draft from this earlier content and revalidate it against the current snapshot."><GitBranch size={14} /> Restore r{selectedRevision.revisionNumber} as draft</button>}
                            {isLatestCurrent && <button className="secondary-button" onClick={beginEditing} disabled={busy}><Pencil size={14} /> {selectedRevision.lifecycle === 'draft' ? 'Edit as new draft' : selectedRevision.lifecycle === 'rejected' ? 'Repair in new draft' : 'Start a new draft'}</button>}
                            {isLatestCurrent && selectedRevision.lifecycle === 'draft' && <>
                              <button className="primary-button" onClick={() => { void accept(); }} disabled={busy || selectedRevision.validationStatus !== 'valid'}><Check size={14} /> Accept</button>
                              <button className="secondary-button button-reject" onClick={() => setRejecting((value) => !value)} disabled={busy}><X size={14} /> Reject</button>
                            </>}
                          </div>
                        </>}
                      </div>

                      {rejecting && selectedRevision.lifecycle === 'draft' && <form className="chapter-rejection-form" onSubmit={(event) => { void reject(event); }}>
                        <label><span>Why is this draft rejected?</span><textarea disabled={busy} autoFocus rows={2} maxLength={4000} value={rejectReason} onChange={(event) => setRejectReason(event.target.value)} placeholder="Record what needs to change. The draft remains in history." required /></label>
                        <div><button className="text-button" type="button" onClick={() => setRejecting(false)}>Cancel</button><button className="secondary-button" type="submit" disabled={busy || !rejectReason.trim()}>Record rejection</button></div>
                      </form>}

                      {selectedFreshness?.loading && <p className="chapter-review-feedback tone-info" role="status">Comparing this accepted revision with snapshot {snapshotId.slice(-12)}…</p>}
                      {selectedFreshness?.error && <p className="coverage-storage-warning" role="alert">{selectedFreshness.error}<button className="text-button" onClick={() => { void assessFreshness(); }}>Retry</button></p>}
                      {selectedFreshness?.report && <section className="chapter-studio-freshness" aria-label="Read-only chapter freshness assessment">
                        <div className="chapter-review-feedback tone-info" role="status">
                          <strong>Read-only comparison</strong> · accepted revision r{selectedRevision.revisionNumber} from snapshot {selectedFreshness.report.fromSnapshotId.slice(-12)} to {selectedFreshness.report.toSnapshotId.slice(-12)}. This assessment does not edit the accepted revision or carry changes forward.
                          <div className="chapter-freshness-index-summary">
                            {(['from', 'to'] as const).map((side) => {
                              const index = selectedFreshness.report!.indexes[side];
                              const label = side === 'from' ? 'Older snapshot' : 'Selected snapshot';
                              const counts = index.indexedFiles !== null && index.totalFiles !== null
                                ? ` · ${index.indexedFiles}/${index.totalFiles} files indexed`
                                : '';
                              return <span key={side}>{label} index: {index.status}{counts}</span>;
                            })}
                          </div>
                        </div>
                        {selectedFreshness.report.sections.map((sectionResult) => {
                          const section = activeDocument.sections.find((item) => item.sectionId === sectionResult.sectionId);
                          const labels = { current: 'Current', safeToCarry: 'Safe to carry forward', stale: 'Stale', needsReview: 'Needs review' };
                          const sectionTitle = section?.title ?? sectionResult.sectionId;
                          const targetPath = selectedTarget.paths.length === 1 ? selectedTarget.paths[0] : null;
                          const sectionPaths = section && targetPath ? declaredCurrentPaths(section, targetPath) : [];
                          const dependentSections = Array.from(new Set(selectedFreshness.report!.reverseDependencies
                            .filter((dependency) => dependency.dependentSectionIds.includes(sectionResult.sectionId))
                            .flatMap((dependency) => dependency.dependentSectionIds)
                            .filter((id) => id !== sectionResult.sectionId)));
                          const assetIds = Array.from(new Set(selectedFreshness.report!.reverseDependencies
                            .filter((dependency) => dependency.dependentSectionIds.includes(sectionResult.sectionId))
                            .flatMap((dependency) => dependency.assetIds)));
                          return <article className="chapter-studio-section chapter-freshness-section" key={sectionResult.sectionId}>
                            <div className="chapter-section-title-row">
                              <h4>{sectionTitle}</h4>
                              <span className={`chapter-review-status status-${sectionResult.status}`}>{labels[sectionResult.status]}</span>
                              <span className="chapter-review-status">Evidence {sectionResult.evidenceState}</span>
                              {sectionResult.status === 'stale' && section && targetPath && isNativeRuntime() && <button
                                className="secondary-button"
                                disabled={busy || sectionPaths.length === 0 || sectionPaths.length > 12}
                                onClick={() => onRegenerateSection(selectedRevision, section, sectionPaths)}
                                title="Review and regenerate only this section using its declared source evidence."
                              ><Sparkles size={13} /> Regenerate this section</button>}
                            </div>
                            {sectionResult.reasons.length > 0 && <ul className="chapter-freshness-reasons">
                              {sectionResult.reasons.map((reason, index) => <li key={`${reason.code}:${reason.path ?? ''}:${index}`}>
                                <code>{reason.code}</code> {reason.message}{reason.path && <> <code>{reason.path}</code></>}
                              </li>)}
                            </ul>}
                            {sectionResult.sourceChanges.length > 0 && <div className="chapter-freshness-changes">
                              {sectionResult.sourceChanges.map((change, index) => {
                                const kind = change.kind.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/[_-]+/g, ' ');
                                const rangeText = (range: { startLine: number; endLine: number } | null) => range ? `${range.startLine}-${range.endLine}` : null;
                                return <div className="chapter-freshness-change" key={`${change.path}:${change.kind}:${index}`}>
                                  <div><code>{change.path}</code><span>{kind}</span></div>
                                  <div className="chapter-review-actions">
                                    {change.oldRange
                                      ? <button className="text-button" onClick={() => { onSelectRevision(selectedRevision, sectionResult.sectionId); onOpenFile(change.oldRange!.path, change.oldRange!.snapshotId, change.oldRange!); }}><FileCode2 size={12} /> Older lines {rangeText(change.oldRange)}</button>
                                      : <span className="chapter-freshness-unavailable">No older range</span>}
                                    {change.newRange
                                      ? <button className="text-button" onClick={() => { onSelectRevision(selectedRevision, sectionResult.sectionId); onOpenFile(change.newRange!.path, change.newRange!.snapshotId, change.newRange!); }}><FileCode2 size={12} /> Current lines {rangeText(change.newRange)}</button>
                                      : <span className="chapter-freshness-unavailable">No current range</span>}
                                  </div>
                                  <ChapterSourceDiff projectId={projectId} change={change} />
                                </div>;
                              })}
                            </div>}
                            {dependentSections.length > 0 && <p className="chapter-freshness-clear"><strong>Other sections share changed evidence:</strong> {dependentSections.map((id) => activeDocument.sections.find((item) => item.sectionId === id)?.title ?? id).join(', ')}. Each section is reviewed and updated separately.</p>}
                            <div className="chapter-freshness-assets">
                              <strong>Asset input fingerprints</strong>
                              {assetIds.length > 0 ? <ul>{assetIds.map((assetId) => <li key={assetId}><code>{assetId}</code></li>)}</ul> : <span>No visual or hotspot assets reference this section's changed evidence.</span>}
                              {sectionResult.narrationScriptSha256
                                ? <span>Narration script SHA-256 <code>{sectionResult.narrationScriptSha256}</code>. Audio cache keys also include the chosen model, voice, style, and format.</span>
                                : <span>No narration script is attached to this section.</span>}
                            </div>
                            {sectionResult.reasons.length === 0 && sectionResult.sourceChanges.length === 0 && <p className="chapter-freshness-clear">{sectionResult.status === 'needsReview' || sectionResult.evidenceState !== 'complete' ? 'The available index evidence is not enough to confirm this section.' : 'No changed evidence was found for this section.'}</p>}
                          </article>;
                        })}
                      </section>}

                      {feedback && <p className={`chapter-review-feedback tone-${feedback.tone}`} role="status">{feedback.message}</p>}
                      {validation && validation.issues.length > 0 && <div className="chapter-review-issues" role="alert"><strong>Evidence or schema issues</strong><ul>{validation.issues.map((issue, index) => {
                        const anchor = activeDocument.sections.flatMap((section) => section.claims.flatMap((claim) => claim.anchors)).find((item) => item.anchorId === issue.anchorId);
                        const range = anchor ? `:${anchor.startLine}-${anchor.endLine}` : '';
                        return <li key={`${issue.code}:${issue.sectionId}:${issue.visualItemId}:${issue.path}:${issue.anchorId}:${index}`}><span>{issue.code}</span> {issue.message}{issue.sectionId && <code>section {issue.sectionId}</code>}{issue.visualItemId && <code>visual item {issue.visualItemId}</code>}{issue.anchorId && <code>anchor {issue.anchorId}</code>}{issue.hotspotId && <code>hotspot {issue.hotspotId}</code>}{issue.path && <code>{issue.path}{range}</code>}</li>;
                      })}</ul></div>}

                      <div className="chapter-edit-title-row"><span>Chapter title</span>{editing && candidate ? <input disabled={busy} value={candidate.title} maxLength={200} onChange={(event) => { setCandidate({ ...candidate, title: event.target.value }); setValidation(null); }} aria-label="Chapter title" /> : null}</div>
                      {activeDocument.sections.map((section, index) => <article className={`chapter-studio-section${routeSectionId === section.sectionId ? ' is-route-focused' : ''}`} key={section.sectionId} id={`studio-section-${section.sectionId}`} tabIndex={-1} data-focus-key={`chapter-section:${section.sectionId}`}>
                        <div className="chapter-section-title-row"><span className="chapter-section-number">{String(index + 1).padStart(2, '0')}</span>{editing && candidate ? <input disabled={busy} value={section.title} maxLength={160} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, title: event.target.value }))} aria-label={`Section ${index + 1} title`} /> : <h4>{section.title}</h4>}
                          {editing && <div className="chapter-section-reorder"><button className="icon-button" onClick={() => moveSection(section.sectionId, -1)} disabled={busy || index === 0} aria-label={`Move ${section.title} up`}><MoveUp size={14} /></button><button className="icon-button" onClick={() => moveSection(section.sectionId, 1)} disabled={busy || index === activeDocument.sections.length - 1} aria-label={`Move ${section.title} down`}><MoveDown size={14} /></button></div>}
                        </div>
                        <div className="chapter-section-copy"><span>EXPLANATION</span>{editing && candidate ? <textarea disabled={busy} rows={3} value={section.prose} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, prose: event.target.value }))} aria-label={`Explanation for ${section.title}`} /> : <p>{section.prose}</p>}</div>
                        <div className="chapter-section-copy chapter-spoken-copy"><span>VOICE SCRIPT</span>{editing && candidate ? <textarea disabled={busy} rows={2} value={section.spokenText} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, spokenText: event.target.value }))} aria-label={`Spoken text for ${section.title}`} placeholder="Optional narration script" /> : <p>{section.spokenText || <em>No narration script saved.</em>}</p>}</div>
                        {section.claims.map((claim: ChapterClaim) => <div className="chapter-claim-card" key={claim.claimId}>
                          <div className="chapter-claim-heading"><span className={`claim-label claim-${claim.category.kind}`}>{claim.category.kind === 'codeBehavior' ? 'Code behavior' : claim.category.kind === 'repositoryContext' ? 'Repository context' : claim.category.visibleLabel || 'Inference'}</span>{editing && <button className="text-button chapter-remove-claim" disabled={busy} onClick={() => removeClaim(section.sectionId, claim.claimId)}><Trash2 size={12} /> Remove claim</button>}</div>
                          {editing && candidate ? <textarea disabled={busy} rows={2} value={claim.prose} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, claims: item.claims.map((saved) => saved.claimId === claim.claimId ? { ...saved, prose: event.target.value } : saved) }))} aria-label={`Claim ${claim.claimId}`} /> : <p>{claim.prose}</p>}
                          {claim.category.kind === 'inference' && <p className="chapter-inference-basis"><strong>{claim.category.visibleLabel} basis:</strong> {claim.category.basis}</p>}
                          <div className="chapter-citations"><span>EXACT SOURCE</span>{claim.anchors.length === 0 ? <small>No source range is attached to this claim.</small> : claim.anchors.map((anchor) => <div className="chapter-citation" key={anchor.anchorId}>
                            <div><code>{anchor.path}:{anchor.startLine}-{anchor.endLine}</code><button className="text-button" onClick={() => { onSelectRevision(selectedRevision, section.sectionId); onOpenFile(anchor.path, anchor.snapshotId, codeLensRangeFromChapterAnchor(anchor)); }}><FileCode2 size={12} /> Open source</button></div>
                            <pre>{anchor.excerpt}</pre>
                          </div>)}</div>
                        </div>)}
                      </article>)}
                    </> : <div className="chapter-studio-empty"><BookOpen size={20} /><strong>No revision selected.</strong><p>Choose a file or generate a chapter from its captured source.</p></div>}
              </>
            )}
          </main>
        </div>
      </section>
    </div>
  );
}

function CoverageTable({ rows, statusLabel, showGrouping, selectedPaths, allVisibleSelected, visibleEligible, excludePath, excludeReason, setExcludePath, setExcludeReason, onToggleSelected, onToggleVisible, onOpenFile, onReadRevision, onReviewRevision, onExclude, onRestore, onCancelExclude }: {
  rows: CoverageRow[];
  statusLabel: Record<CoverageStatus, string>;
  showGrouping: boolean;
  selectedPaths: string[];
  allVisibleSelected: boolean;
  visibleEligible: number;
  excludePath: string | null;
  excludeReason: string;
  setExcludePath: (path: string) => void;
  setExcludeReason: (reason: string) => void;
  onToggleSelected: (path: string) => void;
  onToggleVisible: () => void;
  onOpenFile: (path: string) => void;
  onReadRevision: (revisionId: string) => void;
  onReviewRevision: (revisionId: string) => void;
  onExclude: (event: FormEvent<HTMLFormElement>, row: CoverageRow) => void;
  onRestore: (path: string) => void;
  onCancelExclude: () => void;
}) {
  return (
    <div className="coverage-table-wrap">
      <div className={`coverage-table${showGrouping ? ' is-grouping' : ''}`} role="table" aria-label="Guide status for repository files">
        <div className="coverage-table-header" role="row">
          {showGrouping && <label className="coverage-select-all"><input type="checkbox" checked={allVisibleSelected} disabled={visibleEligible === 0} onChange={onToggleVisible} aria-label="Select all eligible files on this page" /><span>Group</span></label>}
          <span role="columnheader">File</span><span role="columnheader">Status</span><span role="columnheader">Next step</span>
        </div>
        {rows.length === 0 ? <div className="coverage-empty">No files in this view. Choose “All paths” or adjust your search and filters.</div> : rows.map((row) => (
          <div className="coverage-row" role="row" key={`${row.inventory.path}:${row.inventory.kind}:${row.inventory.reason}`}>
            {showGrouping && <div className="coverage-select-cell" role="cell">{row.canSelectForGroup ? <input type="checkbox" checked={selectedPaths.includes(row.inventory.path)} onChange={() => onToggleSelected(row.inventory.path)} aria-label={`Select ${row.inventory.path} for an explicit group`} /> : <span aria-hidden="true">—</span>}</div>}
            <div className="coverage-path-cell" role="cell">
              <code title={row.inventory.path}>{row.inventory.path}</code>
              <span>{row.inventory.language}{row.group ? ` · ${row.group.name}` : ''}</span>
              <details className="coverage-row-details">
                <summary>File details</summary>
                <div>{row.inventory.kind} · {row.inventory.disposition} · {row.inventory.contentKind} · {languageSupportLabel(languageSupportTier(row.inventory))}</div>
                <div>{languageSupportNote(languageSupportTier(row.inventory), row.inventory.language)}</div>
                {row.inventory.categories.length > 0 && <div>Category: {row.inventory.categories.join(' · ')}</div>}
                {row.group && <div className="coverage-row-group"><strong>Group: {row.group.name}</strong><span>{row.group.rationale}</span></div>}
                {row.reason && <div className={`coverage-reason coverage-reason-${row.status}`}>{row.reason}</div>}
                {row.status === 'excluded' && row.userExclusion && <div className="coverage-row-reason"><span>Your reason: {row.userExclusion.reason}</span><button className="text-button" onClick={() => onRestore(row.inventory.path)}>Include again</button></div>}
                {row.revisionId && <div>Revision {row.revisionNumber} · <code>{row.revisionId}</code></div>}
              </details>
            </div>
            <div className="coverage-status-cell" role="cell"><span className={`coverage-status coverage-status-${row.status}`}>{statusLabel[row.status]}</span>{row.status === 'unsupported' && <small>Captured text and citations are available; local structural analysis is not.</small>}{row.status === 'unavailable' && <small>Inventory is retained; no chapter is counted.</small>}</div>
            <div className="coverage-action-cell" role="cell">
              {row.status === 'accepted' && row.revisionId && <button className="text-button" onClick={() => onReadRevision(row.revisionId!)}><Play size={13} /> Read explanation</button>}
              {['draft', 'rejected', 'stale'].includes(row.status) && row.revisionId && <button className="text-button" onClick={() => onReviewRevision(row.revisionId!)}><Pencil size={13} /> Review explanation</button>}
              {row.canOpenSource && <button className="text-button" onClick={() => onOpenFile(row.inventory.path)}><FileCode2 size={13} /> Open source</button>}
              {row.canExclude && excludePath !== row.inventory.path && <button className="text-button" onClick={() => setExcludePath(row.inventory.path)}>Exclude with reason</button>}
              {excludePath === row.inventory.path && (
                <form className="coverage-exclusion-form" onSubmit={(event) => onExclude(event, row)}>
                  <label><span>Why exclude this file?</span><textarea autoFocus required rows={2} maxLength={300} value={excludeReason} onChange={(event) => setExcludeReason(event.target.value)} /></label>
                  <div><button className="text-button" type="button" onClick={onCancelExclude}>Cancel</button><button className="secondary-button" type="submit" disabled={!excludeReason.trim()}>Save exclusion</button></div>
                </form>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function makeCoverageId(): string {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? `grp_${crypto.randomUUID()}`
    : `grp_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
}

function AtlasEmptyState({ project, onOpen, onCreateSnapshot, busy, limits }: {
  project: RecentProject | null;
  onOpen: () => void;
  onCreateSnapshot: () => void;
  busy: boolean;
  limits: CaptureLimits | null;
}) {
  if (project) {
    return (
      <div className="empty-workspace atlas-empty project-preflight">
        <div className="project-preflight-card">
          <div className="preflight-heading">
            <div className="empty-state-icon"><FolderOpen size={23} /></div>
            <div>
              <div className="step-pill"><span>READY</span> PROJECT PREFLIGHT</div>
              <h2>{project.displayName} is ready for a snapshot.</h2>
              <p>Code Atlas checked folder access and Git metadata. Repository source files have not been read.</p>
            </div>
          </div>
          <dl className="project-facts">
            <div><dt>Selected folder</dt><dd title={project.selectedPath}>{project.selectedPath}</dd></div>
            <div><dt>Git root</dt><dd title={project.repositoryRoot}>{project.repositoryRoot}</dd></div>
            <div><dt>Branch</dt><dd>{project.branch ?? 'Detached or unborn HEAD'}</dd></div>
            <div><dt>HEAD</dt><dd>{project.head ?? 'No commit yet'}</dd></div>
            <div><dt>Folder access</dt><dd><span className="access-indicator" />Available</dd></div>
          </dl>
          <div className="preflight-footer">
            <div className="privacy-note"><HardDrive size={14} /><span>Only folder and Git metadata were checked. Create snapshot is the action that reads source bytes.</span></div>
            <button className="primary-button" data-focus-key="action:create-snapshot" onClick={onCreateSnapshot} disabled={busy}>
              <HardDrive size={16} /> {busy ? 'Starting capture…' : 'Create snapshot'} <span>LOCAL</span>
            </button>
          </div>
        </div>
        <CapturePolicy limits={limits} />
        <div className="preflight-next-step"><span className="step-number active">1</span><span><strong>Repository selected</strong><small>Source has not been captured</small></span><span className="step-connector" /><span className="step-number">2</span><span><strong>Create snapshot</strong><small>Start a bounded local read</small></span></div>
      </div>
    );
  }

  return (
    <div className="empty-workspace atlas-empty">
      <div className="empty-main-card">
        <div className="atlas-preview" aria-hidden="true">
          <div className="preview-grid" />
          <div className="preview-orbit orbit-one" />
          <div className="preview-orbit orbit-two" />
          <div className="preview-line line-one" />
          <div className="preview-line line-two" />
          <div className="preview-node node-a"><FolderOpen size={17} /><span>repository</span></div>
          <div className="preview-node node-b"><FileCode2 size={16} /><span>source files</span></div>
          <div className="preview-node node-c"><Workflow size={16} /><span>relationships</span></div>
          <div className="preview-center"><Network size={23} /></div>
          <span className="preview-label label-a">FILE TREE</span>
          <span className="preview-label label-b">SOURCE EVIDENCE</span>
        </div>
        <div className="empty-card-copy">
          <div className="step-pill"><span>01</span> START WITH THE CODEBASE</div>
          <h2>Bring a repository into view.</h2>
          <p>Code Atlas starts with a read-only local snapshot. Then you can browse the real file tree, inspect exact source, and build explanations file by file.</p>
          <button className="primary-button" data-focus-key="atlas:open-repository" onClick={onOpen}><FolderOpen size={16} /> Open a local repository <span>⌘O</span></button>
          <div className="privacy-note"><HardDrive size={14} /><span>Local browsing works without an account, API key, or network connection.</span></div>
        </div>
      </div>
      <div className="empty-steps">
        <div className="empty-step active"><span className="step-number">1</span><div><strong>Choose a repository</strong><small>Pick a local Git working copy</small></div></div>
        <div className="step-connector" />
        <div className="empty-step"><span className="step-number">2</span><div><strong>Capture a snapshot</strong><small>Choose when source is read</small></div></div>
        <div className="step-connector" />
        <div className="empty-step"><span className="step-number">3</span><div><strong>Explore the evidence</strong><small>Map, inspect, and understand</small></div></div>
      </div>
    </div>
  );
}

function CodeLensEmptyState({ projectId, snapshotId, manifest, onGoAtlas, onGoWalkthrough, onOpenFile, onRequestExplanation, onRequestRangeExplanation, source, sourceRange, sourceRangeIssue, loading, error, selectedPath, inline = false }: {
  projectId: string | null;
  snapshotId: string | null;
  manifest: SnapshotManifest | null;
  onGoAtlas?: () => void;
  onGoWalkthrough?: () => void;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onRequestExplanation: (path: string) => void;
  onRequestRangeExplanation: (path: string, range: SourceLineRange) => void;
  source: SnapshotSource | null;
  sourceRange: SourceLineRange | null;
  sourceRangeIssue: WorkspaceRoute['sourceRangeIssue'] | null;
  loading: boolean;
  error: string | null;
  selectedPath: string | null;
  inline?: boolean;
}) {
  const [showCompleteSource, setShowCompleteSource] = useState(false);
  const [showAllRelationships, setShowAllRelationships] = useState(false);
  const [snapshotIndex, setSnapshotIndex] = useState<SnapshotIndex | null>(null);
  const [relationshipLoading, setRelationshipLoading] = useState(false);
  const [relationshipError, setRelationshipError] = useState<string | null>(null);
  const citedLineRef = useRef<HTMLSpanElement>(null);
  const sourceLines = source ? splitCapturedSourceLines(source.contents) : [];
  const highlightedLines = source ? highlightSourceLines(source.path, sourceLines) : null;
  const citation = resolveCodeLensCitation(sourceRange ?? undefined, sourceRangeIssue ?? undefined, source?.sha256, sourceLines.length, source?.contents);
  const citationHashMismatch = citation.status === 'stale';
  const invalidAnchorReason = citation.status === 'unavailable'
    ? citation.reason === 'invalid-anchor'
      ? 'The saved citation anchor is not an object with a valid source range.'
      : citation.reason === 'invalid-line-range'
        ? 'The saved citation has an invalid line range. It must use positive 1-based lines with the end at or after the start.'
        : citation.reason === 'invalid-source-hash'
          ? 'The saved citation has an invalid SHA-256 value.'
          : citation.reason === 'missing-source-hash'
            ? 'This line range has no source SHA-256, so it cannot be verified as a citation.'
          : citation.reason === 'invalid-byte-range'
            ? 'The saved citation byte range is invalid, does not align to UTF-8 characters, or does not match its line range.'
            : `The citation requests lines ${sourceRange?.startLine}–${sourceRange?.endLine}, but the captured file has ${sourceLines.length} lines.`
    : null;
  const visibleSourceRange = citation.status === 'verified' ? citation.range : null;
  const sourceTextRanges = source ? sourceLineTextRanges(source.contents) : [];
  useEffect(() => {
    if (!projectId || !snapshotId || !selectedPath) {
      setSnapshotIndex(null);
      setRelationshipLoading(false);
      setRelationshipError(null);
      return;
    }
    if (!isNativeRuntime()) {
      setSnapshotIndex(null);
      setRelationshipLoading(false);
      setRelationshipError('Related source links are available in the Mac app.');
      return;
    }
    let active = true;
    setSnapshotIndex(null);
    setRelationshipLoading(true);
    setRelationshipError(null);
    void loadSnapshotIndex(projectId, snapshotId)
      .then((loaded) => {
        if (!active) return;
        setSnapshotIndex(loaded?.projectId === projectId && loaded.snapshotId === snapshotId ? loaded : null);
      })
      .catch((loadError) => {
        if (active) setRelationshipError(errorMessageOr(loadError, 'The saved snapshot index could not be opened.'));
      })
      .finally(() => { if (active) setRelationshipLoading(false); });
    return () => { active = false; };
  }, [projectId, selectedPath, snapshotId]);
  const relatedRelationships = useMemo(() => {
    if (!snapshotIndex || !selectedPath) return [];
    return snapshotIndex.relationships
      .filter((relationship) => relationship.evidence.path === selectedPath || relationship.targetPath === selectedPath)
      .sort((left, right) => left.kind.localeCompare(right.kind)
        || left.evidence.path.localeCompare(right.evidence.path)
        || left.evidence.startLine - right.evidence.startLine
        || left.relationshipId.localeCompare(right.relationshipId));
  }, [selectedPath, snapshotIndex]);
  const resolvedRelationships = useMemo(() => relatedRelationships.filter((relationship) => (
    relationship.confidence !== 'unresolved' && Boolean(relationship.targetId) && !relationship.unresolvedReason
  )), [relatedRelationships]);
  const unresolvedRelationships = useMemo(() => relatedRelationships.filter((relationship) => (
    relationship.confidence === 'unresolved' || !relationship.targetId || Boolean(relationship.unresolvedReason)
  )), [relatedRelationships]);
  const rangeStart = visibleSourceRange?.startLine ?? null;
  const rangeEnd = visibleSourceRange?.endLine ?? null;
  const contextBefore = 24;
  const contextAfter = 24;
  const firstDisplayedLine = rangeStart === null || showCompleteSource ? 1 : Math.max(1, rangeStart - contextBefore);
  const lastDisplayedLine = rangeEnd === null || showCompleteSource ? sourceLines.length : Math.min(sourceLines.length, rangeEnd + contextAfter);
  const displayedLines = sourceLines.slice(firstDisplayedLine - 1, lastDisplayedLine);

  useEffect(() => {
    setShowCompleteSource(false);
    setShowAllRelationships(false);
  }, [selectedPath, source?.snapshotId, source?.path, sourceRange?.startLine, sourceRange?.endLine, sourceRange?.sourceSha256]);

  const renderRelationshipRows = (relationships: typeof relatedRelationships) => relationships.map((relationship) => {
    const sourceSymbol = snapshotIndex?.symbols.find((symbol) => symbol.symbolId === relationship.sourceId);
    const targetSymbol = relationship.targetId
      ? snapshotIndex?.symbols.find((symbol) => symbol.symbolId === relationship.targetId)
      : null;
    const targetName = targetSymbol?.qualifiedName ?? targetSymbol?.name ?? relationship.targetLabel ?? relationship.targetPath ?? 'unresolved target';
    return <li key={relationship.relationshipId}>
      <div><strong>{codeLensRelationshipLabel(relationship.kind, relationship.confidence)}</strong>
        <span>{sourceSymbol?.qualifiedName ?? sourceSymbol?.name ?? relationship.sourceId} → {targetName}</span>
        <small>{relationship.confidence === 'unresolved' || !relationship.targetId || Boolean(relationship.unresolvedReason)
          ? relationship.unresolvedReason ?? 'The target could not be resolved in this snapshot.'
          : `${relationship.evidence.path}:${relationship.evidence.startLine}-${relationship.evidence.endLine}`}</small>
      </div>
      <button className="text-button" onClick={() => onOpenFile(
        relationship.evidence.path,
        snapshotIndex?.snapshotId,
        codeLensRangeFromIndexedAnchor(relationship.evidence),
      )}>Open evidence</button>
    </li>;
  });

  useEffect(() => {
    if (!visibleSourceRange || !source) return;
    const frame = requestAnimationFrame(() => citedLineRef.current?.scrollIntoView({
      block: 'center',
      behavior: scrollBehaviorForMotionPreference(window.matchMedia('(prefers-reduced-motion: reduce)').matches),
    }));
    return () => cancelAnimationFrame(frame);
  }, [source?.snapshotId, source?.path, visibleSourceRange?.startLine, visibleSourceRange?.endLine, showCompleteSource]);

  if (selectedPath && !isNativeRuntime()) {
    return (
      <div className={`empty-workspace simple-empty${inline ? ' source-inline-state' : ''}`} role="status" data-focus-key={inline ? 'files-source' : undefined} tabIndex={inline ? -1 : undefined}>
        <div className="empty-state-icon code-icon"><HardDrive size={22} /></div>
        <div className="step-pill"><span>LOCAL SOURCE</span> MAC APP REQUIRED</div>
        <h2>Open this file in the Code Atlas Mac app</h2>
        <p>The browser preview cannot read local snapshots. No source was loaded.</p>
        {!inline && onGoAtlas && <button className="secondary-button" onClick={onGoAtlas}>Back to inventory <ArrowRight size={15} /></button>}
      </div>
    );
  }
  if (selectedPath && (loading || (!source && !error))) {
    return <div className={`empty-workspace snapshot-loading${inline ? ' source-inline-state' : ''}`} role="status" data-focus-key={inline ? 'files-source' : undefined} tabIndex={inline ? -1 : undefined}><HardDrive size={18} /><span>Verifying and opening {selectedPath} from the selected snapshot…</span></div>;
  }
  if (selectedPath && error) {
    const capturedFile = manifest?.files.find((file) => file.path === selectedPath);
    const skippedFile = manifest?.skipped.find((path) => path.path === selectedPath);
    const unavailableState = classifyCodeLensUnavailableState({
      contentKind: capturedFile?.contentKind,
      skippedReason: skippedFile?.reason,
      manifestLoaded: Boolean(manifest),
    });
    const unavailableTitle = unavailableState === 'binary'
      ? 'Binary source cannot be shown as text.'
      : unavailableState === 'invalid-encoding'
        ? 'Captured source has invalid text encoding.'
        : unavailableState === 'oversized'
          ? 'Source exceeded the snapshot capture limit.'
          : unavailableState === 'uncaptured'
            ? 'Source was not captured in this snapshot.'
            : unavailableState === 'missing'
              ? 'Source is not captured in this snapshot.'
              : 'Captured source could not be opened.';
    const unavailableMessage = unavailableState === 'binary'
      ? 'The snapshot recorded this path as binary. Binary bytes are not decoded or substituted with the live file.'
      : unavailableState === 'invalid-encoding'
        ? 'The snapshot contains bytes that are not valid UTF-8 source text, so Code Atlas will not guess an encoding.'
        : unavailableState === 'oversized' || unavailableState === 'uncaptured'
          ? skippedFile ? `${inventoryReasonLabel(skippedFile.reason)}. ${skippedFile.details}` : error
          : unavailableState === 'missing'
            ? 'The selected snapshot has no captured or skipped entry for this path. The live repository was not checked.'
            : error;
    return (
      <div className={`empty-workspace simple-empty source-error${inline ? ' source-inline-state' : ''}`} role="alert" data-focus-key={inline ? 'files-source' : undefined} tabIndex={inline ? -1 : undefined}>
        <div className="empty-state-icon code-icon"><TriangleAlert size={22} /></div>
        <div className="step-pill"><span>SNAPSHOT</span> SOURCE UNAVAILABLE</div>
        <h2>{unavailableTitle}</h2>
        <p>{unavailableMessage} Code Atlas did not fall back to the live repository.</p>
        {!inline && onGoWalkthrough && <button className="secondary-button" onClick={onGoWalkthrough}><ArrowLeft size={15} /> Return to guide</button>}
        {!inline && onGoAtlas && <button className="secondary-button" onClick={onGoAtlas}>Back to files <ArrowRight size={15} /></button>}
      </div>
    );
  }
  if (selectedPath && source) {
    return (
      <div className={`empty-workspace source-view${inline ? ' source-view-inline' : ''}`} data-focus-key={inline ? 'files-source' : undefined} tabIndex={inline ? -1 : undefined}>
        <div className="source-view-toolbar">
          <div><span className="step-pill"><span>READ ONLY</span> SNAPSHOT SOURCE</span><h2>{source.path}</h2></div>
          <div className="source-view-actions">
            {!inline && onGoWalkthrough && <button className="secondary-button" onClick={onGoWalkthrough}><ArrowLeft size={15} /> Return to guide</button>}
            {visibleSourceRange && <button className="secondary-button" onClick={() => onRequestRangeExplanation(source.path, visibleSourceRange)}><Sparkles size={15} /> Explain cited range</button>}
            {!inline && <button className="secondary-button" onClick={() => onRequestExplanation(source.path)}><Sparkles size={15} /> Explain this file</button>}
            {!inline && onGoAtlas && <button className="secondary-button" onClick={onGoAtlas}>Back to files <ArrowRight size={15} /></button>}
          </div>
        </div>
        <div className="source-provenance">
          <span>{highlightedLines ? `${languageForPath(source.path)} source` : 'Plain text source'} · {sourceLines.length.toLocaleString()} lines</span>
          {visibleSourceRange && <strong>Citation verified · lines {visibleSourceRange.startLine}–{visibleSourceRange.endLine}</strong>}
          {citationHashMismatch && <strong className="is-stale">Citation stale</strong>}
          {invalidAnchorReason && <strong className="is-stale">Anchor unavailable</strong>}
          <details className="source-metadata-details"><summary>Snapshot details</summary><span>Snapshot <code>{source.snapshotId}</code></span><span>SHA-256 <code>{source.sha256}</code></span><span>{formatBytes(source.sizeBytes)}</span></details>
        </div>
        {citationHashMismatch && <div className="source-range-focus" role="alert">
          <span><strong>Stale citation</strong><small>The chapter expects source SHA-256 {sourceRange?.sourceSha256}, but this snapshot contains {source.sha256}. The line range was not highlighted and Code Atlas did not search for a nearby match.</small></span>
        </div>}
        {!citationHashMismatch && invalidAnchorReason && <div className="source-range-focus is-invalid" role="alert">
          <span><strong>Invalid or unavailable anchor</strong><small>{invalidAnchorReason} No lines are highlighted; the captured source remains available for inspection.</small></span>
        </div>}
        {!inline && !sourceRange && !sourceRangeIssue && <div className="source-range-focus is-unanchored" role="status">
          <span><strong>No citation anchor</strong><small>No source range was supplied. Showing the complete captured file with its original line numbers.</small></span>
        </div>}
        {visibleSourceRange && <div className="source-range-focus" role="status">
          <span><strong>Citation focus</strong><code>{source.path}:{visibleSourceRange.startLine}-{visibleSourceRange.endLine}</code>
            <small>{visibleSourceRange.startByte !== undefined && visibleSourceRange.endByteExclusive !== undefined
              ? `Highlighted exact UTF-8 bytes [${visibleSourceRange.startByte}, ${visibleSourceRange.endByteExclusive}); showing nearby context from the exact snapshot.`
              : 'Showing the cited lines with nearby context from the exact snapshot.'}</small>
          </span>
          <button className="text-button" onClick={() => setShowCompleteSource((value) => !value)}>{showCompleteSource ? 'Show cited context' : 'Show complete file'}</button>
        </div>}
        <pre className="snapshot-source-code" role="group" tabIndex={0} aria-label={visibleSourceRange
          ? `Read-only source from snapshot ${source.snapshotId}, citation lines ${visibleSourceRange.startLine} to ${visibleSourceRange.endLine}`
          : `Read-only source from snapshot ${source.snapshotId}`}>
          <code>
            {visibleSourceRange && !showCompleteSource && firstDisplayedLine > 1 && <span className="snapshot-source-omission">… {firstDisplayedLine - 1} earlier lines …</span>}
            {sourceLines.length === 0 && <span className="snapshot-source-empty">Empty file · 0 source lines.</span>}
            {displayedLines.map((line, offset) => {
              const lineNumber = firstDisplayedLine + offset;
              const isCited = Boolean(visibleSourceRange && lineNumber >= visibleSourceRange.startLine && lineNumber <= visibleSourceRange.endLine);
              const tokens = highlightedLines?.[lineNumber - 1];
              const lineTextRange = sourceTextRanges[lineNumber - 1];
              const exactRange = citation.status === 'verified' ? citation.textRange : undefined;
              const localStart = exactRange && lineTextRange ? Math.max(0, exactRange.start - lineTextRange.start) : 0;
              const localEnd = exactRange && lineTextRange ? Math.max(0, Math.min(lineTextRange.end - lineTextRange.start, exactRange.end - lineTextRange.start)) : 0;
              let tokenOffset = 0;
              const renderedText = (tokens ?? [{ text: line || ' ' }]).flatMap((token, tokenIndex) => {
                const chunks = exactRange
                  ? splitSourceTextAtRange(token.text, localStart - tokenOffset, localEnd - tokenOffset)
                  : [{ text: token.text, cited: false }];
                tokenOffset += token.text.length;
                return chunks.map((chunk, chunkIndex) => {
                  const key = `${lineNumber}:${tokenIndex}:${chunkIndex}`;
                  const text = token.kind
                    ? <span className={`syntax-${token.kind}`} key={key}>{chunk.text}</span>
                    : <span key={key}>{chunk.text}</span>;
                  return chunk.cited
                    ? <mark className="snapshot-source-cited-bytes" key={`${key}:citation`}>{text}</mark>
                    : text;
                });
              });
              return <span
                key={lineNumber}
                ref={lineNumber === visibleSourceRange?.startLine ? citedLineRef : undefined}
                className={`snapshot-source-line${isCited ? ' is-cited' : ''}`}
                id={`snapshot-source-line-${lineNumber}`}
              ><span className="snapshot-source-line-number" aria-hidden="true">{lineNumber}</span><span>{renderedText}</span></span>;
            })}
            {visibleSourceRange && !showCompleteSource && lastDisplayedLine < sourceLines.length && <span className="snapshot-source-omission">… {sourceLines.length - lastDisplayedLine} later lines …</span>}
          </code>
        </pre>
        {!inline && <p className="source-safety-note"><HardDrive size={13} /> Loaded from immutable local snapshot storage and checked against the manifest hash. The live working tree was not read.</p>}
        <section className="code-lens-related" aria-labelledby="code-lens-related-title">
          <details>
            <summary><span><strong id="code-lens-related-title">Related files and tests</strong><small>{relationshipLoading ? 'Loading the local index…' : `${resolvedRelationships.length} resolved · ${unresolvedRelationships.length} unresolved`}</small></span></summary>
            <p>Open an indexed link to inspect its exact source lines. Unresolved links are kept separate because the snapshot cannot prove their destination.</p>
            {relationshipError ? <p role="status">{relationshipError}</p>
              : resolvedRelationships.length > 0 ? <>
                <ul>{renderRelationshipRows(showAllRelationships ? resolvedRelationships : resolvedRelationships.slice(0, 8))}</ul>
                {resolvedRelationships.length > 8 && <button className="text-button" onClick={() => setShowAllRelationships((value) => !value)}>{showAllRelationships ? 'Show fewer links' : `Show all ${resolvedRelationships.length} resolved links`}</button>}
              </> : <p>{snapshotIndex ? 'No resolved caller, import, or test links point to or from this file.' : relationshipLoading ? 'The local snapshot index is loading.' : 'No matching local index is available for this snapshot.'}</p>}
            {unresolvedRelationships.length > 0 && <details className="code-lens-unresolved-links">
              <summary>Unresolved links ({unresolvedRelationships.length})</summary>
              <p>These references are visible for investigation but are not treated as proven connections.</p>
              <ul>{renderRelationshipRows(unresolvedRelationships)}</ul>
            </details>}
          </details>
        </section>
      </div>
    );
  }
  return (
    <div className={`empty-workspace simple-empty${inline ? ' source-inline-state' : ''}`} data-focus-key={inline ? 'files-source' : undefined} tabIndex={inline ? -1 : undefined}>
      <div className="empty-state-icon code-icon"><Code2 size={24} /></div>
      <div className="step-pill"><span>FILES</span> SOURCE</div>
      <h2>Choose a file to see its source.</h2>
      <p>Source opens from the selected snapshot and is checked against its saved hash. The live working copy is not substituted.</p>
      {!inline && onGoAtlas && <button className="secondary-button" data-focus-key="cta:code-lens-atlas" onClick={onGoAtlas}>Browse files <ArrowRight size={15} /></button>}
      <div className="state-note"><HardDrive size={14} /> No source file is selected.</div>
    </div>
  );
}

function WalkthroughEmptyState({ onGoPlan }: { onGoPlan: () => void }) {
  return (
    <div className="empty-workspace simple-empty walkthrough-empty">
      <div className="empty-state-icon play-icon"><Play size={22} /></div>
      <div className="step-pill"><span>GUIDE</span> READ</div>
      <h2>Explanations stay connected to code.</h2>
      <p>Accepted explanations appear one section at a time. Follow a source link, return to the same place, and continue when you are ready.</p>
      <button className="secondary-button" data-focus-key="cta:walkthrough-plan" onClick={onGoPlan}>Back to Guide <ArrowRight size={15} /></button>
      <div className="state-note"><BookOpen size={14} /> No accepted explanations in this project.</div>
    </div>
  );
}

function WorkflowEmptyState({ kind, onGoAtlas, onGoPlan }: { kind: 'plan' | 'studio'; onGoAtlas?: () => void; onGoPlan?: () => void }) {
  const isPlan = kind === 'plan';
  const Icon = isPlan ? ListChecks : BookOpen;
  return (
    <div className="empty-workspace simple-empty workflow-empty">
      <div className={`empty-state-icon ${isPlan ? 'plan-icon' : 'studio-icon'}`}><Icon size={23} /></div>
      <div className="step-pill"><span>{isPlan ? 'GUIDE' : 'EDIT'}</span> {isPlan ? 'GET STARTED' : 'REVIEW'}</div>
      <h2>{isPlan ? 'Capture a repository to start a guide.' : 'Choose an explanation to review.'}</h2>
      <p>{isPlan ? 'Open a local repository and capture its files. The Guide will show what needs an explanation and what is ready to read.' : 'Open the Guide and choose a draft or accepted explanation. You can check its claims beside the source.'}</p>
      <button className="secondary-button" data-focus-key={`cta:${kind}`} onClick={isPlan ? onGoAtlas : onGoPlan}>{isPlan ? 'Open Files' : 'Open Guide'} <ArrowRight size={15} /></button>
      <div className="state-note"><HardDrive size={14} /> {isPlan ? 'Source is read only when you choose to capture a snapshot.' : 'Nothing is sent to an AI provider until you review a request.'}</div>
    </div>
  );
}

export { App };
