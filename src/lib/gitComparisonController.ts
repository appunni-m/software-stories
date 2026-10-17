import type { GitComparisonResult } from './commands';

type Input = { requestId: string; projectId: string; baseRef: string; headRef: string };
type Transport = {
  compare: (input: Input) => Promise<GitComparisonResult>;
  cancel: (requestId: string) => Promise<unknown>;
};
type Request = { id: string; controller: AbortController; dispatched: boolean };

function stopped(): DOMException {
  return new DOMException('Comparison stopped.', 'AbortError');
}

/** One current comparison per repository; late results never replace it. */
export class GitComparisonController {
  private readonly requests = new Map<string, Request>();
  private readonly listeners = new Set<() => void>();
  private readonly transport: Transport;
  private readonly newId: () => string;

  constructor(
    transport: Transport,
    newId: () => string = () => crypto.randomUUID(),
  ) {
    this.transport = transport;
    this.newId = newId;
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  isBusy(projectId: string | null): boolean {
    return projectId !== null && this.requests.has(projectId);
  }

  cancel(projectId: string): void {
    this.requests.get(projectId)?.controller.abort();
  }

  private changed(): void {
    for (const listener of this.listeners) listener();
  }

  async compare(projectId: string, baseRef: string, headRef: string, signal?: AbortSignal): Promise<GitComparisonResult> {
    if (signal?.aborted) throw stopped();
    this.cancel(projectId);
    const request: Request = { id: this.newId(), controller: new AbortController(), dispatched: false };
    const onAbort = () => {
      if (request.dispatched) void this.transport.cancel(request.id).catch(() => {});
      if (this.requests.get(projectId) === request) {
        this.requests.delete(projectId);
        this.changed();
      }
    };
    const onExternalAbort = () => request.controller.abort();
    request.controller.signal.addEventListener('abort', onAbort, { once: true });
    signal?.addEventListener('abort', onExternalAbort, { once: true });
    this.requests.set(projectId, request);
    try {
      this.changed();
      if (request.controller.signal.aborted) throw stopped();
      request.dispatched = true;
      const result = await this.transport.compare({ requestId: request.id, projectId, baseRef, headRef });
      if (request.controller.signal.aborted || this.requests.get(projectId) !== request) throw stopped();
      return result;
    } catch (error) {
      if (request.controller.signal.aborted) throw stopped();
      throw error;
    } finally {
      signal?.removeEventListener('abort', onExternalAbort);
      request.controller.signal.removeEventListener('abort', onAbort);
      if (this.requests.get(projectId) === request) {
        this.requests.delete(projectId);
        this.changed();
      }
    }
  }
}
