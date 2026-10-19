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
