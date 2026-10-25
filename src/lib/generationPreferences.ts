/** Workspace preferences contain no credentials, source, or consent. */
export type GenerationBackend = 'codex' | 'claude-setup' | 'handoff' | 'api';
export type RepositoryPlanningBackend = 'codex' | 'claude-agent';
const key = 'code-atlas:generation-backend';
const directApiSelectionKey = 'code-atlas:direct-api-selection';

export interface DirectApiSelection {
  providerId: string;
  modelId: string;
}

export function readDirectApiSelection(): DirectApiSelection {
  try {
    const raw = localStorage.getItem(directApiSelectionKey);
    if (!raw) return { providerId: '', modelId: '' };
    const saved: unknown = JSON.parse(raw);
    if (!saved || typeof saved !== 'object') return { providerId: '', modelId: '' };
    const record = saved as Record<string, unknown>;
    return {
      providerId: typeof record.providerId === 'string' && record.providerId.length <= 256 ? record.providerId : '',
      modelId: typeof record.modelId === 'string' && record.modelId.length <= 256 ? record.modelId : '',
    };
  } catch { /* Storage is optional. */ }
  return { providerId: '', modelId: '' };
}

export function rememberDirectApiSelection(selection: DirectApiSelection): void {
  const saved = {
    providerId: selection.providerId.slice(0, 256),
    modelId: selection.modelId.slice(0, 256),
  };
  try { localStorage.setItem(directApiSelectionKey, JSON.stringify(saved)); } catch { /* The current session still works. */ }
  if (typeof window !== 'undefined') window.dispatchEvent(new Event('code-atlas:direct-api-selection-changed'));
}

export function readGenerationBackend(): GenerationBackend | null {
  try {
    const saved = localStorage.getItem(key);
    if (saved === null) return 'codex';
    if (saved === 'codex' || saved === 'claude-setup' || saved === 'handoff' || saved === 'api') return saved;
  } catch { /* An unreadable saved selection must not silently choose another backend. */ }
  return null;
}

export function rememberGenerationBackend(backend: GenerationBackend): void {
  try { localStorage.setItem(key, backend); } catch { /* The current session still works. */ }
  if (typeof window !== 'undefined') window.dispatchEvent(new Event('code-atlas:connection-changed'));
}

/** Repository plans can only use the connected coding agents. Never silently
 * turn a saved API or manual-handoff preference into a Codex request. */
export function repositoryPlanningBackend(backend: GenerationBackend | RepositoryPlanningBackend | null | undefined): RepositoryPlanningBackend | null {
  if (backend === 'codex') return 'codex';
  if (backend === 'claude-setup' || backend === 'claude-agent') return 'claude-agent';
  return null;
}
