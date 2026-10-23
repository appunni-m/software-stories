import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { linearFlowSteps, unlinkedFlowNodes } from '../src/workspace/flowPresentation.ts';

const nodes = Array.from({length: 6}, (_, index) => ({
  nodeId: `node-${index}`, label: `Step ${index}`, detail: null, hotspotId: null, evidenceAnchorIds: [`anchor-${index}`],
}));
const edge = (from, to, label = '') => ({edgeId: `edge-${from}-${to}`, fromNodeId: `node-${from}`, toNodeId: `node-${to}`, label, evidenceAnchorIds: ['anchor-0']});
const visual = (overrides = {}) => ({
  schemaVersion: 2, kind: 'sequence', title: 'A source-backed path', layoutHint: {direction: 'leftToRight'},
  nodes, edges: nodes.slice(0, -1).map((node, index) => edge(index, index + 1)), ...overrides,
});

test('long linear evidence paths become ordered, source-linked reading steps', () => {
  const steps = linearFlowSteps(visual());
  assert.deepEqual(steps.map(({node}) => node.nodeId), nodes.map(({nodeId}) => nodeId));
  assert.equal(steps[0].next.label, '');
  assert.equal(steps.at(-1).next, undefined);
});

test('short paths remain in the interactive graph view', () => {
  assert.equal(linearFlowSteps(visual({nodes: nodes.slice(0, 4), edges: [edge(0, 1), edge(1, 2), edge(2, 3)]})), null);
});

test('branching, cyclic, disconnected and duplicate-node diagrams stay in the graph view', () => {
  const branching = visual({edges: [edge(0, 1), edge(1, 2), edge(1, 3), edge(3, 4), edge(4, 5)]});
  const cyclic = visual({edges: [edge(0, 1), edge(1, 0), edge(2, 3), edge(3, 4), edge(4, 5)]});
  const disconnected = visual({edges: [edge(0, 1), edge(1, 2), edge(2, 3), edge(3, 4)]});
  const duplicate = visual({nodes: [...nodes, nodes[0]], edges: [...nodes.slice(0, -1).map((node, index) => edge(index, index + 1)), edge(5, 0)]});
  for (const candidate of [branching, cyclic, disconnected, duplicate]) assert.equal(linearFlowSteps(candidate), null);
});

test('node-only explanations use unlinked source points instead of a relationship graph', () => {
  const candidate=visual({nodes:nodes.slice(0,4),edges:[]});
  assert.deepEqual(unlinkedFlowNodes(candidate)?.map(({nodeId})=>nodeId),nodes.slice(0,4).map(({nodeId})=>nodeId));
  assert.equal(unlinkedFlowNodes(visual()),null);
  assert.equal(unlinkedFlowNodes(visual({nodes:[],edges:[]})),null);
  assert.equal(unlinkedFlowNodes(visual({nodes:[nodes[0],nodes[0]],edges:[]})),null);
});

test('the production Flow renders sequence steps with evidence links and preserves graph controls for other diagrams', () => {
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
  assert.match(source, /const linearSteps=linearFlowSteps\(visual\)/);
  assert.match(source, /const unlinkedNodes=unlinkedFlowNodes\(visual\)/);
  assert.match(source, /const sourceValues=sourceValuesByVisualNode\(visual\)/);
  assert.match(source, /sourceAnchorForVisualNode\(anchor,node,node\.sourceValue\?\?sourceValues\.get\(node\.nodeId\)\)/);
  assert.match(source, /n\.sourceAnchorId&&n\.evidenceAnchorIds\.includes\(n\.sourceAnchorId\)/);
  assert.match(source, /onClick=\{\(\)=>openNodeSource\(n,anchor\)\}/);
  assert.match(source, /onClick=\{\(\)=>onOpen\(anchor\)\}/);
  assert.match(source, /const positions=layoutFlowNodes\(visual\.nodes,visual\.edges,visual\.layoutHint\.direction\)/);
  assert.match(source, /className="dw-linear-flow" aria-label=\{`Steps in \$\{visual\.title\}`\}/);
  assert.match(source, /className="dw-linear-flow-step" aria-label=\{`Step \$\{index\+1\}: \$\{node\.label\}\. Open source \$\{sourceLabel\}`\}/);
  assert.match(source, /className="dw-unlinked-flow" aria-label=\{`Source points in \$\{visual\.title\}`\}/);
  assert.match(source, /className="dw-unlinked-flow-point" aria-label=\{`Open source for \$\{node\.label\} at \$\{sourceLabel\}`\}/);
  assert.match(source, /aria-label=\{`Read-only code relationship diagram: \$\{nodes\.length\} nodes`\}/);
  assert.match(source, /<EvidenceFlowCanvas key=\{`\$\{identity\.revisionId\}:\$\{identity\.sectionRevisionId\}:\$\{FLOW_DIAGRAM_LAYOUT_VERSION\}`\}/);
  assert.match(source, /function EvidenceFlowCanvas\(/);
  assert.match(source, /viewportClipsNodeBounds\(bounds,flow\.getViewport\(\),size\.width,size\.height,28\)/);
  assert.match(styles, /\.dw-linear-flow-step:focus-visible/);
});

test('Flow stays focused on its selected diagram while saved questions live in Chat history', () => {
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  const flowStart = source.indexOf('<Tabs.Content value="flow">');
  const flowEnd = source.indexOf('</Tabs.Content>', flowStart);
  assert.notEqual(flowStart, -1);
  assert.notEqual(flowEnd, -1);
  const flow = source.slice(flowStart, flowEnd);
  assert.match(flow, /<EvidenceFlow/);
  assert.doesNotMatch(flow, /dw-flow-conversation|<Answer\b/);
  assert.match(source, /<Tabs.Content value="history">/);
});
