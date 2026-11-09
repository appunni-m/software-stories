import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  EMPTY_INLINE_SECTION_EDIT_ENTRY,
  updateInlineSectionEditEntries,
} from '../src/workspace/inlineSectionEditState.ts';

const workspaceSource = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');

test('inline edits live above the workspace panes so source and Guide/Flow switches keep the draft', () => {
  const providerStart = workspaceSource.indexOf('<InlineSectionEditProvider>');
  const readerContextStart = workspaceSource.indexOf('<ReaderProgressContext.Provider', providerStart);
  const providerEnd = workspaceSource.indexOf('</InlineSectionEditProvider>', readerContextStart);
  const tabRoot = workspaceSource.indexOf('<Tabs.Root', readerContextStart);
  const sourcePaneUse = workspaceSource.indexOf('{sourcePane}', providerStart);

  assert.ok(providerStart >= 0 && readerContextStart > providerStart);
  assert.ok(tabRoot > readerContextStart && providerEnd > tabRoot);
  assert.ok(sourcePaneUse > readerContextStart && providerEnd > sourcePaneUse, 'the source pane stays under the same edit-state provider');

  const hookStart = workspaceSource.indexOf('function useInlineSectionEdit(');
  const hookEnd = workspaceSource.indexOf('\nfunction WorkspaceSharingDetails', hookStart);
  const hook = workspaceSource.slice(hookStart, hookEnd);
  assert.match(hook, /useContext\(InlineSectionEditContext\)/);
  assert.doesNotMatch(hook, /useState\(/);
});

test('dirty drafts and validation status stay keyed to their section across view switches', () => {
  const sectionKey = 'revision:old:section:one:section-revision:one';
  const draft = {
    key: sectionKey,
    baseRevisionId: 'revision:old',
    selectedRevisionId: 'revision:old',
    title: 'Updated title',
    prose: 'Unsaved explanation',
    spokenText: 'Unsaved narration',
  };
  const reviseQuestion = 'Explain the failure path with a concrete example.';
  let workspace = { view: 'guide', edits: {} };
  workspace = { ...workspace, edits: updateInlineSectionEditEntries(workspace.edits, sectionKey, (previous) => ({
    ...previous,
    draft,
    reviseQuestion,
    reviseOpen: true,
    busy: true,
    issues: { key: sectionKey, items: ['Awaiting source validation'] },
  })) };

  workspace = { ...workspace, view: 'flow' };
  assert.equal(workspace.view, 'flow');
  assert.deepEqual(workspace.edits[sectionKey], {
    ...EMPTY_INLINE_SECTION_EDIT_ENTRY,
    draft,
    reviseQuestion,
    reviseOpen: true,
    busy: true,
    issues: { key: sectionKey, items: ['Awaiting source validation'] },
  });
  workspace = { ...workspace, view: 'source' };
  assert.equal(workspace.view, 'source');
  assert.equal(workspace.edits[sectionKey].busy, true, 'pending validation stays attached while source is active');
  assert.equal(workspace.edits[sectionKey].draft.prose, 'Unsaved explanation');
  assert.equal(workspace.edits[sectionKey].reviseQuestion, reviseQuestion);
  assert.equal(workspace.edits[sectionKey].reviseOpen, true);
  workspace = { ...workspace, view: 'guide' };
  assert.equal(workspace.edits[sectionKey].draft.prose, 'Unsaved explanation');
});

test('canceling one draft removes only that section state', () => {
  const first = 'revision:one:section:a:section-revision:a';
  const second = 'revision:one:section:b:section-revision:b';
  let entries = {};
  for (const key of [first, second]) {
    entries = updateInlineSectionEditEntries(entries, key, (previous) => ({
      ...previous,
      draft: { key, baseRevisionId: 'revision:one', selectedRevisionId: 'revision:one', title: key, prose: '', spokenText: '' },
    }));
  }

  entries = updateInlineSectionEditEntries(entries, first, () => EMPTY_INLINE_SECTION_EDIT_ENTRY);
  assert.equal(first in entries, false);
  assert.equal(entries[second].draft?.key, second);
});
