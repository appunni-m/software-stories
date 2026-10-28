import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  canNavigateBack,
  canNavigateForward,
  initialNavigationState,
  navigateBack,
  navigateForward,
  navigateToRoute,
  navigateToWorkspace,
  replaceCurrentRoute,
  sourceRangeMatchesHash,
} from '../src/lib/navigation.ts';

describe('workspace navigation', () => {
  it('does not add duplicate history when reselecting the current workspace', () => {
    const next = navigateToWorkspace(initialNavigationState, 'atlas');
    assert.equal(next, initialNavigationState);
    assert.equal(canNavigateBack(next), false);
  });

  it('preserves project, snapshot, and source selection between current workspaces', () => {
    const withSource = navigateToRoute(initialNavigationState, {
      workspace: 'atlas',
      projectId: 'project-a',
      snapshotId: 'snapshot-42',
      relativePath: 'src/main.ts',
      symbolId: 'main',
    });
    const guide = navigateToWorkspace(withSource, 'guide-plan');
    assert.equal(guide.current.projectId, 'project-a');
    assert.equal(guide.current.snapshotId, 'snapshot-42');
    assert.equal(guide.current.relativePath, 'src/main.ts');
    assert.equal(guide.current.symbolId, 'main');
  });

  it('restores exact route state on back and forward', () => {
    const atlas = navigateToRoute(initialNavigationState, {
      workspace: 'atlas', projectId: 'p', snapshotId: 's', treeSelection: 'src',
      mapViewport: { x: 30, y: -12, scale: 1.4 },
      focusTarget: 'nav:atlas',
    });
    const sourceVisit = navigateToRoute(atlas, {
      workspace: 'atlas', projectId: 'p', snapshotId: 's', relativePath: 'src/index.ts',
    });
    const back = navigateBack(sourceVisit);
    assert.deepEqual(back.current, atlas.current);
    assert.equal(back.current.focusTarget, 'nav:atlas');
    assert.equal(canNavigateForward(back), true);
    const forward = navigateForward(back);
    assert.deepEqual(forward.current, sourceVisit.current);
  });

  it('restores reader section, narration time, transcript scroll, and focus after a code citation visit', () => {
    const reader = navigateToRoute(initialNavigationState, {
      workspace: 'guide-plan', projectId: 'p', snapshotId: 'accepted-snapshot',
      chapterId: 'chapter-1', chapterRevisionId: 'revision-4', sectionId: 'section-2',
      playbackPosition: 18.25, transcriptPosition: 146, focusTarget: 'reader-citation:anchor-8',
      readerSnapshotBaselineId: 'newer-selected-snapshot',
    });
    const sourceVisit = navigateToRoute(reader, {
      workspace: 'guide-plan', projectId: 'p', snapshotId: 'accepted-snapshot',
      relativePath: 'src/parser.ts', sourceRange: { startLine: 12, endLine: 16 },
      focusTarget: 'workspace-source',
    });
    assert.deepEqual(navigateBack(sourceVisit).current, reader.current);
    assert.deepEqual(navigateForward(navigateBack(sourceVisit)).current, sourceVisit.current);
  });

  it('preserves and validates the source hash attached to a citation range', () => {
    const sourceSha256 = 'a'.repeat(64);
    const sourceRoute = navigateToRoute(initialNavigationState, {
      workspace: 'atlas', projectId: 'p', snapshotId: 's', relativePath: 'src/parser.ts',
      sourceRange: { startLine: 12, endLine: 16, sourceSha256 },
    });
    assert.deepEqual(sourceRoute.current.sourceRange, { startLine: 12, endLine: 16, sourceSha256 });
    assert.equal(sourceRangeMatchesHash(sourceRoute.current.sourceRange, sourceSha256), true);
    assert.equal(sourceRangeMatchesHash(sourceRoute.current.sourceRange, 'b'.repeat(64)), false);
    assert.equal(sourceRangeMatchesHash({ startLine: 12, endLine: 16 }, 'b'.repeat(64)), true);
  });

  it('preserves well-formed half-open byte ranges and rejects malformed byte anchors', () => {
    const sourceSha256 = 'f'.repeat(64);
    const valid = navigateToRoute(initialNavigationState, {
      workspace: 'atlas', projectId: 'p', snapshotId: 's', relativePath: 'src/main.rs',
      sourceRange: { startLine: 4, endLine: 4, sourceSha256, startByte: 18, endByteExclusive: 27 },
    });
    assert.deepEqual(valid.current.sourceRange, {
      startLine: 4, endLine: 4, sourceSha256, startByte: 18, endByteExclusive: 27,
    });

    const malformed = navigateToRoute(initialNavigationState, {
      workspace: 'atlas', projectId: 'p', snapshotId: 's', relativePath: 'src/main.rs',
      sourceRange: { startLine: 4, endLine: 4, sourceSha256, startByte: 27, endByteExclusive: 18 },
    });
    assert.equal(malformed.current.sourceRange, undefined);
    assert.equal(malformed.current.sourceRangeIssue, 'invalid-byte-range');
  });

  it('replaces current player position without adding a navigation history entry', () => {
    const reader = navigateToRoute(initialNavigationState, {
      workspace: 'guide-plan', projectId: 'p', snapshotId: 's', sectionId: 'one', playbackPosition: 0,
    });
    const moved = replaceCurrentRoute(reader, { ...reader.current, sectionId: 'two', playbackPosition: 7.5, transcriptPosition: 28 });
    assert.deepEqual(moved.current, { ...reader.current, sectionId: 'two', playbackPosition: 7.5, transcriptPosition: 28 });
    assert.deepEqual(moved.back, reader.back);
    assert.deepEqual(moved.forward, reader.forward);
  });

  it('preserves an Atlas hotspot selection and the originating reader snapshot baseline', () => {
    const reader = navigateToRoute(initialNavigationState, {
      workspace: 'guide-plan', projectId: 'p', snapshotId: 'accepted-snapshot',
      readerSnapshotBaselineId: 'current-snapshot', chapterId: 'c', chapterRevisionId: 'r', sectionId: 's',
      focusTarget: 'reader-hotspot:node-3',
    });
    const atlas = navigateToRoute(reader, {
      workspace: 'atlas', projectId: 'p', snapshotId: 'accepted-snapshot',
      atlasSelection: { kind: 'symbol', path: 'src/main.ts', symbolId: 'symbol-7' },
      focusTarget: 'atlas-destination',
    });
    assert.deepEqual(navigateBack(atlas).current, reader.current);
    assert.deepEqual(navigateForward(navigateBack(atlas)).current, atlas.current);
  });

  it('restores the explicit local Git ref pair and selected change after visiting another workspace', () => {
    const changes = navigateToRoute(initialNavigationState, {
      workspace: 'changes',
      projectId: 'project-local',
      snapshotId: null,
      baseRef: 'main',
      headRef: 'feature/session-refresh',
      selectedChangePath: 'src/session.ts',
      focusTarget: 'git-change:src/session.ts',
    });
    const atlas = navigateToWorkspace(changes, 'atlas');

    assert.equal(navigateBack(atlas).current.workspace, 'changes');
    assert.equal(navigateBack(atlas).current.baseRef, 'main');
    assert.equal(navigateBack(atlas).current.headRef, 'feature/session-refresh');
    assert.equal(navigateBack(atlas).current.selectedChangePath, 'src/session.ts');
    assert.deepEqual(navigateForward(navigateBack(atlas)).current, atlas.current);
  });

  it('returns from a source citation to the exact Atlas hotspot, map, tree, and search state', () => {
    const sourceSha256 = 'c'.repeat(64);
    const atlas = navigateToRoute(initialNavigationState, {
      workspace: 'atlas', projectId: 'project-8', snapshotId: 'snapshot-19',
      atlasSelection: { kind: 'symbol', path: 'src/router.ts', symbolId: 'symbol:dispatch' },
      treeSelection: 'src/router.ts', searchQuery: 'dispatch',
      mapViewport: { x: -24, y: 88, scale: 1.25 },
      focusTarget: 'atlas-open-symbol:symbol:dispatch',
    });
    const sourceVisit = navigateToRoute(atlas, {
      workspace: 'atlas', projectId: 'project-8', snapshotId: 'snapshot-19',
      relativePath: 'src/router.ts', sourceRange: { startLine: 40, endLine: 43, sourceSha256 },
      focusTarget: 'workspace-source',
    });

    const back = navigateBack(sourceVisit);
    assert.deepEqual(back.current, atlas.current);
    assert.deepEqual(back.current.atlasSelection, { kind: 'symbol', path: 'src/router.ts', symbolId: 'symbol:dispatch' });
    assert.deepEqual(back.current.mapViewport, { x: -24, y: 88, scale: 1.25 });
    assert.equal(back.current.treeSelection, 'src/router.ts');
    assert.equal(back.current.searchQuery, 'dispatch');
    assert.deepEqual(navigateForward(back).current, sourceVisit.current);
  });

  it('clears forward history when the user chooses a new destination', () => {
    const atlas = navigateToRoute(initialNavigationState, { workspace: 'atlas', projectId: null, snapshotId: null });
    const guide = navigateToWorkspace(atlas, 'guide-plan');
    const back = navigateBack(guide);
    const changes = navigateToWorkspace(back, 'changes');
    assert.equal(canNavigateForward(changes), false);
    assert.equal(changes.back.at(-1)?.workspace, 'atlas');
  });

  it('remembers the originating control so Back can return focus there', () => {
    const guide = navigateToWorkspace(initialNavigationState, 'guide-plan', 'nav:atlas');
    assert.equal(guide.current.focusTarget, 'heading:guide-plan');
    assert.equal(navigateBack(guide).current.focusTarget, 'nav:atlas');
  });

});
