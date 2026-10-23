import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

test('source citations stay below the editor header when opened or restored after zoom', async () => {
  const source = await readFile(new URL('../src/design/CodePane.tsx', import.meta.url), 'utf8');
  assert.equal((source.match(/EditorView\.scrollIntoView\([^\n]+\{ y: 'start', yMargin: 24 \}/g) ?? []).length, 2);
});

test('source editor preserves code line boundaries for horizontal inspection', async () => {
  const source = await readFile(new URL('../src/design/CodePane.tsx', import.meta.url), 'utf8');
  assert.match(source, /lineNumbers\(\)/);
  assert.doesNotMatch(source, /EditorView\.lineWrapping/);
});
