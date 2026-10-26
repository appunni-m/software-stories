import { languageForPath } from './inventory.ts';
import { normalizeSourceLineRange, sourceRangeMatchesHash, type SourceLineRange, type WorkspaceRoute } from './navigation.ts';

export type SourceTokenKind = 'keyword' | 'string' | 'comment' | 'number' | 'property' | 'operator';

export interface SourceToken {
  text: string;
  kind?: SourceTokenKind;
}

export type CitationResolution =
  | { status: 'none'; range: null }
  | { status: 'pending'; range: null }
  | { status: 'verified'; range: SourceLineRange; textRange?: SourceTextRange }
  | { status: 'stale'; range: null; expectedSha256: string; actualSha256: string }
  | { status: 'unavailable'; range: null; reason: 'invalid-anchor' | 'invalid-line-range' | 'invalid-source-hash' | 'invalid-byte-range' | 'missing-source-hash' | 'out-of-bounds' };

export interface SourceTextRange {
  /** UTF-16 string offset, inclusive. */
  start: number;
  /** UTF-16 string offset, exclusive. */
  end: number;
}

/** Preserve the indexer's half-open UTF-8 byte span as a Code Lens citation. */
export function codeLensRangeFromIndexedAnchor(anchor: {
  startLine: number;
  endLine: number;
  sourceSha256: string;
  startByte: number;
  endByte: number;
}): SourceLineRange {
  return {
    startLine: anchor.startLine,
    endLine: anchor.endLine,
    sourceSha256: anchor.sourceSha256,
    startByte: anchor.startByte,
    endByteExclusive: anchor.endByte,
  };
}

/** Convert a chapter's inclusive byte endpoint to the Code Lens half-open span. */
export function codeLensRangeFromChapterAnchor(anchor: {
  startLine: number;
  endLine: number;
  sourceSha256: string;
  startByte: number;
  endByteInclusive: number;
}): SourceLineRange {
  return {
    startLine: anchor.startLine,
    endLine: anchor.endLine,
    sourceSha256: anchor.sourceSha256,
    startByte: anchor.startByte,
    endByteExclusive: anchor.endByteInclusive + 1,
  };
}

/** Map a UTF-8 byte boundary to the corresponding JavaScript string boundary. */
export function utf8ByteOffsetToCodeUnitOffset(contents: string, byteOffset: number): number | null {
  if (!Number.isSafeInteger(byteOffset) || byteOffset < 0) return null;
  let bytes = 0;
  let codeUnits = 0;
  for (const character of contents) {
    if (bytes === byteOffset) return codeUnits;
    const codePoint = character.codePointAt(0)!;
    const characterBytes = codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
    bytes += characterBytes;
    codeUnits += character.length;
    if (bytes > byteOffset) return null;
  }
  return bytes === byteOffset ? codeUnits : null;
}

/** The source-text offsets represented by splitCapturedSourceLines, excluding line terminators. */
export function sourceLineTextRanges(contents: string): SourceTextRange[] {
  if (contents.length === 0) return [];
  const ranges: SourceTextRange[] = [];
  let start = 0;
  for (let index = 0; index < contents.length;) {
    if (contents[index] !== '\r' && contents[index] !== '\n') {
      index += 1;
      continue;
    }
    ranges.push({ start, end: index });
    index += contents[index] === '\r' && contents[index + 1] === '\n' ? 2 : 1;
    start = index;
  }
  if (start < contents.length) ranges.push({ start, end: contents.length });
  return ranges;
}

/** Count source lines without allocating a string or range for every line. */
export function capturedSourceLineCount(contents: string): number {
  if (contents.length === 0) return 0;
  let count = 0;
  for (let index = 0; index < contents.length; index += 1) {
    if (contents[index] === '\r') {
      count += 1;
      if (contents[index + 1] === '\n') index += 1;
    } else if (contents[index] === '\n') count += 1;
  }
  const last = contents.at(-1);
  return count + (last === '\r' || last === '\n' ? 0 : 1);
}

export interface CapturedSourceLineIndex {
  contents: string;
  lineCount: number;
  checkpoints: { line: number; offset: number }[];
}

