export const WORKSPACES = ['atlas', 'code-lens', 'walkthrough', 'guide-plan', 'chapter-studio', 'changes'] as const;

export type Workspace = (typeof WORKSPACES)[number];

/** Inclusive source line range. Both endpoints are 1-based positive integers. */
export interface SourceLineRange {
  startLine: number;
  endLine: number;
  /** Optional citation source hash; present when opened from a chapter anchor. */
  sourceSha256?: string;
  /** Optional exact UTF-8 byte range, half-open [startByte, endByteExclusive). */
  startByte?: number;
  endByteExclusive?: number;
}

export type AtlasHotspotSelection =
  | { kind: 'file'; path: string }
  | { kind: 'symbol'; path: string; symbolId: string }
  | { kind: 'relationship'; relationshipId: string };

export interface WorkspaceRoute {
  workspace: Workspace;
  projectId: string | null;
  snapshotId: string | null;
  relativePath?: string;
  sourceRange?: SourceLineRange;
  /** A citation range was supplied but could not be normalized safely. */
  sourceRangeIssue?: 'invalid-anchor' | 'invalid-line-range' | 'invalid-source-hash' | 'invalid-byte-range';
  symbolId?: string;
  chapterId?: string;
  chapterRevisionId?: string;
  sectionId?: string;
  playbackPosition?: number;
  transcriptPosition?: number;
  /** Snapshot selected before following a chapter pinned to its own source snapshot. */
  readerSnapshotBaselineId?: string;
  atlasSelection?: AtlasHotspotSelection;
  searchQuery?: string;
  treeSelection?: string;
  mapViewport?: { x: number; y: number; scale: number };
  focusTarget?: string;
  /** Explicit local Git refs used by the read-only change comparison workflow. */
  baseRef?: string;
  headRef?: string;
  selectedChangePath?: string;
}

export interface NavigationState {
  current: WorkspaceRoute;
  back: WorkspaceRoute[];
  forward: WorkspaceRoute[];
}

export const initialNavigationState: NavigationState = {
  current: { workspace: 'atlas', projectId: null, snapshotId: null, focusTarget: 'atlas:open-repository' },
  back: [],
  forward: [],
};

function sameRoute(left: WorkspaceRoute, right: WorkspaceRoute): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

/**
 * Return a canonical inclusive line range, or undefined when the input is
 * absent or malformed. Extra input properties are intentionally discarded.
 */
export function normalizeSourceLineRange(value: unknown): SourceLineRange | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const range = value as {
    startLine?: unknown; endLine?: unknown; sourceSha256?: unknown;
    startByte?: unknown; endByteExclusive?: unknown;
  };
  if (!Number.isSafeInteger(range.startLine) || !Number.isSafeInteger(range.endLine)) return undefined;
  const startLine = range.startLine as number;
  const endLine = range.endLine as number;
  if (startLine < 1 || endLine < startLine) return undefined;
  if (range.sourceSha256 !== undefined
    && (typeof range.sourceSha256 !== 'string' || !/^[a-f0-9]{64}$/.test(range.sourceSha256))) return undefined;
  const hasStartByte = range.startByte !== undefined;
  const hasEndByte = range.endByteExclusive !== undefined;
  if (hasStartByte !== hasEndByte) return undefined;
  if (hasStartByte && (!Number.isSafeInteger(range.startByte) || !Number.isSafeInteger(range.endByteExclusive)
    || (range.startByte as number) < 0 || (range.endByteExclusive as number) <= (range.startByte as number))) return undefined;
  return {
    startLine,
    endLine,
    ...(range.sourceSha256 === undefined ? {} : { sourceSha256: range.sourceSha256 as string }),
    ...(hasStartByte ? { startByte: range.startByte as number, endByteExclusive: range.endByteExclusive as number } : {}),
  };
}

export function sourceRangeMatchesHash(range: SourceLineRange | undefined, sourceSha256: string | undefined): boolean {
  return !range?.sourceSha256 || range.sourceSha256 === sourceSha256;
}

function normalizeRoute(route: WorkspaceRoute): WorkspaceRoute {
  const sourceRange = normalizeSourceLineRange(route.sourceRange);
  if (sourceRange) {
    const { sourceRangeIssue: _ignoredIssue, ...withoutIssue } = route;
    return { ...withoutIssue, sourceRange };
  }
  if (route.sourceRange === undefined) return route;
  const withoutRange = { ...route };
  delete withoutRange.sourceRange;
  const range = route.sourceRange as {
    startLine?: unknown; endLine?: unknown; sourceSha256?: unknown;
    startByte?: unknown; endByteExclusive?: unknown;
  } | null;
  const issue = !range || typeof range !== 'object' || Array.isArray(range)
    ? 'invalid-anchor'
    : (range.startLine !== undefined || range.endLine !== undefined)
      && (!Number.isSafeInteger(range.startLine)
        || !Number.isSafeInteger(range.endLine)
        || (range.startLine as number) < 1
        || (range.endLine as number) < (range.startLine as number))
        ? 'invalid-line-range'
        : range.startByte !== undefined || range.endByteExclusive !== undefined
          ? 'invalid-byte-range'
          : 'invalid-source-hash';
  return { ...withoutRange, sourceRangeIssue: issue };
}

export function navigateToRoute(state: NavigationState, route: WorkspaceRoute): NavigationState {
  const normalizedRoute = normalizeRoute(route);
  if (sameRoute(state.current, normalizedRoute)) return state;
  return {
    current: normalizedRoute,
    back: [...state.back, state.current],
    forward: [],
  };
}

/** Update in-place player context without making every section or scroll step a history entry. */
export function replaceCurrentRoute(state: NavigationState, route: WorkspaceRoute): NavigationState {
  const normalizedRoute = normalizeRoute(route);
  if (sameRoute(state.current, normalizedRoute)) return state;
  return { ...state, current: normalizedRoute };
}

export function navigateToWorkspace(
  state: NavigationState,
  workspace: Workspace,
  returnFocusTarget?: string,
): NavigationState {
  if (state.current.workspace === workspace) return state;
  const stateWithOriginFocus = returnFocusTarget
    ? { ...state, current: { ...state.current, focusTarget: returnFocusTarget } }
    : state;
  return navigateToRoute(stateWithOriginFocus, {
      ...stateWithOriginFocus.current,
      workspace,
      focusTarget: `heading:${workspace}`,
    });
}

export function navigateBack(state: NavigationState): NavigationState {
  if (state.back.length === 0) return state;
  const current = state.back[state.back.length - 1];
  return {
    current,
    back: state.back.slice(0, -1),
    forward: [state.current, ...state.forward],
  };
}

export function navigateForward(state: NavigationState): NavigationState {
  if (state.forward.length === 0) return state;
  const [current, ...forward] = state.forward;
  return {
    current,
    back: [...state.back, state.current],
    forward,
  };
}

export function canNavigateBack(state: NavigationState): boolean {
  return state.back.length > 0;
}

export function canNavigateForward(state: NavigationState): boolean {
  return state.forward.length > 0;
}
