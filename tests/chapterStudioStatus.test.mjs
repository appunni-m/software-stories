import test from 'node:test';
import assert from 'node:assert/strict';
import { chapterStudioRowStatus, chapterStudioStatus, chapterStudioStatusLabel } from '../src/lib/chapterStudioStatus.ts';

const revision = (snapshotId, lifecycle, revisionId = `${snapshotId}-${lifecycle}`) => ({ snapshotId, lifecycle, revisionId });

test('the selected chapter status distinguishes missing, draft, rejected, stale and accepted', () => {
  const current = 'snapshot-current';
  const cases = [
    ['missing', false, null, null],
    ['draft', true, revision(current, 'draft'), revision(current, 'draft')],
    ['rejected', true, revision(current, 'rejected'), revision(current, 'rejected')],
    ['accepted', true, revision(current, 'accepted'), revision(current, 'accepted')],
    ['stale', true, revision('snapshot-old', 'accepted'), revision(current, 'draft')],
    ['stale', true, null, null],
  ];
  for (const [expected, hasHistory, selected, latest] of cases) {
    assert.equal(chapterStudioStatus(hasHistory, selected, current, latest), expected);
  }
  assert.deepEqual(cases.map(([status]) => chapterStudioStatusLabel(status)), [
    'Needs explanation', 'Draft', 'Rejected', 'Accepted', 'Older snapshot', 'Older snapshot',
  ]);
});

test('the target list reports its newest current version while the review pane may inspect an older one', () => {
  const current = revision('snapshot-current', 'draft');
  assert.equal(chapterStudioRowStatus(true, current), 'draft');
  assert.equal(chapterStudioStatus(true, revision('snapshot-old', 'accepted'), 'snapshot-current', current), 'stale');
  assert.equal(chapterStudioRowStatus(true, null), 'stale');
  assert.equal(chapterStudioRowStatus(false, null), 'missing');
});
