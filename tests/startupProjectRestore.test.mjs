import assert from 'node:assert/strict';
import { test } from 'node:test';
import { startupProjectToRestore } from '../src/lib/startupProjectRestore.ts';

const input = {
  persistedProjectId: 'fontdone',
  recentProjectIds: ['pillow-rs', 'fontdone'],
  automaticRestoreAllowed: true,
  currentProjectId: null,
  projectOpening: false,
};

test('restores the last project when it is still in Recents', () => {
  assert.equal(startupProjectToRestore(input), 'fontdone');
});

test('does not restore an unavailable or removed recent project', () => {
  assert.equal(startupProjectToRestore({ ...input, recentProjectIds: ['pillow-rs'] }), null);
  assert.equal(startupProjectToRestore({ ...input, persistedProjectId: null }), null);
  assert.equal(startupProjectToRestore({ ...input, persistedProjectId: ' fontdone ' }), null);
  assert.equal(startupProjectToRestore({ ...input, persistedProjectId: 'x'.repeat(257), recentProjectIds: ['x'.repeat(257)] }), null);
  assert.equal(startupProjectToRestore({ ...input, persistedProjectId: 'fontdone\n' }), null);
});

test('an explicit startup choice takes precedence over automatic restore', () => {
  assert.equal(startupProjectToRestore({ ...input, automaticRestoreAllowed: false }), null);
  assert.equal(startupProjectToRestore({ ...input, currentProjectId: 'other' }), null);
  assert.equal(startupProjectToRestore({ ...input, projectOpening: true }), null);
});
