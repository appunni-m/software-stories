import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { sharedSourceBranch } from '../src/lib/answerVisualLayout.ts';

const edge = (edgeId, fromNodeId, toNodeId) => ({
  edgeId, fromNodeId, toNodeId, label: edgeId, evidenceAnchorIds: [],
});
const anchor = (anchorId, startByte = 12) => ({
  anchorId, path: 'src/image.rs', snapshotId: 'snapshot', sourceSha256: 'a'.repeat(64),
  startByte, endByteInclusive: startByte + 8, startLine: 4, endLine: 4,
  startColumn: 1, endColumn: 9, excerpt: 'image input',
});
const node = (nodeId, anchorId) => ({
  nodeId, label: 'Image input', detail: null, hotspotId: null,
  sourceAnchorId: anchorId, evidenceAnchorIds: [anchorId],
});

test('groups multiple visible outcomes that share one source', () => {
  const edges = [edge('gray', 'image', 'gray-format'), edge('rgb', 'image', 'rgb-format')];
  assert.deepEqual(sharedSourceBranch(edges), { sourceNodeId: 'image', edges });
});

test('leaves single or mixed-source relationships in the ordinary relation list', () => {
  assert.equal(sharedSourceBranch([edge('one', 'image', 'format')]), null);
  assert.equal(sharedSourceBranch([edge('a', 'image', 'gray'), edge('b', 'decoder', 'rgb')]), null);
});

test('groups repeated source nodes only when their cited source location is the same', () => {
  const edges = [edge('gray', 'image-1', 'gray-format'), edge('rgb', 'image-2', 'rgb-format')];
  const nodes = [node('image-1', 'source-a'), node('image-2', 'source-b')];
  const anchors = new Map([['source-a', anchor('source-a')], ['source-b', anchor('source-b')]]);
  assert.deepEqual(sharedSourceBranch(edges, nodes, anchors), {
    sourceNodeId: 'image-1', edges,
  });

  const differentLocation = new Map([['source-a', anchor('source-a')], ['source-b', anchor('source-b', 40)]]);
  assert.equal(sharedSourceBranch(edges, nodes, differentLocation), null);
  assert.equal(sharedSourceBranch(edges, nodes), null);
});

test('renders shared-source outcomes as one source with linked branches', () => {
  const component = readFileSync(new URL('../src/workspace/AnswerVisualPreview.tsx', import.meta.url), 'utf8');
  const css = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
  assert.match(component, /const sourceBranch = sharedSourceBranch\(shownEdges, visibleNodes, anchorsById/u);
  assert.match(component, /One source with multiple outcomes/u);
  assert.match(component, /sourceBranch\.edges\.map/u);
  assert.match(component, /onClick=\{\(\) => openNode\(visual, to\.nodeId\)\}/u);
  assert.match(css, /\.dw-answer-visual-fork\s*\{display:grid;grid-template-columns:/u);
  assert.match(css, /@container guide-pane \(max-width: 700px\)[\s\S]*?\.dw-answer-visual-fork \{grid-template-columns:minmax\(0,1fr\)/u);
});
