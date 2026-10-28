import type { ChapterSourceAnchor, LinkedVisualEdge, LinkedVisualNode } from './commands';

function sourceIdentity(
  node: LinkedVisualNode | undefined,
  anchorsById: ReadonlyMap<string, ChapterSourceAnchor>,
): string | null {
  if (!node) return null;
  const label = node.label.normalize('NFKC').trim().replace(/\s+/gu, ' ').toLowerCase();
  if (!label) return null;

  const sourceAnchor = node.sourceAnchorId ? anchorsById.get(node.sourceAnchorId) : undefined;
  const evidenceAnchors = node.evidenceAnchorIds.map((id) => anchorsById.get(id)).filter(
    (anchor): anchor is ChapterSourceAnchor => Boolean(anchor),
  );
  const sourceAnchors = sourceAnchor ? [sourceAnchor] : evidenceAnchors;
  const locations = sourceAnchors.map((anchor) => [
    anchor.path,
    anchor.sourceSha256,
    anchor.startByte,
    anchor.endByteInclusive,
  ].join('\u0000')).sort();
  const location = locations.length > 0
    ? locations.join('\u0001')
    : node.sourceAnchorId ? `anchor:${node.sourceAnchorId}` : null;
  return location ? `${label}\u0002${location}` : null;
}

/** A compact fork is clearer when relationships cite the same source node. */
export function sharedSourceBranch(
  edges: readonly LinkedVisualEdge[],
  nodes?: readonly LinkedVisualNode[],
  anchorsById: ReadonlyMap<string, ChapterSourceAnchor> = new Map(),
): { sourceNodeId: string; edges: readonly LinkedVisualEdge[] } | null {
  if (edges.length < 2) return null;
  const sourceNodeId = edges[0]?.fromNodeId;
  if (!sourceNodeId) return null;
  if (edges.every((edge) => edge.fromNodeId === sourceNodeId)) return { sourceNodeId, edges };

  const nodesById = new Map((nodes ?? []).map((node) => [node.nodeId, node]));
  const firstIdentity = sourceIdentity(nodesById.get(sourceNodeId), anchorsById);
  if (!firstIdentity || !edges.every((edge) =>
    sourceIdentity(nodesById.get(edge.fromNodeId), anchorsById) === firstIdentity)) return null;
  return { sourceNodeId, edges };
}
