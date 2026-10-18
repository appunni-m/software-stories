/** Workspace preferences contain no credentials, source, or consent. */
export type GenerationBackend = 'codex' | 'claude-setup' | 'handoff' | 'api';
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

export function readGenerationBackend(): GenerationBackend {
  try {
    const saved = localStorage.getItem(key);
    if (saved === 'codex' || saved === 'claude-setup' || saved === 'handoff' || saved === 'api') return saved;
  } catch { /* Storage is optional. */ }
  return 'codex';
}

export function rememberGenerationBackend(backend: GenerationBackend): void {
  try { localStorage.setItem(key, backend); } catch { /* The current session still works. */ }
  if (typeof window !== 'undefined') window.dispatchEvent(new Event('code-atlas:connection-changed'));
}
