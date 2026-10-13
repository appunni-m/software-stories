import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type RefObject } from 'react';
import { listen } from '@tauri-apps/api/event';
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
  getApplicationStatus,
  getCaptureLimits,
  assessChapterFreshness,
  getSnapshotIndexProgress,
  isNativeRuntime,
  loadSnapshotIndex,
  listRecentProjects,
  listInterruptedOperations,
  listPendingGeneratedChapterPreviews,
  loadPendingGeneratedChapterPreview,
  pickProjectFolder,
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
  type SnapshotIndex,
  type IndexProgress,
  type RelationshipKind,
  type GitComparisonResult,
} from './lib/commands';
import { WalkthroughScreen, type ReaderRouteContext } from './Walkthrough';
import { GitChangesScreen } from './GitChanges';
import {
  buildGeminiRequestReview,
  cancelGeminiRequest,
  discardGeminiRequestReview,
  discardGeneratedChapterDraft,
  getProviderCredentialStatus,
  getProviderDisclosure,
  removeGeminiApiKey,
  sendGeminiRequest,
  saveGeneratedChapterDraft,
  listChapterRevisions,
  listProjectChapterRevisions,
  validateChapterRevision,
  saveChapterEdits,
  acceptChapterDraft,
  rejectChapterDraft,
  storeGeminiApiKey,
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
  languageForPath,
  type CaptureDisposition,
  type InventoryCategory,
  type InventoryRow,
  type ParserReadiness,
} from './lib/inventory';
import { ATLAS_MAX_ZOOM, ATLAS_MIN_ZOOM, ATLAS_ZOOM_STEP, atlasRowIndexStatus, atlasRowIndexStatusLabel, buildInventoryTree, filterProvenRelationshipEdges, fitAtlasViewport, flattenInventoryTree, normalizeAtlasViewport, panAtlasViewport, projectProvenRelationshipGraph, summarizeAtlasPathIndexCounts, zoomAtlasViewport, type AtlasViewport, type ProvenGraphEdge } from './lib/atlas';
import { classifyCodeLensUnavailableState, codeLensRangeFromChapterAnchor, codeLensRangeFromIndexedAnchor, codeLensRelationshipLabel, highlightSourceLines, resolveCodeLensCitation, sourceLineTextRanges, splitCapturedSourceLines, splitSourceTextAtRange } from './lib/codeLens';
import { loadShellStartupData, PRIMARY_WORKSPACES, WORKSPACE_COPY } from './lib/shell';
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
type ProviderDialogMode =
  | { kind: 'settings'; recoveryOperationId?: string }
  | { kind: 'review'; projectId: string; snapshotId: string; targetPath: string; recoveryOperationId?: string }
  | { kind: 'overview'; projectId: string; snapshotId: string; recoveryOperationId?: string };

const workspaceIcons: Record<Workspace, typeof Network> = {
  atlas: Network,
  'code-lens': Code2,
  walkthrough: Play,
  'guide-plan': ListChecks,
  'chapter-studio': BookOpen,
  changes: GitBranch,
};

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

function useModalFocusTrap(dialogRef: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Tab') return;
      const dialog = dialogRef.current;
      if (!dialog) return;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(
        'a[href], button:not(:disabled), input:not(:disabled):not([type="hidden"]), select:not(:disabled), textarea:not(:disabled), [contenteditable="true"], [tabindex]:not([tabindex="-1"])',
      )).filter((element) => (
        !element.hasAttribute('hidden')
        && element.getAttribute('aria-hidden') !== 'true'
        && element.getClientRects().length > 0
      ));
      if (focusable.length === 0) {
        event.preventDefault();
        dialog.focus();
        return;
      }

      const active = document.activeElement;
      const activeIndex = focusable.indexOf(active as HTMLElement);
      if (!dialog.contains(active) || activeIndex < 0) {
        event.preventDefault();
        (event.shiftKey ? focusable[focusable.length - 1] : focusable[0]).focus();
      } else if (event.shiftKey && activeIndex === 0) {
        event.preventDefault();
        focusable[focusable.length - 1].focus();
      } else if (!event.shiftKey && activeIndex === focusable.length - 1) {
        event.preventDefault();
        focusable[0].focus();
      }
    };
    document.addEventListener('keydown', onKeyDown, true);
    return () => document.removeEventListener('keydown', onKeyDown, true);
  }, [dialogRef]);
}

