import test from 'node:test';
import assert from 'node:assert/strict';
import { nextFocusableCommitIndex } from '../src/workspace/gitCommitNavigation.ts';

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
