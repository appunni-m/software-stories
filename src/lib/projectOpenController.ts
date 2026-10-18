import type { ProjectOpenResult } from './commands';

type Destination = { type: 'picker' } | { type: 'recent'; projectId: string };
type Transport = {
  open: (input: Destination & { requestId: string }) => Promise<ProjectOpenResult>;
  cancel: (requestId: string) => Promise<unknown>;
};
type Request = { id: string; controller: AbortController; dispatched: boolean };
const stopped = () => new DOMException('Repository opening stopped.', 'AbortError');

/** One project-opening intent owns the app, across picker, Recents and recovery.
 * Cancellation releases UI state immediately; late results cannot navigate. */
export class ProjectOpenController {
  private current: Request | null = null;
  private readonly listeners = new Set<() => void>();
  private readonly transport: Transport;
  private readonly newId: () => string;
  constructor(transport: Transport, newId: () => string = () => crypto.randomUUID()) {
    this.transport = transport;
    this.newId = newId;
  }
  readonly isBusy = (): boolean => this.current !== null;
  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };
  readonly cancel = (): void => { this.current?.controller.abort(); };
  private changed(): void { for (const listener of this.listeners) listener(); }

  async open(destination: Destination, signal?: AbortSignal): Promise<ProjectOpenResult> {
    if (signal?.aborted) throw stopped();
    this.cancel();
    const request: Request = { id: this.newId(), controller: new AbortController(), dispatched: false };
    const abort = () => {
      if (request.dispatched) void this.transport.cancel(request.id).catch(() => {});
      if (this.current === request) { this.current = null; this.changed(); }
    };
    const externalAbort = () => request.controller.abort();
    request.controller.signal.addEventListener('abort', abort, { once: true });
    signal?.addEventListener('abort', externalAbort, { once: true });
    this.current = request;
    try {
      this.changed();
      if (request.controller.signal.aborted) throw stopped();
      request.dispatched = true;
      const result = await this.transport.open({ ...destination, requestId: request.id });
      if (request.controller.signal.aborted || this.current !== request) throw stopped();
      return result;
    } catch (error) {
      if (request.controller.signal.aborted) throw stopped();
      throw error;
    } finally {
      signal?.removeEventListener('abort', externalAbort);
      request.controller.signal.removeEventListener('abort', abort);
      if (this.current === request) { this.current = null; this.changed(); }
    }
  }
}
