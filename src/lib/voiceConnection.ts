import type { NarrationPreferencesSnapshot } from './narration';

export interface VoiceConnection {
  backend: 'pocket-tts' | 'gemini';
  googleVoice: string;
  googleStyle: string;
}
export const VOICE_CONNECTION_KEY = 'code-atlas:voice-connection:v2';
export const DEFAULT_VOICE_CONNECTION: VoiceConnection = { backend: 'pocket-tts', googleVoice: 'Kore', googleStyle: '' };

/** Global delivery settings never inherit a cloud choice from a saved section. */
export function parseVoiceConnection(serialized: string | null): VoiceConnection {
  if (serialized === null) return { ...DEFAULT_VOICE_CONNECTION };
  let value: unknown;
  try { value = JSON.parse(serialized); }
  catch { throw new Error('Saved voice settings are malformed. Choose a voice engine in Connections.'); }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Saved voice settings are malformed. Choose a voice engine in Connections.');
  }
  const record = value as Record<string, unknown>;
  if (Object.keys(record).length !== 3 || !['backend', 'googleVoice', 'googleStyle'].every(key => Object.hasOwn(record, key))) {
    throw new Error('Saved voice settings use an unsupported format. Choose a voice engine in Connections.');
  }
  if (record.backend !== 'pocket-tts' && record.backend !== 'gemini') {
    throw new Error('Saved voice settings use an unsupported voice engine. Choose Pocket TTS or Gemini in Connections.');
  }
  if (typeof record.googleVoice !== 'string' || !/^[A-Za-z]{1,40}$/u.test(record.googleVoice)
    || typeof record.googleStyle !== 'string' || new TextEncoder().encode(record.googleStyle).byteLength > 160) {
    throw new Error('Saved voice settings are malformed. Choose a supported voice in Connections.');
  }
  return { backend: record.backend, googleVoice: record.googleVoice, googleStyle: record.googleStyle };
}

/** Keep section text/pronunciation edits; Connections owns delivery for every section. */
export function applyVoiceConnection(section: NarrationPreferencesSnapshot, connection: VoiceConnection): NarrationPreferencesSnapshot {
  return { ...section, backend: connection.backend,
    voice: connection.backend === 'pocket-tts' ? 'alba' : connection.googleVoice,
    style: connection.backend === 'pocket-tts' ? '' : connection.googleStyle };
}
