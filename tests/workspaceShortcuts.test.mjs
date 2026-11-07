import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { workspaceViewForShortcut } from '../src/lib/workspaceShortcuts.ts';

describe('reader view shortcuts', () => {
  it('maps Cmd/Ctrl+1, 2 and 3 to the current Guide, Flow and history tabs', () => {
    assert.equal(workspaceViewForShortcut('1'), 'guide');
    assert.equal(workspaceViewForShortcut('2'), 'flow');
    assert.equal(workspaceViewForShortcut('3'), 'history');
  });

  it('ignores keys that are not reader view shortcuts', () => {
    assert.equal(workspaceViewForShortcut('k'), null);
    assert.equal(workspaceViewForShortcut('4'), null);
  });

  it('routes the global shortcut to Guide Plan and activates the requested reader tab after mount', async () => {
    const app = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    assert.match(app, /const readerView = workspaceViewForShortcut\(key\)/);
    assert.match(app, /focusTarget: `workspace-view:\$\{readerView\}:\$\{focusRequest\}`/);
    assert.match(app, /const workspaceView = focusTarget\.match\(\/\^workspace-view:\(guide\|flow\|history\)\(\?::\|\$\)\/\)/);
    assert.match(app, /if \(workspaceView && target\.getAttribute\('aria-selected'\) !== 'true'\) target\.click\(\)/);
    assert.doesNotMatch(app, /if \(key === '2'\) \{\s*event\.preventDefault\(\);\s*navigationActions\.workspace\('atlas'\)/);
    assert.doesNotMatch(app, /if \(key === '3'\) \{\s*event\.preventDefault\(\);\s*navigationActions\.workspace\('changes'\)/);
  });
});
