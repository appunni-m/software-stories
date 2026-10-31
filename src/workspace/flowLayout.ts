import type { LinkedVisualEdge, LinkedVisualNode, VisualLayoutDirection } from '../lib/commands';

export interface FlowPoint { x: number; y: number }

interface LabeledFlowEdge extends Pick<LinkedVisualEdge, 'edgeId' | 'fromNodeId' | 'toNodeId' | 'label'> {}

const FLOW_NODE_WIDTH = 250;
// Node labels can wrap to several lines. Reserve their maximum expected height
// when placing edge labels so a diagram remains readable before React Flow has
// measured each DOM node.
const FLOW_NODE_HEIGHT = 120;
// Dense overviews use compact title-only cards. Their full explanation and
// citations are shown in the detail panel after selection. Three visible title
// lines make the 3×4 overview useful without forcing readers to select every
// card just to distinguish long labels.
const DENSE_FLOW_NODE_WIDTH = 140;
const DENSE_FLOW_NODE_HEIGHT = 72;
const FLOW_REFERENCE_VIEWPORT = { width: 700, height: 500 };

/** Dense maps use smaller cards so their overview remains readable in the workspace's side pane. */
export function flowNodeSize(nodeCount: number): { width: number; height: number } {
  return nodeCount >= 10
    ? { width: DENSE_FLOW_NODE_WIDTH, height: DENSE_FLOW_NODE_HEIGHT }
    : { width: FLOW_NODE_WIDTH, height: FLOW_NODE_HEIGHT };
}

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
  // A 12-node overview fits the narrow pane better as three columns and four
  // rows. The previous 4×3 grid forced React Flow to shrink the labels and
  // placed the last column beyond the visible reader width.
  const columns = nodes.length >= 10 && nodes.length <= 14
    ? 3
    : Math.ceil(Math.sqrt(nodes.length));
  const { width, height } = flowNodeSize(nodes.length);
  const dense = nodes.length >= 10;
  const columnStep = dense ? width + 24 : 340;
  const rowStep = dense ? height + 20 : 260;
  return new Map(nodes.map((node, index) => {
    const column = index % columns;
    const row = Math.floor(index / columns);
    return [node.nodeId, direction === 'leftToRight'
      ? { x: column * columnStep, y: row * rowStep }
      : { x: row * columnStep, y: column * rowStep }];
  }));
}

function bestGridFit(nodes: Pick<LinkedVisualNode, 'nodeId'>[], direction: VisualLayoutDirection): Map<string, FlowPoint> {
  const preferred = stableFallback(nodes, direction);
  const alternateDirection = direction === 'leftToRight' ? 'auto' : 'leftToRight';
  const alternate = stableFallback(nodes, alternateDirection);
  const { width, height } = flowNodeSize(nodes.length);
  return fitScale(alternate, width, height) > fitScale(preferred, width, height) ? alternate : preferred;
}

