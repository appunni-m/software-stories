import { useSyncExternalStore } from 'react';
import { getCodexAgentStatus, resumeCodexConnection, listCodexModels, isNativeRuntime, type CodexAgentStatus, type CodexModel } from './commands';
import { restoreCodexConnection } from './codexConnection';

export const codexPreferenceKey = 'code-atlas:codex-chapter-preferences';
export function readCodexSelection(): { modelId: string; effort: string } {
  try {
    const value = JSON.parse(localStorage.getItem(codexPreferenceKey) ?? 'null');
    if (typeof value?.modelId === 'string' && typeof value?.effort === 'string') return value;
  } catch { /* Optional preference. */ }
  return { modelId: 'gpt-6-luna', effort: 'max' };
}
export function rememberCodexSelection(modelId: string, effort: string) {
  try { localStorage.setItem(codexPreferenceKey, JSON.stringify({ modelId, effort })); } catch { /* Optional. */ }
  publish({ selection: { modelId, effort } });
}
interface ConnectionState { status: CodexAgentStatus | null; models: CodexModel[]; checking: boolean; error: string | null; selection: { modelId: string; effort: string } }
let state: ConnectionState = { status: null, models: [], checking: false, error: null, selection: readCodexSelection() };
const listeners = new Set<() => void>();
let pending: Promise<{ status: CodexAgentStatus; models: CodexModel[] }> | null = null;
function publish(update: Partial<ConnectionState>) { state = { ...state, ...update }; listeners.forEach((listener) => listener()); }
export function publishCodexStatus(status: CodexAgentStatus) { publish({ status, error: null }); }
export function publishCodexModels(models: CodexModel[]) { publish({ models }); }
export function useConnectionState() {
  return useSyncExternalStore((listener) => { listeners.add(listener); return () => listeners.delete(listener); }, () => state);
}
export async function ensureCodexConnection(refresh = false) {
  if (pending) return pending;
  if (!refresh && state.status?.authenticated && state.status.runtimeRunning && state.models.length) return { status: state.status, models: state.models };
  publish({ checking: true, error: null });
  pending = restoreCodexConnection({ status: getCodexAgentStatus, resume: resumeCodexConnection, models: listCodexModels, onStatus: publishCodexStatus })
    .then((connection) => { publish({ ...connection, checking: false }); return connection; })
    .catch((error) => { publish({ checking: false, error: 'Connection needs attention' }); throw error; })
    .finally(() => { pending = null; });
  return pending;
}
export function initializeConnection() { if (isNativeRuntime()) void ensureCodexConnection().catch(() => {}); }
