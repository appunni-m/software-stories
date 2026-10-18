/** Local recovery only. Rejected output is never promoted, rendered as HTML,
 * submitted to a model or retried by this controller. */
export interface SavedAttemptEntry {
  id: string;
  reviewToken: string | null;
  bytes: number;
  projectId: string | null;
  snapshotId: string | null;
  startedAtUnixMs: number | null;
  candidateCount: number;
  available: boolean;
  question: string | null;
}
export interface SavedAttemptInventory {
  entries: SavedAttemptEntry[];
  limited: boolean;
  operationLimit: number;
  busy: boolean;
}
export interface SavedAttemptReview {
  entry: SavedAttemptEntry;
  attempts: { ordinal: number; responseText: string; rejection: string | null; findings: unknown[] }[];
}
export interface SavedAttemptsApi {
  list: () => Promise<SavedAttemptInventory>;
  read: (entryId: string, reviewToken: string) => Promise<SavedAttemptReview>;
  trash: (entryId: string, reviewToken: string) => Promise<void>;
}
export interface SavedAttemptsState {
  inventory: SavedAttemptInventory | null;
  loading: boolean;
  reviewing: boolean;
  removing: boolean;
  selectedId: string | null;
  review: SavedAttemptReview | null;
  requiresRefresh: boolean;
  error: string | null;
  message: string | null;
}

export class SavedAttempts {
  private state: SavedAttemptsState = { inventory: null, loading: false, reviewing: false, removing: false,
    selectedId: null, review: null, requiresRefresh: false, error: null, message: null };
  private version = 0;
  private alive = true;
  private frozen = false;
  private pendingRemoval: Promise<void> | null = null;
  private listeners = new Set<() => void>();
  private api: SavedAttemptsApi;
  constructor(api: SavedAttemptsApi) { this.api = api; }
  getSnapshot = (): SavedAttemptsState => this.state;
  subscribe = (listener: () => void): (() => void) => { this.listeners.add(listener); return () => this.listeners.delete(listener); };
  private publish(change: Partial<SavedAttemptsState>) {
    if (!this.alive) return;
    this.state = { ...this.state, ...change };
    this.listeners.forEach(listener => listener());
  }
  private owns(version: number): boolean { return this.alive && this.version === version; }
  private current(entry: SavedAttemptEntry): SavedAttemptEntry | undefined {
    if (!entry.reviewToken) return undefined;
    return this.state.inventory?.entries.find(item => item.id === entry.id && item.reviewToken === entry.reviewToken);
  }
  canRemove(entry: SavedAttemptEntry): boolean {
    return this.alive && !this.frozen && !!this.current(entry) && !this.state.inventory?.busy && !this.state.loading && !this.state.removing && !this.state.requiresRefresh;
  }
  async refresh(): Promise<void> {
    if (!this.alive || this.frozen || this.state.removing) return;
    const owner = ++this.version;
    this.publish({ loading: true, reviewing: false, selectedId: null, review: null, error: null, message: null });
    try {
      const inventory = await this.api.list();
      if (this.owns(owner)) this.publish({ inventory, requiresRefresh: false });
    } catch {
      if (this.owns(owner)) this.publish({ error: 'Saved attempts could not be loaded. Try Refresh.', requiresRefresh: true });
    } finally { if (this.owns(owner)) this.publish({ loading: false }); }
  }
  async select(entry: SavedAttemptEntry): Promise<void> {
    if (!this.alive || this.frozen || !this.current(entry) || this.state.loading || this.state.removing || this.state.requiresRefresh) return;
    const owner = ++this.version;
    this.publish({ selectedId: entry.id, review: null, reviewing: entry.available, error: null, message: null });
    if (!entry.available) return;
    try {
      const review = await this.api.read(entry.id, entry.reviewToken!);
      if (!this.owns(owner)) return;
      if (review.entry.id !== entry.id || review.entry.reviewToken !== entry.reviewToken) throw new Error('Changed identity');
      this.publish({ review });
    } catch {
      if (this.owns(owner)) this.publish({ error: 'This saved attempt could not be verified. Refresh before trying again.', requiresRefresh: true });
    } finally { if (this.owns(owner)) this.publish({ reviewing: false }); }
  }
  remove(entry: SavedAttemptEntry): Promise<void> {
    if (!this.canRemove(entry)) return Promise.resolve();
    const pending = this.removeOwned(entry);
    this.pendingRemoval = pending;
    void pending.finally(() => { if (this.pendingRemoval === pending) this.pendingRemoval = null; });
    return pending;
  }
  private async removeOwned(entry: SavedAttemptEntry): Promise<void> {
    const owner = ++this.version;
    this.publish({ removing: true, reviewing: false, error: null, message: null });
    try {
      await this.api.trash(entry.id, entry.reviewToken!);
    } catch {
      if (this.owns(owner)) this.publish({ removing: false, requiresRefresh: true,
        error: 'The move could not be confirmed. Refresh the list before trying again.' });
      return;
    }
    if (!this.owns(owner)) return;
    this.publish({ inventory: null, selectedId: null, review: null,
      message: 'Moved to Trash. You can recover the file in Finder. Saved explanations are unchanged.' });
    try {
      const inventory = await this.api.list();
      if (this.owns(owner)) this.publish({ inventory, requiresRefresh: false });
    } catch {
      if (this.owns(owner)) this.publish({ error: 'The move finished, but the list could not be refreshed. Try Refresh.', requiresRefresh: true });
    } finally { if (this.owns(owner)) this.publish({ removing: false }); }
  }
  beginClose(): () => void { this.frozen = true; return () => { this.frozen = false; }; }
  flush(): Promise<void> { return this.pendingRemoval ?? Promise.resolve(); }
  dispose(): void { this.alive = false; ++this.version; this.listeners.clear(); }
}