/** Build one sparse line index so later Code Lens pages don't rescan from the file start. */
export function capturedSourceLineIndex(contents: string): CapturedSourceLineIndex {
  const checkpoints = contents.length > 0 ? [{ line: 1, offset: 0 }] : [];
  if (contents.length === 0) return { contents, lineCount: 0, checkpoints };

  let line = 1;
  let offset = 0;
  let breaks = 0;
  for (let index = 0; index < contents.length; index += 1) {
    if (contents[index] !== '\r' && contents[index] !== '\n') continue;
    if (contents[index] === '\r' && contents[index + 1] === '\n') index += 1;
    breaks += 1;
    line += 1;
    offset = index + 1;
    if (offset < contents.length && (line - 1) % 512 === 0) checkpoints.push({ line, offset });
  }
  const last = contents.at(-1);
  return { contents, lineCount: breaks + (last === '\r' || last === '\n' ? 0 : 1), checkpoints };
}

/** Read a bounded line window and preserve the visible text's original UTF-16 offsets. */
export function capturedSourceLineWindow(contents: string, firstLine: number, lastLine: number, options: {
  maxLineCharacters?: number;
  focusRange?: SourceTextRange;
  lineIndex?: CapturedSourceLineIndex;
} = {}): {
  lines: string[];
  ranges: SourceTextRange[];
  omittedBefore: boolean[];
  omittedAfter: boolean[];
} {
  const first = Number.isSafeInteger(firstLine) ? Math.max(1, firstLine) : 1;
  const last = Number.isSafeInteger(lastLine) ? Math.max(first, lastLine) : Number.MAX_SAFE_INTEGER;
  const maxLineCharacters = Number.isSafeInteger(options.maxLineCharacters) && (options.maxLineCharacters ?? 0) > 0
    ? options.maxLineCharacters!
    : Number.MAX_SAFE_INTEGER;
  const lines: string[] = [];
  const ranges: SourceTextRange[] = [];
  const omittedBefore: boolean[] = [];
  const omittedAfter: boolean[] = [];
  if (contents.length === 0) return { lines, ranges, omittedBefore, omittedAfter };

  const appendLine = (lineStart: number, lineEnd: number) => {
    const lineLength = lineEnd - lineStart;
    let visibleStart = lineStart;
    let visibleEnd = lineEnd;
    if (lineLength > maxLineCharacters) {
      const focus = options.focusRange;
      const overlapsFocus = focus && focus.start < lineEnd && focus.end > lineStart;
      if (overlapsFocus) {
        const focusStart = Math.max(0, Math.min(lineLength, focus.start - lineStart));
        const focusEnd = Math.max(focusStart, Math.min(lineLength, focus.end - lineStart));
        const focusLength = focusEnd - focusStart;
        const windowStart = focusLength <= maxLineCharacters
          ? Math.max(0, Math.min(lineLength - maxLineCharacters, focusStart - Math.floor((maxLineCharacters - focusLength) / 2)))
          : focusStart;
        visibleStart = lineStart + windowStart;
        visibleEnd = Math.min(lineEnd, visibleStart + maxLineCharacters);
      } else {
        visibleEnd = lineStart + maxLineCharacters;
      }
      if (visibleStart > lineStart) {
        const firstVisibleCodeUnit = contents.charCodeAt(visibleStart);
        if (firstVisibleCodeUnit >= 0xdc00 && firstVisibleCodeUnit <= 0xdfff) visibleStart += 1;
      }
      if (visibleEnd < lineEnd) {
        const lastVisibleCodeUnit = contents.charCodeAt(visibleEnd - 1);
        if (lastVisibleCodeUnit >= 0xd800 && lastVisibleCodeUnit <= 0xdbff) visibleEnd -= 1;
      }
      if (visibleEnd <= visibleStart && lineLength > 0) {
        const codePoint = contents.codePointAt(visibleStart);
        visibleEnd = Math.min(lineEnd, visibleStart + (codePoint !== undefined && codePoint > 0xffff ? 2 : 1));
      }
    }
    lines.push(contents.slice(visibleStart, visibleEnd));
    ranges.push({ start: visibleStart, end: visibleEnd });
    omittedBefore.push(visibleStart > lineStart);
    omittedAfter.push(visibleEnd < lineEnd);
  };

  const indexRecord = options.lineIndex?.contents === contents ? options.lineIndex : undefined;
  let checkpoint = indexRecord?.checkpoints[0];
  if (indexRecord && indexRecord.checkpoints.length > 1) {
    let low = 0;
    let high = indexRecord.checkpoints.length - 1;
    while (low <= high) {
      const middle = Math.floor((low + high) / 2);
      const candidate = indexRecord.checkpoints[middle];
      if (candidate.line <= first) { checkpoint = candidate; low = middle + 1; }
      else high = middle - 1;
    }
  }
  let line = checkpoint?.line ?? 1;
  let start = checkpoint?.offset ?? 0;
  for (let index = start; index < contents.length;) {
    const character = contents[index];
    if (character !== '\r' && character !== '\n') {
      index += 1;
      continue;
    }
    if (line >= first && line <= last) {
      appendLine(start, index);
    }
    index += character === '\r' && contents[index + 1] === '\n' ? 2 : 1;
    line += 1;
    start = index;
    if (line > last) break;
  }
  if (start < contents.length && line >= first && line <= last) appendLine(start, contents.length);
  return { lines, ranges, omittedBefore, omittedAfter };
}

