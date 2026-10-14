import type { SnapshotIndex, SnapshotManifest, IndexedFile, IndexedRelationship, IndexedSymbol, IndexFileStatus, RelationshipKind } from './commands';
import type { InventoryRow } from './inventory';

/** A path segment in the snapshot inventory. Rows belong to the segment matching their path. */
export interface InventoryTreeNode {
  path: string;
  name: string;
  rows: InventoryRow[];
  children: InventoryTreeNode[];
}

export interface FlattenedInventoryRow {
  /** The original row object; skipped and unsupported rows are preserved verbatim. */
  row: InventoryRow;
  path: string;
  depth: number;
  hasChildren: boolean;
  expanded: boolean;
  positionInSet: number;
  setSize: number;
}

export type AtlasTreeKeyboardAction =
  | { kind: 'focus'; index: number }
  | { kind: 'expand' | 'collapse'; path: string }
  | { kind: 'activate' | 'select' | 'handled' | 'none' };

/** Maps standard tree keys to a roving focus, expand/collapse, or activation action. */
export function atlasTreeKeyboardAction(
  items: readonly FlattenedInventoryRow[],
  index: number,
  key: string,
): AtlasTreeKeyboardAction {
  const item = items[index];
  if (!item) return { kind: 'none' };
  if (key === 'ArrowDown' || key === 'ArrowUp' || key === 'Home' || key === 'End') {
    const target = key === 'Home' ? 0 : key === 'End' ? items.length - 1 : index + (key === 'ArrowDown' ? 1 : -1);
    return { kind: 'focus', index: Math.max(0, Math.min(items.length - 1, target)) };
  }
  if (key === 'ArrowRight') {
    if (!item.hasChildren) return { kind: 'handled' };
    if (!item.expanded) return { kind: 'expand', path: item.path };
    const child = items.findIndex((candidate, candidateIndex) => candidateIndex > index && candidate.depth > item.depth);
    return child >= 0 ? { kind: 'focus', index: child } : { kind: 'handled' };
  }
  if (key === 'ArrowLeft') {
    if (item.hasChildren && item.expanded) return { kind: 'collapse', path: item.path };
    for (let parent = index - 1; parent >= 0; parent -= 1) {
      if (items[parent].depth < item.depth) return { kind: 'focus', index: parent };
    }
    return { kind: 'handled' };
  }
  if (key === 'Enter') return { kind: 'activate' };
  if (key === ' ') return { kind: 'select' };
  return { kind: 'none' };
}

