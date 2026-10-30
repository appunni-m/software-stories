import test from 'node:test';
import assert from 'node:assert/strict';
import { commitHistoryEmptyMessage, nextFocusableCommitIndex } from '../src/workspace/gitCommitNavigation.ts';

const commits = [
  { parents: ['parent'] },
  { parents: ['parent'] },
  { parents: [] }, // The initial commit is visible but cannot be compared to a parent.
  { parents: ['parent'] },
];

test('commit arrows move to adjacent comparable commits and skip the initial commit', () => {
  assert.equal(nextFocusableCommitIndex(commits, 0, -1), 0);
  assert.equal(nextFocusableCommitIndex(commits, 0, 1), 1);
  assert.equal(nextFocusableCommitIndex(commits, 1, 1), 3);
  assert.equal(nextFocusableCommitIndex(commits, 3, -1), 1);
  assert.equal(nextFocusableCommitIndex(commits, 3, 1), 3);
});

test('commit navigation is stable for empty, all-initial, and invalid positions', () => {
  assert.equal(nextFocusableCommitIndex([], 0, 1), 0);
  assert.equal(nextFocusableCommitIndex([{ parents: [] }], 0, 1), 0);
  assert.equal(nextFocusableCommitIndex(commits, -1, 1), -1);
  assert.equal(nextFocusableCommitIndex(commits, commits.length, -1), commits.length);
});

test('commit history gives useful empty and initial-only guidance only after loading completes', () => {
  const ready = { loaded: true, loading: false, stopped: false, error: null };
  assert.equal(commitHistoryEmptyMessage([], { ...ready, loaded: false }), null, 'do not flash an empty state before the first read');
  assert.equal(commitHistoryEmptyMessage([], { ...ready, loading: true }), null);
  assert.equal(commitHistoryEmptyMessage([], { ...ready, stopped: true }), null);
  assert.equal(commitHistoryEmptyMessage([], { ...ready, error: 'Git unavailable' }), null);
  assert.match(commitHistoryEmptyMessage([], ready), /No commits yet.*create the first commit.*refresh/i);
  assert.match(commitHistoryEmptyMessage([{ parents: [], isHead: true }], ready), /initial commit.*create another commit/i);
  assert.equal(commitHistoryEmptyMessage(commits, ready), null, 'a comparable history keeps its commit list');
});
