import test from 'node:test';
import assert from 'node:assert/strict';
import { requiresBoundedFileExplanation, WHOLE_FILE_EXPLANATION_LIMIT } from '../src/lib/largeFileExplanation.ts';

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
