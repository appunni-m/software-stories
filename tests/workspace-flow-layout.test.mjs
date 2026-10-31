import test from 'node:test';
import assert from 'node:assert/strict';
import { layoutFlowEdgeLabels, layoutFlowNodes } from '../src/workspace/flowLayout.ts';

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
  assert.deepEqual(cycle.get('node-3'), {x: 300, y: 520});
  assert.deepEqual(cycle.get('node-4'), {x: 300, y: 260});
  assert.deepEqual(cycle.get('node-8'), {x: 600, y: 520});
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
  assert.deepEqual(points.get('range'), {x: 680, y: 460});
  assert.deepEqual(points.get('slice'), {x: 340, y: 330});
  assert.deepEqual(points.get('none'), {x: 340, y: 590});
});

test('long branching paths wrap into readable snake-shaped rank bands', () => {
  const path = Array.from({length: 9}, (_, index) => ({nodeId: `step-${index}`}));
  const edges = [
    [0, 1], [1, 2], [2, 3], [2, 4], [3, 5], [4, 5], [5, 6], [6, 7], [7, 8],
  ].map(([from, to]) => ({fromNodeId: path[from].nodeId, toNodeId: path[to].nodeId}));
  const points = layoutFlowNodes(path, edges, 'leftToRight');
  assert.deepEqual(points.get('step-0'), {x: 0, y: 0});
  assert.deepEqual(points.get('step-3'), {x: 680, y: 170});
  assert.deepEqual(points.get('step-4'), {x: 680, y: 430});
  assert.deepEqual(points.get('step-5'), {x: 340, y: 300});
  assert.deepEqual(points.get('step-8'), {x: 340, y: 760});

  const shuffled = layoutFlowNodes([...path].reverse(), edges, 'leftToRight');
  assert.deepEqual(shuffled.get('step-0'), points.get('step-0'));
  assert.deepEqual(shuffled.get('step-8'), points.get('step-8'));
  assert.equal(shuffled.get('step-3').x, points.get('step-3').x);
  assert.equal(shuffled.get('step-4').x, points.get('step-4').x);
  assert.deepEqual(new Set([shuffled.get('step-3').y, shuffled.get('step-4').y]), new Set([170, 430]));
});

test('five-rank evidence loops wrap into the narrow reader instead of a tiny wide strip', () => {
  const loop = ['records', 'tag', 'offset', 'range', 'slice', 'none'].map((nodeId) => ({nodeId}));
  const points = layoutFlowNodes(loop, [
    {fromNodeId: 'records', toNodeId: 'tag'},
    {fromNodeId: 'tag', toNodeId: 'records'},
    {fromNodeId: 'tag', toNodeId: 'offset'},
    {fromNodeId: 'offset', toNodeId: 'range'},
    {fromNodeId: 'range', toNodeId: 'slice'},
    {fromNodeId: 'range', toNodeId: 'none'},
  ], 'leftToRight');
  const x = [...points.values()].map((point) => point.x);
  const y = [...points.values()].map((point) => point.y);
  assert.equal(Math.max(...x) - Math.min(...x), 680);
  assert.equal(Math.max(...y) - Math.min(...y), 430);
  assert.deepEqual(points.get('range'), {x: 680, y: 300});
  assert.deepEqual(points.get('slice'), {x: 340, y: 170});
  assert.deepEqual(points.get('none'), {x: 340, y: 430});
});

test('a long branched flow fills more of the reader pane without card overlap', () => {
  const flow = Array.from({length: 12}, (_, index) => ({nodeId: `step-${index}`}));
  const flowEdges = [
    [0, 1], [0, 2], [1, 3], [2, 3], [3, 4], [4, 5], [4, 6],
    [5, 7], [6, 7], [7, 8], [8, 9], [8, 10], [9, 11], [10, 11],
  ].map(([from, to]) => ({fromNodeId: flow[from].nodeId, toNodeId: flow[to].nodeId}));
  const points = layoutFlowNodes(flow, flowEdges, 'auto');
  const xs = [...points.values()].map((point) => point.x);
  const ys = [...points.values()].map((point) => point.y);
  const fittedZoom = Math.min(
    700 / (Math.max(...xs) - Math.min(...xs) + 250),
    500 / (Math.max(...ys) - Math.min(...ys) + 120),
  );

  assert.equal(points.size, flow.length);
  assert.ok(fittedZoom >= 0.4, `expected a useful overview scale, got ${fittedZoom.toFixed(2)}`);
  for (let left = 0; left < flow.length; left += 1) {
    const a = points.get(flow[left].nodeId);
    for (let right = left + 1; right < flow.length; right += 1) {
      const b = points.get(flow[right].nodeId);
      const overlaps = Math.abs(a.x - b.x) < 250 && Math.abs(a.y - b.y) < 120;
      assert.equal(overlaps, false, `${flow[left].nodeId} overlaps ${flow[right].nodeId}`);
    }
  }
});

