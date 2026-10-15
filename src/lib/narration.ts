import type { NarrationBackend } from './commands';

export interface NarrationPreferencesSnapshot {
  backend: NarrationBackend;
  spokenText: string;
  voice: string;
  style: string;
  lexicon: Array<{ name: string; sayAs: string; include: boolean }>;
}

export const DEFAULT_NARRATION_VOICE = 'alba';
export const DEFAULT_NARRATION_STYLE = '';

/** Decode the saved per-section narration preferences without trusting storage data. */
export function parseNarrationPreferences(
  serialized: string | null,
  fallbackText: string,
): NarrationPreferencesSnapshot {
  const fallback: NarrationPreferencesSnapshot = {
    backend: 'pocket-tts',
    spokenText: fallbackText,
    voice: DEFAULT_NARRATION_VOICE,
    style: DEFAULT_NARRATION_STYLE,
    lexicon: [],
  };
  if (!serialized) return fallback;
  try {
    const parsed = JSON.parse(serialized) as Partial<NarrationPreferencesSnapshot>;
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return fallback;
    // Existing saved preferences predate local TTS and retain their explicit
    // cloud voice. Invalid or future backend values must not select cloud TTS.
    const legacyCloudPreferences = parsed.backend === undefined
      && typeof parsed.spokenText === 'string'
      && typeof parsed.voice === 'string'
      && typeof parsed.style === 'string'
      && Array.isArray(parsed.lexicon);
    const backend: NarrationBackend = parsed.backend === 'gemini' || legacyCloudPreferences ? 'gemini' : 'pocket-tts';
    return {
      backend,
      spokenText: typeof parsed.spokenText === 'string' ? parsed.spokenText : fallbackText,
      // Pocket supports one pinned voice and no style prompt. Normalize old or
      // malformed values so hidden style settings cannot block a local review.
      voice: backend === 'pocket-tts' ? DEFAULT_NARRATION_VOICE : typeof parsed.voice === 'string' ? parsed.voice : 'Kore',
      style: backend === 'pocket-tts' ? DEFAULT_NARRATION_STYLE : typeof parsed.style === 'string' ? parsed.style : 'Clear, calm, conversational.',
      lexicon: Array.isArray(parsed.lexicon)
        ? parsed.lexicon.filter(isPronunciationEntry).slice(0, 12)
        : [],
    };
  } catch {
    return fallback;
  }
}

/** Apply the enabled, whole-identifier pronunciation substitutions used by TTS. */
export function applyPronunciationLexicon(
  spokenText: string,
  lexicon: NarrationPreferencesSnapshot['lexicon'],
): string {
  const encoder = new TextEncoder();
  const entries = lexicon
    .filter((entry) => entry.include)
    .map((entry) => ({ name: entry.name.trim(), sayAs: entry.sayAs.trim() }))
    .filter((entry) => entry.name.length > 0 && entry.sayAs.length > 0)
    .sort((left, right) => encoder.encode(right.name).byteLength - encoder.encode(left.name).byteLength);
  const input = Array.from(spokenText);
  const output: string[] = [];
  let cursor = 0;

  while (cursor < input.length) {
    const match = entries.find((entry) => {
      const name = Array.from(entry.name);
      if (name.length === 0 || name.some((character, index) => input[cursor + index] !== character)) return false;
      return !isIdentifierCharacter(input[cursor - 1]) && !isIdentifierCharacter(input[cursor + name.length]);
    });
    if (match) {
      output.push(match.sayAs);
      cursor += Array.from(match.name).length;
    } else {
      output.push(input[cursor]);
      cursor += 1;
    }
  }

  return output.join('');
}

export function narrationPreferencesMatch(
  left: NarrationPreferencesSnapshot,
  right: NarrationPreferencesSnapshot,
): boolean {
  return (left.backend ?? 'gemini') === (right.backend ?? 'gemini')
    && applyPronunciationLexicon(left.spokenText.trim(), left.lexicon)
      === applyPronunciationLexicon(right.spokenText.trim(), right.lexicon)
    && left.voice === right.voice
    && left.style.trim() === right.style.trim();
}

function isIdentifierCharacter(character: string | undefined): boolean {
  return Boolean(character && /[\p{L}\p{N}_$]/u.test(character));
}

function isPronunciationEntry(value: unknown): value is NarrationPreferencesSnapshot['lexicon'][number] {
  return Boolean(value && typeof value === 'object'
    && typeof (value as NarrationPreferencesSnapshot['lexicon'][number]).name === 'string'
    && typeof (value as NarrationPreferencesSnapshot['lexicon'][number]).sayAs === 'string'
    && typeof (value as NarrationPreferencesSnapshot['lexicon'][number]).include === 'boolean');
}
