import test from 'node:test';
import assert from 'node:assert/strict';
import { chapterStudioCanGenerate, chapterStudioRowStatus, chapterStudioStatus, chapterStudioStatusLabel } from '../src/lib/chapterStudioStatus.ts';

const revision = (snapshotId, lifecycle, revisionId = `${snapshotId}-${lifecycle}`) => ({ snapshotId, lifecycle, revisionId });

test('the selected chapter status distinguishes history availability and chapter lifecycle', () => {
  const current = 'snapshot-current';
  const cases = [
    ['loading', 'loading', false, null, null],
    ['unavailable', 'unavailable', false, null, null],
    ['missing', 'ready', false, null, null],
    ['draft', 'ready', true, revision(current, 'draft'), revision(current, 'draft')],
    ['rejected', 'ready', true, revision(current, 'rejected'), revision(current, 'rejected')],
    ['accepted', 'ready', true, revision(current, 'accepted'), revision(current, 'accepted')],
    ['stale', 'ready', true, revision('snapshot-old', 'accepted'), revision(current, 'draft')],
    ['stale', 'ready', true, null, null],
  ];
  for (const [expected, availability, hasHistory, selected, latest] of cases) {
    assert.equal(chapterStudioStatus(availability, hasHistory, selected, current, latest), expected);
  }
  assert.deepEqual(cases.map(([status]) => chapterStudioStatusLabel(status)), [
    'Loading', 'History unavailable', 'Needs explanation', 'Draft', 'Rejected', 'Accepted', 'Older snapshot', 'Older snapshot',
  ]);
});

test('the target list reports its newest current version while the review pane may inspect an older one', () => {
  const current = revision('snapshot-current', 'draft');
  assert.equal(chapterStudioRowStatus('ready', true, current), 'draft');
  assert.equal(chapterStudioStatus('ready', true, revision('snapshot-old', 'accepted'), 'snapshot-current', current), 'stale');
  assert.equal(chapterStudioRowStatus('ready', true, null), 'stale');
  assert.equal(chapterStudioRowStatus('ready', false, null), 'missing');
  assert.equal(chapterStudioRowStatus('loading', false, null), 'loading');
  assert.equal(chapterStudioRowStatus('unavailable', false, null), 'unavailable');
});

test('known older-snapshot chapters stay stale during an unavailable current-history read', () => {
  const currentSnapshot = 'snapshot-current';
  const knownOlder = revision('snapshot-old', 'accepted', 'older-accepted');

  assert.equal(chapterStudioStatus('unavailable', true, knownOlder, currentSnapshot, null), 'stale');
  assert.equal(chapterStudioRowStatus('unavailable', true, null), 'stale');
  assert.equal(chapterStudioStatusLabel('stale'), 'Older snapshot');
  assert.equal(chapterStudioCanGenerate('unavailable', 'stale'), false);
});

test('Chapter Studio offers generation only when a completed read proves missing or stale history', () => {
  for (const status of ['loading', 'unavailable', 'draft', 'rejected', 'accepted']) {
    assert.equal(chapterStudioCanGenerate('ready', status), false, `${status} must not start generation`);
  }
  assert.equal(chapterStudioCanGenerate('ready', 'missing'), true);
  assert.equal(chapterStudioCanGenerate('ready', 'stale'), true);
  for (const availability of ['loading', 'unavailable']) {
    assert.equal(chapterStudioCanGenerate(availability, 'missing'), false);
    assert.equal(chapterStudioCanGenerate(availability, 'stale'), false);
  }
});
