export type InlineCodePart = { kind: 'text' | 'code'; text: string };

function runLengthAt(value: string, index: number): number {
  let end = index;
  while (value[end] === '`') end += 1;
  return end - index;
}

function isEscaped(value: string, index: number): boolean {
  let slashes = 0;
  for (let cursor = index - 1; cursor >= 0 && value[cursor] === '\\'; cursor -= 1) slashes += 1;
  return slashes % 2 === 1;
}

function singleDelimiterAt(value: string, index: number): boolean {
  return value[index] === '`'
    && runLengthAt(value, index) === 1
    && (index === 0 || value[index - 1] !== '`')
    && !isEscaped(value, index);
}

/** Split the small inline-code subset used in explanation text. Unmatched,
 * multiline and multi-backtick spans remain literal text. */
export function inlineCodeParts(value: string): InlineCodePart[] {
  const parts: InlineCodePart[] = [];
  let plainStart = 0;
  let cursor = 0;

  while (cursor < value.length) {
    if (!singleDelimiterAt(value, cursor)) {
      cursor += value[cursor] === '`' ? runLengthAt(value, cursor) : 1;
      continue;
    }

    let close = cursor + 1;
    while (close < value.length && !singleDelimiterAt(value, close)) {
      close += value[close] === '`' ? runLengthAt(value, close) : 1;
    }
    if (close >= value.length || value.slice(cursor + 1, close).includes('\n')) {
      cursor += 1;
      continue;
    }

    if (cursor > plainStart) parts.push({ kind: 'text', text: value.slice(plainStart, cursor) });
    parts.push({ kind: 'code', text: value.slice(cursor + 1, close) });
    cursor = close + 1;
    plainStart = cursor;
  }

  if (plainStart < value.length) parts.push({ kind: 'text', text: value.slice(plainStart) });
  return parts.length ? parts : [{ kind: 'text', text: value }];
}
