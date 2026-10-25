import { useSyncExternalStore } from 'react';
import { getCodexAgentStatus, resumeCodexConnection, listCodexModels, isNativeRuntime, type CodexAgentStatus, type CodexModel } from './commands';
import { restoreCodexConnection } from './codexConnection';

export const codexPreferenceKey = 'code-atlas:codex-chapter-preferences';
export function readCodexSelection(): { modelId: string; effort: string } {
  try {
    const serialized = localStorage.getItem(codexPreferenceKey);
    if (serialized === null) return { modelId: 'gpt-6-luna', effort: 'max' };
    const value: unknown = JSON.parse(serialized);
    if (!value || typeof value !== 'object' || Array.isArray(value)) return { modelId: '', effort: '' };
    const record = value as Record<string, unknown>;
    if (Object.keys(record).length !== 2 || !Object.hasOwn(record, 'modelId') || !Object.hasOwn(record, 'effort')
      || typeof record.modelId !== 'string' || !record.modelId.trim() || record.modelId.length > 256
      || /[\u0000-\u001f\u007f-\u009f]/u.test(record.modelId)
      || typeof record.effort !== 'string' || !record.effort.trim() || record.effort.length > 32
      || /[\u0000-\u001f\u007f-\u009f]/u.test(record.effort)) return { modelId: '', effort: '' };
    return { modelId: record.modelId, effort: record.effort };
  } catch { /* An unreadable saved selection must not choose another model. */ }
  return { modelId: '', effort: '' };
}
export function rememberCodexSelection(modelId: string, effort: string) {
  if (!modelId.trim() || modelId.length > 256 || /[\u0000-\u001f\u007f-\u009f]/u.test(modelId)
    || !effort.trim() || effort.length > 32 || /[\u0000-\u001f\u007f-\u009f]/u.test(effort)) return;
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
