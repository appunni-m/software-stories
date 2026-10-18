import { useSyncExternalStore } from 'react';
import { DEFAULT_VOICE_CONNECTION, parseVoiceConnection, VOICE_CONNECTION_KEY, type VoiceConnection } from './voiceConnection';

let issue: string | null = null;
function readSaved() {
  try { issue = null; return parseVoiceConnection(localStorage.getItem(VOICE_CONNECTION_KEY)); }
  catch (error) { issue = error instanceof Error ? error.message : 'Saved voice settings are unsupported. Choose a voice engine in Connections.'; return { ...DEFAULT_VOICE_CONNECTION }; }
}
let current = readSaved();
const listeners = new Set<() => void>();
export function readVoiceConnection() { return current; }
export function readVoiceConnectionIssue() { return issue; }
export function useVoiceConnection() {
  return useSyncExternalStore((listener) => { listeners.add(listener); return () => listeners.delete(listener); }, readVoiceConnection);
}
export function saveVoiceConnection(value: VoiceConnection) {
  const next = parseVoiceConnection(JSON.stringify(value));
  // A failed save must not claim that the connection will survive a restart.
  localStorage.setItem(VOICE_CONNECTION_KEY, JSON.stringify(next));
  current = next;
  issue = null;
  listeners.forEach((listener) => listener());
}
if (typeof window !== 'undefined') window.addEventListener('storage', (event) => {
  if (event.key !== VOICE_CONNECTION_KEY && event.key !== null) return;
  current = readSaved(); listeners.forEach((listener) => listener());
});
export function openVoiceConnections() { window.dispatchEvent(new Event('code-atlas:voice-connections')); }
