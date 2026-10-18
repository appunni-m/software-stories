export interface CloseParticipant {
  /** Capture current DOM/media state before any store stops accepting edits. */
  capture?: () => void;
  freeze?: () => () => void;
  /** Later stages can persist acknowledgments produced by earlier writers. */
  flushOrder?: number;
  flush: () => Promise<void>;
}

/** A close is a transaction: capture, stop new edits, then finish local writes.
 * Cancellation releases the reader even if an old filesystem call is stuck. */
export class CloseCoordinator {
  private participants = new Set<CloseParticipant>();
  register(participant: CloseParticipant): () => void {
    this.participants.add(participant);
    return () => { this.participants.delete(participant); };
  }
  async prepare(signal: AbortSignal): Promise<() => void> {
    const participants = [...this.participants];
    const releases: (() => void)[] = [];
    let released = false;
    const release = () => {
      if (released) return;
      released = true;
      signal.removeEventListener('abort', release);
      // One failing subscriber must not leave other readers frozen.
      for (const resume of releases.reverse()) {try {resume();} catch {/* Keep releasing the other stores. */}}
    };
    let abort!: () => void;
    const cancelled = new Promise<never>((_, reject) => {
      abort = () => { release(); reject(new Error('Closing was cancelled.')); };
    });
    signal.addEventListener('abort', abort, {once:true});
    try {
      if (signal.aborted) throw new Error('Closing was cancelled.');
      for (const participant of participants) participant.capture?.();
      for (const participant of participants) {
        const resume = participant.freeze?.();
        if (resume) releases.push(resume);
      }
      const writes = async () => {
        const orders = [...new Set(participants.map(participant=>participant.flushOrder??0))].sort((a,b)=>a-b);
        for(const order of orders){
          if(signal.aborted)throw new Error('Closing was cancelled.');
          await Promise.all(participants.filter(participant=>(participant.flushOrder??0)===order).map(participant=>participant.flush()));
        }
      };
      await Promise.race([writes(), cancelled]);
      if (signal.aborted) throw new Error('Closing was cancelled.');
      // Keep the stores frozen until native close succeeds or the caller cancels.
      signal.removeEventListener('abort', abort);
      signal.addEventListener('abort', release, {once:true});
      return release;
    } catch (error) {
      signal.removeEventListener('abort', abort);
      release();
      throw error;
    }
  }
}

interface CloseIO {
  complete: (requestId: string, saved: boolean) => Promise<void>;
  busy: (value: boolean) => void;
  failed: () => void;
}
const requestId = (value: unknown): value is string => typeof value === 'string'
  && /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(value);

/** Native owns the close intent and deadline. The renderer can acknowledge only
 * the currently issued request, after every registered local writer settles. */
export class CloseRequestController {
  private current: {id:string;abort:AbortController;release?:()=>void;task:Promise<void>} | null = null;
  private coordinator: CloseCoordinator;
  private io: CloseIO;
  constructor(coordinator: CloseCoordinator, io: CloseIO) {this.coordinator=coordinator;this.io=io;}
  request(id: unknown): Promise<void> {
    if (!requestId(id)) return Promise.resolve();
    if (this.current?.id === id) return this.current.task;
    if (this.current) this.cancel(this.current.id, false);
    const entry = {id,abort:new AbortController(),release:undefined as (()=>void)|undefined,task:Promise.resolve()};
    this.current = entry;
    this.io.busy(true);
    entry.task = (async () => {
      try {
        entry.release = await this.coordinator.prepare(entry.abort.signal);
        if (this.current !== entry || entry.abort.signal.aborted) { entry.release(); return; }
        await this.io.complete(id, true);
      } catch {
        if (this.current !== entry) return;
        this.current = null;
        entry.abort.abort(); entry.release?.();
        this.io.busy(false);
        try { await this.io.complete(id, false); } catch { /* Native keeps the window open on a stale/failed acknowledgment. */ }
        this.io.failed();
      }
    })();
    return entry.task;
  }
  cancel(id: unknown, report = true): void {
    if (!requestId(id) || this.current?.id !== id) return;
    const entry = this.current; this.current = null;
    entry.abort.abort(); entry.release?.();
    this.io.busy(false);
    if (report) this.io.failed();
  }
}

export const appCloseCoordinator = new CloseCoordinator();
