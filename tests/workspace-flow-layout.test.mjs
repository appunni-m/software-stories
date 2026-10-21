import test from 'node:test';
import assert from 'node:assert/strict';
import { layoutFlowNodes } from '../src/workspace/flowLayout.ts';

const nodes = ['entry', 'left', 'right', 'merge'].map((nodeId) => ({nodeId}));
const edges = [
  {fromNodeId: 'entry', toNodeId: 'left'},
  {fromNodeId: 'entry', toNodeId: 'right'},
  {fromNodeId: 'left', toNodeId: 'merge'},
  {fromNodeId: 'right', toNodeId: 'merge'},
];

test('top-to-bottom layout places branch lanes between their entry and merge', () => {
  const points = layoutFlowNodes(nodes, edges, 'topToBottom');
  assert.deepEqual(points.get('entry'), {x: 0, y: 0});
  assert.deepEqual(points.get('left'), {x: -160, y: 260});
  assert.deepEqual(points.get('right'), {x: 160, y: 260});
  assert.deepEqual(points.get('merge'), {x: 0, y: 520});
});

test('left-to-right layout uses the same stable lanes on the other axis', () => {
  const points = layoutFlowNodes(nodes, edges, 'leftToRight');
  assert.deepEqual(points.get('entry'), {x: 0, y: 0});
  assert.deepEqual(points.get('left'), {x: 340, y: -130});
  assert.deepEqual(points.get('right'), {x: 340, y: 130});
  assert.deepEqual(points.get('merge'), {x: 680, y: 0});
});

test('disconnected components remain visible and cycles use deterministic source order', () => {
  const disconnected = layoutFlowNodes(nodes, [edges[0]], 'topToBottom');
  assert.deepEqual(disconnected.get('entry'), {x: -320, y: 0});
  assert.deepEqual(disconnected.get('left'), {x: 0, y: 260});
  assert.deepEqual(disconnected.get('right'), {x: 0, y: 0});
  assert.deepEqual(disconnected.get('merge'), {x: 320, y: 0});

  const cycle = layoutFlowNodes(nodes.slice(0, 3), [
    {fromNodeId: 'entry', toNodeId: 'left'},
    {fromNodeId: 'left', toNodeId: 'right'},
    {fromNodeId: 'right', toNodeId: 'entry'},
  ], 'leftToRight');
  assert.deepEqual([...cycle.values()], [{x: 0, y: 0}, {x: 340, y: 0}, {x: 680, y: 0}]);
});

test('large disconnected and cyclic diagrams use a compact grid instead of a single row', () => {
  const many = Array.from({length: 9}, (_, index) => ({nodeId: `node-${index}`}));
  const disconnected = layoutFlowNodes(many, [], 'leftToRight');
  assert.deepEqual(disconnected.get('node-0'), {x: 0, y: 0});
  assert.deepEqual(disconnected.get('node-2'), {x: 680, y: 0});
  assert.deepEqual(disconnected.get('node-3'), {x: 0, y: 260});
  assert.deepEqual(disconnected.get('node-8'), {x: 680, y: 520});

  const cycle = layoutFlowNodes(many, many.map((node, index) => ({
    fromNodeId: node.nodeId,
    toNodeId: many[(index + 1) % many.length].nodeId,
  })), 'topToBottom');
  assert.deepEqual(cycle.get('node-0'), {x: 0, y: 0});
  assert.deepEqual(cycle.get('node-3'), {x: 320, y: 0});
  assert.deepEqual(cycle.get('node-8'), {x: 640, y: 520});
});

test('long branching paths wrap into readable snake-shaped rank bands', () => {
  const path = Array.from({length: 9}, (_, index) => ({nodeId: `step-${index}`}));
  const edges = [
    [0, 1], [1, 2], [2, 3], [2, 4], [3, 5], [4, 5], [5, 6], [6, 7], [7, 8],
  ].map(([from, to]) => ({fromNodeId: path[from].nodeId, toNodeId: path[to].nodeId}));
  const points = layoutFlowNodes(path, edges, 'leftToRight');
  assert.deepEqual(points.get('step-0'), {x: 0, y: 0});
  assert.deepEqual(points.get('step-3'), {x: 1020, y: -130});
  assert.deepEqual(points.get('step-4'), {x: 1020, y: 130});
  assert.deepEqual(points.get('step-5'), {x: 1020, y: 460});
  assert.deepEqual(points.get('step-8'), {x: 0, y: 460});

  const shuffled = layoutFlowNodes([...path].reverse(), edges, 'leftToRight');
  assert.deepEqual(shuffled.get('step-0'), points.get('step-0'));
  assert.deepEqual(shuffled.get('step-8'), points.get('step-8'));
  assert.equal(shuffled.get('step-3').x, points.get('step-3').x);
  assert.equal(shuffled.get('step-4').x, points.get('step-4').x);
  assert.deepEqual(new Set([shuffled.get('step-3').y, shuffled.get('step-4').y]), new Set([-130, 130]));
});
