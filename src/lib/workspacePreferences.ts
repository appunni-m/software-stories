export interface SavedFileLocation { snapshotId: string; path?: string }

export function readFileLocation(projectId: string): SavedFileLocation | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(`code-atlas:last-file:${projectId}`) ?? 'null');
    if (value && typeof value === 'object' && 'snapshotId' in value && typeof value.snapshotId === 'string') {
      return { snapshotId: value.snapshotId, path: 'path' in value && typeof value.path === 'string' ? value.path : undefined };
    }
  } catch { /* Optional UI state never blocks repository browsing. */ }
  return null;
}

export function rememberFileLocation(projectId: string, location: SavedFileLocation): void {
  try { localStorage.setItem(`code-atlas:last-file:${projectId}`, JSON.stringify(location)); } catch { /* Optional. */ }
}
