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

/** Pronounce names in the default script without changing its sentences.
 * Saved user edits and explicit pronunciation entries remain authoritative.
 * Does not parse source, drop sentences or rewrite conditions.
 */
export function defaultNarrationText(text: string): string {
  return text.replace(/`[^`\n]+`|[A-Za-z_$][A-Za-z0-9_$]*(?:(?:::|\.)[A-Za-z_$][A-Za-z0-9_$]*)*/g, (token: string, offset: number) => {
    const quoted = token.startsWith('`');
    const name = quoted ? token.slice(1, -1) : token;
    if (!/^[A-Za-z_$][A-Za-z0-9_$]*(?:(?:::|\.)[A-Za-z_$][A-Za-z0-9_$]*)*$/.test(name)) return token;
    if (/[\p{L}\p{N}_$./:\\]/u.test(text[offset - 1] ?? '')
      || /[\p{L}\p{N}_$/\\:]/u.test(text[offset + token.length] ?? '')) return token;
    // Dotted names can be filenames. Leave them and path-adjacent tokens
    // unchanged; this is a narrow pronunciation rule, not a code parser.
    if (name.includes('.')) return token;
    return name.replace(/::/g, ' ')
      .replace(/([A-Z])([A-Z][a-z])/g, '$1 $2')
      .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
      .replace(/_+/g, ' ')
      .trim();
  });
}

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
  if (serialized === null) return fallback;
  let parsed: unknown;
  try {
    parsed = JSON.parse(serialized);
  } catch {
    throw new Error('Saved narration preferences are malformed. The accepted transcript remains available.');
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Saved narration preferences are malformed. The accepted transcript remains available.');
  }
  const value = parsed as Record<string, unknown>;
  const keys = ['backend', 'spokenText', 'voice', 'style', 'lexicon'];
  if (Object.keys(value).length !== keys.length || !keys.every(key => Object.hasOwn(value, key))) {
    throw new Error('Saved narration preferences use an unsupported format. The accepted transcript remains available.');
  }
  if (value.backend !== 'pocket-tts' && value.backend !== 'gemini') {
    throw new Error('Saved narration settings use an unsupported voice engine. Choose Pocket TTS or Gemini in Connections.');
  }
  if (typeof value.spokenText !== 'string' || new TextEncoder().encode(value.spokenText).byteLength > 16_000
    || typeof value.voice !== 'string' || typeof value.style !== 'string'
    || !Array.isArray(value.lexicon) || value.lexicon.length > 12
    || !value.lexicon.every(isPronunciationEntry)) {
    throw new Error('Saved narration preferences are malformed. The accepted transcript remains available.');
  }
  if (value.backend === 'pocket-tts'
    ? value.voice !== DEFAULT_NARRATION_VOICE || value.style !== DEFAULT_NARRATION_STYLE
    : !/^[A-Za-z]{1,40}$/u.test(value.voice) || new TextEncoder().encode(value.style).byteLength > 160) {
    throw new Error('Saved narration preferences do not match the selected voice engine. Choose a supported voice in Connections.');
  }
  return { backend: value.backend as NarrationBackend, spokenText: value.spokenText,
    voice: value.voice, style: value.style, lexicon: value.lexicon as NarrationPreferencesSnapshot['lexicon'] };
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
  return left.backend === right.backend
    && applyPronunciationLexicon(left.spokenText.trim(), left.lexicon)
      === applyPronunciationLexicon(right.spokenText.trim(), right.lexicon)
    && left.voice === right.voice
    && left.style.trim() === right.style.trim();
}

/** A cache hit can play immediately; new local audio first shows its reviewed script. */
export function localNarrationAction(reviewVisible: boolean, cachedAudioAvailable: boolean): 'show-review' | 'play-cache' | 'synthesize' {
  if (cachedAudioAvailable) return 'play-cache';
  return reviewVisible ? 'synthesize' : 'show-review';
}

function isIdentifierCharacter(character: string | undefined): boolean {
  return Boolean(character && /[\p{L}\p{N}_$]/u.test(character));
}

function isPronunciationEntry(value: unknown): value is NarrationPreferencesSnapshot['lexicon'][number] {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const entry = value as Record<string, unknown>;
  return Object.keys(entry).length === 3 && Object.hasOwn(entry, 'name') && Object.hasOwn(entry, 'sayAs')
    && Object.hasOwn(entry, 'include') && typeof entry.name === 'string' && typeof entry.sayAs === 'string'
    && typeof entry.include === 'boolean';
}
