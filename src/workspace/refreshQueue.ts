/** Serialize local reads and collapse repeated invalidations into one fresh
 * read. A superseded result/error cannot replace newer saved workspace data.
 * Failures never schedule a retry; disposal prevents late UI callbacks. */
export interface RefreshQueue {
  refresh(): Promise<void>;
  dispose(): void;
}

export function createRefreshQueue<T>({ load, onValue, onError, onIdle }: {
  load: () => Promise<T>;
  onValue: (value: T) => void;
  onError: (error: unknown) => void;
  onIdle?: () => void;
}): RefreshQueue {
  let dirty = false;
  let disposed = false;
  let inFlight: Promise<void> | null = null;
  async function drain() {
    while (dirty && !disposed) {
      dirty = false;
      try {
        const value = await load();
        if (!disposed && !dirty) onValue(value);
      } catch (error) {
        if (!disposed && !dirty) onError(error);
      }
    }
  }
  function settle(): void | Promise<void> {
    // A notification can arrive after drain() returns but before its promise
    // settles. Keep the same flight alive until that invalidation is read.
    if (!disposed && dirty) return Promise.resolve().then(drain).finally(settle);
    inFlight = null;
    if (!disposed) onIdle?.();
  }
  return {
    refresh() {
      if (disposed) return Promise.resolve();
      dirty = true;
      if (!inFlight) {
        // Defer the first read so even a synchronous invalidation from load()
        // sees the already registered flight instead of starting another one.
        inFlight = Promise.resolve().then(drain).finally(settle);
      }
      return inFlight;
    },
    dispose() { disposed = true; dirty = false; },
  };
}
