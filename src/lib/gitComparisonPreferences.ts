export interface GitComparisonPreference {
  baseRef: string;
  headRef: string;
}

export interface GitComparisonRoute {
  workspace: string;
  projectId: string | null;
  baseRef?: string;
  headRef?: string;
}

const STORAGE_PREFIX = 'code-atlas:git-comparison:';
const MAX_REF_LENGTH = 1024;

function storageKey(projectId: string): string | null {
  const id = projectId.trim();
  return id && id.length <= 512 ? `${STORAGE_PREFIX}${encodeURIComponent(id)}` : null;
}

function validRef(value: unknown): value is string {
  return typeof value === 'string'
    && value.trim().length > 0
    && value.length <= MAX_REF_LENGTH
    && !/[\u0000-\u001f\u007f]/.test(value);
}

export function parseGitComparisonPreference(value: unknown): GitComparisonPreference | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const candidate = value as { baseRef?: unknown; headRef?: unknown };
  if (!validRef(candidate.baseRef) || !validRef(candidate.headRef)) return null;
  return { baseRef: candidate.baseRef.trim(), headRef: candidate.headRef.trim() };
}

export function restoreGitComparisonPreference<T extends GitComparisonRoute>(
  route: T,
  preference: GitComparisonPreference | null,
): T {
  if (route.workspace !== 'changes' || !route.projectId || !preference
    || route.baseRef?.trim() || route.headRef?.trim()) return route;
  return { ...route, ...preference } as T;
}

export function readGitComparisonPreference(projectId: string): GitComparisonPreference | null {
  const key = storageKey(projectId);
  if (!key) return null;
  try {
    const serialized = localStorage.getItem(key);
    if (!serialized || serialized.length > MAX_REF_LENGTH * 2 + 128) return null;
    return parseGitComparisonPreference(JSON.parse(serialized));
  } catch {
    return null;
  }
}

export function rememberGitComparisonPreference(
  projectId: string,
  baseRef: string,
  headRef: string,
): boolean {
  const key = storageKey(projectId);
  const preference = parseGitComparisonPreference({ baseRef, headRef });
  if (!key || !preference) return false;
  try {
    localStorage.setItem(key, JSON.stringify(preference));
    return true;
  } catch {
    return false;
  }
}
