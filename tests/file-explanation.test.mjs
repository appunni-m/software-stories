import test from 'node:test';
import assert from 'node:assert/strict';
import { createFileExplanation, fileExplanationContexts } from '../src/lib/fileExplanation.ts';

const file = (path, overrides = {}) => ({
  path,
  kind: 'file',
  contentKind: 'text',
  secretLike: false,
  sha256: `hash:${path}`,
  ...overrides,
});

test('small and large captured files use one bounded workspace explanation handoff', () => {
  const files = [
    file('src/cart.ts'),
    file('src/font.rs', { sha256: 'font-hash' }),
    ...Array.from({ length: 12 }, (_, index) => file(`src/helper-${index}.rs`)),
    file('.env', { secretLike: true }),
    file('assets/data.bin', { contentKind: 'binary' }),
  ];

  for (const targetPath of ['src/cart.ts', 'src/font.rs']) {
    const pending = createFileExplanation({
      id: `request:${targetPath}`, projectId: 'project-1', snapshotId: 'snapshot-1', targetPath,
      selectedPaths: ['src/helper-0.rs', '.env', 'assets/data.bin', ...Array.from({ length: 12 }, (_, index) => `src/helper-${index}.rs`)],
      files,
    });
    assert.ok(pending);
    assert.equal(pending.paths[0], targetPath);
    assert.equal(pending.paths.length, 12);
    assert.ok(!pending.paths.includes('.env'));
    assert.ok(!pending.paths.includes('assets/data.bin'));
    assert.match(pending.question, /bounded source ranges/);
    assert.equal(Object.hasOwn(pending, 'contents'), false);
  }
});

test('file explanations reject secret, binary, directory and missing targets', () => {
  const files = [
    file('.env', { secretLike: true }),
    file('assets/data.bin', { contentKind: 'binary' }),
    file('src', { kind: 'directory' }),
  ];
  const handoff = (targetPath) => createFileExplanation({
    id: 'request-1', projectId: 'project-1', snapshotId: 'snapshot-1', targetPath, files,
  });
  assert.equal(handoff('.env'), null);
  assert.equal(handoff('assets/data.bin'), null);
  assert.equal(handoff('src'), null);
  assert.equal(handoff('missing.ts'), null);
});

test('workspace handoff resolves only captured readable paths and hashes', () => {
  const files = [
    file('src/font.rs', { sha256: 'font-hash' }),
    file('src/helper.rs'),
    file('.env', { secretLike: true }),
  ];
  assert.deepEqual(fileExplanationContexts({ paths: ['src/font.rs', 'src/helper.rs', 'src/font.rs', '.env', 'missing.rs'] }, files), [
    { path: 'src/font.rs', sha256: 'font-hash' },
    { path: 'src/helper.rs', sha256: 'hash:src/helper.rs' },
  ]);
});
