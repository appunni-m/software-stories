import test from 'node:test';
import assert from 'node:assert/strict';
import { chapterStudioDraftMatchesRoute } from '../src/lib/chapterStudioDraft.ts';

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
