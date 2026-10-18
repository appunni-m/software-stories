export type SnapshotHistoryLoad = { projectId: string; status: 'loading' | 'ready' | 'error' };
export type SnapshotManifestLoad = SnapshotHistoryLoad & { snapshotId: string };
export type WorkspaceReadiness = 'restoring' | 'loading-history' | 'loading-snapshot' | 'project-error' | 'history-error' | 'snapshot-error' | 'ready';

/** Wait for the selected local data, without depending on a provider connection. */
export function workspaceReadiness(input: {
  recentsPending: boolean;
  projectOpening: boolean;
  projectId: string | null;
  projectAvailable: boolean;
  snapshotId: string | null;
  history: SnapshotHistoryLoad | null;
  manifest: SnapshotManifestLoad | null;
}): WorkspaceReadiness {
  if (!input.projectId) return input.recentsPending || input.projectOpening ? 'restoring' : 'ready';
  if (!input.projectAvailable) return input.recentsPending ? 'restoring' : 'project-error';
  if (input.snapshotId) {
    const load = input.manifest;
    if (!load || load.projectId !== input.projectId || load.snapshotId !== input.snapshotId || load.status === 'loading') return 'loading-snapshot';
    return load.status === 'error' ? 'snapshot-error' : 'ready';
  }
  const load = input.history;
  if (!load || load.projectId !== input.projectId || load.status === 'loading') return 'loading-history';
  return load.status === 'error' ? 'history-error' : 'ready';
}
