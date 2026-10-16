import type { NarrationPreferencesSnapshot } from './narration';

export interface VoiceConnection {
  backend: 'pocket-tts' | 'gemini';
  googleVoice: string;
  googleStyle: string;
}
export const VOICE_CONNECTION_KEY = 'code-atlas:voice-connection:v1';
export const DEFAULT_VOICE_CONNECTION: VoiceConnection = { backend: 'pocket-tts', googleVoice: 'Kore', googleStyle: '' };

/** Global delivery settings never inherit a cloud choice from a saved section. */
export function parseVoiceConnection(serialized: string | null): VoiceConnection {
  try {
    const value = JSON.parse(serialized ?? 'null');
    if (!value || typeof value !== 'object' || Array.isArray(value)) return { ...DEFAULT_VOICE_CONNECTION };
    return {
      backend: value.backend === 'gemini' ? 'gemini' : 'pocket-tts',
      googleVoice: typeof value.googleVoice === 'string' && /^[A-Za-z]{1,40}$/.test(value.googleVoice) ? value.googleVoice : 'Kore',
      googleStyle: typeof value.googleStyle === 'string' && new TextEncoder().encode(value.googleStyle).length <= 160 ? value.googleStyle : '',
    };
  } catch { return { ...DEFAULT_VOICE_CONNECTION }; }
}

/** Keep section text/pronunciation edits; Connections owns delivery for every section. */
export function applyVoiceConnection(section: NarrationPreferencesSnapshot, connection: VoiceConnection): NarrationPreferencesSnapshot {
  return { ...section, backend: connection.backend,
    voice: connection.backend === 'pocket-tts' ? 'alba' : connection.googleVoice,
    style: connection.backend === 'pocket-tts' ? '' : connection.googleStyle };
}
