export interface NarrationPreferencesSnapshot {
  spokenText: string;
  voice: string;
  style: string;
  lexicon: Array<{ name: string; sayAs: string; include: boolean }>;
}

export const DEFAULT_NARRATION_VOICE = 'Kore';
export const DEFAULT_NARRATION_STYLE = 'Clear, calm, conversational.';

/** Decode the saved per-section narration preferences without trusting storage data. */
export function parseNarrationPreferences(
  serialized: string | null,
  fallbackText: string,
): NarrationPreferencesSnapshot {
  const fallback: NarrationPreferencesSnapshot = {
    spokenText: fallbackText,
    voice: DEFAULT_NARRATION_VOICE,
    style: DEFAULT_NARRATION_STYLE,
    lexicon: [],
  };
  if (!serialized) return fallback;
  try {
    const parsed = JSON.parse(serialized) as Partial<NarrationPreferencesSnapshot>;
    return {
      spokenText: typeof parsed.spokenText === 'string' ? parsed.spokenText : fallbackText,
      voice: typeof parsed.voice === 'string' ? parsed.voice : DEFAULT_NARRATION_VOICE,
      style: typeof parsed.style === 'string' ? parsed.style : DEFAULT_NARRATION_STYLE,
      lexicon: Array.isArray(parsed.lexicon)
        ? parsed.lexicon.filter(isPronunciationEntry).slice(0, 12)
        : [],
    };
  } catch {
    return fallback;
  }
}

/** Apply the enabled, whole-identifier pronunciation substitutions sent to TTS. */
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
  return applyPronunciationLexicon(left.spokenText.trim(), left.lexicon)
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
