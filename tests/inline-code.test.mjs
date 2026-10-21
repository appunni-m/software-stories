import test from 'node:test';
import assert from 'node:assert/strict';
import { inlineCodeParts } from '../src/lib/inlineCode.ts';

test('inline code spans keep source text and separate adjacent prose', () => {
  assert.deepEqual(inlineCodeParts('`Font::memory_face` calls `Self::pfr_face`.'), [
    { kind: 'code', text: 'Font::memory_face' },
    { kind: 'text', text: ' calls ' },
    { kind: 'code', text: 'Self::pfr_face' },
    { kind: 'text', text: '.' },
  ]);
});

test('unmatched, escaped, multiline and multi-backtick delimiters stay literal', () => {
  const values = ['unmatched `code', 'escaped \\`code\\`', '```block```', '`two\nlines`'];
  for (const value of values) {
    assert.deepEqual(inlineCodeParts(value), [{ kind: 'text', text: value }]);
  }
});

test('markup-looking source stays text and is never promoted to executable markup', () => {
  const value = '`<img src=x onerror=alert(1)>`';
  assert.deepEqual(inlineCodeParts(value), [{ kind: 'code', text: '<img src=x onerror=alert(1)>' }]);
});
