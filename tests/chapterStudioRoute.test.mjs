import test from 'node:test';
import assert from 'node:assert/strict';
import { resolveChapterStudioRoute } from '../src/lib/chapterStudioRoute.ts';

const savedRevision = {
  chapterId: 'chapter:src/font.rs',
  revisionId: 'revision:src/font.rs:1',
  revisionNumber: 1,
  snapshotId: 'snapshot-1',
  lifecycle: 'draft',
  target: { targetId: 'target:src/font.rs', scope: { kind: 'file', path: 'src/font.rs' } },
};
const target = {
  key: 'target:src/font.rs',
  targetId: 'target:src/font.rs',
  paths: ['src/font.rs'],
  revisions: [savedRevision],
  latestCurrent: savedRevision,
  latestAny: savedRevision,
  missing: false,
};

test('waits for history before deciding that a routed chapter is missing', () => {
  assert.deepEqual(resolveChapterStudioRoute('chapter:src/font.rs', 'revision:src/font.rs:1', 'loading', []), { kind: 'loading' });
  assert.deepEqual(resolveChapterStudioRoute('chapter:src/font.rs', 'revision:src/font.rs:1', 'unavailable', []), { kind: 'unavailable' });
});

test('resolves the exact chapter revision and never substitutes the latest revision', () => {
  assert.deepEqual(resolveChapterStudioRoute('chapter:src/font.rs', 'revision:src/font.rs:1', 'ready', [target]), {
    kind: 'resolved', targetKey: target.key, revisionId: savedRevision.revisionId,
  });
  assert.deepEqual(resolveChapterStudioRoute('chapter:src/font.rs', 'revision:missing', 'ready', [target]), {
    kind: 'revision-missing', targetKey: target.key,
  });
  assert.deepEqual(resolveChapterStudioRoute('chapter:missing', 'revision:missing', 'ready', [target]), { kind: 'chapter-missing' });
  assert.deepEqual(resolveChapterStudioRoute(null, 'revision:orphaned', 'ready', [target]), { kind: 'chapter-missing' });
});

test('uses the latest saved revision only when the route does not name a revision', () => {
  assert.deepEqual(resolveChapterStudioRoute('chapter:src/font.rs', null, 'ready', [target]), {
    kind: 'resolved', targetKey: target.key, revisionId: savedRevision.revisionId,
  });
  assert.deepEqual(resolveChapterStudioRoute(null, null, 'ready', [target]), { kind: 'none' });
});
