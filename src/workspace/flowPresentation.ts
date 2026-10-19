import type { EvidenceLinkedVisualSpec, LinkedVisualEdge, LinkedVisualNode } from '../lib/commands';

export interface LinearFlowStep {
  node: LinkedVisualNode;
  next?: LinkedVisualEdge;
}

/**
 * Long, unbranched diagrams are easier to read as a source-linked sequence than
 * as a graph shrunk to fit one narrow canvas. Branches, cycles and disconnected
 * evidence stay in the interactive graph view.
 */
export function linearFlowSteps(visual: EvidenceLinkedVisualSpec, minimumSteps = 5): LinearFlowStep[] | null {
  const { nodes, edges } = visual;
  if (nodes.length < minimumSteps || edges.length !== nodes.length - 1) return null;

  const nodeById = new Map(nodes.map((node) => [node.nodeId, node]));
  if (nodeById.size !== nodes.length) return null;

  const outgoing = new Map<string, LinkedVisualEdge[]>();
  const incomingCount = new Map(nodes.map((node) => [node.nodeId, 0]));
  for (const edge of edges) {
    if (!nodeById.has(edge.fromNodeId) || !nodeById.has(edge.toNodeId)) return null;
    const from = outgoing.get(edge.fromNodeId) ?? [];
    from.push(edge);
    if (from.length > 1) return null;
    outgoing.set(edge.fromNodeId, from);

    const nextIncomingCount = (incomingCount.get(edge.toNodeId) ?? 0) + 1;
    if (nextIncomingCount > 1) return null;
    incomingCount.set(edge.toNodeId, nextIncomingCount);
  }

  const starts = nodes.filter((node) => incomingCount.get(node.nodeId) === 0);
  if (starts.length !== 1) return null;

  const steps: LinearFlowStep[] = [];
  const visited = new Set<string>();
  let current: LinkedVisualNode | undefined = starts[0];
  while (current) {
    if (visited.has(current.nodeId)) return null;
    visited.add(current.nodeId);
    const next: LinkedVisualEdge | undefined = outgoing.get(current.nodeId)?.[0];
    steps.push({ node: current, ...(next ? { next } : {}) });
    current = next ? nodeById.get(next.toNodeId) : undefined;
  }

  return visited.size === nodes.length ? steps : null;
}
