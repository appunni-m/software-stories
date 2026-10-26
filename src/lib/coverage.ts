import type { ChapterRevision } from './commands.ts';
import type { InventoryRow } from './inventory.ts';
import { inventoryReasonLabel } from './inventory.ts';

export type CoverageStatus = 'missing' | 'draft' | 'rejected' | 'stale' | 'accepted' | 'excluded' | 'unsupported' | 'unavailable';

export interface CoverageGroup {
  groupId: string;
  name: string;
  rationale: string;
  paths: string[];
  createdAtUnixMs: number;
}

export interface CoverageExclusion {
  path: string;
  reason: string;
  excludedAtUnixMs: number;
}

export interface CoveragePlan {
  schemaVersion: 1;
  projectId: string;
  snapshotId: string;
  groups: CoverageGroup[];
  exclusions: CoverageExclusion[];
}

export type CoveragePlanRead =
  | { status: 'ready'; plan: CoveragePlan }
  | { status: 'unavailable'; plan: CoveragePlan; message: string };

export interface CoverageRow {
  inventory: InventoryRow;
  status: CoverageStatus;
  reason: string | null;
  targetId: string | null;
  revisionId: string | null;
  revisionNumber: number | null;
  group: CoverageGroup | null;
  userExclusion: CoverageExclusion | null;
  canOpenSource: boolean;
  canSelectForGroup: boolean;
  canExclude: boolean;
  searchText: string;
}

export interface CoverageTotals extends Record<CoverageStatus, number> {
  total: number;
}

export interface CoveragePage<T> {
  rows: T[];
  total: number;
  page: number;
  pageSize: number;
  pageCount: number;
  firstVisible: number;
  lastVisible: number;
}

const MAX_COVERAGE_PLAN_BYTES = 4 * 1024 * 1024;

function exceedsCoveragePlanLimit(serialized: string): boolean {
  return serialized.length > MAX_COVERAGE_PLAN_BYTES
    || new TextEncoder().encode(serialized).byteLength > MAX_COVERAGE_PLAN_BYTES;
}

export const COVERAGE_STATUSES: CoverageStatus[] = [
  'missing', 'draft', 'rejected', 'stale', 'accepted', 'excluded', 'unsupported', 'unavailable',
];

export function emptyCoveragePlan(projectId: string, snapshotId: string): CoveragePlan {
  return { schemaVersion: 1, projectId, snapshotId, groups: [], exclusions: [] };
}

export function coverageStorageKey(projectId: string, snapshotId: string): string {
  return `code-atlas:coverage:v1:${encodeURIComponent(projectId)}:${encodeURIComponent(snapshotId)}`;
}

export function readCoveragePlan(projectId: string, snapshotId: string, storage?: Pick<Storage, 'getItem'>): CoveragePlanRead {
  const empty = emptyCoveragePlan(projectId, snapshotId);
  let saved: string | null;
  try {
    saved = (storage ?? window.localStorage).getItem(coverageStorageKey(projectId, snapshotId));
  } catch {
    return { status: 'unavailable', plan: empty, message: 'Saved file groups and exclusions could not be read. Their stored data has not been changed.' };
  }
  if (saved === null) return { status: 'ready', plan: empty };
  if (exceedsCoveragePlanLimit(saved)) {
    return { status: 'unavailable', plan: empty, message: 'Saved file groups and exclusions exceed the local size limit. Their stored data has not been changed.' };
  }
  let parsed: unknown;
  try { parsed = JSON.parse(saved); }
  catch { return { status: 'unavailable', plan: empty, message: 'Saved file groups and exclusions are malformed. Their stored data has not been changed.' }; }
  if (!isCoveragePlan(parsed) || parsed.projectId !== projectId || parsed.snapshotId !== snapshotId) {
    return { status: 'unavailable', plan: empty, message: 'Saved file groups and exclusions use an unsupported format or repository snapshot. Their stored data has not been changed.' };
  }
  return { status: 'ready', plan: parsed };
}

/** Strict convenience reader for current records. Callers that render UI should
 * use readCoveragePlan so unavailable stored data can be shown and preserved. */
