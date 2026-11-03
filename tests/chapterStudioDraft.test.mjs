import test from 'node:test';
import assert from 'node:assert/strict';
import { CHAPTER_SCHEMA_VERSION } from '../src/lib/chapterSchema.ts';
import { chapterStudioDraftMatchesRoute, readChapterStudioDraft } from '../src/lib/chapterStudioDraft.ts';

const draft = {
  baseRevisionId: 'revision-original',
  candidate: { chapterId: 'chapter-original' },
};

test('a saved edit restores when its original chapter and revision match the route', () => {
  assert.equal(chapterStudioDraftMatchesRoute(draft, 'chapter-original', 'revision-original'), true);
});

test('a saved edit remains pending when navigation changes its chapter or revision', () => {
  assert.equal(chapterStudioDraftMatchesRoute(draft, 'chapter-other', 'revision-original'), false);
  assert.equal(chapterStudioDraftMatchesRoute(draft, 'chapter-original', 'revision-other'), false);
});

test('a saved edit restores on a general Chapter Studio route without a selected chapter', () => {
  assert.equal(chapterStudioDraftMatchesRoute(draft, null, null), true);
});

test('unsupported saved edit schema versions remain preserved for recovery', () => {
  for (const candidate of [
    { schemaVersion: CHAPTER_SCHEMA_VERSION + 1 },
    { schemaVersion: CHAPTER_SCHEMA_VERSION, generationOrigin: { schemaVersion: 7 } },
  ]) {
    const serialized = JSON.stringify({
      version: 1,
      projectId: 'project',
      snapshotId: 'snapshot',
      targetKey: 'file:src/main.ts',
      baseRevisionId: 'revision-original',
      candidate,
    });
    assert.deepEqual(readChapterStudioDraft(
      serialized, 'project', 'snapshot', null, null,
    ), { status: 'preserved', reason: 'unsupported' });
    assert.equal(JSON.parse(serialized).candidate.schemaVersion, candidate.schemaVersion,
      'reading an unsupported checkpoint must not rewrite its stored version');
  }
});
