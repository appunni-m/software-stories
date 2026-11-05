import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const workspaceSource = await readFile(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
const workspaceStyles = await readFile(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
const appSource = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');

test('Chapter Studio stays inside the persistent code workspace', () => {
  assert.match(workspaceSource, /return manifest\s*\? <SavedWorkspace/u,
    'a captured-repository tool route should keep the shared workspace mounted');
  assert.doesNotMatch(workspaceSource, /props\.workspace !== 'chapter-studio'/u,
    'Chapter Studio must not bypass the shared workspace');
  assert.match(workspaceSource, /const tool = workspace==='changes'\|\|workspace==='chapter-studio';/u);
  assert.match(workspaceSource, /dw-tool-content dw-tool-content-chapter-studio/u,
    'the editor should use the shared contextual-tool pane');
  assert.match(workspaceSource, /<Panel id="source"[\s\S]*?<Panel id="explanations"/u,
    'the shared reading workspace should retain its resizable source pane');
  assert.match(appSource, /currentWorkspace === 'chapter-studio'[\s\S]*?<ChapterStudioScreen/u,
    'the existing editor route remains the tool content');

  assert.match(workspaceStyles, /\.dw-tool-content-chapter-studio\s*\{[^}]*display:flex;[^}]*overflow:hidden;/u);
  assert.match(workspaceStyles, /\.dw-tool-content-chapter-studio>\.chapter-studio-screen\s*\{[^}]*flex:1;[^}]*min-height:0;/u,
    'the editor must fit its pane so its file list and review area can scroll independently');
  assert.match(workspaceStyles, /\.dw-tool-content-chapter-studio \.chapter-studio-top-actions\s*\{display:none;\}/u,
    'the editor should not repeat Guide navigation and snapshot details already in the workspace');
});
