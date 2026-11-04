import test from 'node:test';
import assert from 'node:assert/strict';
import { buildChapterStudioTargets, chapterStudioHasReadableCurrentSource } from '../src/lib/chapterStudioTargets.ts';
import { chapterStudioCanGenerate, chapterStudioStatus } from '../src/lib/chapterStudioStatus.ts';

const revision = (path, snapshotId, revisionNumber = 1, scope = { kind: 'file', path }) => ({
  chapterId: `chapter:${path}`,
  revisionId: `revision:${path}:${snapshotId}:${revisionNumber}`,
  revisionNumber,
  snapshotId,
  lifecycle: 'accepted',
  target: { targetId: `target:${path}`, scope },
});

const row = (path, disposition = 'captured', contentKind = 'text') => ({
  path, disposition, kind: 'file', contentKind,
});

test('old file chapters stay reviewable when their current path is deleted, skipped, excluded, or binary', () => {
  const currentSnapshot = 'snapshot-current';
  const prior = [
    revision('src/deleted.rs', 'snapshot-before'),
    revision('src/skipped.rs', 'snapshot-before'),
    revision('src/excluded.rs', 'snapshot-before'),
    revision('src/binary.rs', 'snapshot-before'),
    revision('src/readable.rs', 'snapshot-before'),
  ];
  const rows = [
    row('src/skipped.rs', 'skipped'),
    row('src/excluded.rs', 'excluded'),
    row('src/binary.rs', 'captured', 'binary'),
    row('src/readable.rs', 'captured', 'text'),
  ];

  const targets = buildChapterStudioTargets(rows, prior, currentSnapshot);
  for (const old of prior) {
    const target = targets.find((candidate) => candidate.targetId === old.target.targetId);
    assert.ok(target, `${old.target.scope.path} must remain in Chapter Studio`);
    assert.equal(target.latestCurrent, null);
    assert.equal(target.latestAny.revisionId, old.revisionId);
    assert.equal(target.latestAny.snapshotId, 'snapshot-before');
    assert.equal(target.missing, false);
    assert.equal(chapterStudioStatus('ready', true, target.latestAny, currentSnapshot, target.latestCurrent), 'stale');
  }

  assert.equal(chapterStudioHasReadableCurrentSource('src/deleted.rs', rows), false);
  assert.equal(chapterStudioHasReadableCurrentSource('src/skipped.rs', rows), false);
  assert.equal(chapterStudioHasReadableCurrentSource('src/excluded.rs', rows), false);
  assert.equal(chapterStudioHasReadableCurrentSource('src/binary.rs', rows), false);
  assert.equal(chapterStudioHasReadableCurrentSource('src/readable.rs', rows), true);
  assert.equal(
    chapterStudioCanGenerate('ready', 'stale') && chapterStudioHasReadableCurrentSource('src/deleted.rs', rows),
    false,
    'stale history alone does not offer generation without readable current source',
  );
});

test('new captured text targets remain available for generation without prior history', () => {
  const targets = buildChapterStudioTargets([row('src/new.rs'), row('src/unknown.rs', 'captured', 'unknown')], [], 'snapshot-current');
  assert.equal(targets.find((target) => target.paths[0] === 'src/new.rs')?.missing, true);
  assert.equal(chapterStudioHasReadableCurrentSource('src/new.rs', [row('src/new.rs')]), true);
  assert.equal(chapterStudioHasReadableCurrentSource('src/unknown.rs', [row('src/unknown.rs', 'captured', 'unknown')]), true);
});

test('group and repository overview histories remain available', () => {
  const group = revision('group', 'snapshot-before', 2, { kind: 'group', paths: ['src/a.rs', 'src/b.rs'] });
  const overview = revision('overview', 'snapshot-before', 1, { kind: 'overview' });
  const targets = buildChapterStudioTargets([], [group, overview], 'snapshot-current');
  assert.deepEqual(targets.map((target) => target.key), ['overview', group.target.targetId]);
  assert.equal(targets[0].overview, true);
  assert.equal(targets[1].paths.join(','), 'src/a.rs,src/b.rs');
});
