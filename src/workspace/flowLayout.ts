import type { LinkedVisualEdge, LinkedVisualNode, VisualLayoutDirection } from '../lib/commands';

export interface FlowPoint { x: number; y: number }

interface LabeledFlowEdge extends Pick<LinkedVisualEdge, 'edgeId' | 'fromNodeId' | 'toNodeId' | 'label'> {}

function rankAcyclicProjection(
  nodes: Pick<LinkedVisualNode, 'nodeId'>[],
  edges: Pick<LinkedVisualEdge, 'fromNodeId' | 'toNodeId'>[],
): { ordered: string[]; rank: Map<string, number> } {
  const incoming = new Map(nodes.map((node) => [node.nodeId, 0]));
  const outgoing = new Map(nodes.map((node) => [node.nodeId, [] as string[]]));
  for (const edge of edges) {
    outgoing.get(edge.fromNodeId)!.push(edge.toNodeId);
    incoming.set(edge.toNodeId, incoming.get(edge.toNodeId)! + 1);
  }
  const ready = nodes.filter((node) => incoming.get(node.nodeId) === 0).map((node) => node.nodeId);
  const remainingIncoming = new Map(incoming);
  const rank = new Map(nodes.map((node) => [node.nodeId, 0]));
  const ordered: string[] = [];
  for (let readyIndex = 0; readyIndex < ready.length; readyIndex += 1) {
    const current = ready[readyIndex]!;
    ordered.push(current);
    for (const target of outgoing.get(current)!) {
      rank.set(target, Math.max(rank.get(target)!, rank.get(current)! + 1));
      const remaining = remainingIncoming.get(target)! - 1;
      remainingIncoming.set(target, remaining);
      if (remaining === 0) ready.push(target);
    }
  }
  return { ordered, rank };
}

function stableFallback(nodes: Pick<LinkedVisualNode, 'nodeId'>[], direction: VisualLayoutDirection): Map<string, FlowPoint> {
  if (nodes.length < 7) return new Map(nodes.map((node, index) => [node.nodeId, {
    x: direction === 'leftToRight' ? index * 340 : 0,
    y: direction === 'leftToRight' ? 0 : index * 260,
  }]));

  // Disconnected and cyclic graphs have no trustworthy rank order. Keep their
  // source order, but use the available canvas area instead of a tiny row.
  const columns = Math.ceil(Math.sqrt(nodes.length));
  return new Map(nodes.map((node, index) => {
    const column = index % columns;
    const row = Math.floor(index / columns);
    return [node.nodeId, direction === 'leftToRight'
      ? { x: column * 340, y: row * 260 }
      : { x: row * 320, y: column * 260 }];
  }));
}