function compare(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function canonicalPath(path: string): string {
  const segments = path.split('/').filter((segment) => segment !== '' && segment !== '.');
  return segments.join('/') || '.';
}

function rowOrder(left: InventoryRow, right: InventoryRow): number {
  return compare(left.kind, right.kind)
    || compare(left.reason, right.reason)
    || compare(left.disposition, right.disposition)
    || compare(left.details, right.details)
    || compare(left.sha256 ?? '', right.sha256 ?? '')
    || compare(left.language, right.language)
    || compare(left.contentKind, right.contentKind)
    || compare(left.parserReadiness, right.parserReadiness)
    || compare(left.indexEligibility, right.indexEligibility)
    || compare(left.categories.join(','), right.categories.join(','))
    || compare(String(left.sizeBytes), String(right.sizeBytes));
}

/**
 * Builds a path tree without dropping inventory records. Missing intermediate folders are
 * structural nodes only; every supplied row is retained on exactly one node, including rows
 * for skipped files, excluded paths, and unsupported content.
 */
export function buildInventoryTree(rows: readonly InventoryRow[]): InventoryTreeNode {
  const root: InventoryTreeNode = { path: '.', name: '.', rows: [], children: [] };
  const byPath = new Map<string, InventoryTreeNode>([['.', root]]);

  for (const row of rows) {
    const path = canonicalPath(row.path);
    const segments = path === '.' ? [] : path.split('/');
    let parent = root;
    let currentPath = '';
    for (const segment of segments) {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment;
      let node = byPath.get(currentPath);
      if (!node) {
        node = { path: currentPath, name: segment, rows: [], children: [] };
        byPath.set(currentPath, node);
        parent.children.push(node);
      }
      parent = node;
    }
    parent.rows.push(row);
  }

  const sortTree = (node: InventoryTreeNode): void => {
    node.rows.sort(rowOrder);
    node.children.sort((left, right) => compare(left.name, right.name) || compare(left.path, right.path));
    node.children.forEach(sortTree);
  };
  sortTree(root);
  return root;
}

/** Flattens a tree in deterministic path order; collapsed path nodes hide their descendants. */
export function flattenInventoryTree(
  tree: InventoryTreeNode,
  collapsedPaths: ReadonlySet<string> = new Set(),
): FlattenedInventoryRow[] {
  const flattened: FlattenedInventoryRow[] = [];
  const visit = (node: InventoryTreeNode, depth: number, positionInSet: number, setSize: number): void => {
    const hasChildren = node.children.length > 0;
    const expanded = hasChildren && !collapsedPaths.has(node.path);
    for (const row of node.rows) flattened.push({ row, path: node.path, depth, hasChildren, expanded, positionInSet, setSize });
    if (expanded) node.children.forEach((child, index) => visit(child, depth + 1, index + 1, node.children.length));
  };
  visit(tree, 0, 1, 1);
  return flattened;
}

/** Filter inventory rows for the Atlas tree while retaining matching ancestors. */
export function filterInventoryTreeRows(
  rows: readonly InventoryRow[],
  folderPath: string | null,
  queryText: string,
): InventoryRow[] {
  const query = queryText.trim().toLowerCase();
  const withinFolder = (path: string, folder: string | null): boolean =>
    !folder || folder === '.' || path === folder || path.startsWith(`${folder}/`);
  const matched = rows.filter((row) => withinFolder(row.path, folderPath)
    && (!query || `${row.path} ${row.reason} ${row.details} ${row.language}`.toLowerCase().includes(query)));
  const matchingRows = new Set(matched);
  const retainedPaths = new Set(matched.map((row) => row.path));

  for (const row of matched) {
    const parts = row.path.split('/').filter(Boolean);
    while (parts.length > 1) {
      parts.pop();
      const parent = parts.join('/');
      if (withinFolder(parent, folderPath)) retainedPaths.add(parent);
    }
    if (!folderPath && row.path !== '.') retainedPaths.add('.');
  }

  return rows.filter((row) => retainedPaths.has(row.path)
    && (matchingRows.has(row) || row.kind === 'directory'));
}

export interface VirtualRowWindow {
  startIndex: number;
  endIndex: number;
  topSpacerHeight: number;
  bottomSpacerHeight: number;
}

/** Return the overscanned row window and spacer sizes for a fixed-height virtual list. */
export function virtualRowWindow(
  itemCount: number,
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number,
  overscan: number,
): VirtualRowWindow {
  const count = Number.isFinite(itemCount) ? Math.max(0, Math.floor(itemCount)) : 0;
  const height = Number.isFinite(rowHeight) && rowHeight > 0 ? rowHeight : 1;
  const scroll = Number.isFinite(scrollTop) ? Math.max(0, scrollTop) : 0;
  const viewport = Number.isFinite(viewportHeight) ? Math.max(0, viewportHeight) : 0;
  const extra = Number.isFinite(overscan) ? Math.max(0, Math.floor(overscan)) : 0;
  const firstVisible = Math.min(count, Math.floor(scroll / height));
  const afterLastVisible = Math.min(count, Math.ceil((scroll + viewport) / height));
  const startIndex = Math.max(0, firstVisible - extra);
  const endIndex = Math.min(count, afterLastVisible + extra);

  return {
    startIndex,
    endIndex,
    topSpacerHeight: startIndex * height,
    bottomSpacerHeight: (count - endIndex) * height,
  };
}

export interface ProvenGraphNode {
  symbol: IndexedSymbol;
  snapshotId: string;
  sourceSha256: string;
}

export interface ProvenGraphEdge {
  relationship: IndexedRelationship;
  snapshotId: string;
  sourceId: string;
  targetId: string;
  evidence: {
    path: string;
    sha256: string;
    startLine: number;
    endLine: number;
    startByte: number;
    endByte: number;
  };
}

export interface ProvenRelationshipGraph {
  snapshotId: string;
  nodes: ProvenGraphNode[];
  edges: ProvenGraphEdge[];
}

export interface RelationshipFilterOptions {
  /** A selected folder includes edges with at least one endpoint below it. */
  folderPath: string | null;
  relationshipKind: 'all' | RelationshipKind;
  /** A selected language includes edges with at least one endpoint in that language. */
  language?: string;
  query: string;
}

/** Apply Atlas's visible relationship scope and filters without changing snapshot identity. */
export function filterProvenRelationshipEdges(
  edges: readonly ProvenGraphEdge[],
  symbols: readonly IndexedSymbol[],
  options: RelationshipFilterOptions,
): ProvenGraphEdge[] {
  const symbolsById = new Map(symbols.map((symbol) => [symbol.symbolId, symbol]));
  const query = options.query.trim().toLowerCase();
  const withinFolder = (path: string, folder: string | null): boolean =>
    !folder || folder === '.' || path === folder || path.startsWith(`${folder}/`);

  return edges.filter((edge) => {
    const source = symbolsById.get(edge.sourceId);
    const target = symbolsById.get(edge.targetId);
    if (!source || !target) return false;

    const inScope = withinFolder(source.path, options.folderPath)
      || withinFolder(target.path, options.folderPath);
    if (!inScope) return false;
    if (options.relationshipKind !== 'all' && edge.relationship.kind !== options.relationshipKind) return false;
    if (options.language && options.language !== 'all'
      && source.language !== options.language && target.language !== options.language) return false;
    if (!query) return true;

    const searchable = [
      edge.relationship.kind,
      source.qualifiedName,
      target.qualifiedName,
      source.path,
      target.path,
      edge.evidence.path,
    ].join(' ').toLowerCase();
    return searchable.includes(query);
  });
}

export type AtlasRowIndexStatus = IndexFileStatus | 'indexing' | 'pending' | 'unavailable' | 'notIndexed' | 'stale' | 'unsupported';

/** Resolve a captured inventory file's actual local-index state instead of reusing its initial eligibility. */
export function atlasRowIndexStatus(
  row: InventoryRow,
  index: SnapshotIndex | null,
  indexState: 'loading' | 'pending' | 'ready' | 'error',
  staleEvidence = false,
): AtlasRowIndexStatus | null {
  if (row.kind !== 'file' || row.disposition !== 'captured' || row.contentKind !== 'text') return null;
  if (row.parserReadiness === 'unsupported') return 'unsupported';
  if (indexState === 'loading') return 'indexing';
  if (indexState === 'pending') return 'pending';
  if (indexState === 'error') return 'unavailable';
  if (staleEvidence) return 'stale';
  const indexedFile = index?.files.find((file) => file.path === row.path);
  if (!indexedFile) return index?.status === 'cancelled' ? 'cancelled' : 'notIndexed';
  if (row.sha256 && indexedFile.sourceSha256 !== row.sha256) return 'stale';
  return indexedFile.status;
}

export function atlasRowIndexStatusLabel(status: AtlasRowIndexStatus | null): string | null {
  if (status === null) return null;
  const labels: Record<AtlasRowIndexStatus, string> = {
    indexed: 'indexed',
    partialParse: 'partial parse',
    parseError: 'parse error',
    unsupportedLanguage: 'unsupported language',
    binary: 'binary',
    invalidEncoding: 'invalid encoding',
    sourceUnavailable: 'source unavailable',
    fileTooLarge: 'over limit',
    indexLimitExceeded: 'index limit',
    analysisLimitExceeded: 'analysis limit',
    parserUnavailable: 'parser unavailable',
    cancelled: 'cancelled',
    indexing: 'indexing',
    pending: 'index pending',
    unavailable: 'index unavailable',
    notIndexed: 'not indexed',
    stale: 'stale index',
    unsupported: 'unsupported',
  };
  return labels[status];
}

export interface AtlasPathIndexCounts {
  indexedFiles: number;
  symbols: number;
  relationships: number;
}

export interface AtlasViewport {
  x: number;
  y: number;
  scale: number;
}

export const ATLAS_MIN_ZOOM = 0.35;
export const ATLAS_MAX_ZOOM = 2;
export const ATLAS_ZOOM_STEP = 0.15;

export function normalizeAtlasViewport(value: Partial<AtlasViewport> | null | undefined): AtlasViewport {
  const scale = Number.isFinite(value?.scale) ? value!.scale! : 1;
  return {
    x: Number.isFinite(value?.x) ? value!.x! : 0,
    y: Number.isFinite(value?.y) ? value!.y! : 0,
    scale: Math.max(ATLAS_MIN_ZOOM, Math.min(ATLAS_MAX_ZOOM, scale)),
  };
}

export function panAtlasViewport(viewport: AtlasViewport, dx: number, dy: number): AtlasViewport {
  return { ...viewport, x: viewport.x + dx, y: viewport.y + dy };
}

/** Zoom while keeping the chosen point in the same screen position. */
export function zoomAtlasViewport(viewport: AtlasViewport, delta: number, centerX = 0, centerY = 0): AtlasViewport {
  const nextScale = Math.max(ATLAS_MIN_ZOOM, Math.min(ATLAS_MAX_ZOOM, viewport.scale + delta));
  const ratio = nextScale / viewport.scale;
  return {
    x: centerX - (centerX - viewport.x) * ratio,
    y: centerY - (centerY - viewport.y) * ratio,
    scale: nextScale,
  };
}

/** Return a scale that fits the complete relationship surface into the visible panel. */
export function fitAtlasViewport(contentWidth: number, contentHeight: number, viewportWidth: number, viewportHeight: number): AtlasViewport {
  if (![contentWidth, contentHeight, viewportWidth, viewportHeight].every((value) => Number.isFinite(value) && value > 0)) {
    return { x: 0, y: 0, scale: 1 };
  }
  const scale = Math.max(ATLAS_MIN_ZOOM, Math.min(1, (viewportWidth / contentWidth), (viewportHeight / contentHeight)));
  return { x: 0, y: 0, scale };
}

/** Summarize observed index counts for one file or all indexed descendants of a folder. */
export function summarizeAtlasPathIndexCounts(
  files: readonly IndexedFile[],
  selectedPath: string,
  isDirectory: boolean,
): AtlasPathIndexCounts {
  const matchingFiles = isDirectory
    ? files.filter((file) => selectedPath === '.' || file.path.startsWith(`${selectedPath}/`))
    : files.filter((file) => file.path === selectedPath);
  return matchingFiles.reduce<AtlasPathIndexCounts>((counts, file) => ({
    indexedFiles: counts.indexedFiles + 1,
    symbols: counts.symbols + file.symbolCount,
    relationships: counts.relationships + file.relationshipCount,
  }), { indexedFiles: 0, symbols: 0, relationships: 0 });
}

/**
 * Projects proven graph evidence for the selected snapshot files. A relationship is admitted
 * only when it is resolved/high confidence, both symbol endpoints and its evidence file are
 * selected, and every corresponding index hash exactly matches the captured manifest hash.
 */
export function projectProvenRelationshipGraph(
  index: SnapshotIndex,
  manifest: SnapshotManifest,
  selectedPaths: ReadonlySet<string>,
): ProvenRelationshipGraph {
  const snapshotId = manifest.summary.snapshotId;
  if (index.snapshotId !== snapshotId) return { snapshotId, nodes: [], edges: [] };

  const captured = new Map(manifest.files.map((file) => [file.path, file.sha256]));
  const indexedFiles = new Map(index.files.map((file) => [file.path, file.sourceSha256]));
  const symbols = new Map(index.symbols.map((symbol) => [symbol.symbolId, symbol]));
  const admittedNodes = new Map<string, ProvenGraphNode>();
  const edges: ProvenGraphEdge[] = [];
  const isExactCapturedFile = (path: string): boolean => {
    const manifestHash = captured.get(path);
    return selectedPaths.has(path)
      && manifestHash !== undefined
      && indexedFiles.get(path) === manifestHash;
  };

  for (const relationship of index.relationships) {
    if (relationship.confidence !== 'high' || relationship.unresolvedReason !== null || !relationship.targetId) continue;
    const source = symbols.get(relationship.sourceId);
    const target = symbols.get(relationship.targetId);
    if (!source || !target) continue;
    if (relationship.targetPath !== null && relationship.targetPath !== target.path) continue;

    const evidencePath = relationship.evidence.path;
    if (!isExactCapturedFile(source.path) || !isExactCapturedFile(target.path) || !isExactCapturedFile(evidencePath)) continue;
    if (source.anchor.path !== source.path || source.anchor.sourceSha256 !== captured.get(source.path)
      || target.anchor.path !== target.path || target.anchor.sourceSha256 !== captured.get(target.path)
      || relationship.evidence.sourceSha256 !== captured.get(evidencePath)) continue;

    const sourceHash = captured.get(source.path)!;
    const targetHash = captured.get(target.path)!;
    admittedNodes.set(source.symbolId, { symbol: source, snapshotId, sourceSha256: sourceHash });
    admittedNodes.set(target.symbolId, { symbol: target, snapshotId, sourceSha256: targetHash });
    edges.push({
      relationship,
      snapshotId,
      sourceId: source.symbolId,
      targetId: target.symbolId,
      evidence: {
        path: evidencePath,
        sha256: relationship.evidence.sourceSha256,
        startLine: relationship.evidence.startLine,
        endLine: relationship.evidence.endLine,
        startByte: relationship.evidence.startByte,
        endByte: relationship.evidence.endByte,
      },
    });
  }

  edges.sort((left, right) => compare(left.relationship.relationshipId, right.relationship.relationshipId));
  return {
    snapshotId,
    nodes: [...admittedNodes.values()].sort((left, right) => compare(left.symbol.symbolId, right.symbol.symbolId)),
    edges,
  };
}