export function loadCoveragePlan(projectId: string, snapshotId: string, storage?: Pick<Storage, 'getItem'>): CoveragePlan {
  const result = readCoveragePlan(projectId, snapshotId, storage);
  if (result.status !== 'ready') throw new Error(result.message);
  return result.plan;
}

export function discardCoveragePlan(projectId: string, snapshotId: string, storage?: Pick<Storage, 'removeItem'>): void {
  (storage ?? window.localStorage).removeItem(coverageStorageKey(projectId, snapshotId));
}

export function saveCoveragePlan(plan: CoveragePlan, storage?: Pick<Storage, 'getItem' | 'setItem'>): boolean {
  if (!isCoveragePlan(plan)) return false;
  try {
    const serialized = JSON.stringify(plan);
    if (exceedsCoveragePlanLimit(serialized)) return false;
    const target = storage ?? window.localStorage;
    const key = coverageStorageKey(plan.projectId, plan.snapshotId);
    const current = target.getItem(key);
    if (current !== null) {
      if (exceedsCoveragePlanLimit(current)) return false;
      let parsed: unknown;
      try { parsed = JSON.parse(current); } catch { return false; }
      if (!isCoveragePlan(parsed) || parsed.projectId !== plan.projectId || parsed.snapshotId !== plan.snapshotId) return false;
    }
    target.setItem(key, serialized);
    return true;
  } catch {
    return false;
  }
}

function isCoveragePlan(value: unknown): value is CoveragePlan {
  const object = (candidate: unknown): candidate is Record<string, unknown> =>
    !!candidate && typeof candidate === 'object' && !Array.isArray(candidate);
  const exactKeys = (candidate: Record<string, unknown>, keys: readonly string[]) =>
    Object.keys(candidate).length === keys.length && keys.every((key) => Object.hasOwn(candidate, key));
  const validPath = (path: unknown): path is string => typeof path === 'string' && path.length > 0
    && !path.startsWith('/') && !path.includes('\\') && !/[\u0000-\u001f\u007f]/u.test(path)
    && new TextEncoder().encode(path).byteLength <= 1024
    && !path.split('/').some((part) => !part || part === '.' || part === '..');
  if (!object(value) || !exactKeys(value, ['schemaVersion', 'projectId', 'snapshotId', 'groups', 'exclusions'])
    || value.schemaVersion !== 1 || typeof value.projectId !== 'string' || !value.projectId
    || typeof value.snapshotId !== 'string' || !value.snapshotId
    || !Array.isArray(value.groups) || value.groups.length > 25_000
    || !Array.isArray(value.exclusions) || value.exclusions.length > 25_000) return false;

  const groupIds = new Set<string>();
  const occupiedPaths = new Set<string>();
  for (const candidate of value.groups) {
    if (!object(candidate) || !exactKeys(candidate, ['groupId', 'name', 'rationale', 'paths', 'createdAtUnixMs'])
      || typeof candidate.groupId !== 'string' || !/^[A-Za-z0-9_-]{1,256}$/u.test(candidate.groupId)
      || groupIds.has(candidate.groupId) || typeof candidate.name !== 'string' || !candidate.name.trim() || candidate.name.length > 100
      || typeof candidate.rationale !== 'string' || !candidate.rationale.trim() || candidate.rationale.length > 500
      || !Array.isArray(candidate.paths) || candidate.paths.length < 2
      || !Number.isSafeInteger(candidate.createdAtUnixMs) || (candidate.createdAtUnixMs as number) < 0
      || new Set(candidate.paths).size !== candidate.paths.length
      || candidate.paths.some((path) => !validPath(path) || occupiedPaths.has(path))) return false;
    groupIds.add(candidate.groupId);
    for (const path of candidate.paths as string[]) occupiedPaths.add(path);
  }

  const exclusions = new Set<string>();
  for (const candidate of value.exclusions) {
    if (!object(candidate) || !exactKeys(candidate, ['path', 'reason', 'excludedAtUnixMs'])
      || !validPath(candidate.path) || occupiedPaths.has(candidate.path) || exclusions.has(candidate.path)
      || typeof candidate.reason !== 'string' || !candidate.reason.trim() || candidate.reason.length > 500
      || !Number.isSafeInteger(candidate.excludedAtUnixMs) || (candidate.excludedAtUnixMs as number) < 0) return false;
    exclusions.add(candidate.path);
  }
  return true;
}

