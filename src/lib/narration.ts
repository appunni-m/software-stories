export interface NarrationPreferencesSnapshot {
  spokenText: string;
  voice: string;
  style: string;
  lexicon: Array<{ name: string; sayAs: string; include: boolean }>;
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
