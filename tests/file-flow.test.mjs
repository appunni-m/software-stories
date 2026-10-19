import test from 'node:test';
import assert from 'node:assert/strict';
import { edgesForFileFlowSymbol, focusedFileFlowSymbol } from '../src/workspace/fileFlow.ts';

const symbol = (symbolId, name, startLine, endLine, sourceSha256 = 'hash-a') => ({
  symbolId, name, path: 'src/font.rs', language: 'rust', kind: 'function', qualifiedName: name,
  declarationIdentity: symbolId, signatureSha256: `signature-${symbolId}`, isExported: false, isTest: false,
  anchor: { path: 'src/font.rs', sourceSha256, startByte: 0, endByte: 100, startLine, endLine, startColumn: 0, endColumn: 0 },
});

test('file flow resolves the narrowest indexed symbol containing the selected source range', () => {
  const outer = symbol('outer', 'Font', 4000, 4300);
  const selected = symbol('selected', 'memory_face', 4053, 4100);
  const siblings = symbol('sibling', 'pcf_face', 4110, 4180);
  assert.equal(focusedFileFlowSymbol([outer, siblings, selected], 'src/font.rs', {
    path: 'src/font.rs', sha256: 'hash-a', from: 4055, to: 4060,
  }), selected);
});

test('file flow refuses a focus from another path, source revision, or unmatched range', () => {
  const selected = symbol('selected', 'memory_face', 4053, 4100);
  assert.equal(focusedFileFlowSymbol([selected], 'src/font.rs', { path: 'src/api.rs', from: 4053 }), null);
  assert.equal(focusedFileFlowSymbol([selected], 'src/font.rs', { path: 'src/font.rs', sha256: 'hash-b', from: 4053 }), null);
  assert.equal(focusedFileFlowSymbol([selected], 'src/font.rs', { path: 'src/font.rs', from: 4200 }), null);
});

test('selected-symbol flow keeps incoming and outgoing edges for that symbol only', () => {
  const selected = symbol('selected', 'memory_face', 4053, 4100);
  const edges = [
    { sourceId: 'selected', targetId: 'callee' },
    { sourceId: 'caller', targetId: 'selected' },
    { sourceId: 'sibling', targetId: 'other' },
  ];
  assert.deepEqual(edgesForFileFlowSymbol(edges, selected), edges.slice(0, 2));
});
