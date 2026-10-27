import { useEffect, useState } from 'react';
import { ArrowDown, FileCode2 } from 'lucide-react';
import type { SnapshotIndex } from './lib/commands';
import type { SourceLineRange } from './lib/navigation';
import type { ProvenGraphEdge } from './lib/atlas';
import { codeLensRangeFromIndexedAnchor } from './lib/codeLens';
import { edgesForFileFlowSymbol, focusedFileFlowSymbol } from './workspace/fileFlow';
import type { Context } from './workspace/model';

/** A focused view of verified static links, never a claimed execution trace. */
export function FileFlow({ path, index, edges, loading, onOpenFile, focus, showAll:storedShowAll, onShowAllChange }: {
  path: string | null;
  index: SnapshotIndex | null;
  edges: ProvenGraphEdge[];
  loading: boolean;
  onOpenFile: (path: string, snapshotId?: string, range?: SourceLineRange) => void;
  focus?: Context | null;
  showAll?:boolean;
  onShowAllChange?:(showAll:boolean)=>void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [fileWideFocus, setFileWideFocus] = useState<string | null>(null);
  const showAll=storedShowAll??expanded;
  useEffect(()=>{setExpanded(false);setFileWideFocus(null);},[path,focus?.path,focus?.sha256,focus?.from,focus?.to]);
  if (!path) return null;
  const symbols = new Map(index?.symbols.map((symbol) => [symbol.symbolId, symbol]));
  const fileLinked = edges.filter((edge) => ['calls', 'imports', 'testReference'].includes(edge.relationship.kind)
    && (symbols.get(edge.sourceId)?.path === path || symbols.get(edge.targetId)?.path === path));
  const focusedSymbol = focusedFileFlowSymbol(index?.symbols ?? [], path, focus ?? null);
  const hasSourceSelection = !!focus && focus.path === path && (focus.from !== undefined || focus.to !== undefined);
  const focusKey = `${focus?.path ?? ''}:${focus?.sha256 ?? ''}:${focus?.from ?? ''}:${focus?.to ?? ''}`;
  const showingFileWide = !hasSourceSelection || fileWideFocus === focusKey;
  const linked = showingFileWide
    ? fileLinked
    : focusedSymbol ? edgesForFileFlowSymbol(fileLinked, focusedSymbol) : [];
  const visible = showAll ? linked : linked.slice(0, 12);
  return <section className="file-flow" aria-label={focusedSymbol && !showingFileWide ? `Code flow for ${focusedSymbol.name}` : 'Code flow for selected file'}>
    <h2>{focusedSymbol && !showingFileWide ? `Follow ${focusedSymbol.name}` : hasSourceSelection && !showingFileWide ? 'Follow selected code' : 'Explore file connections'}</h2>
    <p className="file-flow-note">{showingFileWide
      ? 'Verified calls, imports, and test references in this file. These static links do not show runtime execution.'
      : focusedSymbol
        ? 'Verified static links that touch this code. Click a node to open its captured source.'
        : 'No indexed symbol matches these selected lines. File-level links are available below.'}</p>
    {hasSourceSelection && <button className="text-button" onClick={() => setFileWideFocus(showingFileWide ? null : focusKey)}>
      {showingFileWide ? 'Return to selected code' : `Show file connections (${fileLinked.length})`}
    </button>}
    {loading ? <p role="status">Reading the local code index…</p>
      : visible.length === 0 ? <p>{focusedSymbol && !showingFileWide
          ? `No verified calls, imports, or test references touch ${focusedSymbol.name}.`
          : 'No verified calls, imports, or test references are available for this file.'}</p>
          : <div className="file-flow-routes">{visible.map((edge) => {
            const from = symbols.get(edge.sourceId);
            const to = symbols.get(edge.targetId);
            if (!from || !to) return null;
            return <article className="file-flow-route" key={edge.relationship.relationshipId}>
              <button onClick={() => onOpenFile(from.path, edge.snapshotId, codeLensRangeFromIndexedAnchor(from.anchor))}>
                <FileCode2 size={14} /><span><strong>{from.name}</strong><code>{from.path}:{from.anchor.startLine}</code></span>
              </button>
              <button className="file-flow-connector" aria-label={`Inspect ${edge.relationship.kind} evidence at ${edge.evidence.path}:${edge.evidence.startLine}`}
                onClick={() => onOpenFile(edge.evidence.path, edge.snapshotId, {
                  startLine: edge.evidence.startLine, endLine: edge.evidence.endLine, sourceSha256: edge.evidence.sha256,
                  startByte: edge.evidence.startByte, endByteExclusive: edge.evidence.endByte,
                })}><ArrowDown size={16} /><span>{edge.relationship.kind === 'testReference' ? 'tests' : edge.relationship.kind}</span></button>
              <button onClick={() => onOpenFile(to.path, edge.snapshotId, codeLensRangeFromIndexedAnchor(to.anchor))}>
                <FileCode2 size={14} /><span><strong>{to.name}</strong><code>{to.path}:{to.anchor.startLine}</code></span>
              </button>
            </article>;
          })}</div>}
    {linked.length > 12 && <button className="text-button" onClick={() => {if(onShowAllChange)onShowAllChange(!showAll);else setExpanded(!showAll);}}>{showAll ? 'Show fewer connections' : `Show all ${linked.length} connections`}</button>}
  </section>;
}
