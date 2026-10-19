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

export const COVERAGE_STATUSES: CoverageStatus[] = [
  'missing', 'draft', 'rejected', 'stale', 'accepted', 'excluded', 'unsupported', 'unavailable',
];

export function emptyCoveragePlan(projectId: string, snapshotId: string): CoveragePlan {
  return { schemaVersion: 1, projectId, snapshotId, groups: [], exclusions: [] };
}

export function coverageStorageKey(projectId: string, snapshotId: string): string {
  return `code-atlas:coverage:v1:${encodeURIComponent(projectId)}:${encodeURIComponent(snapshotId)}`;
}

export function loadCoveragePlan(projectId: string, snapshotId: string, storage?: Pick<Storage, 'getItem'>): CoveragePlan {
  const empty = emptyCoveragePlan(projectId, snapshotId);
  try {
    const saved = (storage ?? window.localStorage).getItem(coverageStorageKey(projectId, snapshotId));
    if (!saved) return empty;
    const parsed: unknown = JSON.parse(saved);
    if (!isCoveragePlan(parsed) || parsed.projectId !== projectId || parsed.snapshotId !== snapshotId) return empty;
    return parsed;
  } catch {
    return empty;
  }
}

export function saveCoveragePlan(plan: CoveragePlan, storage?: Pick<Storage, 'setItem'>): boolean {
  try {
    (storage ?? window.localStorage).setItem(coverageStorageKey(plan.projectId, plan.snapshotId), JSON.stringify(plan));
    return true;
  } catch {
    return false;
  }
}

function isCoveragePlan(value: unknown): value is CoveragePlan {
  if (!value || typeof value !== 'object') return false;
  const plan = value as Partial<CoveragePlan>;
  return plan.schemaVersion === 1
    && typeof plan.projectId === 'string'
    && typeof plan.snapshotId === 'string'
    && Array.isArray(plan.groups)
    && plan.groups.every((group) => group && typeof group.groupId === 'string'
      && typeof group.name === 'string' && typeof group.rationale === 'string'
      && Array.isArray(group.paths) && group.paths.every((path) => typeof path === 'string')
      && typeof group.createdAtUnixMs === 'number')
    && Array.isArray(plan.exclusions)
    && plan.exclusions.every((exclusion) => exclusion && typeof exclusion.path === 'string'
      && typeof exclusion.reason === 'string' && typeof exclusion.excludedAtUnixMs === 'number');
}

function isLocallyReadable(row: InventoryRow): boolean {
  return row.disposition === 'captured' && row.kind === 'file'
    && (row.contentKind === 'text' || row.contentKind === 'unknown');
}

function isEligibleForChapter(row: InventoryRow): boolean {
  return isLocallyReadable(row);
}

function matchingRevision(
  row: InventoryRow,
  projectId: string,
  snapshotId: string,
  revisions: ChapterRevision[],
): ChapterRevision | null {
  const matching = revisions.filter((revision) => {
    if (revision.projectId !== projectId) return false;
    const scope = revision.target.scope;
    return scope.kind === 'file' && scope.path === row.path;
  });
  const current = matching.filter((revision) => revision.snapshotId === snapshotId);
  return (current.length > 0 ? current : matching)
    .sort((left, right) => right.revisionNumber - left.revisionNumber || right.createdAtUnixMs - left.createdAtUnixMs)[0] ?? null;
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

  return inventory.map((row) => {
    const group = groupsByPath.get(row.path) ?? null;
    const userExclusion = exclusionsByPath.get(row.path) ?? null;
    // Coverage remains path-specific. A manually named group is organization metadata;
    // its revision must never imply that each member has an individual explanation.
    const revision = isEligibleForChapter(row) ? matchingRevision(row, plan.projectId, plan.snapshotId, revisions) : null;
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