function App() {
  const [navigation, setNavigation] = useState<NavigationState>(initialNavigationState);
  const [runtime, setRuntime] = useState<ApplicationStatus | null>(null);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice>(null);
  const [toolsOpen, setToolsOpen] = useState(false);
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const [projectBusy, setProjectBusy] = useState(false);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const [knownProjects, setKnownProjects] = useState<Record<string, RecentProject>>({});
  const [snapshotsByProject, setSnapshotsByProject] = useState<Record<string, SnapshotSummary[]>>({});
  const [captureByProject, setCaptureByProject] = useState<Record<string, SnapshotProgress>>({});
  const [snapshotManifest, setSnapshotManifest] = useState<SnapshotManifest | null>(null);
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
  const [sourceDocument, setSourceDocument] = useState<SnapshotSource | null>(null);
  const [sourceLoading, setSourceLoading] = useState(false);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [providerDialog, setProviderDialog] = useState<ProviderDialogMode | null>(null);
  const providerReturnFocusRef = useRef<HTMLElement | null>(null);
  const recoveredPreviewReturnFocusRef = useRef<HTMLButtonElement | null>(null);
  const searchButtonRef = useRef<HTMLButtonElement>(null);
  const projectButtonRef = useRef<HTMLButtonElement>(null);
  const pickerReturnFocusRef = useRef<string | undefined>(undefined);
  const currentWorkspace = navigation.current.workspace;
  const copy = WORKSPACE_COPY[currentWorkspace];
  const currentProject = navigation.current.projectId
    ? knownProjects[navigation.current.projectId] ?? recentProjects.find((project) => project.projectId === navigation.current.projectId) ?? null
    : null;
  const currentSnapshots = currentProject ? snapshotsByProject[currentProject.projectId] ?? [] : [];
  const currentCapture = currentProject ? captureByProject[currentProject.projectId] ?? null : null;
  const selectedSnapshot = currentSnapshots.find((snapshot) => snapshot.snapshotId === navigation.current.snapshotId) ?? null;

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
        message: error instanceof Error ? error.message : 'The local project list is unavailable.',
        actionLabel: 'Retry',
        action: () => { void retryRecentProjectsLoad(); },
        tone: 'error',
      });
    }
  }

  const completeRecoveredProviderReview = useCallback(async (operationId: string) => {
    try {
      await discardInterruptedOperation(operationId);
      setInterruptedOperations((current) => current.filter((operation) => operation.operationId !== operationId));
    } catch (error) {
      setNotice({
        title: 'Recovery notice could not be cleared',
        message: error instanceof Error ? error.message : 'The old request notice remains. The new request still requires fresh review and consent.',
        tone: 'error',
      });
    }
  }, []);

  const refreshInterruptedOperations = async () => {
    try {
      setInterruptedOperations(await listInterruptedOperations());
      setInterruptedOperationsUnavailable(false);
    } catch {
      setInterruptedOperationsUnavailable(true);
    }
  };

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
        message: error instanceof Error ? error.message : 'The local recovery copy is still available.',
        tone: 'error',
      });
    }
  };

  const discardRecovery = async (operation: InterruptedOperation) => {
    setRecoveryBusyId(operation.operationId);
    try {
      await discardInterruptedOperation(operation.operationId);
      setInterruptedOperations((current) => current.filter((item) => item.operationId !== operation.operationId));
    } catch (error) {
      setNotice({
        title: 'Recovery notice could not be cleared',
        message: error instanceof Error ? error.message : 'The local recovery record could not be updated.',
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
        setNotice({
          title: 'A fresh capture has started',
          message: 'The interrupted capture was not resumed. Existing snapshots remain available while this new capture runs.',
        });
      } catch (error) {
        setNotice({
          title: 'A fresh capture started; old recovery data remains',
          message: error instanceof Error ? error.message : 'The old staging bytes could not be cleared. The interrupted item remains available to discard.',
          tone: 'warning',
        });
      }
    } catch (error) {
      setNotice({
        title: 'Capture retry could not start',
        message: error instanceof Error ? error.message : 'Open the repository and try a new capture again.',
        tone: 'error',
      });
    } finally {
      setRecoveryBusyId(null);
    }
  };

  const reviewInterruptedGeneration = async (operation: InterruptedOperation) => {
    if (operation.kind !== 'providerGeneration') return;
    setRecoveryBusyId(operation.operationId);
    try {
      const opened = await openRecentProject(operation.projectId);
      if (!opened.project || (opened.status !== 'opened' && opened.status !== 'storageError')) {
        handleProjectOpenResult(opened);
        return;
      }
      handleProjectOpenResult(opened);
      setNavigation((state) => navigateToRoute(state, {
        workspace: 'chapter-studio',
        projectId: operation.projectId,
        snapshotId: operation.snapshotId,
        focusTarget: 'heading:chapter-studio',
      }));
      setProviderDialog(operation.targetPath
        ? {
          kind: 'review',
          projectId: operation.projectId,
          snapshotId: operation.snapshotId,
          targetPath: operation.targetPath,
          recoveryOperationId: operation.operationId,
        }
        : {
          kind: 'overview',
          projectId: operation.projectId,
          snapshotId: operation.snapshotId,
          recoveryOperationId: operation.operationId,
        });
    } catch (error) {
      setNotice({
        title: 'A fresh request review could not open',
        message: error instanceof Error ? error.message : 'Reopen the repository and review a new exact-context request.',
        tone: 'error',
      });
    } finally {
      setRecoveryBusyId(null);
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
      setProjectBusy(true);
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
      setNotice({
        title: 'Could not open the folder picker',
        message: error instanceof Error ? error.message : 'The native folder picker did not respond.',
        actionLabel: 'Try again',
        action: () => { void openLocalRepository(); },
        tone: 'error',
      });
    } finally {
      setProjectBusy(false);
    }
  };

  const openRecent = async (projectId: string) => {
    pickerReturnFocusRef.current = 'project-button';
    setProjectMenuOpen(false);
    setProjectBusy(true);
    try {
      handleProjectOpenResult(await openRecentProject(projectId));
    } catch (error) {
      setNotice({
        title: 'Could not open this recent project',
        message: error instanceof Error ? error.message : 'Local project metadata could not be read.',
        actionLabel: 'Choose a folder',
        action: () => { void openLocalRepository(); },
        tone: 'error',
      });
    } finally {
      setProjectBusy(false);
    }
  };

  const forgetRecent = async (project: RecentProject) => {
    setProjectMenuOpen(false);
    setProjectBusy(true);
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
        message: error instanceof Error ? error.message : 'The local project list could not be updated.',
        tone: 'error',
      });
    } finally {
      setProjectBusy(false);
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
        message: error instanceof Error ? error.message : 'The local capture service did not respond.',
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
        message: error instanceof Error ? error.message : 'The capture may have already finished.',
        tone: 'error',
      });
    }
  };

  const selectSnapshot = (summary: SnapshotSummary) => {
    if (!currentProject) return;
    setNavigation((state) => navigateToRoute(state, {
      workspace: state.current.workspace,
      projectId: currentProject.projectId,
      snapshotId: summary.snapshotId,
      focusTarget: `heading:${state.current.workspace}`,
    }));
    setSnapshotMenuOpen(false);
  };

  useEffect(() => {
    let active = true;
    void loadShellStartupData({ getApplicationStatus, getCaptureLimits, listRecentProjects, listInterruptedOperations, listPendingGeneratedChapterPreviews }).then((startup) => {
      if (!active) return;
      setRuntime(startup.applicationStatus);
      setRuntimeError(startup.applicationStatusUnavailable ? 'The desktop bridge did not respond. The workspace is still available.' : null);
      setCaptureLimits(startup.captureLimits);
      setInterruptedOperations(startup.interruptedOperations ?? []);
      setInterruptedOperationsUnavailable(startup.interruptedOperationsUnavailable);
      setPendingChapterPreviews(startup.pendingChapterPreviews ?? []);
      setPendingChapterPreviewsUnavailable(startup.pendingChapterPreviewsUnavailable);
      if (startup.recentProjectsUnavailable || startup.recentProjects === null) {
        setNotice({
          title: 'Recent projects could not be loaded',
          message: 'The workspace is still available. Retry the local Recents lookup or choose a repository to add it again.',
          actionLabel: 'Retry',
          action: () => { void retryRecentProjectsLoad(); },
          tone: 'error',
        });
      } else {
        setRecentProjects(startup.recentProjects);
        setKnownProjects(Object.fromEntries(startup.recentProjects.map((project) => [project.projectId, project])));
      }
    });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    const refresh = () => { void refreshPendingChapterPreviews(); };
    window.addEventListener('code-atlas:pending-chapter-previews-changed', refresh);
    return () => window.removeEventListener('code-atlas:pending-chapter-previews-changed', refresh);
  }, []);

  useEffect(() => {
    if (!currentProject) {
      setSnapshotManifest(null);
      return;
    }
    let active = true;
    void listProjectSnapshots(currentProject.projectId)
      .then(({ snapshots }) => {
        if (active) setSnapshotsByProject((current) => ({ ...current, [currentProject.projectId]: newestSnapshotsFirst(snapshots) }));
      })
      .catch((error) => {
        if (active && isNativeRuntime()) {
          setNotice({ title: 'Snapshot history could not be loaded', message: error instanceof Error ? error.message : 'Local snapshot history is unavailable.', tone: 'error' });
        }
      });
    return () => { active = false; };
  }, [currentProject?.projectId]);

  useEffect(() => {
    if (!currentProject || !navigation.current.snapshotId) {
      setSnapshotManifest(null);
      return;
    }
    setSnapshotManifest(null);
    let active = true;
    void loadSnapshotManifest(currentProject.projectId, navigation.current.snapshotId)
      .then((manifest) => { if (active) setSnapshotManifest(manifest); })
      .catch((error) => {
        if (active) setNotice({ title: 'Snapshot details could not be opened', message: error instanceof Error ? error.message : 'The immutable snapshot manifest is unavailable.', tone: 'error' });
      });
    return () => { active = false; };
  }, [currentProject?.projectId, navigation.current.snapshotId]);

  useEffect(() => {
    const route = navigation.current;
    if (route.workspace !== 'code-lens' || !route.projectId || !route.snapshotId || !route.relativePath) {
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
        if (active) setSourceError(error instanceof Error ? error.message : 'Snapshot source could not be opened.');
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
        if (providerDialog) {
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
        navigationActions.workspace('atlas');
      } else if (key === '2') {
        event.preventDefault();
        navigationActions.workspace('code-lens');
      } else if (key === '3') {
        event.preventDefault();
        navigationActions.workspace('walkthrough');
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
        searchButtonRef.current?.focus();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [navigation.current.snapshotId, navigationActions, notice, openLocalRepository, projectMenuOpen, providerDialog, snapshotMenuOpen, toolsOpen]);

  const showComingSoon = (title: string, message: string) => {
    setNotice({ title, message });
    setToolsOpen(false);
    setProjectMenuOpen(false);
    setSnapshotMenuOpen(false);
  };

  const openProviderSettings = () => {
    providerReturnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setToolsOpen(false);
    setProjectMenuOpen(false);
    setSnapshotMenuOpen(false);
    setProviderDialog({ kind: 'settings' });
  };

  const reviewFileWithGemini = (targetPath: string) => {
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
      return navigateToRoute(origin, {
        workspace: 'code-lens',
        projectId: currentProject.projectId,
        snapshotId: targetSnapshotId,
        relativePath,
        sourceRange,
        focusTarget: 'heading:code-lens',
      });
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
        workspace: 'code-lens',
        projectId: context.projectId,
        snapshotId: anchor.snapshotId,
        relativePath: anchor.path,
        sourceRange: codeLensRangeFromChapterAnchor(anchor),
        focusTarget: 'heading:code-lens',
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
        focusTarget: 'atlas-destination',
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

  return (
    <div className="app-window">
      <header className="topbar">
        <div className="brand-lockup" aria-label="Code Atlas">
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
            <span>{projectBusy ? 'Opening repository…' : currentProject?.displayName ?? 'Open a local repository'}</span>
            <ChevronDown size={14} className="muted-icon" />
          </button>
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

        <nav className="primary-nav" aria-label="Primary workspaces">
          {PRIMARY_WORKSPACES.map(({ id, label }, index) => {
            const Icon = workspaceIcons[id];
            return (
            <button
              key={id}
              className={`nav-tab ${currentWorkspace === id ? 'active' : ''}`}
              onClick={() => chooseWorkspace(id)}
              data-focus-key={`nav:${id}`}
              aria-current={currentWorkspace === id ? 'page' : undefined}
              aria-keyshortcuts={`Meta+${index + 1}`}
            >
              <Icon size={15} strokeWidth={1.9} />
              <span>{label}</span>
            </button>
            );
          })}
        </nav>

        <div className="topbar-actions">
          <button
            ref={searchButtonRef}
            className="search-control"
            disabled={!navigation.current.snapshotId}
            onClick={() => showComingSoon('Search needs a snapshot', 'Open and capture a repository to search its files and symbols.')}
            aria-label="Search files and symbols, Command K"
          >
            <Search size={15} />
            <span>Search repository</span>
            <kbd><Command size={10} /> K</kbd>
          </button>
          <div className="tools-wrap">
            <button
              className={`icon-button ${toolsOpen ? 'selected' : ''}`}
              onClick={() => { setProjectMenuOpen(false); setSnapshotMenuOpen(false); setToolsOpen((open) => !open); }}
              data-focus-key="tools-button"
              aria-label="Open project tools"
              aria-expanded={toolsOpen}
              aria-haspopup="menu"
            >
              <Settings2 size={17} />
            </button>
            {toolsOpen && (
              <div className="tools-menu" role="menu" aria-label="Project tools">
                <div className="menu-label">PROJECT WORKFLOWS</div>
                <button role="menuitem" data-focus-key="tools-button" onClick={() => chooseWorkspace('guide-plan')}>
                  <ListChecks size={16} /> <span>Guide Plan</span>
                </button>
                <button role="menuitem" data-focus-key="tools-button" onClick={() => chooseWorkspace('chapter-studio')}>
                  <BookOpen size={16} /> <span>Chapter Studio</span>
                </button>
                <button role="menuitem" data-focus-key="tools-button" onClick={() => chooseWorkspace('changes')}>
                  <GitBranch size={16} /> <span>Compare local revisions</span>
                </button>
              </div>
            )}
          </div>
          <button
            className="icon-button help-button"
            onClick={() => showComingSoon('Code Atlas help', 'Shortcuts: ⌘1 Atlas · ⌘2 Code Lens · ⌘3 Walkthrough · ⌘[ Back · ⌘] Forward.')}
            aria-label="Help and keyboard shortcuts"
          >
            <CircleHelp size={17} />
          </button>
        </div>
      </header>

      {projectMenuOpen && <button className="menu-scrim" aria-label="Close local repositories menu" onClick={() => { setProjectMenuOpen(false); projectButtonRef.current?.focus(); }} />}
      {toolsOpen && <button className="menu-scrim" aria-label="Close project tools" onClick={() => { setToolsOpen(false); document.querySelector<HTMLElement>('[data-focus-key="tools-button"]')?.focus(); }} />}
      {snapshotMenuOpen && <button className="menu-scrim" aria-label="Close snapshot history" onClick={() => { setSnapshotMenuOpen(false); document.querySelector<HTMLElement>('[data-focus-key="snapshot-button"]')?.focus(); }} />}

      <div className="app-body">
        <aside className="sidebar" aria-label="Project context">
          <div className="sidebar-section">
            <div className="sidebar-heading">CURRENT PROJECT</div>
            {currentProject ? (
              <div className="project-active-card">
                <div className="empty-folder"><FolderOpen size={18} /></div>
                <div className="project-active-copy">
                  <strong>{currentProject.displayName}</strong>
                  <p title={currentProject.repositoryRoot}>{currentProject.repositoryRoot}</p>
                  <small>{currentProject.branch ?? 'No branch'} · {currentProject.head?.slice(0, 10) ?? 'No commit'} · Access available</small>
                </div>
              </div>
            ) : (
              <div className="project-empty-card">
                <div className="empty-folder"><FolderOpen size={18} /></div>
                <div>
                  <strong>No repository yet</strong>
                  <p>Your code stays on this Mac.</p>
                </div>
              </div>
            )}
            <button
              className="sidebar-open-button"
              onClick={() => { void openLocalRepository(); }}
              data-focus-key="action:sidebar-open-repository"
            >
              <FolderOpen size={15} /> {currentProject ? 'Choose another repository' : 'Open repository'}
            </button>
          </div>

          <div className="sidebar-rule" />

          <div className="sidebar-section snapshot-sidebar">
            <div className="sidebar-heading">SNAPSHOT</div>
            {selectedSnapshot ? (
              <div className="snapshot-sidebar-summary">
                <div className="snapshot-sidebar-title"><HardDrive size={15} /><strong>{selectedSnapshot.status === 'partial' ? 'Partial capture' : 'Complete under policy'}</strong></div>
                <span>{selectedSnapshot.filesCaptured.toLocaleString()} files · {formatBytes(selectedSnapshot.totalBytes)}</span>
                <span>{selectedSnapshot.pathsSkipped.toLocaleString()} paths skipped</span>
                {currentSnapshots.length > 1 && <button onClick={() => setSnapshotMenuOpen(true)}>Switch snapshot</button>}
              </div>
            ) : (
              <>
                <div className="snapshot-empty"><HardDrive size={16} /><span>{currentProject ? 'No snapshot selected' : 'No captured source'}</span></div>
                <p className="sidebar-help">{currentProject ? currentSnapshots.length ? 'Choose a saved capture or create a new immutable snapshot.' : 'The repository is selected. Source files stay unread until you create a snapshot.' : 'A snapshot gives every map, citation, and chapter the same source revision.'}</p>
              </>
            )}
          </div>

          <div className="sidebar-bottom">
            <div className="local-status-dot" />
            <div>
              <strong>Local-first workspace</strong>
              <span>No source loaded or sent</span>
            </div>
          </div>
        </aside>

        <main className="workspace" id="main-content">
          <div className="workspace-toolbar">
            <div className="breadcrumb"><span>Workspace</span><span className="breadcrumb-slash">/</span><strong>{currentWorkspace === 'code-lens' ? 'Code Lens' : currentWorkspace === 'changes' ? 'Changes' : copy.title.split(' ')[0]}</strong></div>
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

          {runtimeError && (
            <div className="runtime-error" role="alert">
              <span>{runtimeError}</span>
              <button onClick={loadRuntime}>Retry</button>
            </div>
          )}

          {notice && (
            <div className={`notice-banner ${notice.tone ? `notice-${notice.tone}` : ''}`} role={notice.tone === 'error' ? 'alert' : 'status'}>
              <div className="notice-icon">{notice.tone === 'error' ? <TriangleAlert size={15} /> : <Check size={15} />}</div>
              <div><strong>{notice.title}</strong><span>{notice.message}</span></div>
              <div className="notice-actions">
                {notice.action && <button onClick={notice.action}>{notice.actionLabel ?? 'Continue'}</button>}
                <button onClick={() => setNotice(null)} aria-label="Dismiss message">Dismiss</button>
              </div>
            </div>
          )}

          {interruptedOperationsUnavailable && (
            <div className="notice-banner notice-error interrupted-work" role="alert">
              <div className="notice-icon"><TriangleAlert size={15} /></div>
              <div><strong>Recovery state could not be loaded</strong><span>Retry the local recovery lookup. Existing snapshots and accepted chapters are unchanged.</span></div>
              <div className="notice-actions">
                <button onClick={() => { void refreshInterruptedOperations(); }}>Retry</button>
              </div>
            </div>
          )}

          {interruptedOperations.map((operation) => (
            <div className="notice-banner notice-warning interrupted-work" role="status" key={operation.operationId}>
              <div className="notice-icon"><TriangleAlert size={15} /></div>
              <div>
                <strong>{operation.kind === 'capture' ? 'A repository capture was interrupted' : 'Chapter generation was interrupted'}</strong>
                <span>{operation.kind === 'capture'
                  ? 'Retry starts a fresh capture; existing snapshots remain available.'
                  : 'No generated revision was accepted. Review a new exact-context request; prior consent is not reused.'}</span>
              </div>
              <div className="notice-actions">
                {operation.kind === 'capture'
                  ? <button disabled={recoveryBusyId === operation.operationId} onClick={() => { void retryInterruptedCapture(operation); }}>Retry capture</button>
                  : <button disabled={recoveryBusyId === operation.operationId} onClick={() => { void reviewInterruptedGeneration(operation); }}>Review a new request</button>}
                <button disabled={recoveryBusyId === operation.operationId} onClick={() => { void discardRecovery(operation); }}>Discard</button>
              </div>
            </div>
          ))}

          {pendingChapterPreviewsUnavailable && (
            <div className="notice-banner notice-error interrupted-work" role="alert">
              <div className="notice-icon"><TriangleAlert size={15} /></div>
              <div><strong>Saved chapter previews could not be loaded</strong><span>Retry the local recovery lookup. Accepted chapters remain unchanged.</span></div>
              <div className="notice-actions"><button onClick={() => { void refreshPendingChapterPreviews(); }}>Retry</button></div>
            </div>
          )}

          {pendingChapterPreviews.map((preview) => (
            <div className="notice-banner notice-warning interrupted-work" role="status" key={preview.generationId}>
              <div className="notice-icon"><BookOpen size={15} /></div>
              <div>
                <strong>{preview.unavailableReason ? 'A saved chapter preview needs attention' : 'An unaccepted chapter preview is saved locally'}</strong>
                <span>{preview.unavailableReason ?? `${preview.title}${preview.targetPath ? ` · ${preview.targetPath}` : ' · repository overview'} · ${preview.modelId ?? 'saved provider'}. Review or discard it; saving creates only a draft revision.`}</span>
              </div>
              <div className="notice-actions">
                <button disabled={Boolean(preview.unavailableReason)} onClick={(event) => { recoveredPreviewReturnFocusRef.current = event.currentTarget; setRecoveredPreviewId(preview.generationId); }}>Review saved preview</button>
                <button onClick={() => { void discardPendingPreview(preview); }}>Discard</button>
              </div>
            </div>
          ))}

          <section className={`workspace-content view-${currentWorkspace}`} aria-labelledby="workspace-title">
            <div className="workspace-heading">
              <div className="eyebrow"><span className="eyebrow-mark" />{copy.eyebrow}</div>
              <h1 id="workspace-title" tabIndex={-1} data-focus-key={`heading:${currentWorkspace}`}>{copy.title}</h1>
              <p>{copy.subtitle}</p>
            </div>

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
                <SnapshotReadyState key={snapshotManifest.summary.snapshotId} manifest={snapshotManifest} atlasSelection={navigation.current.atlasSelection} mapViewport={navigation.current.mapViewport} onCaptureAgain={() => { void createSnapshot(); }} onOpenFile={openCapturedFile} onAtlasSelection={(selection, treeSelection, mapViewport) => {
                  setNavigation((state) => replaceCurrentRoute(state, {
                    ...state.current,
                    atlasSelection: undefined,
                    relativePath: selection && (selection.kind === 'file' || selection.kind === 'symbol') ? selection.path : undefined,
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
                onRequestExplanation={reviewFileWithGemini}
                source={sourceDocument}
                sourceRange={navigation.current.sourceRange ?? null}
                sourceRangeIssue={navigation.current.sourceRangeIssue ?? null}
                loading={sourceLoading}
                error={sourceError}
                selectedPath={navigation.current.relativePath ?? null}
              />
            ) : currentWorkspace === 'walkthrough' ? (
              currentProject ? (
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
              ) : <WalkthroughEmptyState onGoPlan={() => chooseWorkspace('guide-plan')} />
            ) : currentWorkspace === 'guide-plan' && currentProject && selectedSnapshot && snapshotManifest ? (
              <GuidePlanScreen
                key={`${currentProject.projectId}:${selectedSnapshot.snapshotId}`}
                projectId={currentProject.projectId}
                manifest={snapshotManifest}
                rows={buildInventory(snapshotManifest)}
                onGoAtlas={() => chooseWorkspace('atlas')}
                onGoStudio={() => chooseWorkspace('chapter-studio')}
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
              <GitChangesScreen
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
                onOpenProviderSettings={() => setProviderDialog({ kind: 'settings' })}
              />
            ) : (
              <WorkflowEmptyState kind="studio" onGoPlan={() => chooseWorkspace('guide-plan')} />
            )}

            <footer className="workspace-footer">
              <span>{runtime?.native ? 'Mac app' : 'Desktop preview'} <span className="footer-dot">·</span> {runtime?.version ?? 'Checking shell…'}</span>
              <span>{currentWorkspace === 'changes' ? 'Local Git comparison reads existing commit objects only. No checkout or remote request.' : selectedSnapshot ? `Immutable local snapshot ${selectedSnapshot.snapshotId.slice(-10)} · ${selectedSnapshot.status === 'partial' ? 'partial capture' : 'complete under policy'}` : currentCapture?.status === 'running' ? 'Capturing bounded repository bytes locally.' : currentProject ? 'No source bytes are read until you create a snapshot.' : 'No repository source is read until you choose a folder and create a snapshot.'}</span>
            </footer>
          </section>
        </main>
      </div>
      {providerDialog && (
        <ProviderDialog
          key={providerDialog.kind === 'settings' ? 'settings' : `${providerDialog.kind}:${providerDialog.projectId}:${providerDialog.snapshotId}:${providerDialog.kind === 'review' ? providerDialog.targetPath : ''}:${providerDialog.recoveryOperationId ?? ''}`}
          mode={providerDialog}
          native={runtime?.native ?? isNativeRuntime()}
          onClose={closeProviderDialog}
          onRecoveryReviewReady={completeRecoveredProviderReview}
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

function ProviderDialog({ mode, native, onClose, onRecoveryReviewReady }: {
  mode: ProviderDialogMode;
  native: boolean;
  onClose: () => void;
  onRecoveryReviewReady: (operationId: string) => void;
}) {
  const [disclosure, setDisclosure] = useState<ProviderDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCommandResult<ProviderCredentialStatus> | null>(null);
  const [informationLoading, setInformationLoading] = useState(native);
  const [informationError, setInformationError] = useState<string | null>(null);
  const [keyDraft, setKeyDraft] = useState('');
  const [keyBusy, setKeyBusy] = useState(false);
  const [keyMessage, setKeyMessage] = useState<string | null>(null);
  const [modelId, setModelId] = useState('gemini-3.8-flash');
  const [requestReview, setRequestReview] = useState<ProviderRequestReview | null>(null);
  const [snapshotManifest, setSnapshotManifest] = useState<SnapshotManifest | null>(null);
  const [snapshotManifestError, setSnapshotManifestError] = useState<string | null>(null);
  const [snapshotManifestLoading, setSnapshotManifestLoading] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<string[]>(mode.kind === 'review' ? [mode.targetPath] : []);
  const [contextQuery, setContextQuery] = useState('');
  const [requestBuilding, setRequestBuilding] = useState(false);
  const [requestBuildError, setRequestBuildError] = useState<ProviderFailure | null>(null);
  const [requestError, setRequestError] = useState<ProviderFailure | null>(null);
  const [sendResult, setSendResult] = useState<ChapterDraftPreview | null>(null);
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
      if (saveBusy) return;
      if (sending) {
        setCancelConfirmation(true);
        return;
      }
      if (requestReview) void discardGeminiRequestReview(requestReview.reviewId);
      onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onClose, requestReview, saveBusy, sending]);

  useEffect(() => {
    if (!reviewMode || !native) return;
    let active = true;
    setSnapshotManifestLoading(true);
    setSnapshotManifestError(null);
    void loadSnapshotManifest(mode.projectId, mode.snapshotId)
      .then((manifest) => { if (active) setSnapshotManifest(manifest); })
    .catch(() => { if (active) setSnapshotManifestError('The captured file list could not be loaded. Select source files again after reopening the snapshot.'); })
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
    Promise.all([getProviderDisclosure(), getProviderCredentialStatus()])
      .then(([nextDisclosure, nextCredential]) => {
        if (!active) return;
        setDisclosure(nextDisclosure);
        setCredential(nextCredential);
        const firstModel = nextDisclosure.models[0]?.modelId;
        if (firstModel) setModelId((current) => nextDisclosure.models.some((item) => item.modelId === current) ? current : firstModel);
      })
      .catch(() => {
        if (active) setInformationError('Code Atlas could not load local provider settings. No model request was sent.');
      })
      .finally(() => { if (active) setInformationLoading(false); });
    return () => { active = false; };
  }, [native]);

  useEffect(() => {
    if (!reviewMode || !native || !disclosure) return;
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
    setSavedRevision(null);
    setSaveError(null);
    void buildGeminiRequestReview({
      projectId: mode.projectId,
      snapshotId: mode.snapshotId,
      targetPath: mode.kind === 'review' ? mode.targetPath : null,
      selectedPaths,
      modelId,
    }).then((result) => {
      if (result.status === 'error') {
        if (active) setRequestBuildError(result.error);
        return;
      }
      createdReviewId = result.data.reviewId;
      if (active) {
        setRequestReview(result.data);
        if (mode.recoveryOperationId) onRecoveryReviewReady(mode.recoveryOperationId);
      }
      else void discardGeminiRequestReview(result.data.reviewId);
    }).catch(() => {
      if (active) setRequestBuildError({
        kind: 'invalidReview',
        message: 'Code Atlas could not build a local request preview. No model request was sent.',
      });
    }).finally(() => { if (active) setRequestBuilding(false); });
    return () => {
      active = false;
      if (createdReviewId) void discardGeminiRequestReview(createdReviewId);
    };
  }, [disclosure, mode, modelId, native, onRecoveryReviewReady, reviewGeneration, reviewMode, selectedPaths]);

  const updateConsent = (key: keyof ProviderConsent, checked: boolean) => {
    setConsent((current) => ({ ...current, [key]: checked }));
    setRequestError(null);
  };

  const handleSaveKey = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setKeyMessage(null);
    setKeyBusy(true);
    try {
      const result = await storeGeminiApiKey(keyDraft);
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
    setKeyMessage(null);
    setKeyBusy(true);
    try {
      const result = await removeGeminiApiKey();
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
    if (sending || saveBusy || sendResult) return;
    if (requestReview) void discardGeminiRequestReview(requestReview.reviewId);
    setConsent(noProviderConsent);
    setRequestReview(null);
    setSendResult(null);
    setUnavailableModel(null);
    setRequestError(null);
    setModelId(nextModelId);
  };

  const handleContextFileChange = (path: string, checked: boolean) => {
    if (sending || saveBusy || sendResult) return;
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
    if (!requestReview || sending || saveBusy || sendResult || !hasApiKey || !consent.privacyAndDataUseAcknowledged
      || !consent.billingUncertaintyAcknowledged || !consent.adultSupportedRegionAcknowledged
      || unavailableModel === modelId) return;
    setSending(true);
    cancelRequested.current = false;
    setCancelConfirmation(false);
    setCancelStatus(null);
    setRequestError(null);
    try {
      const result = await sendGeminiRequest({
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
            message: 'Cancellation was requested after Gemini had already returned. The response was discarded and not accepted. Google has already received the selected source.',
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
      const stopped = await cancelGeminiRequest(requestReview.reviewId);
      setCancelStatus(stopped
        ? 'Cancellation reached the local request. Google may already have received the selected source; no response will be accepted.'
        : 'The request had already finished locally. Its response will be discarded because you chose Cancel.');
    } catch {
      setCancelStatus('The app could not confirm a local abort. Google may already have received the selected source; any returned response will be discarded.');
    } finally {
      setCancelBusy(false);
    }
  };

  const handleClose = () => {
    if (sending || saveBusy) return;
    if (requestReview) void discardGeminiRequestReview(requestReview.reviewId);
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
        setSaveError(typeof error === 'string' ? error : 'The draft could not be saved. Existing revisions were preserved.');
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
      setSaveError(typeof error === 'string' ? error : 'The local recovery copy could not be discarded.');
      return;
    }
    if (requestReview) void discardGeminiRequestReview(requestReview.reviewId);
    window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    setSendResult(null);
    setSavedRevision(null);
    setSaveError(null);
    setRequestReview(null);
    setConsent(noProviderConsent);
    setReviewGeneration((generation) => generation + 1);
  };

  const requestReady = Boolean(
    requestReview && hasApiKey && consent.privacyAndDataUseAcknowledged
      && consent.billingUncertaintyAcknowledged && consent.adultSupportedRegionAcknowledged
      && unavailableModel !== modelId,
  );

  return (
    <div className="provider-overlay">
      <section ref={providerDialogRef} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="provider-dialog-title" tabIndex={-1}>
        <header className="provider-dialog-header">
          <div>
            <div className="step-pill"><span>{reviewMode ? 'EXACT CONTEXT' : 'OPTIONAL'}</span> GOOGLE GEMINI</div>
            <h2 id="provider-dialog-title">{reviewMode ? (overviewMode ? 'Map the repository' : 'Review before sending') : 'AI and narration setup'}</h2>
            <p>{reviewMode ? 'Inspect the exact snapshot source and disclosures. Nothing is sent until you confirm below.' : 'Local browsing works without Gemini. A key is optional and stays in macOS Keychain.'}</p>
          </div>
          <button className="icon-button" onClick={handleClose} disabled={sending || saveBusy} aria-label="Close provider dialog"><X size={17} /></button>
        </header>

        {!native ? (
          <div className="provider-preview-only" role="status">
            <HardDrive size={18} />
            <div><strong>Gemini requests run in the macOS app.</strong><p>The desktop preview does not read Keychain or send repository content. No API key or source was requested here.</p></div>
          </div>
        ) : informationLoading ? (
          <div className="provider-loading" role="status"><Sparkles size={17} /> Loading versioned disclosures and local Keychain status…</div>
        ) : informationError ? (
          <div className="provider-error" role="alert"><TriangleAlert size={16} /><span>{informationError}</span></div>
        ) : (
          <div className="provider-dialog-content">
            <details className="provider-key-card" open={!hasApiKey || !reviewMode}>
              <summary>Gemini API key · {credentialInfo?.keychainAvailable ? (hasApiKey ? 'stored in Keychain' : 'Keychain ready') : 'Keychain unavailable'}</summary>
              <div className="provider-key-content">
                <p>{credentialInfo?.explanation ?? credentialError ?? 'Code Atlas checks Keychain only after you open provider setup.'}</p>
                <form className="provider-key-form" onSubmit={(event) => { void handleSaveKey(event); }}>
                  <label htmlFor="gemini-api-key">Your Gemini API key</label>
                  <input
                    id="gemini-api-key"
                    type="password"
                    value={keyDraft}
                    onChange={(event) => setKeyDraft(event.target.value)}
                    autoComplete="new-password"
                    spellCheck={false}
                    placeholder="Paste a key from Google AI Studio"
                    disabled={!credentialInfo?.keychainAvailable || keyBusy}
                  />
                  <div className="provider-key-actions">
                    <button className="primary-button" type="submit" disabled={!credentialInfo?.keychainAvailable || keyDraft.length === 0 || keyBusy}>{keyBusy ? 'Saving…' : 'Save to Keychain'}</button>
                    {hasApiKey && <button className="secondary-button" type="button" onClick={() => { void handleRemoveKey(); }} disabled={keyBusy}>Remove saved key</button>}
                    {disclosure && <a href={disclosure.apiKeyUrl} target="_blank" rel="noreferrer">Get a Gemini API key ↗</a>}
                  </div>
                </form>
                {keyMessage && <p className="provider-status-note" role="status">{keyMessage}</p>}
                {credentialError && <p className="provider-error-text" role="alert">{credentialError}</p>}
                {disclosure && <p className="provider-small-note">{disclosure.apiKeySecurityNotice}</p>}
                <p className="provider-small-note">The key is sent only in an HTTPS request header from Rust. It is not returned to the WebView or written to project files, settings, or logs.</p>
              </div>
            </details>

            {disclosure && (
              <section className="provider-disclosure-card" aria-labelledby="provider-model-title">
                <div className="provider-section-heading">
                  <div><span>MODEL ALLOWLIST · CHECKED {disclosure.checkedDate}</span><h3 id="provider-model-title">Versioned models and current terms</h3></div>
                  <code>{disclosure.version}</code>
                </div>
                {reviewMode && (
                  <label className="provider-model-select">Model
                    <select value={modelId} onChange={(event) => handleModelChange(event.target.value)} disabled={sending || saveBusy || Boolean(sendResult) || disclosure.models.length === 0}>
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
                    </article>
                  ))}
                </div>
                <div className="provider-source-links">
                  <a href={disclosure.pricingUrl} target="_blank" rel="noreferrer">Current Google pricing ↗</a>
                  <a href={disclosure.termsUrl} target="_blank" rel="noreferrer">Gemini API terms ↗</a>
                  <a href={disclosure.regionsUrl} target="_blank" rel="noreferrer">Supported regions ↗</a>
                </div>
                <p className="provider-small-note">The model list is shipped with the app; Code Atlas does not discover models, change a model automatically, or use a paid fallback. Pricing and availability can change after the checked date.</p>
              </section>
            )}

            {reviewMode && (
              <section className="provider-review-card" aria-labelledby="provider-review-title">
                  <div className="provider-section-heading">
                  <div><span>LOCAL PREVIEW · {overviewMode ? 'REPOSITORY OVERVIEW' : 'FILE CHAPTER DRAFT'}</span><h3 id="provider-review-title">Exact request context</h3></div>
                  {requestReview && <span className="provider-expiry">Expires {new Intl.DateTimeFormat(undefined, { timeStyle: 'short' }).format(new Date(requestReview.expiresAtUnixMs))}</span>}
                </div>
                {requestBuilding && <div className="provider-loading" role="status"><HardDrive size={16} /> Building a local preview from the selected snapshot. No model request is sent.</div>}
                {requestBuildError && <div className="provider-error" role="alert"><TriangleAlert size={16} /><span>{requestBuildError.message}</span></div>}
                {cancelConfirmation && sending && (
                  <div className="provider-cancel-confirm" role="group" aria-labelledby="provider-cancel-title" aria-describedby="provider-cancel-description">
                    <strong id="provider-cancel-title">Cancel this request?</strong>
                    <p id="provider-cancel-description">Code Atlas will try to stop its local HTTPS request. Google may already have received the selected source. Any returned response will be discarded.</p>
                    <div><button className="secondary-button" onClick={() => setCancelConfirmation(false)}>Keep waiting</button><button className="danger-button" onClick={() => { void handleConfirmCancel(); }} disabled={cancelBusy}>{cancelBusy ? 'Stopping…' : 'Confirm cancel'}</button></div>
                  </div>
                )}
                {cancelStatus && <p className="provider-small-note" role="status">{cancelStatus}</p>}
                <div className="provider-context-picker">
                  <div className="provider-context-heading"><strong>Choose source files</strong><span>{selectedPaths.length} of 12 selected</span></div>
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
                </div>
                {requestReview && (
                  <>
                    <div ref={requestSummaryRef} className="provider-request-summary" tabIndex={-1} aria-label="Reviewed request summary">
                      <div><span>{requestReview.targetPath ? 'Target file' : 'Chapter scope'}</span><code>{requestReview.targetPath ?? 'Repository overview'}</code></div>
                      <div><span>Provider and model</span><strong>{requestReview.providerId} · {requestReview.modelId}</strong></div>
                      <div><span>Endpoint</span><code>{requestReview.endpointUrl}</code></div>
                      <div><span>Source snapshot</span><code>{requestReview.snapshotId}</code></div>
                      <div><span>Source selected</span><strong>{requestReview.selectedFiles.length} {requestReview.selectedFiles.length === 1 ? 'file' : 'files'} · {formatBytes(requestReview.sourceBytes)}</strong></div>
                      <div><span>Input estimate</span><strong>About {requestReview.estimatedInputTokens.toLocaleString()} tokens · not a bill</strong></div>
                      <div><span>Request body</span><strong>{formatBytes(requestReview.requestBodyBytes)}</strong></div>
                      <div><span>Local index evidence</span><strong>{requestReview.selectedSymbols.length} declarations · {requestReview.selectedRelationships.length} relationships{requestReview.indexEvidenceTruncated ? ' · incomplete context' : ''}</strong></div>
                    </div>
                    <p className="provider-warning-note"><TriangleAlert size={14} /> Review code for credentials and confidential or personal data before sending. The local secret-like scan catches common names and patterns, not every secret.</p>
                    {requestReview.selectedFiles.map((file) => (
                      <details className="provider-source-file" open key={`${file.path}:${file.sha256}`}>
                        <summary><code>{file.path}</code><span>{formatBytes(file.sizeBytes)} · SHA-256 {file.sha256.slice(0, 16)}…</span></summary>
                        <pre><code>{file.contents}</code></pre>
                      </details>
                    ))}
                    <details className="provider-index-preview">
                      <summary>Inspect selected declarations and relationships</summary>
                      <p>These entries come from the saved local index. Every entry refers to selected source; unresolved relationships remain uncertain.</p>
                      {requestReview.indexEvidenceTruncated && <p>The index context is incomplete or reached its bound. Omitted entries are not sent or available as chapter evidence.</p>}
                      <ul>{requestReview.selectedSymbols.map((symbol) => <li key={symbol.symbolId}><code>{symbol.path}:{symbol.startLine}–{symbol.endLine}</code><strong>{symbol.kind} · {symbol.qualifiedName ?? symbol.name}</strong><small>{symbol.isExported ? 'exported' : 'local declaration'}{symbol.isTest ? ' · test' : ''}</small></li>)}</ul>
                      <ul>{requestReview.selectedRelationships.map((relationship) => <li key={relationship.relationshipId}><strong>{relationship.kind} · {relationship.confidence} confidence</strong><code>{relationship.sourcePath} → {relationship.targetPath ?? 'unresolved'}</code><small>{relationship.unresolvedReason ?? `${relationship.sourceName ?? 'file'} → ${relationship.targetName ?? 'file'}`}</small><pre>{relationship.evidenceExcerpt}</pre></li>)}</ul>
                      {requestReview.selectedSymbols.length === 0 && requestReview.selectedRelationships.length === 0 && <p>No saved structural evidence is available for this selection. The draft must use exact source text and identify its limits.</p>}
                    </details>
                    <details className="provider-prompt-preview">
                      <summary>Inspect exact prompt and encoded provider request</summary>
                      <div><strong>System instruction</strong><pre>{requestReview.systemInstruction}</pre><strong>User prompt</strong><pre>{requestReview.userPrompt}</pre><strong>Generation settings</strong><pre>{JSON.stringify(requestReview.generationConfig, null, 2)}</pre><strong>Exact JSON request body · SHA-256 {requestReview.requestBodySha256}</strong><pre>{JSON.stringify(requestReview.requestBodyPreview, null, 2)}</pre></div>
                    </details>
                    <div className="provider-fingerprint"><span>Consent applies to this request fingerprint</span><code>{requestReview.fingerprint}</code><small>Provider, model, endpoint, exact encoded body hash, prompt purpose, source paths and hashes, prompt version, disclosure version, and generation settings are included.</small></div>
                  </>
                )}
                {requestError && <div className="provider-error" role="alert"><TriangleAlert size={16} /><div><strong>{providerFailureTitle(requestError.kind)}</strong><span>{requestError.message}</span></div></div>}
                {unavailableModel === modelId && <p className="provider-error-text" role="alert">This model was rejected by Google. Choose another listed model explicitly; Code Atlas will not switch it for you.</p>}
                {sendResult && (
                  <section className="provider-response" aria-labelledby="provider-response-title">
                    <div><span>{savedRevision ? 'SAVED · UNACCEPTED DRAFT' : 'VALIDATED · UNSAVED PREVIEW'}</span><h4 id="provider-response-title">{sendResult.revision.title}</h4></div>
                    <p>{savedRevision ? `Draft revision ${savedRevision.revisionNumber} is saved locally. Guide Plan counts it as a draft. Acceptance requires a separate review.` : 'The schema and exact source citations passed local validation. Review what the claims mean, then save an unaccepted draft. Closing keeps this preview in local recovery until you save or discard it.'}</p>
                    <GeneratedChapterPreview revision={sendResult.revision} />
                    {saveError && <p className="provider-error-text" role="alert">{saveError}</p>}
                    <small>{sendResult.inputTokens ?? '—'} input tokens · {sendResult.outputTokens ?? '—'} output tokens · one request · no fallback</small>
                    <small>{sendResult.modelVersion ?? sendResult.modelId}{savedRevision ? ` · ${savedRevision.revisionId}` : ''}</small>
                  </section>
                )}
                {disclosure && (
                  <div className="provider-consent-list">
                    <label><input type="checkbox" checked={consent.privacyAndDataUseAcknowledged} onChange={(event) => updateConsent('privacyAndDataUseAcknowledged', event.target.checked)} disabled={sending || Boolean(sendResult)} /><span><strong>Privacy and data use</strong>{disclosure.unpaidServiceDataUse} {disclosure.regionalDataUseException}</span></label>
                    <label><input type="checkbox" checked={consent.billingUncertaintyAcknowledged} onChange={(event) => updateConsent('billingUncertaintyAcknowledged', event.target.checked)} disabled={sending || Boolean(sendResult)} /><span><strong>Billing uncertainty</strong>{disclosure.billingUncertainty}</span></label>
                    <label><input type="checkbox" checked={consent.adultSupportedRegionAcknowledged} onChange={(event) => updateConsent('adultSupportedRegionAcknowledged', event.target.checked)} disabled={sending || Boolean(sendResult)} /><span><strong>Age, region, and use</strong>{disclosure.adultAndRegionRequirements}</span></label>
                  </div>
                )}
              </section>
            )}
          </div>
        )}

        <footer className="provider-dialog-footer">
          <button className="secondary-button" onClick={handleClose} disabled={sending || saveBusy}>{sending ? 'Gemini request in progress…' : savedRevision ? 'Done' : 'Close'}</button>
          {reviewMode && (
            sending ? (
              <button className="secondary-button" onClick={() => setCancelConfirmation(true)} disabled={cancelBusy || cancelRequested.current}>
                {cancelBusy || cancelRequested.current ? 'Cancelling…' : 'Cancel request'}
              </button>
            ) : sendResult ? (
              savedRevision ? <span className="provider-saved-status" role="status"><Check size={15} /> Draft saved locally</span> : <div className="provider-draft-actions"><button className="secondary-button" onClick={() => { void handleDiscardPreview(); }} disabled={saveBusy}>Discard preview</button><button className="primary-button" onClick={() => { void handleSaveDraft(); }} disabled={saveBusy}><Check size={15} /> {saveBusy ? 'Saving draft…' : 'Save unaccepted draft'}</button></div>
            ) : (
              <button className="primary-button" onClick={() => { void handleSend(); }} disabled={!requestReady || requestBuilding || informationLoading || Boolean(sendResult)}>
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
        setError(typeof failure === 'string' ? failure : 'The saved preview could not be revalidated against its local snapshot.');
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
        setError(typeof failure === 'string' ? failure : 'The draft could not be saved. Existing revisions were preserved.');
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
      setError(typeof failure === 'string' ? failure : 'The local recovery copy could not be discarded.');
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
    missingApiKey: 'Gemini key required',
    invalidApiKey: 'Gemini key rejected',
    consentRequired: 'Review and confirm every disclosure',
    reviewExpired: 'Request review expired',
    reviewChanged: 'Request context changed',
    reviewUnavailable: 'Request review unavailable',
    localStorageUnavailable: 'Local storage unavailable',
    invalidReview: 'Request could not be validated',
    contextTooLarge: 'Selected context is too large',
    secretLikeContext: 'Secret-like path blocked',
    unavailableModel: 'Model unavailable',
    billingRequired: 'Google requires billing for this key',
    quotaExceeded: 'Google quota or rate limit reached',
    regionUnavailable: 'Google Gemini is unavailable in this region',
    networkUnavailable: 'Gemini could not be reached',
    requestTimedOut: 'Gemini request timed out',
    requestCancelled: 'Request cancelled',
    providerRejectedRequest: 'Google rejected this request',
    providerResponseTooLarge: 'Response exceeded the size limit',
    malformedProviderResponse: 'Gemini returned an unreadable response',
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

function SnapshotReadyState({ manifest, atlasSelection, mapViewport, onCaptureAgain, onOpenFile, onAtlasSelection, onAtlasViewportChange }: { manifest: SnapshotManifest; atlasSelection?: WorkspaceRoute['atlasSelection']; mapViewport?: AtlasViewport; onCaptureAgain: () => void; onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void; onAtlasSelection: (selection: WorkspaceRoute['atlasSelection'], treeSelection: string, mapViewport?: AtlasViewport) => void; onAtlasViewportChange: (mapViewport: AtlasViewport) => void }) {
  const { summary } = manifest;
  const partial = summary.status === 'partial';
  const inventory = useMemo(() => buildInventory(manifest), [manifest]);
  const omittedRows = inventory.filter((row) => row.disposition !== 'captured').length;
  const destinationRef = useRef<HTMLDivElement>(null);
  const atlasSelectionKey = atlasSelection ? JSON.stringify(atlasSelection) : '';

  useEffect(() => {
    if (!atlasSelection) return;
    const frame = requestAnimationFrame(() => destinationRef.current?.focus({ preventScroll: false }));
    return () => cancelAnimationFrame(frame);
  }, [atlasSelectionKey, summary.snapshotId]);

  return (
    <div className="empty-workspace snapshot-ready-state">
      <section className="snapshot-ready-card" aria-labelledby="snapshot-ready-title">
        <div className="snapshot-ready-heading">
          <div className="snapshot-state-icon"><Check size={22} /></div>
          <div>
            <div className="step-pill"><span>{partial ? 'PARTIAL' : 'IMMUTABLE'}</span> SOURCE SNAPSHOT</div>
            <h2 id="snapshot-ready-title">{partial ? 'Snapshot saved with visible gaps.' : 'Repository captured under policy.'}</h2>
            <p>{formatSnapshotDate(summary.createdAtUnixMs)} · {summary.branch ?? 'Detached or unborn HEAD'} · {summary.head ?? 'No commit yet'}</p>
          </div>
          <button className="secondary-button" onClick={onCaptureAgain}>Capture again</button>
        </div>
        <div className="snapshot-metrics">
          <div><strong>{inventory.length.toLocaleString()}</strong><span>inventory paths</span></div>
          <div><strong>{summary.filesCaptured.toLocaleString()}</strong><span>files captured</span></div>
          <div><strong>{formatBytes(summary.totalBytes)}</strong><span>source bytes</span></div>
          <div><strong>{omittedRows.toLocaleString()}</strong><span>excluded or skipped rows</span></div>
        </div>
        <div className="snapshot-identity-row"><span>Snapshot ID</span><code>{summary.snapshotId}</code><span>Parent</span><code>{summary.parentSnapshotId ?? 'First capture'}</code></div>
        <div className="snapshot-dirty-state"><GitBranch size={15} /><span>Git index: <strong>{summary.indexState}</strong></span><span>Working tree: <strong>{summary.worktreeState}</strong></span><span>HEAD: <code>{summary.head?.slice(0, 12) ?? 'unborn'}</code></span></div>
        {atlasSelection && <AtlasDestinationCard refElement={destinationRef} manifest={manifest} selection={atlasSelection} onOpenFile={onOpenFile} />}
        <AtlasWorkbench key={`${summary.projectId}:${summary.snapshotId}`} manifest={manifest} rows={inventory} routeSelection={atlasSelection} routeViewport={mapViewport} onOpenFile={onOpenFile} onAtlasSelection={onAtlasSelection} onViewportChange={onAtlasViewportChange} />
        <CapturePolicy limits={manifest.captureLimits} />
        <div className="snapshot-next-step"><HardDrive size={15} /><span>{partial ? 'This capture is partial. Search and review skipped rows before using it as source evidence.' : 'Complete under the visible capture policy. Open any captured text file to inspect the exact snapshot bytes.'}</span></div>
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
        if (active) setIndexError(error instanceof Error ? error.message : 'The local source index could not be opened.');
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
        {selectedPath && <button className="atlas-destination-open" data-focus-key="atlas-destination-open" onClick={() => onOpenFile(selectedPath, snapshotId, selectedRange)}><FileCode2 size={14} /> {selectedRange ? 'Open exact source evidence' : 'Open this file in Code Lens'} <ArrowRight size={13} /></button>}
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

function AtlasWorkbench({ manifest, rows, routeSelection, routeViewport, onOpenFile, onAtlasSelection, onViewportChange }: {
  manifest: SnapshotManifest;
  rows: InventoryRow[];
  routeSelection?: WorkspaceRoute['atlasSelection'];
  routeViewport?: AtlasViewport;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onAtlasSelection: (selection: WorkspaceRoute['atlasSelection'], treeSelection: string, mapViewport?: AtlasViewport) => void;
  onViewportChange: (viewport: AtlasViewport) => void;
}) {
  const snapshotId = manifest.summary.snapshotId;
  const projectId = manifest.summary.projectId;
  const storageKey = `atlas-view:${projectId}:${snapshotId}`;
  const [view, setView] = useState(() => {
    const restored = loadAtlasWorkbenchState(storageKey);
    return routeViewport ? { ...restored, viewport: normalizeAtlasViewport(routeViewport) } : restored;
  });
  const [index, setIndex] = useState<SnapshotIndex | null>(null);
  const [indexState, setIndexState] = useState<'loading' | 'pending' | 'ready' | 'error'>('loading');
  const [indexError, setIndexError] = useState<string | null>(null);
  const [indexJobId, setIndexJobId] = useState<string | null>(null);
  const [indexProgress, setIndexProgress] = useState<IndexProgress | null>(null);
  const [startingIndex, setStartingIndex] = useState(false);
  const treeRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLDivElement>(null);
  const canvasContentRef = useRef<HTMLDivElement>(null);
  const panGestureRef = useRef<{ pointerId: number; startX: number; startY: number; startViewport: AtlasViewport; pendingViewport: AtlasViewport } | null>(null);
  const [dragViewport, setDragViewport] = useState<AtlasViewport | null>(null);
  const routeKey = routeSelection ? JSON.stringify(routeSelection) : '';

  useEffect(() => {
    try { sessionStorage.setItem(storageKey, JSON.stringify(view)); } catch { /* Session view persistence is optional. */ }
  }, [storageKey, view]);
  useEffect(() => {
    if (!routeSelection) return;
    setView((current) => ({
      ...current,
      selectedPath: routeSelection.kind === 'file' || routeSelection.kind === 'symbol' ? routeSelection.path : null,
      selectedSymbol: routeSelection.kind === 'symbol' ? routeSelection.symbolId : null,
      selectedRelationship: routeSelection.kind === 'relationship' ? routeSelection.relationshipId : null,
      selectedFolder: null,
    }));
  }, [routeKey]);
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
      setIndexError(error instanceof Error ? error.message : 'The local snapshot index could not be loaded.');
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
        setIndexError(error instanceof Error ? error.message : 'Index progress could not be read.');
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
  const withinFolder = (path: string, folder: string | null) => !folder || folder === '.' || path === folder || path.startsWith(`${folder}/`);
  const matchingRows = useMemo(() => {
    const q = view.treeQuery.trim().toLowerCase();
    const matched = rows.filter((row) => withinFolder(row.path, view.selectedFolder)
      && (!q || `${row.path} ${row.reason} ${row.details} ${row.language}`.toLowerCase().includes(q)));
    const keep = new Set(matched.map((row) => row.path));
    for (const row of matched) {
      const parts = row.path.split('/').filter(Boolean);
      while (parts.length > 1) {
        parts.pop();
        const parent = parts.join('/');
        if (withinFolder(parent, view.selectedFolder)) keep.add(parent);
      }
      if (!view.selectedFolder && row.path !== '.') keep.add('.');
    }
    return rows.filter((row) => keep.has(row.path) && (matched.includes(row) || row.kind === 'directory'));
  }, [rows, view.selectedFolder, view.treeQuery]);
  const flattened = useMemo(() => flattenInventoryTree(buildInventoryTree(matchingRows), new Set(view.collapsed)), [matchingRows, view.collapsed]);
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
      if (treeRef.current) treeRef.current.scrollTop = view.treeTop;
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
    const nextView = { ...view, selectedPath: row.path, selectedFolder: folder, selectedSymbol: null, selectedRelationship: null };
    setView(nextView);
    try { sessionStorage.setItem(storageKey, JSON.stringify(nextView)); } catch { /* Session view persistence is optional. */ }
    onAtlasSelection(row.kind === 'directory' ? undefined : { kind: 'file', path: row.path }, row.path, view.viewport);
  };
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
      setIndexError(error instanceof Error ? error.message : 'Snapshot indexing could not start.');
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

  return <section className="atlas-workbench" aria-label="Atlas repository explorer">
    <div className="atlas-workbench-heading"><div><strong>Repository map</strong><span>Snapshot {snapshotId} · inventory paths and local index evidence</span></div><div className="atlas-map-scope">Scope: {view.selectedFolder ?? 'entire snapshot'} · Only resolved, hash-verified high-confidence edges are drawn.</div></div>
    <div className="atlas-columns">
      <section className="atlas-tree-panel" aria-labelledby="atlas-tree-title">
        <header><div><strong id="atlas-tree-title">Files</strong><small>{rows.length.toLocaleString()} inventory paths · {flattened.length.toLocaleString()} shown</small></div></header>
        <label className="atlas-filter-search"><Search size={13} /><input aria-label="Search snapshot tree" placeholder="Search paths or skip reasons" value={view.treeQuery} onChange={(event) => setView((s) => ({ ...s, treeQuery: event.target.value }))} /></label>
        {view.selectedFolder && <button className="atlas-scope-chip" onClick={() => { setView((s) => ({ ...s, selectedFolder: null })); onAtlasSelection(undefined, view.selectedPath ?? '', view.viewport); }}>Folder scope: {view.selectedFolder} <span>×</span></button>}
        <div className="atlas-tree-scroll" ref={treeRef} role="tree" aria-label="Snapshot inventory tree" onScroll={(event) => setView((s) => ({ ...s, treeTop: event.currentTarget.scrollTop }))}>
          {flattened.length === 0 ? <div className="atlas-empty-note"><strong>No paths match this search.</strong><span>Clear the search or widen the selected folder scope to see matching files.</span><button className="text-button" onClick={() => setView((current) => ({ ...current, treeQuery: '', selectedFolder: null }))}>Clear search and folder scope</button></div> : flattened.map((item, rowOrdinal) => {
            const row = item.row;
            const selected = row.path === view.selectedPath && !view.selectedRelationship;
            const openable = row.disposition === 'captured' && (row.contentKind === 'text' || row.contentKind === 'unknown');
            const rowIndexLabel = atlasRowIndexStatusLabel(atlasRowIndexStatus(row, index, indexState, stalePathSet.has(row.path)));
            return <div className={`atlas-tree-row${selected ? ' selected' : ''}${row.disposition !== 'captured' ? ' omitted' : ''}`} role="treeitem" aria-selected={selected} aria-expanded={item.hasChildren ? item.expanded : undefined} key={`${row.path}:${row.reason}:${rowOrdinal}`} style={{ paddingLeft: `${7 + item.depth * 13}px` }} title={row.details}>
              {item.hasChildren ? <button className="atlas-tree-toggle" aria-label={`${item.expanded ? 'Collapse' : 'Expand'} ${row.path}`} onClick={() => setView((s) => ({ ...s, collapsed: s.collapsed.includes(item.path) ? s.collapsed.filter((p) => p !== item.path) : [...s.collapsed, item.path] }))}><ChevronDown size={12} className={item.expanded ? '' : 'collapsed'} /></button> : <span className="atlas-tree-spacer" />}
              <button className="atlas-tree-entry" onClick={() => choosePath(row)} onKeyDown={(event) => {
                if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
                  const entries = Array.from(treeRef.current?.querySelectorAll<HTMLButtonElement>('.atlas-tree-entry') ?? []);
                  const currentIndex = entries.indexOf(event.currentTarget);
                  const nextIndex = event.key === 'Home' ? 0 : event.key === 'End' ? entries.length - 1 : Math.min(entries.length - 1, Math.max(0, currentIndex + (event.key === 'ArrowDown' ? 1 : -1)));
                  event.preventDefault();
                  entries[nextIndex]?.focus();
                } else if (event.key === 'Enter' && openable) {
                  event.preventDefault();
                  choosePath(row);
                  onOpenFile(row.path, snapshotId);
                }
              }}><span className={`atlas-state-mark ${row.disposition}`} />{row.kind === 'directory' ? <FolderOpen size={13} /> : <FileCode2 size={13} />}<code>{row.path === '.' ? 'repository root' : row.path.split('/').at(-1)}</code><small>{row.disposition}</small>{rowIndexLabel && <em>{rowIndexLabel}</em>}</button>
              {openable && <button className="atlas-tree-open" data-focus-key={`atlas-tree-open:${row.path}`} title="Open in Code Lens" aria-label={`Open ${row.path} in Code Lens`} onClick={() => { choosePath(row); onOpenFile(row.path, snapshotId); }}><ArrowRight size={12} /></button>}
            </div>;
          })}
        </div>
        <footer>Captured, skipped, excluded, and directory rows remain visible in the snapshot tree.</footer>
      </section>

      <section className="atlas-map-panel" aria-labelledby="atlas-map-title">
        <header><div><strong id="atlas-map-title">Relationships</strong><small>{shownEdges.length} shown · {provenEdges.length} proven for snapshot</small></div><span className="atlas-snapshot-badge">{index?.status ?? (indexState === 'loading' ? 'indexing' : indexState)}</span></header>
        <div className="atlas-map-filters"><label className="atlas-filter-search"><Search size={13} /><input aria-label="Search relationships" placeholder="Search symbol, path, or evidence" value={view.mapQuery} onChange={(event) => setView((s) => ({ ...s, mapQuery: event.target.value }))} /></label><label><span>Type</span><select aria-label="Filter relationships by type" value={view.kind} onChange={(event) => setView((s) => ({ ...s, kind: event.target.value as AtlasWorkbenchViewState['kind'] }))}><option value="all">All types</option><option value="calls">Calls</option><option value="imports">Imports</option><option value="testReference">Test references</option><option value="defines">Defines</option><option value="exports">Exports</option></select></label><label><span>Language</span><select aria-label="Filter relationships by language" value={view.language} onChange={(event) => setView((s) => ({ ...s, language: event.target.value }))}><option value="all">All languages</option>{languages.map((language) => <option key={language} value={language}>{language}</option>)}</select></label></div>
        {(view.selectedFolder || view.kind !== 'all' || view.language !== 'all') && <div className="atlas-active-filter-chips" aria-label="Active relationship filters">
          {view.selectedFolder && <button className="atlas-active-filter-chip" onClick={() => setView((s) => ({ ...s, selectedFolder: null }))}>Path: {view.selectedFolder} <span aria-hidden="true">×</span></button>}
          {view.kind !== 'all' && <button className="atlas-active-filter-chip" onClick={() => setView((s) => ({ ...s, kind: 'all' }))}>Type: {view.kind} <span aria-hidden="true">×</span></button>}
          {view.language !== 'all' && <button className="atlas-active-filter-chip" onClick={() => setView((s) => ({ ...s, language: 'all' }))}>Language: {view.language} <span aria-hidden="true">×</span></button>}
          <button className="atlas-clear-filters" onClick={() => setView((s) => ({ ...s, selectedFolder: null, kind: 'all', language: 'all' }))}>Clear filters</button>
        </div>}
        {indexState === 'loading' && <div className="atlas-index-banner" role="status">{indexProgress ? `Indexing ${indexProgress.filesProcessed}/${indexProgress.filesTotal} files · ${indexProgress.relationshipsFound} relationships` : 'Loading the local snapshot index…'}</div>}
        {indexState === 'pending' && <div className="atlas-index-banner" role="status"><strong>Index pending</strong><span>{indexError ?? 'No saved index is available for this snapshot yet.'}</span>{!indexError && <button onClick={() => { void startIndex(); }} disabled={startingIndex}>{startingIndex ? 'Starting…' : 'Index this snapshot'}</button>}</div>}
        {indexState === 'error' && <div className="atlas-index-banner stale" role="alert"><strong>Index unavailable</strong><span>{indexError}</span><button onClick={() => { setIndexError(null); setIndexState('pending'); }}>Try indexing</button></div>}
        {indexState === 'ready' && stalePaths.length > 0 && <div className="atlas-index-banner stale" role="status"><strong>Stale snapshot evidence</strong><span>{stalePaths.length} indexed file hash{stalePaths.length === 1 ? '' : 'es'} differ from the selected snapshot; affected edges are withheld.</span>{isNativeRuntime() && <button onClick={() => { void startIndex(); }} disabled={startingIndex || Boolean(indexJobId)}>{startingIndex || indexJobId ? 'Re-indexing…' : 'Re-index snapshot'}</button>}</div>}
        {indexState === 'ready' && unsupportedCount > 0 && <div className="atlas-index-banner unsupported" role="status"><strong>{unsupportedCount} unsupported path{unsupportedCount === 1 ? '' : 's'}</strong><span>They remain in the tree but cannot contribute indexed symbols.</span></div>}
        <div className="atlas-canvas-toolbar" aria-label="Relationship map viewport controls">
          <span>Zoom {Math.round((dragViewport ?? view.viewport).scale * 100)}% · drag empty canvas to pan</span>
          <button aria-label="Zoom out relationship map" title="Zoom out (−)" onClick={() => zoomAtCenter(-ATLAS_ZOOM_STEP)} disabled={(dragViewport ?? view.viewport).scale <= ATLAS_MIN_ZOOM}>−</button>
          <button aria-label="Zoom in relationship map" title="Zoom in (+)" onClick={() => zoomAtCenter(ATLAS_ZOOM_STEP)} disabled={(dragViewport ?? view.viewport).scale >= ATLAS_MAX_ZOOM}>+</button>
          <button onClick={fitMap}>Fit</button>
          <button onClick={resetMap}>Reset</button>
        </div>
        <div className={`atlas-relationship-canvas${dragViewport ? ' is-panning' : ''}`} ref={canvasRef} tabIndex={0} aria-label="Relationship map viewport. Use arrow keys to pan, plus and minus to zoom, F to fit, or 0 to reset." aria-keyshortcuts="ArrowUp ArrowDown ArrowLeft ArrowRight + - F 0" onKeyDown={handleCanvasKeyDown} onPointerDown={handleCanvasPointerDown} onPointerMove={handleCanvasPointerMove} onPointerUp={finishCanvasPointer} onPointerCancel={finishCanvasPointer}>
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
          : selectedSymbol ? <div className="atlas-detail-content"><span className="atlas-detail-kicker">INDEXED SYMBOL</span><h3>{selectedSymbol.qualifiedName}</h3><code>{selectedSymbol.path}</code><p>{selectedSymbol.kind} · {selectedSymbol.language} · lines {selectedSymbol.anchor.startLine}–{selectedSymbol.anchor.endLine}</p><dl><div><dt>SHA-256</dt><dd><code>{selectedSymbol.anchor.sourceSha256}</code></dd></div></dl><button className="atlas-open-evidence" data-focus-key={`atlas-open-symbol:${selectedSymbol.symbolId}`} onClick={() => onOpenFile(selectedSymbol.path, snapshotId, codeLensRangeFromIndexedAnchor(selectedSymbol.anchor))}><FileCode2 size={14} /> Open symbol in Code Lens</button></div>
            : rawRelationship ? <div className="atlas-detail-content"><span className="atlas-detail-kicker warning">NOT PROVEN</span><h3>{rawRelationship.kind} · {rawRelationship.confidence}</h3><p>This relationship is unresolved or its exact snapshot evidence is unavailable, so it is withheld from the canvas.</p><code>{rawRelationship.evidence.path}</code><small>{rawRelationship.unresolvedReason ?? 'Relationship did not pass endpoint and hash verification.'}</small><button className="atlas-open-evidence" data-focus-key={`atlas-open-relationship:${rawRelationship.relationshipId}`} onClick={() => onOpenFile(rawRelationship.evidence.path, snapshotId, codeLensRangeFromIndexedAnchor(rawRelationship.evidence))}><FileCode2 size={14} /> Inspect indexed anchor</button></div>
                : selectedRow ? <div className="atlas-detail-content"><span className="atlas-detail-kicker">{selectedRow.kind} · {selectedRow.disposition}</span><h3>{selectedRow.path === '.' ? 'Repository root' : selectedRow.path.split('/').at(-1)}</h3><code>{selectedRow.path}</code><dl><div><dt>Language</dt><dd>{selectedRow.language}</dd></div><div><dt>Capture</dt><dd>{inventoryReasonLabel(selectedRow.reason)}</dd></div><div><dt>Parser / index</dt><dd>{selectedRow.parserReadiness} · {atlasRowIndexStatusLabel(atlasRowIndexStatus(selectedRow, index, indexState, stalePathSet.has(selectedRow.path))) ?? 'not applicable'}</dd></div>{selectedIndexCounts && <><div><dt>Indexed files</dt><dd>{selectedIndexCounts.indexedFiles}</dd></div><div><dt>Symbols</dt><dd>{selectedIndexCounts.symbols}</dd></div><div><dt>Relationships</dt><dd>{selectedIndexCounts.relationships}</dd></div></>}{selectedRow.sizeBytes !== null && <div><dt>Size</dt><dd>{formatBytes(selectedRow.sizeBytes)}</dd></div>}{selectedRow.sha256 && <div><dt>SHA-256</dt><dd><code>{selectedRow.sha256}</code></dd></div>}</dl>{selectedRow.parserReadiness === 'unsupported' && <p className="atlas-detail-note">Unsupported by the local indexer; this row remains available in inventory.</p>}{selectedRow.disposition !== 'captured' && <p className="atlas-detail-note warning">No source bytes were captured. {selectedRow.details}</p>}{selectedRow.kind === 'directory' && <p className="atlas-detail-note">Folder selection scopes the relationship canvas to descendants.</p>}{selectedRow.disposition === 'captured' && (selectedRow.contentKind === 'text' || selectedRow.contentKind === 'unknown') && <button className="atlas-open-evidence" data-focus-key={`atlas-open-file:${selectedRow.path}`} onClick={() => onOpenFile(selectedRow.path, snapshotId)}><FileCode2 size={14} /> Open in Code Lens</button>}</div>
                : <div className="atlas-detail-empty"><Network size={18} /><strong>Select a folder, file, symbol, or edge.</strong><span>Details retain exact snapshot path and source hash evidence.</span></div>}
        <footer>Dynamic and unresolved relationships are never presented as proven.</footer>
      </aside>
    </div>
    <p className="atlas-workbench-note">Tree and map search state is remembered for this snapshot in the current session.</p>
  </section>;
}

function inventoryFilterKey(row: Pick<InventoryRow, 'path' | 'kind' | 'reason'>): string {
  return `${row.path}\u0000${row.kind}\u0000${row.reason}`;
}

function GuidePlanScreen({ projectId, manifest, rows, onGoAtlas, onGoStudio, onOpenFile }: {
  projectId: string;
  manifest: SnapshotManifest;
  rows: InventoryRow[];
  onGoAtlas: () => void;
  onGoStudio: () => void;
  onOpenFile: (path: string) => void;
}) {
  const snapshotId = manifest.summary.snapshotId;
  const [plan, setPlan] = useState<CoveragePlan>(() => loadCoveragePlan(projectId, snapshotId));
  const [query, setQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | CoverageStatus>('all');
  const [languageFilter, setLanguageFilter] = useState('all');
  const [captureFilter, setCaptureFilter] = useState<'all' | CaptureDisposition>('all');
  const [parserFilter, setParserFilter] = useState<'all' | ParserReadiness>('all');
  const [contentFilter, setContentFilter] = useState<'all' | InventoryRow['contentKind']>('all');
  const [categoryFilter, setCategoryFilter] = useState<'all' | InventoryCategory>('all');
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(50);
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [groupFormOpen, setGroupFormOpen] = useState(false);
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
  const filtered = useMemo(() => filterCoverageRows(coverageRows, query, statusFilter)
    .filter((row) => inventoryMatches.has(inventoryFilterKey(row.inventory))), [coverageRows, inventoryMatches, query, statusFilter]);
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
      .catch((error) => { if (active) setRevisionsError(typeof error === 'string' ? error : 'Saved chapter revisions could not be loaded. Coverage counts are incomplete until you retry.'); })
      .finally(() => { if (active) setRevisionsLoading(false); });
    return () => { active = false; };
  }, [projectId, snapshotId, revisionRefresh]);

  const setQueryFilter = (value: string) => { setQuery(value); setPage(0); };
  const setStatus = (value: 'all' | CoverageStatus) => { setStatusFilter(value); setPage(0); };
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
      setFeedback(error instanceof Error ? error.message : 'The group could not be created.');
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
      setFeedback(error instanceof Error ? error.message : 'The file could not be excluded.');
    }
  };
  const dissolveGroup = (groupId: string, name: string) => {
    setPlan((current) => removeCoverageGroup(current, groupId));
    setFeedback(`“${name}” was removed. Its files are independent chapters again.`);
  };
  const statusLabel: Record<CoverageStatus, string> = {
    missing: 'Needs chapter', draft: 'Draft', rejected: 'Rejected', stale: 'Older snapshot', accepted: 'Accepted',
    excluded: 'Excluded by you', unsupported: 'Parser unsupported', unavailable: 'Source unavailable',
  };

  return (
    <div className="empty-workspace guide-plan-screen">
      <section className="guide-plan-card" aria-labelledby="guide-plan-title">
        <div className="guide-plan-topline">
          <div>
            <div className="step-pill"><span>SNAPSHOT COVERAGE</span> FILE BY FILE</div>
            <h2 id="guide-plan-title">Every recorded path stays accounted for.</h2>
            <p>Captured text files start as independent chapters. Group files only when you choose them and explain why they belong together.</p>
          </div>
          <div className="chapter-studio-top-actions"><button className="secondary-button" onClick={onGoStudio}><BookOpen size={14} /> Chapter Studio</button><button className="secondary-button" onClick={onGoAtlas}><ArrowLeft size={14} /> Atlas inventory</button></div>
        </div>

        <div className="guide-plan-identity">
          <span>{manifest.summary.status === 'partial' ? 'Partial snapshot' : 'Immutable snapshot'}</span>
          <code>{snapshotId}</code>
          <span>{projectId}</span>
        </div>

        {revisionsLoading && <p className="coverage-feedback" role="status">Loading saved chapter revisions across snapshots…</p>}
        {revisionsError && <p className="coverage-storage-warning" role="alert">{revisionsError}<button className="text-button" onClick={() => setRevisionRefresh((value) => value + 1)}>Retry</button></p>}

        {coverageReady && <div className="coverage-total-strip" aria-label={`${totals.total} inventory paths accounted for`}>
          <div className="coverage-total-main"><strong>{totals.total.toLocaleString()}</strong><span>inventory paths</span><small>Each row below has exactly one status.</small></div>
          {COVERAGE_STATUSES.map((status) => (
            <button
              className={`coverage-total coverage-total-${status}${statusFilter === status ? ' is-active' : ''}`}
              key={status}
              onClick={() => setStatus(statusFilter === status ? 'all' : status)}
              aria-pressed={statusFilter === status}
            >
              <strong>{totals[status].toLocaleString()}</strong><span>{statusLabel[status]}</span>
            </button>
          ))}
        </div>}

        <div className="guide-plan-actions">
          <div><strong>Make a group from selected files</strong><span>Grouping is always explicit. There is no generate-all action.</span></div>
          <div className="guide-plan-action-buttons">
            <span aria-live="polite">{selectedPaths.length} selected</span>
            <button className="secondary-button" onClick={startGroup} disabled={selectedPaths.length < 2 || revisionsLoading || Boolean(revisionsError)}>Create group</button>
            {selectedPaths.length > 0 && <button className="text-button" onClick={() => setSelectedPaths([])}>Clear selection</button>}
          </div>
        </div>

        {groupFormOpen && (
          <form className="coverage-group-form" onSubmit={submitGroup}>
            <div><strong>Name this group</strong><p>All selected paths remain visible as separate rows and source citations.</p></div>
            <label><span>Group name</span><input autoFocus value={groupName} onChange={(event) => setGroupName(event.target.value)} placeholder="For example, request handling" required maxLength={100} /></label>
            <label><span>Why do these files belong together?</span><textarea value={groupRationale} onChange={(event) => setGroupRationale(event.target.value)} placeholder="Describe their shared responsibility. This rationale is shown beside every member." required maxLength={500} rows={3} /></label>
            <div className="coverage-form-footer"><span>{selectedPaths.length} paths · {selectedPaths.slice(0, 3).join(', ')}{selectedPaths.length > 3 ? ` and ${selectedPaths.length - 3} more` : ''}</span><div><button type="button" className="text-button" onClick={() => setGroupFormOpen(false)}>Cancel</button><button className="primary-button" type="submit">Create named group</button></div></div>
          </form>
        )}

        {plan.groups.length > 0 && (
          <section className="coverage-groups" aria-labelledby="coverage-groups-heading">
            <div className="coverage-subheading"><div><strong id="coverage-groups-heading">Groups you created</strong><span>Membership and rationale are attached to each listed path.</span></div><span>{plan.groups.length} group{plan.groups.length === 1 ? '' : 's'}</span></div>
            {plan.groups.map((group) => (
              <article className="coverage-group-card" key={group.groupId}>
                <div><strong>{group.name}</strong><p>{group.rationale}</p><div className="coverage-group-members">{group.paths.map((path) => <button key={path} onClick={() => onOpenFile(path)}><FileCode2 size={12} /><code>{path}</code></button>)}</div></div>
                <button className="text-button" onClick={() => dissolveGroup(group.groupId, group.name)}>Remove group</button>
              </article>
            ))}
          </section>
        )}

        <div className="coverage-filter-bar">
          <label className="inventory-search"><Search size={14} /><input aria-label="Search guide coverage paths, groups, and reasons" placeholder="Search every path, group, or reason" value={query} onChange={(event) => setQueryFilter(event.target.value)} /></label>
          <label><span>Status</span><select aria-label="Filter guide coverage by status" value={statusFilter} onChange={(event) => setStatus(event.target.value as 'all' | CoverageStatus)}><option value="all">All statuses ({totals.total})</option>{COVERAGE_STATUSES.map((status) => <option key={status} value={status}>{statusLabel[status]} ({totals[status]})</option>)}</select></label>
          <label><span>Language</span><select aria-label="Filter guide coverage by language" value={languageFilter} onChange={(event) => { setLanguageFilter(event.target.value); setPage(0); }}><option value="all">All languages</option>{languages.map((language) => <option key={language} value={language}>{language}</option>)}</select></label>
          <label><span>Capture outcome</span><select aria-label="Filter inventory by capture outcome" value={captureFilter} onChange={(event) => { setCaptureFilter(event.target.value as 'all' | CaptureDisposition); setPage(0); }}><option value="all">All capture outcomes</option><option value="captured">Captured</option><option value="skipped">Skipped</option><option value="excluded">Excluded</option></select></label>
          <label><span>Parser readiness</span><select aria-label="Filter inventory by parser readiness" value={parserFilter} onChange={(event) => { setParserFilter(event.target.value as 'all' | ParserReadiness); setPage(0); }}><option value="all">All parser states</option><option value="notReady">Supported parser</option><option value="unsupported">Unsupported parser</option><option value="notApplicable">Not applicable</option></select></label>
          <label><span>File category</span><select aria-label="Filter inventory by file category" value={categoryFilter} onChange={(event) => { setCategoryFilter(event.target.value as 'all' | InventoryCategory); setPage(0); }}><option value="all">All categories</option><option value="config">Configuration</option><option value="test">Test</option><option value="generated">Generated</option><option value="secret">Secret-like</option></select></label>
          <label><span>Content</span><select aria-label="Filter inventory by content type" value={contentFilter} onChange={(event) => { setContentFilter(event.target.value as 'all' | InventoryRow['contentKind']); setPage(0); }}><option value="all">All content</option><option value="text">Text</option><option value="binary">Binary</option><option value="invalidEncoding">Invalid encoding</option><option value="unknown">Unknown</option><option value="notApplicable">Not applicable</option></select></label>
          <button className="inventory-clear" onClick={() => { setQueryFilter(''); setStatus('all'); setLanguageFilter('all'); setCaptureFilter('all'); setParserFilter('all'); setContentFilter('all'); setCategoryFilter('all'); setPage(0); }}>Reset filters</button>
          <span className="coverage-match-count" aria-live="polite">{pageData.total.toLocaleString()} matches · {totals.total.toLocaleString()} total paths</span>
        </div>

        {coverageReady && <CoverageTable
          rows={pageData.rows}
          statusLabel={statusLabel}
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

function ChapterStudioScreen({ projectId, manifest, rows, routeChapterId, routeRevisionId, routeSectionId, onSelectRevision, onClearRevisionRoute, onOpenFile, onGoPlan, onGenerate, onGenerateOverview }: {
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
      if (!revision) return;
      setSelectedTargetKey(revision.target.scope.kind === 'overview' ? 'overview' : revision.target.targetId);
      setSelectedRevisionId(revision.revisionId);
      setEditing(false);
      setCandidate(null);
      setValidation(null);
      onSelectRevision(revision);
    };
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => window.removeEventListener('code-atlas:chapters-changed', refresh);
  }, [onSelectRevision]);

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
      .catch((error) => { if (active) setLoadError(typeof error === 'string' ? error : 'Saved chapter history could not be loaded.'); })
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
  const activeDocument = editing && candidate ? candidate : selectedRevision;
  const status = !selectedTarget || selectedTarget.missing
    ? 'missing'
    : selectedRevision
      ? selectedRevision.snapshotId === snapshotId ? selectedRevision.lifecycle : 'stale'
      : selectedTarget.latestCurrent?.lifecycle ?? 'stale';
  const statusLabel = status === 'missing' ? 'Needs chapter'
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
      section?.scrollIntoView({ block: 'start', behavior: 'smooth' });
    });
    return () => cancelAnimationFrame(frame);
  }, [activeDocument?.revisionId, routeSectionId]);

  const reload = async (selectId?: string) => {
    const saved = await listProjectChapterRevisions(projectId);
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
    if (!activeDocument) return;
    setBusy(true);
    setFeedback(null);
    try {
      const report = await validateChapterRevision(projectId, activeDocument);
      setValidation(report);
      setFeedback({
        message: report.status === 'valid'
          ? 'Every cited range matches the captured source.'
          : `${report.issues.length} issue${report.issues.length === 1 ? '' : 's'} need review before acceptance.`,
        tone: report.status === 'valid' ? 'success' : 'error',
      });
    } catch (error) {
      setFeedback({ message: typeof error === 'string' ? error : 'The chapter could not be validated.', tone: 'error' });
    } finally {
      setBusy(false);
    }
  };
  const assessFreshness = async () => {
    if (!selectedRevision || !freshnessKey || editing) return;
    const key = freshnessKey;
    setFreshnessState({ key, report: null, error: null, loading: true });
    try {
      const report = await assessChapterFreshness(projectId, selectedRevision.revisionId, snapshotId);
      setFreshnessState({ key, report, error: null, loading: false });
    } catch (error) {
      setFreshnessState({
        key,
        report: null,
        error: typeof error === 'string' ? error : error instanceof Error ? error.message : 'Freshness could not be assessed. Check indexing and try again.',
        loading: false,
      });
    }
  };
  const saveEdits = async () => {
    if (!candidate || !selectedRevision || !isLatestCurrent) return;
    setBusy(true);
    setFeedback(null);
    try {
      const saved = await saveChapterEdits(projectId, selectedRevision.revisionId, candidate);
      setValidation(saved.validation);
      await reload(saved.revision.revisionId);
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
      setFeedback({ message: typeof error === 'string' ? error : 'The draft could not be saved.', tone: 'error' });
    } finally {
      setBusy(false);
    }
  };
  const accept = async () => {
    if (!selectedRevision || !isLatestCurrent || selectedRevision.lifecycle !== 'draft') return;
    setBusy(true);
    setFeedback(null);
    try {
      const accepted = await acceptChapterDraft(projectId, selectedRevision.revisionId);
      setValidation(accepted.validation);
      await reload(accepted.revision.revisionId);
      onSelectRevision(accepted.revision);
      setFeedback({ message: 'Accepted. The draft and accepted revision remain in history.', tone: 'success' });
    } catch (error) {
      setFeedback({ message: typeof error === 'string' ? error : 'The chapter could not be accepted.', tone: 'error' });
    } finally {
      setBusy(false);
    }
  };
  const reject = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedRevision || !isLatestCurrent || selectedRevision.lifecycle !== 'draft') return;
    setBusy(true);
    setFeedback(null);
    try {
      const rejected = await rejectChapterDraft(projectId, selectedRevision.revisionId, rejectReason);
      await reload(rejected.revisionId);
      onSelectRevision(rejected);
      setRejecting(false);
      setRejectReason('');
      setFeedback({ message: 'Rejected with reason recorded. The draft payload is still available in history.', tone: 'success' });
    } catch (error) {
      setFeedback({ message: typeof error === 'string' ? error : 'The draft could not be rejected.', tone: 'error' });
    } finally {
      setBusy(false);
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
            <div className="step-pill"><span>CHAPTER STUDIO</span> SOURCE FIRST · REVISIONED</div>
            <h2 id="chapter-studio-title" data-focus-key="heading:chapter-studio">Inspect what each file actually does.</h2>
            <p>Each file chapter explains its source directly. An optional repository overview is a separate chapter and never counts files as covered; edits and decisions are saved as local revisions.</p>
          </div>
          <div className="chapter-studio-top-actions">
            <button className="primary-button" onClick={onGenerateOverview} disabled={editing} title={editing ? 'Save or cancel the current edit before creating an overview.' : 'Create a separate overview; this does not mark any file chapter complete.'}><Sparkles size={14} /> Optional overview</button>
            <button className="secondary-button" onClick={onGoPlan}><ListChecks size={14} /> Guide Plan</button>
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
                  disabled={editing}
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
                      : selectedTarget.paths.length === 1 && !selectedTarget.latestCurrent && <button className="primary-button" onClick={() => onGenerate(selectedTarget.paths[0])}><Sparkles size={14} /> Generate current snapshot</button>}
                  </div> : null}
                </div>

                {history.length > 0 && <div className="chapter-revision-history" aria-label="Immutable chapter revision history">
                  <span>Revision history</span>
                  {history.map((revision) => <button
                    key={revision.revisionId}
                    className={`chapter-history-item lifecycle-${revision.lifecycle}${selectedRevision?.revisionId === revision.revisionId ? ' is-current' : ''}`}
                    disabled={editing}
                    onClick={() => { setSelectedRevisionId(revision.revisionId); setEditing(false); setCandidate(null); setValidation(null); setRejecting(false); setFeedback(null); onSelectRevision(revision); }}
                    aria-pressed={selectedRevision?.revisionId === revision.revisionId}
                    title={`${revision.lifecycle} · snapshot ${revision.snapshotId}`}
                  ><strong>r{revision.revisionNumber}</strong><span>{revision.lifecycle}</span></button>)}
                </div>}

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
                            {isLatestCurrent && <button className="secondary-button" onClick={beginEditing} disabled={busy}><Pencil size={14} /> {selectedRevision.lifecycle === 'draft' ? 'Edit as new draft' : selectedRevision.lifecycle === 'rejected' ? 'Repair in new draft' : 'Start a new draft'}</button>}
                            {isLatestCurrent && selectedRevision.lifecycle === 'draft' && <>
                              <button className="primary-button" onClick={() => { void accept(); }} disabled={busy || selectedRevision.validationStatus !== 'valid'}><Check size={14} /> Accept</button>
                              <button className="secondary-button button-reject" onClick={() => setRejecting((value) => !value)} disabled={busy}><X size={14} /> Reject</button>
                            </>}
                          </div>
                        </>}
                      </div>

                      {rejecting && selectedRevision.lifecycle === 'draft' && <form className="chapter-rejection-form" onSubmit={(event) => { void reject(event); }}>
                        <label><span>Why is this draft rejected?</span><textarea autoFocus rows={2} maxLength={4000} value={rejectReason} onChange={(event) => setRejectReason(event.target.value)} placeholder="Record what needs to change. The draft remains in history." required /></label>
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
                          return <article className="chapter-studio-section chapter-freshness-section" key={sectionResult.sectionId}>
                            <div className="chapter-section-title-row">
                              <h4>{sectionTitle}</h4>
                              <span className={`chapter-review-status status-${sectionResult.status}`}>{labels[sectionResult.status]}</span>
                              <span className="chapter-review-status">Evidence {sectionResult.evidenceState}</span>
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
                                </div>;
                              })}
                            </div>}
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

                      <div className="chapter-edit-title-row"><span>Chapter title</span>{editing && candidate ? <input value={candidate.title} maxLength={200} onChange={(event) => { setCandidate({ ...candidate, title: event.target.value }); setValidation(null); }} aria-label="Chapter title" /> : null}</div>
                      {activeDocument.sections.map((section, index) => <article className={`chapter-studio-section${routeSectionId === section.sectionId ? ' is-route-focused' : ''}`} key={section.sectionId} id={`studio-section-${section.sectionId}`} tabIndex={-1} data-focus-key={`chapter-section:${section.sectionId}`}>
                        <div className="chapter-section-title-row"><span className="chapter-section-number">{String(index + 1).padStart(2, '0')}</span>{editing && candidate ? <input value={section.title} maxLength={160} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, title: event.target.value }))} aria-label={`Section ${index + 1} title`} /> : <h4>{section.title}</h4>}
                          {editing && <div className="chapter-section-reorder"><button className="icon-button" onClick={() => moveSection(section.sectionId, -1)} disabled={index === 0} aria-label={`Move ${section.title} up`}><MoveUp size={14} /></button><button className="icon-button" onClick={() => moveSection(section.sectionId, 1)} disabled={index === activeDocument.sections.length - 1} aria-label={`Move ${section.title} down`}><MoveDown size={14} /></button></div>}
                        </div>
                        <div className="chapter-section-copy"><span>EXPLANATION</span>{editing && candidate ? <textarea rows={3} value={section.prose} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, prose: event.target.value }))} aria-label={`Explanation for ${section.title}`} /> : <p>{section.prose}</p>}</div>
                        <div className="chapter-section-copy chapter-spoken-copy"><span>VOICE SCRIPT</span>{editing && candidate ? <textarea rows={2} value={section.spokenText} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, spokenText: event.target.value }))} aria-label={`Spoken text for ${section.title}`} placeholder="Optional narration script" /> : <p>{section.spokenText || <em>No narration script saved.</em>}</p>}</div>
                        {section.claims.map((claim: ChapterClaim) => <div className="chapter-claim-card" key={claim.claimId}>
                          <div className="chapter-claim-heading"><span className={`claim-label claim-${claim.category.kind}`}>{claim.category.kind === 'codeBehavior' ? 'Code behavior' : claim.category.kind === 'repositoryContext' ? 'Repository context' : claim.category.visibleLabel || 'Inference'}</span>{editing && <button className="text-button chapter-remove-claim" onClick={() => removeClaim(section.sectionId, claim.claimId)}><Trash2 size={12} /> Remove claim</button>}</div>
                          {editing && candidate ? <textarea rows={2} value={claim.prose} onChange={(event) => setSection(section.sectionId, (item) => ({ ...item, claims: item.claims.map((saved) => saved.claimId === claim.claimId ? { ...saved, prose: event.target.value } : saved) }))} aria-label={`Claim ${claim.claimId}`} /> : <p>{claim.prose}</p>}
                          {claim.category.kind === 'inference' && <p className="chapter-inference-basis"><strong>{claim.category.visibleLabel} basis:</strong> {claim.category.basis}</p>}
                          <div className="chapter-citations"><span>EXACT SOURCE</span>{claim.anchors.length === 0 ? <small>No source range is attached to this claim.</small> : claim.anchors.map((anchor) => <div className="chapter-citation" key={anchor.anchorId}>
                            <div><code>{anchor.path}:{anchor.startLine}-{anchor.endLine}</code><button className="text-button" onClick={() => { onSelectRevision(selectedRevision, section.sectionId); onOpenFile(anchor.path, anchor.snapshotId, codeLensRangeFromChapterAnchor(anchor)); }}><FileCode2 size={12} /> Open in Code Lens</button></div>
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

function CoverageTable({ rows, statusLabel, selectedPaths, allVisibleSelected, visibleEligible, excludePath, excludeReason, setExcludePath, setExcludeReason, onToggleSelected, onToggleVisible, onOpenFile, onExclude, onRestore, onCancelExclude }: {
  rows: CoverageRow[];
  statusLabel: Record<CoverageStatus, string>;
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
  onExclude: (event: FormEvent<HTMLFormElement>, row: CoverageRow) => void;
  onRestore: (path: string) => void;
  onCancelExclude: () => void;
}) {
  return (
    <div className="coverage-table-wrap">
      <div className="coverage-table" role="table" aria-label="Chapter coverage for every snapshot path">
        <div className="coverage-table-header" role="row">
          <label className="coverage-select-all"><input type="checkbox" checked={allVisibleSelected} disabled={visibleEligible === 0} onChange={onToggleVisible} aria-label="Select all eligible files on this page" /><span>Group</span></label>
          <span role="columnheader">Snapshot path</span><span role="columnheader">Chapter coverage</span><span role="columnheader">Source / action</span>
        </div>
        {rows.length === 0 ? <div className="coverage-empty">No paths match. Clear the search or status filter to see the rest of the inventory.</div> : rows.map((row) => (
          <div className="coverage-row" role="row" key={`${row.inventory.path}:${row.inventory.kind}:${row.inventory.reason}`}>
            <div className="coverage-select-cell" role="cell">{row.canSelectForGroup ? <input type="checkbox" checked={selectedPaths.includes(row.inventory.path)} onChange={() => onToggleSelected(row.inventory.path)} aria-label={`Select ${row.inventory.path} for an explicit group`} /> : <span aria-hidden="true">—</span>}</div>
            <div className="coverage-path-cell" role="cell">
              <code title={row.inventory.path}>{row.inventory.path}</code>
              <span>{row.inventory.language} · {row.inventory.kind} · {row.inventory.disposition} · {row.inventory.contentKind} · {row.inventory.parserReadiness === 'notReady' ? 'supported parser' : row.inventory.parserReadiness === 'unsupported' ? 'unsupported parser' : 'parser not applicable'}</span>
              {row.inventory.categories.length > 0 && <small>Category: {row.inventory.categories.join(' · ')}</small>}
              {row.group && <div className="coverage-row-group"><strong>Group: {row.group.name}</strong><span>{row.group.rationale}</span></div>}
              {(row.status === 'missing' || (row.status === 'unsupported' && row.canSelectForGroup)) && <small>Independent file chapter · generate from its captured source</small>}
              {row.status === 'draft' && <small>Draft revision {row.revisionNumber} · <code>{row.revisionId}</code></small>}
              {row.status === 'rejected' && <small>Rejected revision {row.revisionNumber} · <code>{row.revisionId}</code></small>}
              {row.status === 'stale' && <small>Only an older snapshot chapter exists · <code>{row.revisionId}</code></small>}
              {row.status === 'accepted' && <small>Accepted revision {row.revisionNumber} · <code>{row.revisionId}</code></small>}
              {row.status === 'excluded' && row.userExclusion && <div className="coverage-row-reason"><span>Your reason: {row.userExclusion.reason}</span><button className="text-button" onClick={() => onRestore(row.inventory.path)}>Include again</button></div>}
              {row.reason && row.status !== 'excluded' && <small className={`coverage-reason coverage-reason-${row.status}`}>{row.reason}</small>}
            </div>
            <div className="coverage-status-cell" role="cell"><span className={`coverage-status coverage-status-${row.status}`}>{statusLabel[row.status]}</span>{row.status === 'unsupported' && <small>{row.inventory.language} remains locally readable.</small>}{row.status === 'unavailable' && <small>Inventory is retained; no chapter is counted.</small>}</div>
            <div className="coverage-action-cell" role="cell">
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

function CodeLensEmptyState({ projectId, snapshotId, manifest, onGoAtlas, onGoWalkthrough, onOpenFile, onRequestExplanation, source, sourceRange, sourceRangeIssue, loading, error, selectedPath }: {
  projectId: string | null;
  snapshotId: string | null;
  manifest: SnapshotManifest | null;
  onGoAtlas: () => void;
  onGoWalkthrough?: () => void;
  onOpenFile: (path: string, snapshotId?: string, sourceRange?: SourceLineRange) => void;
  onRequestExplanation: (path: string) => void;
  source: SnapshotSource | null;
  sourceRange: SourceLineRange | null;
  sourceRangeIssue: WorkspaceRoute['sourceRangeIssue'] | null;
  loading: boolean;
  error: string | null;
  selectedPath: string | null;
}) {
  const [showCompleteSource, setShowCompleteSource] = useState(false);
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
        if (active) setRelationshipError(loadError instanceof Error ? loadError.message : 'The saved snapshot index could not be opened.');
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
  const rangeStart = visibleSourceRange?.startLine ?? null;
  const rangeEnd = visibleSourceRange?.endLine ?? null;
  const contextBefore = 24;
  const contextAfter = 24;
  const firstDisplayedLine = rangeStart === null || showCompleteSource ? 1 : Math.max(1, rangeStart - contextBefore);
  const lastDisplayedLine = rangeEnd === null || showCompleteSource ? sourceLines.length : Math.min(sourceLines.length, rangeEnd + contextAfter);
  const displayedLines = sourceLines.slice(firstDisplayedLine - 1, lastDisplayedLine);

  useEffect(() => {
    setShowCompleteSource(false);
  }, [source?.snapshotId, source?.path, sourceRange?.startLine, sourceRange?.endLine, sourceRange?.sourceSha256]);

  useEffect(() => {
    if (!visibleSourceRange || !source) return;
    const frame = requestAnimationFrame(() => citedLineRef.current?.scrollIntoView({ block: 'center', behavior: 'smooth' }));
    return () => cancelAnimationFrame(frame);
  }, [source?.snapshotId, source?.path, visibleSourceRange?.startLine, visibleSourceRange?.endLine, showCompleteSource]);

  if (selectedPath && loading) {
    return <div className="empty-workspace snapshot-loading" role="status"><HardDrive size={18} /><span>Verifying and opening {selectedPath} from the selected snapshot…</span></div>;
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
      <div className="empty-workspace simple-empty source-error" role="alert">
        <div className="empty-state-icon code-icon"><TriangleAlert size={22} /></div>
        <div className="step-pill"><span>SNAPSHOT</span> SOURCE UNAVAILABLE</div>
        <h2>{unavailableTitle}</h2>
        <p>{unavailableMessage} Code Atlas did not fall back to the live repository.</p>
        {onGoWalkthrough && <button className="secondary-button" onClick={onGoWalkthrough}><ArrowLeft size={15} /> Return to walkthrough</button>}
        <button className="secondary-button" onClick={onGoAtlas}>Back to inventory <ArrowRight size={15} /></button>
      </div>
    );
  }
  if (selectedPath && source) {
    return (
      <div className="empty-workspace source-view">
        <div className="source-view-toolbar">
          <div><span className="step-pill"><span>READ ONLY</span> SNAPSHOT SOURCE</span><h2>{source.path}</h2></div>
          <div className="source-view-actions">
            {onGoWalkthrough && <button className="secondary-button" onClick={onGoWalkthrough}><ArrowLeft size={15} /> Return to walkthrough</button>}
            <button className="secondary-button" onClick={() => onRequestExplanation(source.path)}><Sparkles size={15} /> Review Gemini request</button>
            <button className="secondary-button" onClick={onGoAtlas}>Back to inventory <ArrowRight size={15} /></button>
          </div>
        </div>
        <div className="source-provenance"><span>Snapshot <code>{source.snapshotId}</code></span><span>SHA-256 <code>{source.sha256}</code></span><span>{formatBytes(source.sizeBytes)}</span><span>{highlightedLines ? `${languageForPath(source.path)} syntax` : 'Plain text'}</span><strong>{citationHashMismatch ? 'Citation stale' : invalidAnchorReason ? 'Anchor unavailable' : sourceRange ? 'Citation verified' : 'No citation anchor'}</strong></div>
        {citationHashMismatch && <div className="source-range-focus" role="alert">
          <span><strong>Stale citation</strong><small>The chapter expects source SHA-256 {sourceRange?.sourceSha256}, but this snapshot contains {source.sha256}. The line range was not highlighted and Code Atlas did not search for a nearby match.</small></span>
        </div>}
        {!citationHashMismatch && invalidAnchorReason && <div className="source-range-focus is-invalid" role="alert">
          <span><strong>Invalid or unavailable anchor</strong><small>{invalidAnchorReason} No lines are highlighted; the captured source remains available for inspection.</small></span>
        </div>}
        {!sourceRange && !sourceRangeIssue && <div className="source-range-focus is-unanchored" role="status">
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
        <pre className="snapshot-source-code" tabIndex={0} aria-label={visibleSourceRange
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
        <p className="source-safety-note"><HardDrive size={13} /> Loaded from immutable local snapshot storage and checked against the manifest hash. The live working tree was not read.</p>
        <section className="code-lens-related" aria-labelledby="code-lens-related-title">
          <div><span className="step-pill"><span>LOCAL INDEX</span> RELATED CODE</span><h3 id="code-lens-related-title">Callers, imports, and tests</h3></div>
          <p>These links come from the selected snapshot index. Each row preserves the indexed relationship type and confidence.</p>
          {relationshipLoading ? <p role="status">Loading relationships for this snapshot…</p>
            : relationshipError ? <p role="status">{relationshipError}</p>
              : relatedRelationships.length > 0 ? <ul>{relatedRelationships.map((relationship) => {
                const sourceSymbol = snapshotIndex?.symbols.find((symbol) => symbol.symbolId === relationship.sourceId);
                const targetSymbol = relationship.targetId
                  ? snapshotIndex?.symbols.find((symbol) => symbol.symbolId === relationship.targetId)
                  : null;
                const targetName = targetSymbol?.qualifiedName ?? targetSymbol?.name ?? relationship.targetLabel ?? relationship.targetPath ?? 'unresolved target';
                return <li key={relationship.relationshipId}>
                  <div><strong>{codeLensRelationshipLabel(relationship.kind, relationship.confidence)}</strong>
                    <span>{sourceSymbol?.qualifiedName ?? sourceSymbol?.name ?? relationship.sourceId} → {targetName}</span>
                    <small>{relationship.confidence === 'unresolved' ? relationship.unresolvedReason ?? 'The target could not be resolved in this snapshot.' : `${relationship.evidence.path}:${relationship.evidence.startLine}-${relationship.evidence.endLine}`}</small>
                  </div>
                <button className="text-button" onClick={() => onOpenFile(
                    relationship.evidence.path,
                    snapshotIndex?.snapshotId,
                  codeLensRangeFromIndexedAnchor(relationship.evidence),
                  )}>Open evidence</button>
                </li>;
              })}</ul> : <p>{snapshotIndex ? 'No indexed caller, import, or test relationships point to or from this file.' : 'No matching local index is available for this snapshot.'}</p>}
        </section>
      </div>
    );
  }
  return (
    <div className="empty-workspace simple-empty">
      <div className="empty-state-icon code-icon"><Code2 size={24} /></div>
      <div className="step-pill"><span>02</span> SOURCE INSPECTION</div>
      <h2>Open a captured file from Atlas.</h2>
      <p>Code Lens reads a verified blob from the selected immutable snapshot. It never quietly switches to the current working tree.</p>
      <button className="secondary-button" data-focus-key="cta:code-lens-atlas" onClick={onGoAtlas}>Go to Atlas inventory <ArrowRight size={15} /></button>
      <div className="state-note"><HardDrive size={14} /> No source file is selected.</div>
    </div>
  );
}

function WalkthroughEmptyState({ onGoPlan }: { onGoPlan: () => void }) {
  return (
    <div className="empty-workspace simple-empty walkthrough-empty">
      <div className="empty-state-icon play-icon"><Play size={22} /></div>
      <div className="step-pill"><span>03</span> INTERACTIVE LEARNING</div>
      <h2>Explanations stay connected to code.</h2>
      <p>Each accepted file chapter will be split into deliberate sections. Follow a hotspot to source, return to the same place, and continue when you are ready.</p>
      <button className="secondary-button" data-focus-key="cta:walkthrough-plan" onClick={onGoPlan}>View Guide Plan <ArrowRight size={15} /></button>
      <div className="state-note"><BookOpen size={14} /> No accepted chapters in this project.</div>
    </div>
  );
}

function WorkflowEmptyState({ kind, onGoAtlas, onGoPlan }: { kind: 'plan' | 'studio'; onGoAtlas?: () => void; onGoPlan?: () => void }) {
  const isPlan = kind === 'plan';
  const Icon = isPlan ? ListChecks : BookOpen;
  return (
    <div className="empty-workspace simple-empty workflow-empty">
      <div className={`empty-state-icon ${isPlan ? 'plan-icon' : 'studio-icon'}`}><Icon size={23} /></div>
      <div className="step-pill"><span>WORKFLOW</span> {isPlan ? 'COVERAGE' : 'CHAPTER REVIEW'}</div>
      <h2>{isPlan ? 'Every path will have a clear status.' : 'Drafts are reviewed before they become guides.'}</h2>
      <p>{isPlan ? 'Guide Plan will account for captured files individually and show accepted, draft, excluded, or unsupported status.' : 'Chapter Studio will show each explanation beside its source citations so you can edit and validate it.'}</p>
      <button className="secondary-button" data-focus-key={`cta:${kind}`} onClick={isPlan ? onGoAtlas : onGoPlan}>{isPlan ? 'Go to Atlas' : 'Open Guide Plan'} <ArrowRight size={15} /></button>
      <div className="state-note"><HardDrive size={14} /> {isPlan ? 'A snapshot is required before coverage exists.' : 'A file chapter is required before review.'}</div>
    </div>
  );
}

export { App };
