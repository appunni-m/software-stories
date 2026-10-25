export interface SavedFileLocation { snapshotId: string; path?: string }

function isRelativeCapturedPath(value: unknown): value is string {
  return typeof value === 'string'
    && value.length > 0
    && value.length <= 4096
    && !value.startsWith('/')
    && !/[\u0000-\u001f\u007f]/u.test(value)
    && value.split('/').every((part) => part.length > 0 && part !== '.' && part !== '..');
}

export function readFileLocation(projectId: string): SavedFileLocation | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(`code-atlas:last-file:${projectId}`) ?? 'null');
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
    const record = value as Record<string, unknown>;
    const keys = Object.keys(record);
    if (keys.some((key) => key !== 'snapshotId' && key !== 'path')
      || !Object.hasOwn(record, 'snapshotId')
      || typeof record.snapshotId !== 'string'
      || record.snapshotId.trim().length === 0
      || record.snapshotId.length > 256) return null;
    if (Object.hasOwn(record, 'path')) {
      if (!isRelativeCapturedPath(record.path)) return null;
      return { snapshotId: record.snapshotId, path: record.path };
    }
    return { snapshotId: record.snapshotId };
  } catch { /* Optional UI state never blocks repository browsing. */ }
  return null;
}

export function rememberFileLocation(projectId: string, location: SavedFileLocation): void {
  try { localStorage.setItem(`code-atlas:last-file:${projectId}`, JSON.stringify(location)); } catch { /* Optional. */ }
}
