import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { matchesCapturedHead, matchingCapturedHeadPath } from '../src/lib/gitChangeAvailability.ts';

const head = '5249b723719445813b8771a602ea5b96891d4450';
const comparison = { head: { objectId: head } };
const snapshot = { snapshotId: 'snapshot-1', head, paths: ['README.md', 'src/font.rs'] };

describe('captured source links from Git changes', () => {
  it('opens a path only when the selected immutable capture matches the comparison head', () => {
    assert.equal(matchesCapturedHead(comparison, snapshot), true);
    assert.equal(matchesCapturedHead({ head: { objectId: 'another-commit' } }, snapshot), false);
    assert.equal(matchingCapturedHeadPath({ headPath: 'src/font.rs', basePath: 'src/font.rs' }, comparison, snapshot), 'src/font.rs');
    assert.equal(matchingCapturedHeadPath({ headPath: 'src/font.rs', basePath: 'src/font.rs' }, { head: { objectId: 'another-commit' } }, snapshot), null);
  });

  it('does not offer a source link for an uncaptured or deleted head path', () => {
    assert.equal(matchingCapturedHeadPath({ headPath: 'large.tsv', basePath: 'large.tsv' }, comparison, snapshot), null);
    assert.equal(matchingCapturedHeadPath({ headPath: null, basePath: 'removed.rs' }, comparison, snapshot), null);
    assert.equal(matchesCapturedHead(comparison, { ...snapshot, paths: [] }), true);
  });

  it('requires complete snapshot and head identities', () => {
    assert.equal(matchingCapturedHeadPath({ headPath: 'src/font.rs', basePath: 'src/font.rs' }, comparison, null), null);
    assert.equal(matchingCapturedHeadPath({ headPath: 'src/font.rs', basePath: 'src/font.rs' }, { head: null }, snapshot), null);
    assert.equal(matchingCapturedHeadPath({ headPath: 'src/font.rs', basePath: 'src/font.rs' }, comparison, { ...snapshot, head: null }), null);
  });
});
