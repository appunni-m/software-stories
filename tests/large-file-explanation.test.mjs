import test from 'node:test';
import assert from 'node:assert/strict';
import { createLargeFileExplanation, largeFileExplanationContexts, requiresBoundedFileExplanation, WHOLE_FILE_EXPLANATION_LIMIT } from '../src/lib/largeFileExplanation.ts';

const file = (overrides = {}) => ({
  kind: 'file', contentKind: 'text', secretLike: false,
  sizeBytes: WHOLE_FILE_EXPLANATION_LIMIT, ...overrides,
});

test('whole-file requests at the context limit keep the existing chapter path', () => {
  assert.equal(requiresBoundedFileExplanation(file()), false);
  assert.equal(requiresBoundedFileExplanation(file({ sizeBytes: WHOLE_FILE_EXPLANATION_LIMIT + 1 })), true);
});

test('only captured readable non-secret files use bounded range explanation', () => {
  assert.equal(requiresBoundedFileExplanation(undefined), false);
  assert.equal(requiresBoundedFileExplanation(file({ kind: 'directory', sizeBytes: WHOLE_FILE_EXPLANATION_LIMIT + 1 })), false);
  assert.equal(requiresBoundedFileExplanation(file({ contentKind: 'binary', sizeBytes: WHOLE_FILE_EXPLANATION_LIMIT + 1 })), false);
  assert.equal(requiresBoundedFileExplanation(file({ secretLike: true, sizeBytes: WHOLE_FILE_EXPLANATION_LIMIT + 1 })), false);
});

test('large-file handoff keeps the target first and limits attached paths to readable snapshot files', () => {
  const files = [
    { ...file({ sizeBytes: 375_700 }), path: 'src/font.rs', sha256: 'target-hash' },
    ...Array.from({ length: 12 }, (_, index) => ({ ...file(), path: `src/helper-${index}.rs`, sha256: `hash-${index}` })),
    { ...file({ secretLike: true }), path: 'src/secrets.rs', sha256: 'secret-hash' },
    { ...file({ contentKind: 'binary' }), path: 'assets/font.bin', sha256: 'binary-hash' },
  ];
  const pending = createLargeFileExplanation({
    id: 'request-1', projectId: 'project-1', snapshotId: 'snapshot-1', targetPath: 'src/font.rs',
    selectedPaths: ['src/helper-0.rs', 'src/secrets.rs', 'assets/font.bin', ...Array.from({ length: 12 }, (_, index) => `src/helper-${index}.rs`)],
    files,
  });
  assert.ok(pending);
  assert.equal(pending.paths.length, 12);
  assert.equal(pending.paths[0], 'src/font.rs');
  assert.equal(pending.paths.filter((path) => path === 'src/helper-0.rs').length, 1);
  assert.ok(!pending.paths.includes('src/secrets.rs'));
  assert.ok(!pending.paths.includes('assets/font.bin'));
  assert.match(pending.question, /bounded source ranges/);
  assert.equal(Object.hasOwn(pending, 'contents'), false);
});

test('workspace range handoff resolves only readable path and hash metadata', () => {
  const files = [
    { ...file({ sizeBytes: 375_700 }), path: 'src/font.rs', sha256: 'target-hash' },
    { ...file(), path: 'src/helper.rs', sha256: 'helper-hash' },
    { ...file({ secretLike: true }), path: 'src/secrets.rs', sha256: 'secret-hash' },
  ];
  assert.deepEqual(largeFileExplanationContexts({ paths: ['src/font.rs', 'src/helper.rs', 'src/font.rs', 'src/secrets.rs', 'missing.rs'] }, files), [
    { path: 'src/font.rs', sha256: 'target-hash' },
    { path: 'src/helper.rs', sha256: 'helper-hash' },
  ]);
});