function placeRankedLayers(
  layers: Map<number, string[]>,
  direction: VisualLayoutDirection,
  lastRank: number,
  ranksPerBand: number,
  compact: boolean,
  nodeWidth: number,
  nodeHeight: number,
): Map<string, FlowPoint> {
  const isHorizontal = direction === 'leftToRight';
  const depthSpacing = compact
    ? (isHorizontal ? nodeWidth + 36 : nodeHeight + 56)
    : (isHorizontal ? 340 : 260);
  const siblingSpacing = compact
    ? (isHorizontal ? nodeHeight + 28 : nodeWidth + 32)
    : (isHorizontal ? 260 : 320);
  const bands = new Map<number, number>();
  for (const [layerRank, layer] of layers) {
    const band = Math.floor(layerRank / ranksPerBand);
    bands.set(band, Math.max(bands.get(band) ?? 0, layer.length));
  }

  const bandOffsets = new Map<number, number>();
  let bandOffset = 0;
  for (const [band, largestLayer] of [...bands].sort(([left], [right]) => left - right)) {
    bandOffsets.set(band, bandOffset);
    const bandWidth = compact
      ? (largestLayer - 1) * siblingSpacing + nodeWidth + 30
      : (largestLayer - 1) * siblingSpacing + 200;
    bandOffset += Math.max(300, bandWidth);
  }

  const positions = new Map<string, FlowPoint>();
  for (const [layerRank, layer] of layers) {
    const center = (layer.length - 1) / 2;
    const band = Math.floor(layerRank / ranksPerBand);
    const rankInBand = layerRank % ranksPerBand;
    const bandLength = Math.min(ranksPerBand, lastRank - band * ranksPerBand + 1);
    const rankSlot = band % 2 === 1
      ? (compact ? bandLength : ranksPerBand) - 1 - rankInBand
      : rankInBand;
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

function fitScale(positions: Map<string, FlowPoint>, nodeWidth: number, nodeHeight: number): number {
  if (positions.size === 0) return 0;
  const xs = [...positions.values()].map((point) => point.x);
  const ys = [...positions.values()].map((point) => point.y);
  const width = Math.max(...xs) - Math.min(...xs) + nodeWidth;
  const height = Math.max(...ys) - Math.min(...ys) + nodeHeight;
  return Math.min(FLOW_REFERENCE_VIEWPORT.width / width, FLOW_REFERENCE_VIEWPORT.height / height);
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

  // Wide, mostly unrelated evidence is a source index, not a useful rank row.
  // Lay it out as a grid while keeping its actual edges. This also covers a
  // mostly disconnected set with a few real calls (for example, many source
  // points plus two wrappers converging on one shared entry point).
  const widestLayer = Math.max(...[...layers.values()].map((layer) => layer.length));
  if (nodes.length >= 7 && widestLayer >= Math.ceil(nodes.length * 0.6)) {
    return bestGridFit(nodes, direction);
  }

  const lastRank = Math.max(...layers.keys());
  for (const layer of layers.values()) layer.sort((left, right) => nodeOrder.get(left)! - nodeOrder.get(right)!);

  // Long branched paths can become unreadable if every node is fitted into one
  // tall layout. Try bounded rank-band widths and both orientations, then keep
  // the arrangement that uses a typical reader pane best. Prefer the author's
  // requested direction when two layouts are nearly equivalent.
  const compact = nodes.length >= 10;
  const { width: nodeWidth, height: nodeHeight } = flowNodeSize(nodes.length);
  const preferredDirection = direction === 'leftToRight' ? 'leftToRight' : 'topToBottom';
  const directions: VisualLayoutDirection[] = compact
    ? preferredDirection === 'leftToRight' ? ['leftToRight', 'topToBottom'] : ['topToBottom', 'leftToRight']
    : [preferredDirection];
  const bandWidths = compact && lastRank >= 4
    ? Array.from({ length: lastRank }, (_, index) => index + 2)
    : [lastRank >= 4 ? 3 : lastRank + 1];
  let bestPositions: Map<string, FlowPoint> | undefined;
  let bestScore = -1;
  for (const candidateDirection of directions) {
    for (const ranksPerBand of bandWidths) {
      const candidate = placeRankedLayers(layers, candidateDirection, lastRank, ranksPerBand, compact, nodeWidth, nodeHeight);
      const directionPreference = candidateDirection === preferredDirection ? 1 : 0.92;
      const score = fitScale(candidate, nodeWidth, nodeHeight) * directionPreference;
      if (score > bestScore) {
        bestScore = score;
        bestPositions = candidate;
      }
    }
  }
  return bestPositions ?? fallback;
}

/** Move relationship labels off dense edge crossings while keeping them near their link. */
export function layoutFlowEdgeLabels(
  nodes: Pick<LinkedVisualNode, 'nodeId'>[],
  edges: LabeledFlowEdge[],
  positions: Map<string, FlowPoint>,
): Map<string, FlowPoint> {
  const nodeOrder = new Map(nodes.map((node, index) => [node.nodeId, index]));
  const { width: nodeWidth, height: nodeHeight } = flowNodeSize(nodes.length);
  const placed: { point: FlowPoint; width: number; height: number }[] = [];
  const nodeRects = nodes.flatMap((node) => {
    const point = positions.get(node.nodeId);
    return point ? [{
      center: { x: point.x + nodeWidth / 2, y: point.y + nodeHeight / 2 },
      width: nodeWidth,
      height: nodeHeight,
    }] : [];
  });
  const labels = new Map<string, FlowPoint>();

  for (const edge of edges) {
    const from = positions.get(edge.fromNodeId);
    const to = positions.get(edge.toNodeId);
    if (!from || !to || !edge.label.trim()) continue;

    // Use the configured card center for detached edge labels while React Flow
    // continues to route each relationship between the measured card bounds.
    const start = { x: from.x + nodeWidth / 2, y: from.y + nodeHeight / 2 };
    const end = { x: to.x + nodeWidth / 2, y: to.y + nodeHeight / 2 };
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
    let leastOverlap = Number.POSITIVE_INFINITY;

    // Try alternating sides and wider lanes until this label clears the ones
    // already placed. The bounded search keeps malformed diagrams predictable.
    for (let lane = 0; lane < 24; lane += 1) {
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
      const intersectingNodes = nodeRects.reduce((count, node) => count + Number(
        Math.abs(candidate.x - node.center.x) < (width + node.width) / 2 + 8 &&
        Math.abs(candidate.y - node.center.y) < (height + node.height) / 2 + 8,
      ), 0);
      if (intersectingNodes < leastOverlap) {
        chosen = candidate;
        leastOverlap = intersectingNodes;
      }
      if (!overlaps && intersectingNodes === 0) {
        chosen = candidate;
        break;
      }
    }

    labels.set(edge.edgeId, chosen);
    placed.push({ point: chosen, width, height });
  }

  return labels;
}
