/** Workspace preferences contain no credentials, source, or consent. */
export type GenerationBackend = 'codex' | 'claude-setup' | 'handoff' | 'api';
const key = 'code-atlas:generation-backend';

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
