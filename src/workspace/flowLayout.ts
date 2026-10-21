import type { LinkedVisualEdge, LinkedVisualNode, VisualLayoutDirection } from '../lib/commands';

export interface FlowPoint { x: number; y: number }

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

  const incoming = new Map(nodes.map((node) => [node.nodeId, 0]));
  const outgoing = new Map(nodes.map((node) => [node.nodeId, [] as string[]]));
  for (const edge of edges) {
    if (!nodeOrder.has(edge.fromNodeId) || !nodeOrder.has(edge.toNodeId)) return fallback;
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

  // Cycles do not have a trustworthy topological layer; retain the stable order.
  if (ordered.length !== nodes.length) return fallback;

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
  const ranksPerBand = lastRank >= 5 ? 4 : lastRank + 1;
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
