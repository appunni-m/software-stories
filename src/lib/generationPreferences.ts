export type GenerationBackend = 'codex';
const key = 'code-atlas:generation-backend';
let sessionGenerationBackend: GenerationBackend | null = null;
let generationBackendWriteFailed = false;

export function readGenerationBackend(): GenerationBackend | null {
  try {
    const saved = localStorage.getItem(key);
    if (saved === null) return sessionGenerationBackend ?? 'codex';
    if (saved === 'codex' && !generationBackendWriteFailed) return 'codex';
    if (generationBackendWriteFailed) return sessionGenerationBackend;
    return null;
  } catch {
    // An unreadable saved selection must not silently choose another backend.
    return sessionGenerationBackend;
  }
}

export function rememberGenerationBackend(backend: GenerationBackend): void {
  // The first release persists only the user's connected Codex account.
  if (backend !== 'codex') return;
  sessionGenerationBackend = 'codex';
  try {
    localStorage.setItem(key, 'codex');
    generationBackendWriteFailed = false;
  } catch {
    generationBackendWriteFailed = true;
    // The current session still works.
  }
  if (typeof window !== 'undefined') window.dispatchEvent(new Event('code-atlas:connection-changed'));
}
