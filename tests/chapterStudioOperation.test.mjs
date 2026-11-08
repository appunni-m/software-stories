import assert from 'node:assert/strict';
import test from 'node:test';
import { chapterStudioOperationIsCurrent } from '../src/lib/chapterStudioOperation.ts';

const identity = (overrides = {}) => ({
  projectId: 'project-a',
  snapshotId: 'snapshot-a',
  targetKey: 'chapter-a',
  selectedRevisionId: 'revision-a',
  routeChapterId: 'chapter-a',
  routeRevisionId: 'revision-a',
  routeSectionId: 'section-a',
  editing: false,
  candidate: null,
  ...overrides,
});

test('chapter studio operation results apply only to their original view identity', () => {
  const candidate = { revisionId: 'draft-a' };
  const started = identity({ editing: true, candidate });

  assert.equal(chapterStudioOperationIsCurrent(started, { ...started }), true);

  for (const change of [
    { projectId: 'project-b' },
    { snapshotId: 'snapshot-b' },
    { targetKey: 'chapter-b' },
    { selectedRevisionId: 'revision-b' },
    { routeChapterId: 'chapter-b' },
    { routeRevisionId: 'revision-b' },
    { routeSectionId: 'section-b' },
    { editing: false },
    { candidate: { revisionId: 'draft-a' } },
  ]) {
    assert.equal(
      chapterStudioOperationIsCurrent(started, { ...started, ...change }),
      false,
      `stale result must be ignored after ${Object.keys(change)[0]} changes`,
    );
  }
});