/** Return the 1-based source line at a UTF-16 offset, counting CRLF as one terminator. */
function sourceLineNumberAtOffset(contents: string, offset: number): number {
  let line = 1;
  for (let index = 0; index < offset;) {
    const character = contents[index];
    if (character === '\r' && contents[index + 1] === '\n') {
      if (index + 2 > offset) break;
      line += 1;
      index += 2;
    } else if (character === '\r' || character === '\n') {
      line += 1;
      index += 1;
    } else {
      index += 1;
    }
  }
  return line;
}

/**
 * Resolve a citation only against the hash and line count of the captured bytes.
 * Invalid or stale citations deliberately return no range for rendering.
 */
export function resolveCodeLensCitation(
  range: SourceLineRange | undefined,
  issue: WorkspaceRoute['sourceRangeIssue'] | undefined,
  sourceSha256: string | undefined,
  lineCount: number,
  sourceContents?: string,
): CitationResolution {
  if (issue) return { status: 'unavailable', range: null, reason: issue };
  if (!range) return { status: 'none', range: null };
  if (!sourceSha256) return { status: 'pending', range: null };

  const normalized = normalizeSourceLineRange(range);
  if (!normalized) return { status: 'unavailable', range: null, reason: 'invalid-line-range' };
  if (!normalized.sourceSha256) return { status: 'unavailable', range: null, reason: 'missing-source-hash' };
  if (!sourceRangeMatchesHash(normalized, sourceSha256)) {
    return { status: 'stale', range: null, expectedSha256: normalized.sourceSha256, actualSha256: sourceSha256 };
  }
  if (normalized.endLine > lineCount) return { status: 'unavailable', range: null, reason: 'out-of-bounds' };
  if (normalized.startByte !== undefined && normalized.endByteExclusive !== undefined) {
    if (sourceContents === undefined) return { status: 'pending', range: null };
    const start = utf8ByteOffsetToCodeUnitOffset(sourceContents, normalized.startByte);
    const end = utf8ByteOffsetToCodeUnitOffset(sourceContents, normalized.endByteExclusive);
    if (start === null || end === null || end <= start) {
      return { status: 'unavailable', range: null, reason: 'invalid-byte-range' };
    }
    const startLine = sourceLineNumberAtOffset(sourceContents, start);
    const endLine = sourceLineNumberAtOffset(sourceContents, end - 1);
    if (startLine !== normalized.startLine || endLine !== normalized.endLine) {
      return { status: 'unavailable', range: null, reason: 'invalid-byte-range' };
    }
    return { status: 'verified', range: normalized, textRange: { start, end } };
  }
  return { status: 'verified', range: normalized };
}

/** Split a token at a local half-open range so the renderer can mark only cited characters. */
export function splitSourceTextAtRange(text: string, start: number, end: number): Array<{ text: string; cited: boolean }> {
  const boundedStart = Math.max(0, Math.min(text.length, start));
  const boundedEnd = Math.max(boundedStart, Math.min(text.length, end));
  if (boundedStart === boundedEnd) return [{ text, cited: false }];
  return [
    ...(boundedStart > 0 ? [{ text: text.slice(0, boundedStart), cited: false }] : []),
    { text: text.slice(boundedStart, boundedEnd), cited: true },
    ...(boundedEnd < text.length ? [{ text: text.slice(boundedEnd), cited: false }] : []),
  ];
}

export type CodeLensUnavailableState = 'binary' | 'invalid-encoding' | 'oversized' | 'uncaptured' | 'missing' | 'read-error';

/** Keep the indexed relationship kind and confidence value visible verbatim in Code Lens. */
export function codeLensRelationshipLabel(kind: string, confidence: 'high' | 'unresolved'): string {
  return `${kind} · confidence: ${confidence}`;
}

