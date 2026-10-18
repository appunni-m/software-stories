import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { workspaceReadiness } from '../src/lib/workspaceReadiness.ts';

const empty = { recentsPending: false, projectOpening: false, projectId: null, projectAvailable: false, snapshotId: null, history: null, manifest: null };
const project = { ...empty, projectId: 'p', projectAvailable: true };
const snapshot = { ...project, snapshotId: 's' };

describe('saved workspace readiness', () => {
  it('keeps onboarding out of the initial lookup and saved-project opening', () => {
    assert.equal(workspaceReadiness({ ...empty, recentsPending: true }), 'restoring');
    assert.equal(workspaceReadiness({ ...empty, projectOpening: true }), 'restoring');
    assert.equal(workspaceReadiness(project), 'loading-history');
    assert.equal(workspaceReadiness(snapshot), 'loading-snapshot');
    assert.equal(workspaceReadiness({ ...snapshot, manifest: { projectId: 'p', snapshotId: 's', status: 'ready' } }), 'ready');
  });

  it('allows first use, cancelled opening, and projects with no captures', () => {
    assert.equal(workspaceReadiness(empty), 'ready');
    assert.equal(workspaceReadiness({ ...project, history: { projectId: 'p', status: 'ready' } }), 'ready');
  });

  it('does not let an old project or snapshot satisfy the new selection', () => {
    assert.equal(workspaceReadiness({ ...project, history: { projectId: 'other', status: 'ready' } }), 'loading-history');
    for (const manifest of [
      { projectId: 'other', snapshotId: 's', status: 'ready' },
      { projectId: 'p', snapshotId: 'old', status: 'ready' },
      { projectId: 'p', snapshotId: 's', status: 'loading' },
    ]) assert.equal(workspaceReadiness({ ...snapshot, manifest }), 'loading-snapshot');
  });

  it('makes failures recoverable instead of waiting forever or showing onboarding', () => {
    assert.equal(workspaceReadiness({ ...project, projectAvailable: false, recentsPending: true }), 'restoring');
    assert.equal(workspaceReadiness({ ...project, projectAvailable: false }), 'project-error');
    assert.equal(workspaceReadiness({ ...project, history: { projectId: 'p', status: 'error' } }), 'history-error');
    assert.equal(workspaceReadiness({ ...snapshot, manifest: { projectId: 'p', snapshotId: 's', status: 'error' } }), 'snapshot-error');
    assert.equal(workspaceReadiness({ ...snapshot, manifest: null }), 'loading-snapshot', 'clearing a failed load allows retry');
  });

  it('opens captured source independently of background lookups or another folder picker', () => {
    assert.equal(workspaceReadiness({ ...snapshot, recentsPending: true, projectOpening: true,
      history: { projectId: 'p', status: 'error' }, manifest: { projectId: 'p', snapshotId: 's', status: 'ready' } }), 'ready');
  });
});
