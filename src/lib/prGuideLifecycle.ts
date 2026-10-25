export type PrGuideOperationScope = {
  projectId: string;
  baseRef: string;
  headRef: string;
  comparisonId: string;
};

export type PrGuideOperationKind = 'build' | 'send' | 'cancel' | 'decision';

export type PrGuideOperationToken = {
  kind: PrGuideOperationKind;
  id: number;
  scopeKey: string;
};

function scopeKey(scope: PrGuideOperationScope): string {
  return JSON.stringify([scope.projectId, scope.baseRef, scope.headRef, scope.comparisonId]);
}

/** Fences async PR actions to one mounted comparison and prevents duplicate actions. */
export class PrGuideOperationFence {
  private currentScope: string;
  private mounted = true;
  private sequence = 0;
  private readonly active = new Map<PrGuideOperationKind, number>();

  constructor(scope: PrGuideOperationScope) {
    this.currentScope = scopeKey(scope);
  }

  setScope(scope: PrGuideOperationScope): void {
    const nextScope = scopeKey(scope);
    if (this.currentScope === nextScope) return;
    this.currentScope = nextScope;
    this.active.clear();
  }

  mount(scope: PrGuideOperationScope): void {
    this.mounted = true;
    this.setScope(scope);
  }

  unmount(): void {
    this.mounted = false;
    this.active.clear();
  }

  begin(kind: PrGuideOperationKind): PrGuideOperationToken | null {
    if (!this.mounted || this.active.has(kind)) return null;
    const token = { kind, id: ++this.sequence, scopeKey: this.currentScope };
    this.active.set(kind, token.id);
    return token;
  }

  isCurrent(token: PrGuideOperationToken): boolean {
    return this.mounted
      && token.scopeKey === this.currentScope
      && this.active.get(token.kind) === token.id;
  }

  finish(token: PrGuideOperationToken): void {
    if (this.isCurrent(token)) this.active.delete(token.kind);
  }

  invalidate(kind: PrGuideOperationKind): void {
    this.active.delete(kind);
  }

  isActive(kind: PrGuideOperationKind): boolean {
    return this.active.has(kind);
  }
}