/** Classifies failed snapshot reads so the UI can explain the exact unavailable state. */
export function classifyCodeLensUnavailableState(input: {
  contentKind?: 'text' | 'binary' | 'invalidEncoding' | 'unknown';
  skippedReason?: string;
  manifestLoaded: boolean;
}): CodeLensUnavailableState {
  if (input.contentKind === 'binary') return 'binary';
  if (input.contentKind === 'invalidEncoding') return 'invalid-encoding';
  if (input.skippedReason && ['fileByteLimit', 'totalByteLimit'].includes(input.skippedReason)) return 'oversized';
  if (input.skippedReason) return 'uncaptured';
  if (!input.contentKind && input.manifestLoaded) return 'missing';
  return 'read-error';
}

/** Uses source-relative 1-based line positions and omits only the phantom line after a final newline. */
export function splitCapturedSourceLines(contents: string): string[] {
  if (contents.length === 0) return [];
  const lines = contents.split(/\r\n|\n|\r/);
  if (/(?:\r\n|\n|\r)$/.test(contents)) lines.pop();
  return lines;
}

type LexerFamily = 'cLike' | 'python' | 'shell' | 'json';

const lexerFamilyByLanguage: Record<string, LexerFamily> = {
  TypeScript: 'cLike', JavaScript: 'cLike', Rust: 'cLike', Go: 'cLike', Java: 'cLike',
  C: 'cLike', 'C++': 'cLike', 'C#': 'cLike', Swift: 'cLike', Kotlin: 'cLike',
  Python: 'python', Shell: 'shell', JSON: 'json', 'JSON with comments': 'json',
};

const keywords: Record<string, ReadonlySet<string>> = {
  TypeScript: new Set('abstract as asserts async await boolean break case catch class const constructor continue debugger declare default delete do else enum export extends false finally for from function get if implements import in infer instanceof interface is keyof let module namespace never new null number object of package private protected public readonly require return set static string super switch symbol this throw true try type typeof undefined unique unknown var void while with yield'.split(' ')),
  JavaScript: new Set('async await break case catch class const continue debugger default delete do else export extends false finally for function if import in instanceof let new null return super switch this throw true try typeof var void while with yield'.split(' ')),
  Rust: new Set('as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while'.split(' ')),
  Python: new Set('and as assert async await break class continue def del elif else except False finally for from global if import in is lambda None nonlocal not or pass raise return True try while with yield'.split(' ')),
  Go: new Set('break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var true false iota nil'.split(' ')),
  Java: new Set('abstract assert boolean break byte case catch char class const continue default do double else enum extends final finally float for if implements import instanceof int interface long native new package private protected public return short static strictfp super switch synchronized this throw throws transient try void volatile while true false null'.split(' ')),
  C: new Set('auto break case char const continue default do double else enum extern float for goto if inline int long register restrict return short signed sizeof static struct switch typedef union unsigned void volatile while _Bool _Complex _Imaginary'.split(' ')),
  'C++': new Set('alignas alignof asm auto bool break case catch char class const constexpr continue decltype default delete do double else enum explicit export extern false float for friend goto if inline int long mutable namespace new noexcept nullptr operator private protected public register return short signed sizeof static struct switch template this throw true try typedef typename union unsigned using virtual void volatile while'.split(' ')),
  'C#': new Set('abstract as async await base bool break byte case catch char checked class const continue decimal default delegate do double else enum event explicit extern false finally fixed float for foreach goto if implicit in int interface internal is lock long namespace new null object operator out override params private protected public readonly ref return sbyte sealed short sizeof stackalloc static string struct switch this throw true try typeof uint ulong unchecked unsafe ushort using var virtual void volatile while'.split(' ')),
  Swift: new Set('as associatedtype break case catch class continue default defer defer do else enum extension fallthrough false for func guard if import in init inout internal is let nil open operator private protocol public repeat rethrows return self Self static struct subscript super switch throw throws true try typealias var where while'.split(' ')),
  Kotlin: new Set('as break class continue do else false for fun if in interface is null object package return super this throw true try typealias typeof val var when while'.split(' ')),
  Shell: new Set('case do done elif else esac fi for function if in select then until while time coproc'.split(' ')),
};

const numberPattern = /^(?:0[xX][\da-fA-F_]+|0[bB][01_]+|0[oO][0-7_]+|(?:\d[\d_]*\.?[\d_]*|\.\d[\d_]*)(?:[eE][+-]?[\d_]+)?n?)/;
const identifierPattern = /^[\p{L}_$][\p{L}\p{N}_$]*/u;

