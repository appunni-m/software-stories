import type { LinkedVisualEdge } from './commands';

/** A compact fork is clearer when several visible relationships share one source. */
export function sharedSourceBranch(edges: readonly LinkedVisualEdge[]): {
  sourceNodeId: string;
  edges: readonly LinkedVisualEdge[];
} | null {
  if (edges.length < 2) return null;
  const sourceNodeId = edges[0]?.fromNodeId;
  if (!sourceNodeId || !edges.every((edge) => edge.fromNodeId === sourceNodeId)) return null;
  return { sourceNodeId, edges };
}
