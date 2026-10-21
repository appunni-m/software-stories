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

test('disconnected components remain visible and small cycles keep stable source order', () => {
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

test('large disconnected diagrams use a compact grid while cycle evidence keeps its forward path', () => {
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
  assert.deepEqual(cycle.get('node-3'), {x: 0, y: 780});
  assert.deepEqual(cycle.get('node-4'), {x: 300, y: 780});
  assert.deepEqual(cycle.get('node-8'), {x: 600, y: 0});
});

test('a loop keeps source-order forward ranks and places the return target before its loop step', () => {
  const loop = ['records', 'tag', 'offset', 'input', 'range', 'slice', 'none'].map((nodeId) => ({nodeId}));
  const points = layoutFlowNodes(loop, [
    {fromNodeId: 'records', toNodeId: 'tag'},
    {fromNodeId: 'tag', toNodeId: 'records'},
    {fromNodeId: 'tag', toNodeId: 'offset'},
    {fromNodeId: 'offset', toNodeId: 'range'},
    {fromNodeId: 'input', toNodeId: 'range'},
    {fromNodeId: 'range', toNodeId: 'slice'},
    {fromNodeId: 'range', toNodeId: 'none'},
  ], 'leftToRight');
  assert.deepEqual(points.get('records'), {x: 0, y: -130});
  assert.deepEqual(points.get('input'), {x: 0, y: 130});
  assert.deepEqual(points.get('tag'), {x: 340, y: 0});
  assert.deepEqual(points.get('offset'), {x: 680, y: 0});
  assert.deepEqual(points.get('range'), {x: 1020, y: 0});
  assert.deepEqual(points.get('slice'), {x: 1360, y: -130});
  assert.deepEqual(points.get('none'), {x: 1360, y: 130});
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