/**
 * Returns line-preserving tokens for the small set of common languages supported by the
 * built-in lightweight lexer. Unknown extensions deliberately return null so callers can
 * keep the original source as plain text.
 */
export function highlightSourceLines(path: string, lines: readonly string[]): SourceToken[][] | null {
  const language = languageForPath(path);
  const family = lexerFamilyByLanguage[language];
  if (!family) return null;

  const result: SourceToken[][] = [];
  let blockCommentDepth = 0;
  let multilineQuote: string | null = null;

  for (const line of lines) {
    const tokens: SourceToken[] = [];
    let cursor = 0;
    const append = (text: string, kind?: SourceTokenKind) => {
      if (!text) return;
      const previous = tokens[tokens.length - 1];
      if (previous && previous.kind === kind) previous.text += text;
      else tokens.push(kind ? { text, kind } : { text });
    };

    while (cursor < line.length) {
      if (blockCommentDepth > 0) {
        const closeAt = line.indexOf('*/', cursor);
        if (closeAt < 0) {
          append(line.slice(cursor), 'comment');
          cursor = line.length;
        } else {
          append(line.slice(cursor, closeAt + 2), 'comment');
          cursor = closeAt + 2;
          blockCommentDepth = 0;
        }
        continue;
      }

      if (multilineQuote) {
        const closeAt = line.indexOf(multilineQuote, cursor);
        if (closeAt < 0) {
          append(line.slice(cursor), 'string');
          cursor = line.length;
        } else {
          append(line.slice(cursor, closeAt + multilineQuote.length), 'string');
          cursor = closeAt + multilineQuote.length;
          multilineQuote = null;
        }
        continue;
      }

      const rest = line.slice(cursor);
      if ((family === 'cLike' || (family === 'json' && language === 'JSON with comments')) && rest.startsWith('//')) {
        append(rest, 'comment');
        break;
      }
      if ((family === 'python' || family === 'shell' || family === 'json') && rest.startsWith('#')) {
        append(rest, 'comment');
        break;
      }
      if ((family === 'cLike' || (family === 'json' && language === 'JSON with comments')) && rest.startsWith('/*')) {
        const closeAt = line.indexOf('*/', cursor + 2);
        if (closeAt < 0) {
          append(rest, 'comment');
          blockCommentDepth = 1;
          break;
        }
        append(line.slice(cursor, closeAt + 2), 'comment');
        cursor = closeAt + 2;
        continue;
      }

      const quote = line[cursor];
      const tripleQuote = family === 'python' && (line.startsWith("'''", cursor) || line.startsWith('"""', cursor))
        ? line.slice(cursor, cursor + 3)
        : null;
      if (tripleQuote || quote === '"' || quote === "'" || (quote === '`' && family === 'cLike')) {
        const delimiter = tripleQuote ?? quote;
        let end = cursor + delimiter.length;
        let closed = false;
        while (end < line.length) {
          if (line[end] === '\\') {
            end += 2;
            continue;
          }
          if (line.startsWith(delimiter, end)) {
            end += delimiter.length;
            closed = true;
            break;
          }
          end += 1;
        }
        append(line.slice(cursor, Math.min(end, line.length)), 'string');
        if (!closed && (tripleQuote || quote === '`')) multilineQuote = delimiter;
        cursor = line.length < end ? line.length : end;
        continue;
      }

      const number = numberPattern.exec(rest)?.[0];
      if (number) {
        append(number, 'number');
        cursor += number.length;
        continue;
      }

      const identifier = identifierPattern.exec(rest)?.[0];
      if (identifier) {
        const keywordSet = keywords[language];
        const followedByColon = family === 'json' && /^\s*:/.test(rest.slice(identifier.length));
        const kind = followedByColon ? 'property' : keywordSet?.has(identifier) || (family === 'json' && ['true', 'false', 'null'].includes(identifier)) ? 'keyword' : undefined;
        append(identifier, kind);
        cursor += identifier.length;
        continue;
      }

      const operator = /^(?:===?|!==?|=>|->|::|&&|\|\||\?\?|\+=|-=|\*=|\/=|\*\*|\.\.\.)/.exec(rest)?.[0];
      if (operator) {
        append(operator, 'operator');
        cursor += operator.length;
        continue;
      }
      append(line[cursor]);
      cursor += 1;
    }
    result.push(tokens.length > 0 ? tokens : [{ text: '' }]);
  }
  return result;
}
