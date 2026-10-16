import { useState } from 'react';
import { ArrowDown, FileCode2 } from 'lucide-react';
import type { SnapshotIndex } from './lib/commands';
import type { SourceLineRange } from './lib/navigation';
import type { ProvenGraphEdge } from './lib/atlas';
import { codeLensRangeFromIndexedAnchor } from './lib/codeLens';

/** A focused view of verified static links, never a claimed execution trace. */
export function FileFlow({ path, index, edges, loading, onOpenFile }: {
  path: string | null;
  index: SnapshotIndex | null;
  edges: ProvenGraphEdge[];
  loading: boolean;
  onOpenFile: (path: string, snapshotId?: string, range?: SourceLineRange) => void;
}) {
  const [showAll, setShowAll] = useState(false);
  const symbols = new Map(index?.symbols.map((symbol) => [symbol.symbolId, symbol]));
  const linked = edges.filter((edge) => ['calls', 'imports', 'testReference'].includes(edge.relationship.kind)
    && (symbols.get(edge.sourceId)?.path === path || symbols.get(edge.targetId)?.path === path));
  const visible = showAll ? linked : linked.slice(0, 12);
  return <section className="file-flow" aria-label="Code flow for selected file">
    <h2>Follow the code</h2>
    <p className="file-flow-note">Static calls, imports, and test references. Click a node to open its source.</p>
    {loading ? <p role="status">Reading the local code index…</p>
      : !path ? <p>Select a file in the explorer.</p>
        : visible.length === 0 ? <p>No verified calls, imports, or test references are available for this file. This is not an execution trace.</p>
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
    {linked.length > 12 && <button className="text-button" onClick={() => setShowAll((value) => !value)}>{showAll ? 'Show fewer connections' : `Show all ${linked.length} connections`}</button>}
  </section>;
}