test('dense graph edge labels move into stable lanes instead of stacking on crossings', () => {
  const graphNodes = ['records', 'tag', 'offset', 'input', 'range', 'slice', 'none'].map((nodeId) => ({nodeId}));
  const graphEdges = [
    {edgeId: 'scan', fromNodeId: 'records', toNodeId: 'tag', label: 'check each record'},
    {edgeId: 'match', fromNodeId: 'tag', toNodeId: 'offset', label: 'tag matches'},
    {edgeId: 'return', fromNodeId: 'tag', toNodeId: 'records', label: 'tag differs; continue'},
    {edgeId: 'range', fromNodeId: 'offset', toNodeId: 'range', label: 'use start and end'},
    {edgeId: 'borrow', fromNodeId: 'input', toNodeId: 'slice', label: 'borrow this byte slice'},
    {edgeId: 'valid', fromNodeId: 'slice', toNodeId: 'range', label: 'range is valid'},
  ];
  const points = layoutFlowNodes(graphNodes, graphEdges, 'leftToRight');
  const labels = layoutFlowEdgeLabels(graphNodes, graphEdges, points);
  assert.equal(labels.size, graphEdges.length);
  assert.notDeepEqual(labels.get('return'), {x: 295, y: -5});
  for (const point of labels.values()) {
    assert.ok(Number.isFinite(point.x) && Number.isFinite(point.y));
  }
  for (let left = 0; left < graphEdges.length; left += 1) {
    const leftPoint = labels.get(graphEdges[left].edgeId);
    const leftWidth = Math.min(190, Math.max(64, graphEdges[left].label.length * 6.4 + 20));
    for (let right = left + 1; right < graphEdges.length; right += 1) {
      const rightPoint = labels.get(graphEdges[right].edgeId);
      const rightWidth = Math.min(190, Math.max(64, graphEdges[right].label.length * 6.4 + 20));
      const overlaps = Math.abs(leftPoint.x - rightPoint.x) < (leftWidth + rightWidth) / 2 + 10 &&
        Math.abs(leftPoint.y - rightPoint.y) < 30;
      assert.equal(overlaps, false, `${graphEdges[left].label} overlaps ${graphEdges[right].label}`);
    }
  }
});

test('branch labels stay out of source and target node bounds', () => {
  const branchNodes = ['input', 'gray', 'color', 'luma', 'other'].map((nodeId) => ({nodeId}));
  const branchEdges = [
    {edgeId: 'gray', fromNodeId: 'input', toNodeId: 'gray', label: 'gray / gray-alpha match'},
    {edgeId: 'color', fromNodeId: 'input', toNodeId: 'color', label: 'RGB / RGBA match'},
    {edgeId: 'luma', fromNodeId: 'input', toNodeId: 'luma', label: '16-bit luma match'},
    {edgeId: 'other', fromNodeId: 'input', toNodeId: 'other', label: 'no dedicated arm'},
  ];
  const positions = new Map([
    ['input', {x: 0, y: 0}],
    ['gray', {x: 340, y: -130}],
    ['color', {x: 340, y: 130}],
    ['luma', {x: 340, y: 390}],
    ['other', {x: 340, y: 650}],
  ]);

  const labels = layoutFlowEdgeLabels(branchNodes, branchEdges, positions);
  assert.equal(labels.size, branchEdges.length);
  for (const edge of branchEdges) {
    const point = labels.get(edge.edgeId);
    const width = Math.min(190, Math.max(64, edge.label.length * 6.4 + 20));
    const height = Math.ceil(Math.max(1, edge.label.length * 6.4) / 176) * 15 + 8;
    for (const node of branchNodes) {
      const nodePoint = positions.get(node.nodeId);
      const overlaps = Math.abs(point.x - (nodePoint.x + 125)) < (width + 250) / 2 + 8 &&
        Math.abs(point.y - (nodePoint.y + 60)) < (height + 120) / 2 + 8;
      assert.equal(overlaps, false, `edge label ${edge.label} overlaps node ${node.nodeId}`);
    }
  }
});
