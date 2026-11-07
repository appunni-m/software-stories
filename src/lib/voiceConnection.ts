import type { NarrationPreferencesSnapshot } from './narration';

export interface VoiceConnection {
  backend: 'pocket-tts';
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
  catch { throw new Error('Saved voice settings are malformed. Pocket TTS remains selected.'); }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Saved voice settings are malformed. Pocket TTS remains selected.');
  }
  const record = value as Record<string, unknown>;
  if (Object.keys(record).length !== 3 || !['backend', 'googleVoice', 'googleStyle'].every(key => Object.hasOwn(record, key))) {
    throw new Error('Saved voice settings use an unsupported format. Pocket TTS remains selected.');
  }
  if (record.backend !== 'pocket-tts' && record.backend !== 'gemini') {
    throw new Error('Saved voice engine is not available. Pocket TTS remains selected.');
  }
  if (typeof record.googleVoice !== 'string' || !/^[A-Za-z]{1,40}$/u.test(record.googleVoice)
    || typeof record.googleStyle !== 'string' || new TextEncoder().encode(record.googleStyle).byteLength > 160) {
    throw new Error('Saved voice settings are malformed. Pocket TTS remains selected.');
  }
  // Pocket TTS is the only first-release voice engine. Read the older saved
  // cloud choice as local so it cannot trigger a paid or network voice request.
  return { backend: 'pocket-tts', googleVoice: record.googleVoice, googleStyle: record.googleStyle };
}

/** Keep section text/pronunciation edits; Connections owns delivery for every section. */
export function applyVoiceConnection(section: NarrationPreferencesSnapshot, _connection: VoiceConnection): NarrationPreferencesSnapshot {
  return { ...section, backend: 'pocket-tts', voice: 'alba', style: '' };
}
