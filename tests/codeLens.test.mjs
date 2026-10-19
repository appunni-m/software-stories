import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  classifyCodeLensUnavailableState,
  codeLensRangeFromChapterAnchor,
  codeLensRelationshipLabel,
  highlightSourceLines,
  resolveCodeLensCitation,
  sourceLineTextRanges,
  splitCapturedSourceLines,
  splitSourceTextAtRange,
  utf8ByteOffsetToCodeUnitOffset,
} from '../src/lib/codeLens.ts';
import { navigateToRoute } from '../src/lib/navigation.ts';

describe('Code Lens source rendering helpers', () => {
  it('keeps captured line positions stable across LF, CRLF, blank lines, and final newlines', () => {
    assert.deepEqual(splitCapturedSourceLines(''), []);
    assert.deepEqual(splitCapturedSourceLines('first\nsecond\n'), ['first', 'second']);
    assert.deepEqual(splitCapturedSourceLines('first\r\n\r\nthird'), ['first', '', 'third']);
    assert.deepEqual(splitCapturedSourceLines('first\rsecond\r'), ['first', 'second']);
  });

  it('highlights supported source without changing the source text or line count', () => {
    const lines = splitCapturedSourceLines('const answer = "42"; // keep this\nreturn answer;\n');
    const highlighted = highlightSourceLines('src/example.ts', lines);
    assert.ok(highlighted);
    assert.equal(highlighted.length, lines.length);
    assert.deepEqual(highlighted.map((tokens) => tokens.map((token) => token.text).join('')), lines);
    assert.ok(highlighted[0].some((token) => token.text === 'const' && token.kind === 'keyword'));
    assert.ok(highlighted[0].some((token) => token.text === '"42"' && token.kind === 'string'));
    assert.ok(highlighted[0].some((token) => token.text === '// keep this' && token.kind === 'comment'));
  });

  it('keeps multiline comments and strings highlighted across source lines', () => {
    const lines = splitCapturedSourceLines('/* first\nsecond */ const value = 3;\n');
    const highlighted = highlightSourceLines('src/example.rs', lines);
    assert.ok(highlighted);
    assert.equal(highlighted[0][0].kind, 'comment');
    assert.ok(highlighted[1].some((token) => token.text === 'second */' && token.kind === 'comment'));
    assert.ok(highlighted[1].some((token) => token.text === '3' && token.kind === 'number'));
  });

  it('preserves leading plain text before the first highlighted token', () => {
    const lines = ['#![forbid(unsafe_code)]', '  fn start() {}'];
    const highlighted = highlightSourceLines('src/lib.rs', lines);
    assert.ok(highlighted);
    assert.deepEqual(highlighted.map((tokens) => tokens.map((token) => token.text).join('')), lines);
  });

  it('returns no lexer output for unsupported extensions so callers show plain text', () => {
    const lines = splitCapturedSourceLines('plain source\n');
    assert.equal(highlightSourceLines('src/readme.weird', lines), null);
    assert.deepEqual(lines, ['plain source']);
  });

  it('highlights a citation only when its source hash matches and its lines exist', () => {
    const sourceSha256 = 'a'.repeat(64);
    const range = { startLine: 2, endLine: 3, sourceSha256 };
    assert.deepEqual(resolveCodeLensCitation(range, undefined, sourceSha256, 3), {
      status: 'verified', range,
    });
    assert.deepEqual(resolveCodeLensCitation(range, undefined, 'b'.repeat(64), 3), {
      status: 'stale', range: null, expectedSha256: sourceSha256, actualSha256: 'b'.repeat(64),
    });
    assert.deepEqual(resolveCodeLensCitation(range, undefined, sourceSha256, 2), {
      status: 'unavailable', range: null, reason: 'out-of-bounds',
    });
  });

  it('maps exact UTF-8 citation bytes to text offsets and preserves only the selected characters', () => {
    const contents = 'const label = "✨";\r\nnext();';
    const selectedStart = contents.indexOf('✨');
    const startByte = Buffer.byteLength(contents.slice(0, selectedStart));
    const endByteExclusive = startByte + Buffer.byteLength('✨');
    const range = {
      startLine: 1,
      endLine: 1,
      sourceSha256: 'a'.repeat(64),
      startByte,
      endByteExclusive,
    };
    const citation = resolveCodeLensCitation(range, undefined, range.sourceSha256, 2, contents);

    assert.deepEqual(sourceLineTextRanges(contents), [
      { start: 0, end: contents.indexOf('\r\n') },
      { start: contents.indexOf('\r\n') + 2, end: contents.length },
    ]);
    assert.equal(utf8ByteOffsetToCodeUnitOffset(contents, startByte), selectedStart);
    assert.equal(utf8ByteOffsetToCodeUnitOffset(contents, endByteExclusive), selectedStart + '✨'.length);
    assert.deepEqual(citation, {
      status: 'verified', range, textRange: { start: selectedStart, end: selectedStart + '✨'.length },
    });
    assert.equal(contents.slice(citation.textRange.start, citation.textRange.end), '✨');
    assert.deepEqual(splitSourceTextAtRange('const label = "✨";', selectedStart, selectedStart + '✨'.length), [
      { text: 'const label = "', cited: false },
      { text: '✨', cited: true },
      { text: '";', cited: false },
    ]);
  });

  it('converts a chapter anchor inclusive endpoint without widening or shifting the citation', () => {
    assert.deepEqual(codeLensRangeFromChapterAnchor({
      startLine: 2, endLine: 2, sourceSha256: 'e'.repeat(64), startByte: 9, endByteInclusive: 13,
    }), {
      startLine: 2, endLine: 2, sourceSha256: 'e'.repeat(64), startByte: 9, endByteExclusive: 14,
    });
  });

  it('validates byte citations in CR-only source files', () => {
    const contents = 'first\rconst value = 2;\rthird';
    const selectedStart = contents.indexOf('value');
    const startByte = Buffer.byteLength(contents.slice(0, selectedStart));
    const endByteExclusive = startByte + Buffer.byteLength('value');
    const range = {
      startLine: 2,
      endLine: 2,
      sourceSha256: 'c'.repeat(64),
      startByte,
      endByteExclusive,
    };

    assert.deepEqual(resolveCodeLensCitation(range, undefined, range.sourceSha256, 3, contents), {
      status: 'verified',
      range,
      textRange: { start: selectedStart, end: selectedStart + 'value'.length },
    });
  });

  it('rejects byte ranges inside a UTF-8 character or inconsistent with the cited lines', () => {
    const contents = 'a✨\nsecond';
    const hash = 'd'.repeat(64);
    const validStart = Buffer.byteLength('a');
    const validEnd = validStart + Buffer.byteLength('✨');
    assert.deepEqual(resolveCodeLensCitation({
      startLine: 1, endLine: 1, sourceSha256: hash, startByte: validStart + 1, endByteExclusive: validEnd,
    }, undefined, hash, 2, contents), {
      status: 'unavailable', range: null, reason: 'invalid-byte-range',
    });
    assert.deepEqual(resolveCodeLensCitation({
      startLine: 2, endLine: 2, sourceSha256: hash, startByte: validStart, endByteExclusive: validEnd,
    }, undefined, hash, 2, contents), {
      status: 'unavailable', range: null, reason: 'invalid-byte-range',
    });
    assert.deepEqual(resolveCodeLensCitation({
      startLine: 1, endLine: 1, sourceSha256: hash, startByte: validStart, endByteExclusive: validEnd,
    }, undefined, 'b'.repeat(64), 2, contents), {
      status: 'stale', range: null, expectedSha256: hash, actualSha256: 'b'.repeat(64),
    });
  });

  it('surfaces malformed, hashless, and absent citation anchors without highlighting a range', () => {
    const malformedAnchors = [
      [[], 'invalid-anchor'],
      [{ startLine: 0, endLine: 1 }, 'invalid-line-range'],
      [{ startLine: 1, endLine: 2, sourceSha256: 'not-a-sha' }, 'invalid-source-hash'],
    ];
    for (const [sourceRange, expectedIssue] of malformedAnchors) {
      const route = navigateToRoute({ current: { workspace: 'atlas', projectId: 'p', snapshotId: 's' }, back: [], forward: [] }, {
        workspace: 'code-lens', projectId: 'p', snapshotId: 's', relativePath: 'src/main.ts', sourceRange,
      });
      const result = resolveCodeLensCitation(route.current.sourceRange, route.current.sourceRangeIssue, 'a'.repeat(64), 10);
      assert.equal(route.current.sourceRangeIssue, expectedIssue);
      assert.equal(result.status, 'unavailable');
      assert.equal(result.range, null);
    }

    assert.deepEqual(resolveCodeLensCitation({ startLine: 1, endLine: 1 }, undefined, 'a'.repeat(64), 1), {
      status: 'unavailable', range: null, reason: 'missing-source-hash',
    });
    assert.deepEqual(resolveCodeLensCitation(undefined, undefined, 'a'.repeat(64), 1), {
      status: 'none', range: null,
    });
    assert.deepEqual(resolveCodeLensCitation({ startLine: 1, endLine: 1, sourceSha256: 'a'.repeat(64) }, undefined, undefined, 0), {
      status: 'pending', range: null,
    });
  });

  it('classifies unavailable snapshot sources into explicit states', () => {
    assert.equal(classifyCodeLensUnavailableState({ contentKind: 'binary', manifestLoaded: true }), 'binary');
    assert.equal(classifyCodeLensUnavailableState({ contentKind: 'invalidEncoding', manifestLoaded: true }), 'invalid-encoding');
    assert.equal(classifyCodeLensUnavailableState({ skippedReason: 'fileByteLimit', manifestLoaded: true }), 'oversized');
    assert.equal(classifyCodeLensUnavailableState({ skippedReason: 'totalByteLimit', manifestLoaded: true }), 'oversized');
    assert.equal(classifyCodeLensUnavailableState({ skippedReason: 'secretLike', manifestLoaded: true }), 'uncaptured');
    assert.equal(classifyCodeLensUnavailableState({ manifestLoaded: true }), 'missing');
    assert.equal(classifyCodeLensUnavailableState({ contentKind: 'text', manifestLoaded: true }), 'read-error');
    assert.equal(classifyCodeLensUnavailableState({ manifestLoaded: false }), 'read-error');
  });

  it('keeps indexed relationship type and confidence explicit for related-code links', () => {
    assert.equal(codeLensRelationshipLabel('testReference', 'high'), 'testReference · confidence: high');
    assert.equal(codeLensRelationshipLabel('imports', 'unresolved'), 'imports · confidence: unresolved');
  });
});
