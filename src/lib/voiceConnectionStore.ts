import { useSyncExternalStore } from 'react';
import { parseVoiceConnection, VOICE_CONNECTION_KEY, type VoiceConnection } from './voiceConnection';

function readSaved() {
  try { return parseVoiceConnection(localStorage.getItem(VOICE_CONNECTION_KEY)); }
  catch { return parseVoiceConnection(null); }
}
let current = readSaved();
const listeners = new Set<() => void>();
export function readVoiceConnection() { return current; }
export function useVoiceConnection() {
  return useSyncExternalStore((listener) => { listeners.add(listener); return () => listeners.delete(listener); }, readVoiceConnection);
}
export function saveVoiceConnection(value: VoiceConnection) {
  const next = parseVoiceConnection(JSON.stringify(value));
  // A failed save must not claim that the connection will survive a restart.
  localStorage.setItem(VOICE_CONNECTION_KEY, JSON.stringify(next));
  current = next;
  listeners.forEach((listener) => listener());
}
if (typeof window !== 'undefined') window.addEventListener('storage', (event) => {
  if (event.key !== VOICE_CONNECTION_KEY && event.key !== null) return;
  current = readSaved(); listeners.forEach((listener) => listener());
});
export function openVoiceConnections() { window.dispatchEvent(new Event('code-atlas:voice-connections')); }
