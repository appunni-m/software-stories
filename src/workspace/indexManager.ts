import { getSnapshotIndexProgress, loadSnapshotIndex, startSnapshotIndexing, type SnapshotIndex } from '../lib/commands';

const indexLoads = new Map<string, Promise<SnapshotIndex>>();

/** Share one local index build per snapshot between the reader and discovery. */
export function ensureWorkspaceIndex(project: string, snapshot: string): Promise<SnapshotIndex> {
  const id = JSON.stringify([project, snapshot]);
  const existing = indexLoads.get(id);
  if (existing) return existing;
  const pending = (async () => {
    const saved = await loadSnapshotIndex(project, snapshot);
    if (saved) return saved;
    const started = await startSnapshotIndexing(project, snapshot);
    let pollDelayMs = 200;
    for (;;) {
      const progress = await getSnapshotIndexProgress(started.jobId);
      if (!progress) throw new Error('The local code index could not be read. Try again.');
      if (progress.status !== 'running') {
        const result = await loadSnapshotIndex(project, snapshot);
        if (!result) throw new Error(progress.message || 'The local code index could not be built. Try again.');
        return result;
      }
      // Indexing is an owned local job, not an inference request with a
      // wall-clock budget. Back off instead of polling the native job store
      // at a fixed five calls per second indefinitely.
      await new Promise((resolve) => setTimeout(resolve, pollDelayMs));
      pollDelayMs = Math.min(1000, Math.ceil(pollDelayMs * 1.5));
    }
  })().finally(() => indexLoads.delete(id));
  indexLoads.set(id, pending);
  return pending;
}