function isLocallyReadable(row: InventoryRow): boolean {
  return row.disposition === 'captured' && row.kind === 'file'
    && (row.contentKind === 'text' || row.contentKind === 'unknown');
}

function isEligibleForChapter(row: InventoryRow): boolean {
  return isLocallyReadable(row);
}

interface PathRevisionIndex {
  current: ChapterRevision | null;
  newest: ChapterRevision | null;
}

function newerRevision(candidate: ChapterRevision, current: ChapterRevision | null): boolean {
  return current === null || candidate.revisionNumber > current.revisionNumber
    || (candidate.revisionNumber === current.revisionNumber && candidate.createdAtUnixMs > current.createdAtUnixMs);
}

/** Index once so coverage scales with files plus revisions, not their product. */
function revisionsByPath(
  projectId: string,
  snapshotId: string,
  revisions: ChapterRevision[],
): Map<string, PathRevisionIndex> {
  const index = new Map<string, PathRevisionIndex>();
  for (const revision of revisions) {
    if (revision.projectId !== projectId || revision.target.scope.kind !== 'file') continue;
    const path = revision.target.scope.path;
    const entry = index.get(path) ?? { current: null, newest: null };
    if (newerRevision(revision, entry.newest)) entry.newest = revision;
    if (revision.snapshotId === snapshotId && newerRevision(revision, entry.current)) entry.current = revision;
    index.set(path, entry);
  }
  return index;
}

export function buildCoverageRows(
  inventory: InventoryRow[],
  plan: CoveragePlan,
  revisions: ChapterRevision[] = [],
): CoverageRow[] {
  const groupsByPath = new Map<string, CoverageGroup>();
  for (const group of plan.groups) {
    for (const path of group.paths) {
      if (!groupsByPath.has(path)) groupsByPath.set(path, group);
    }
  }
  const exclusionsByPath = new Map(plan.exclusions.map((exclusion) => [exclusion.path, exclusion]));
  const revisionIndex = revisionsByPath(plan.projectId, plan.snapshotId, revisions);

  return inventory.map((row) => {
    const group = groupsByPath.get(row.path) ?? null;
    const userExclusion = exclusionsByPath.get(row.path) ?? null;
    // Coverage remains path-specific. A manually named group is organization metadata;
    // its revision must never imply that each member has an individual explanation.
    const indexedRevision = revisionIndex.get(row.path);
    const revision = isEligibleForChapter(row) ? indexedRevision?.current ?? indexedRevision?.newest ?? null : null;
    let status: CoverageStatus;
    let reason: string | null = null;

    if (!isEligibleForChapter(row)) {
      status = row.disposition === 'captured' && row.parserReadiness === 'unsupported' && row.contentKind === 'text'
        ? 'unsupported'
        : 'unavailable';
      reason = row.disposition !== 'captured'
        ? `${inventoryReasonLabel(row.reason)}. This path has no captured source bytes.`
        : row.contentKind === 'binary'
          ? 'Binary content cannot be explained as source text.'
          : row.contentKind === 'invalidEncoding'
            ? 'The captured bytes are not valid text in the supported encoding.'
            : row.kind !== 'file'
              ? 'This inventory entry is not a captured file.'
              : row.details;
    } else if (userExclusion) {
      status = 'excluded';
      reason = userExclusion.reason;
    } else if (revision) {
      status = revision.snapshotId !== plan.snapshotId ? 'stale' : revision.lifecycle;
    } else if (row.parserReadiness === 'unsupported') {
      status = 'unsupported';
      reason = `Text-only support for ${row.language}: captured source and exact citations remain available, but the local indexer does not extract symbols or relationships.`;
    } else {
      status = 'missing';
    }

    return {
      inventory: row,
      status,
      reason,
      targetId: isEligibleForChapter(row) ? revision?.target.targetId ?? null : null,
      revisionId: revision?.revisionId ?? null,
      revisionNumber: revision?.revisionNumber ?? null,
      group,
      userExclusion,
      canOpenSource: isLocallyReadable(row),
      canSelectForGroup: isEligibleForChapter(row) && !group && !userExclusion && !revision,
      canExclude: isEligibleForChapter(row) && !group && !userExclusion && !revision,
      searchText: [row.path, row.reason, row.details, group?.name, group?.rationale, userExclusion?.reason, reason]
        .filter(Boolean).join('\n').toLowerCase(),
    };
  });
}

