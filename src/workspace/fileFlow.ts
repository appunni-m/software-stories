import type { IndexedSymbol } from '../lib/commands';
import type { ProvenGraphEdge } from '../lib/atlas';
import type { Context } from './model';

/** Find the narrowest indexed code element that contains the current source focus. */
export function focusedFileFlowSymbol(
  symbols: readonly IndexedSymbol[],
  path: string | null,
  focus: Context | null,
): IndexedSymbol | null {
  if (!path || !focus || focus.path !== path) return null;
  const startLine = focus.from ?? focus.to;
  const endLine = focus.to ?? focus.from;
  if (startLine === undefined || endLine === undefined) return null;

  return symbols
    .filter((symbol) => symbol.path === path
      && (!focus.sha256 || symbol.anchor.sourceSha256 === focus.sha256)
      && symbol.anchor.startLine <= Math.min(startLine, endLine)
      && symbol.anchor.endLine >= Math.max(startLine, endLine))
    .sort((left, right) => (left.anchor.endLine - left.anchor.startLine)
      - (right.anchor.endLine - right.anchor.startLine)
      || left.symbolId.localeCompare(right.symbolId))[0] ?? null;
}

/** Keep only verified relationships that touch the selected indexed symbol. */
export function edgesForFileFlowSymbol(
  edges: readonly ProvenGraphEdge[],
  symbol: IndexedSymbol,
): ProvenGraphEdge[] {
  return edges.filter((edge) => edge.sourceId === symbol.symbolId || edge.targetId === symbol.symbolId);
}
