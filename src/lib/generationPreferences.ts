/** Workspace preferences contain no credentials, source, or consent. */
export type GenerationBackend = 'codex' | 'claude-setup' | 'handoff' | 'api';
export type RepositoryPlanningBackend = 'codex' | 'claude-agent';
const key = 'code-atlas:generation-backend';
const directApiSelectionKey = 'code-atlas:direct-api-selection';

export interface DirectApiSelection {
  providerId: string;
  modelId: string;
}

export type DirectApiSelectionIssue = 'invalid' | 'unavailable';
let directApiSelectionIssue: DirectApiSelectionIssue | null = null;

function validSelectionId(value: unknown): value is string {
  return typeof value === 'string' && value.length <= 256
    && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

export function readDirectApiSelection(): DirectApiSelection {
  try {
    const raw = localStorage.getItem(directApiSelectionKey);
    if (raw === null) { directApiSelectionIssue = null; return { providerId: '', modelId: '' }; }
    let saved: unknown;
    try { saved = JSON.parse(raw); }
    catch { directApiSelectionIssue = 'invalid'; return { providerId: '', modelId: '' }; }
    if (!saved || typeof saved !== 'object' || Array.isArray(saved)) {
      directApiSelectionIssue = 'invalid';
      return { providerId: '', modelId: '' };
    }
    const record = saved as Record<string, unknown>;
    if (Object.keys(record).length !== 2 || !Object.hasOwn(record, 'providerId') || !Object.hasOwn(record, 'modelId')
      || !validSelectionId(record.providerId) || !validSelectionId(record.modelId)
      || record.modelId.length > 0 && record.providerId.length === 0) {
      directApiSelectionIssue = 'invalid';
      return { providerId: '', modelId: '' };
    }
    directApiSelectionIssue = null;
    return {
      providerId: record.providerId,
      modelId: record.modelId,
    };
  } catch { directApiSelectionIssue = 'unavailable'; }
  return { providerId: '', modelId: '' };
}

export function readDirectApiSelectionIssue(): DirectApiSelectionIssue | null {
  return directApiSelectionIssue;
}

export function rememberDirectApiSelection(selection: DirectApiSelection): void {
  if (!validSelectionId(selection.providerId) || !validSelectionId(selection.modelId)
    || selection.modelId.length > 0 && selection.providerId.length === 0) return;
  const saved = { providerId: selection.providerId, modelId: selection.modelId };
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