export function summarizeCoverage(rows: CoverageRow[]): CoverageTotals {
  const totals: CoverageTotals = {
    total: rows.length,
    missing: 0,
    draft: 0,
    rejected: 0,
    stale: 0,
    accepted: 0,
    excluded: 0,
    unsupported: 0,
    unavailable: 0,
  };
  for (const row of rows) totals[row.status] += 1;
  return totals;
}

export function filterCoverageRows(
  rows: CoverageRow[],
  query: string,
  status: 'all' | CoverageStatus = 'all',
): CoverageRow[] {
  const normalized = query.trim().toLowerCase();
  return rows.filter((row) => (status === 'all' || row.status === status)
    && (!normalized || row.searchText.includes(normalized)));
}

export function paginateCoverage<T>(rows: T[], requestedPage: number, pageSize = 50): CoveragePage<T> {
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const page = Math.min(Math.max(0, requestedPage), pageCount - 1);
  const offset = page * pageSize;
  const visible = rows.slice(offset, offset + pageSize);
  return {
    rows: visible,
    total: rows.length,
    page,
    pageSize,
    pageCount,
    firstVisible: rows.length === 0 ? 0 : offset + 1,
    lastVisible: offset + visible.length,
  };
}

export function addCoverageGroup(
  plan: CoveragePlan,
  group: CoverageGroup,
  inventory: InventoryRow[],
  revisions: ChapterRevision[] = [],
): CoveragePlan {
  const rows = buildCoverageRows(inventory, plan, revisions);
  const eligiblePaths = new Set(rows.filter((row) => row.canSelectForGroup).map((row) => row.inventory.path));
  const paths = [...new Set(group.paths)].sort();
  const name = group.name.trim();
  const rationale = group.rationale.trim();
  if (!group.groupId.trim()) throw new Error('A group identifier is required.');
  if (!name) throw new Error('A group name is required.');
  if (!rationale) throw new Error('Explain why these files belong together.');
  if (paths.length < 2) throw new Error('Select at least two independent files to create a group.');
  if (paths.some((path) => !eligiblePaths.has(path))) throw new Error('Groups can include only captured files without an existing chapter, group, or exclusion.');
  return { ...plan, groups: [...plan.groups, { ...group, name, rationale, paths }] };
}

export function removeCoverageGroup(plan: CoveragePlan, groupId: string): CoveragePlan {
  return { ...plan, groups: plan.groups.filter((group) => group.groupId !== groupId) };
}

export function excludeCoveragePath(plan: CoveragePlan, row: CoverageRow, reason: string, excludedAtUnixMs = Date.now()): CoveragePlan {
  const trimmed = reason.trim();
  if (!trimmed) throw new Error('A reason is required to exclude this file from the guide.');
  if (!row.canExclude) throw new Error('This path cannot be excluded while it has a group, chapter revision, or no captured source.');
  return {
    ...plan,
    exclusions: [...plan.exclusions.filter((item) => item.path !== row.inventory.path), {
      path: row.inventory.path,
      reason: trimmed,
      excludedAtUnixMs,
    }],
  };
}

export function restoreCoveragePath(plan: CoveragePlan, path: string): CoveragePlan {
  return { ...plan, exclusions: plan.exclusions.filter((item) => item.path !== path) };
}
