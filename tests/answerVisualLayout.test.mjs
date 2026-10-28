import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { sharedSourceBranch } from '../src/lib/answerVisualLayout.ts';

const edge = (edgeId, fromNodeId, toNodeId) => ({
  edgeId, fromNodeId, toNodeId, label: edgeId, evidenceAnchorIds: [],
});

test('groups multiple visible outcomes that share one source', () => {
  const edges = [edge('gray', 'image', 'gray-format'), edge('rgb', 'image', 'rgb-format')];
  assert.deepEqual(sharedSourceBranch(edges), { sourceNodeId: 'image', edges });
});

test('leaves single or mixed-source relationships in the ordinary relation list', () => {
  assert.equal(sharedSourceBranch([edge('one', 'image', 'format')]), null);
  assert.equal(sharedSourceBranch([edge('a', 'image', 'gray'), edge('b', 'decoder', 'rgb')]), null);
});

test('renders shared-source outcomes as one source with linked branches', () => {
  const component = readFileSync(new URL('../src/workspace/AnswerVisualPreview.tsx', import.meta.url), 'utf8');
  const css = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
  assert.match(component, /const sourceBranch = sharedSourceBranch\(shownEdges\)/u);
  assert.match(component, /One source with multiple outcomes/u);
  assert.match(component, /sourceBranch\.edges\.map/u);
  assert.match(component, /onClick=\{\(\) => openNode\(visual, to\.nodeId\)\}/u);
  assert.match(css, /\.dw-answer-visual-fork\s*\{display:grid;grid-template-columns:/u);
  assert.match(css, /@container guide-pane \(max-width: 700px\)[\s\S]*?\.dw-answer-visual-fork \{grid-template-columns:minmax\(0,1fr\)/u);
});