/** Place acyclic evidence graphs in rank layers while retaining source order in each layer. */
export function layoutFlowNodes(
  nodes: Pick<LinkedVisualNode, 'nodeId'>[],
  edges: Pick<LinkedVisualEdge, 'fromNodeId' | 'toNodeId'>[],
  direction: VisualLayoutDirection,
): Map<string, FlowPoint> {
  const fallback = stableFallback(nodes, direction);
  if (nodes.length < 2) return fallback;

  const nodeOrder = new Map(nodes.map((node, index) => [node.nodeId, index]));
  if (nodeOrder.size !== nodes.length) return fallback;

  for (const edge of edges) {
    if (!nodeOrder.has(edge.fromNodeId) || !nodeOrder.has(edge.toNodeId)) return fallback;
  }
  let ranked = rankAcyclicProjection(nodes, edges);
  if (ranked.ordered.length !== nodes.length) {
    // A real evidence loop should remain visible, but using it to rank nodes
    // collapses the whole graph into a grid. Keep forward story edges as the
    // reading spine and draw backward edges as explicit returns in the canvas.
    ranked = rankAcyclicProjection(nodes, edges.filter((edge) => nodeOrder.get(edge.fromNodeId)! < nodeOrder.get(edge.toNodeId)!));
  }
  // The source-order projection is acyclic. This guard keeps malformed input
  // bounded if that contract changes later.
  if (ranked.ordered.length !== nodes.length) return fallback;
  const { rank } = ranked;

  const layers = new Map<number, string[]>();
  for (const node of nodes) {
    const layerRank = rank.get(node.nodeId)!;
    const layer = layers.get(layerRank) ?? [];
    layer.push(node.nodeId);
    layers.set(layerRank, layer);
  }

  // A wide rank of unrelated nodes (common for disconnected evidence) should
  // read as a compact overview. Keep small diagrams in their familiar layout.
  if (layers.size === 1 && nodes.length >= 7) return stableFallback(nodes, direction);

  const isHorizontal = direction === 'leftToRight';
  const depthSpacing = isHorizontal ? 340 : 260;
  const siblingSpacing = isHorizontal ? 260 : 320;
  const positions = new Map<string, FlowPoint>();
  const lastRank = Math.max(...layers.keys());
  // The explanation column is usually narrower than the repository/source
  // panes. Wrap a five-rank path into two bands before fitView shrinks every
  // 250px card into a thin, hard-to-read strip.
  const ranksPerBand = lastRank >= 4 ? 3 : lastRank + 1;
  const bands = new Map<number, number>();
  for (const [layerRank, layer] of layers) {
    const band = Math.floor(layerRank / ranksPerBand);
    bands.set(band, Math.max(bands.get(band) ?? 0, layer.length));
  }
  const bandOffsets = new Map<number, number>();
  let bandOffset = 0;
  for (const [band, largestLayer] of [...bands].sort(([left], [right]) => left - right)) {
    bandOffsets.set(band, bandOffset);
    bandOffset += Math.max(300, (largestLayer - 1) * siblingSpacing + 200);
  }
  for (const [layerRank, layer] of layers) {
    layer.sort((left, right) => nodeOrder.get(left)! - nodeOrder.get(right)!);
    const center = (layer.length - 1) / 2;
    const band = Math.floor(layerRank / ranksPerBand);
    const rankInBand = layerRank % ranksPerBand;
    const reverse = band % 2 === 1;
    const rankSlot = reverse ? ranksPerBand - 1 - rankInBand : rankInBand;
    const bandCrossOffset = bandOffsets.get(band) ?? 0;
    layer.forEach((nodeId, index) => {
      const sibling = (index - center) * siblingSpacing;
      positions.set(nodeId, isHorizontal
        ? { x: rankSlot * depthSpacing, y: bandCrossOffset + sibling }
        : { x: bandCrossOffset + sibling, y: rankSlot * depthSpacing });
    });
  }
  return positions;
}

/** Move relationship labels off dense edge crossings while keeping them near their link. */
export function layoutFlowEdgeLabels(
  nodes: Pick<LinkedVisualNode, 'nodeId'>[],
  edges: LabeledFlowEdge[],
  positions: Map<string, FlowPoint>,
): Map<string, FlowPoint> {
  const nodeOrder = new Map(nodes.map((node, index) => [node.nodeId, index]));
  const placed: { point: FlowPoint; width: number; height: number }[] = [];
  const labels = new Map<string, FlowPoint>();

  for (const edge of edges) {
    const from = positions.get(edge.fromNodeId);
    const to = positions.get(edge.toNodeId);
    if (!from || !to || !edge.label.trim()) continue;

    // Flow nodes are 250px wide; their visual center is close enough for
    // placing a detached edge label, while React Flow still routes the edge.
    const start = { x: from.x + 125, y: from.y + 60 };
    const end = { x: to.x + 125, y: to.y + 60 };
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    const length = Math.hypot(dx, dy);
    const normal = length > 0 ? { x: -dy / length, y: dx / length } : { x: 0, y: 1 };
    const returnEdge = (nodeOrder.get(edge.toNodeId) ?? 0) < (nodeOrder.get(edge.fromNodeId) ?? 0);
    const baseOffset = returnEdge ? 34 : length > 300 ? 24 : 14;
    const width = Math.min(190, Math.max(64, edge.label.length * 6.4 + 20));
    const height = Math.ceil(Math.max(1, edge.label.length * 6.4) / 176) * 15 + 8;
    const midpoint = { x: (start.x + end.x) / 2, y: (start.y + end.y) / 2 };
    let chosen = midpoint;

    // Try alternating sides and wider lanes until this label clears the ones
    // already placed. The bounded search keeps malformed diagrams predictable.
    for (let lane = 0; lane < 12; lane += 1) {
      const magnitude = baseOffset + Math.floor((lane + 1) / 2) * 26;
      const side = lane === 0 ? 1 : lane % 2 === 1 ? -1 : 1;
      const candidate = {
        x: midpoint.x + normal.x * magnitude * side,
        y: midpoint.y + normal.y * magnitude * side,
      };
      const overlaps = placed.some((other) =>
        Math.abs(candidate.x - other.point.x) < (width + other.width) / 2 + 10 &&
        Math.abs(candidate.y - other.point.y) < (height + other.height) / 2 + 8,
      );
      if (!overlaps) {
        chosen = candidate;
        break;
      }
    }

    labels.set(edge.edgeId, chosen);
    placed.push({ point: chosen, width, height });
  }

  return labels;
}
