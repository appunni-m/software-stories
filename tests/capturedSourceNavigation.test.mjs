import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { normalizeCapturedSourceNavigation } from '../src/lib/capturedSourceNavigation.ts';

const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

test('captured source paths must be safe, bounded relative paths', () => {
  assert.deepEqual(normalizeCapturedSourceNavigation('src/font.rs'), { relativePath: 'src/font.rs' });
  assert.deepEqual(normalizeCapturedSourceNavigation('src/東京/font.rs'), { relativePath: 'src/東京/font.rs' });

  for (const path of [
    '', '/src/font.rs', '../font.rs', 'src/../font.rs', './src/font.rs',
    'src//font.rs', 'src\\font.rs', 'src/\0font.rs', 'src/font.rs\n',
    'x'.repeat(4097), null, 12,
  ]) {
    assert.equal(normalizeCapturedSourceNavigation(path), null, `reject ${String(path).slice(0, 40)}`);
  }
});

test('source ranges are canonicalized before a captured source route is returned', () => {
  assert.deepEqual(normalizeCapturedSourceNavigation('src/font.rs', {
    startLine: 12, endLine: 16, extra: 'discarded',
  }), {
    relativePath: 'src/font.rs',
    sourceRange: { startLine: 12, endLine: 16 },
  });

  for (const range of [null, [], { startLine: 0, endLine: 1 }, { startLine: 4, endLine: 3 },
    { startLine: 2, endLine: 3, startByte: 9 },
    { startLine: 2, endLine: 3, startByte: 9, endByteExclusive: 4 }]) {
    assert.equal(normalizeCapturedSourceNavigation('src/font.rs', range), null);
  }
});

test('the workspace rejects malformed links before mutating the navigation route', () => {
  const start = app.indexOf('const openCapturedFile = (relativePath: string');
  const navigation = app.indexOf('setNavigation((state) => {', start);
  assert.ok(start >= 0 && navigation > start);
  const handler = app.slice(start, navigation);
  assert.match(handler, /normalizeCapturedSourceNavigation\(relativePath, sourceRange\)/);
  assert.match(handler, /if \(!sourceLink\)\s*\{[\s\S]*?setNotice\([\s\S]*?return;/);
});
