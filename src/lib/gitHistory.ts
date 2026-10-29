export interface GitCommit { oid: string; parents: string[]; subject: string; author: string; timestamp: string; isHead: boolean }
export interface GitHistoryTransport {
  read: (input: { projectId: string; requestId: string }) => Promise<GitCommit[]>;
  cancel: (requestId: string) => Promise<unknown>;
}

/** One caller owns one native request. Unmount/Stop cancels only that identity;
 * a late IPC result cannot become the caller's current history. */
export async function readGitHistory(transport: GitHistoryTransport, projectId: string, signal?: AbortSignal,
  newId: () => string = () => crypto.randomUUID()): Promise<GitCommit[]> {
  const stopped = () => new DOMException('Git history stopped.', 'AbortError');
  if (signal?.aborted) throw stopped();
  const requestId = newId();
  let dispatched = false;
  const cancel = () => { if (dispatched) void transport.cancel(requestId).catch(() => {}); };
  signal?.addEventListener('abort', cancel, { once: true });
  try {
    if (signal?.aborted) throw stopped();
    dispatched = true;
    const result = await transport.read({ projectId, requestId });
    if (signal?.aborted) throw stopped();
    return result;
  } catch (error) {
    if (signal?.aborted) throw stopped();
    throw error;
  } finally {
    signal?.removeEventListener('abort', cancel);
  }
}
