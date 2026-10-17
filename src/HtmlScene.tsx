import type { ReactNode } from 'react';
import { ArrowDown, Code2 } from 'lucide-react';
import type { EvidenceLinkedVisualSpec } from './lib/commands';
import { containerKinds, type PresentationScene } from './lib/scenePresentation';
import './html-scene.css';

/** The AI composes HTML from bounded layout/data primitives. React owns every
 * tag, style and event; model strings never become executable DOM or CSS.
 */
export function HtmlScene({ scene, visual, onOpenNode, focusedNode }: {
  scene: PresentationScene; visual: EvidenceLinkedVisualSpec;
  onOpenNode: (id: string) => void; focusedNode?: string | null;
}) {
  const draw = (index: number): ReactNode => {
    const e = scene.elements[index];
    const children = scene.elements.flatMap((child, i) => child.parentIndex === index ? [draw(i)] : []);
    const className = `html-scene-element html-scene-${e.kind} html-tone-${e.tone}`;
    if (containerKinds.has(e.kind)) return <div key={index} className={className}>{children}</div>;
    if (e.kind === 'arrow') return <span key={index} className={className} aria-hidden="true"><ArrowDown size={22}/></span>;
    if (e.kind === 'divider') return <hr key={index} className={className}/>;
    const node = visual.nodes[e.nodeIndex!];
    const illustrated = e.valueOrigin === 'illustration';
    const value = e.value;
    let body: ReactNode = value ? <span className="html-scene-value">{value}</span> : null;
    if (e.kind === 'bytes') body = <span className="html-byte-strip">{value.trim().split(/\s+/u).map((cell, i) => <span key={i}><b>{cell}</b><small>{i}</small></span>)}</span>;
    if (e.kind === 'matrix') {
      const rows = value.trimEnd().split(/\r?\n/u);
      body = <span className="html-pixel-matrix" role="img" aria-label={`${rows.length} by ${rows[0].length} cell representation`} style={{ gridTemplateColumns: `repeat(${rows[0].length}, 1fr)` }}>{rows.flatMap((row, y) => [...row].map((cell, x) => <i key={`${x}:${y}`} className={cell === '1' ? 'is-filled' : ''}/>))}</span>;
    }
    if (e.kind === 'table') body = <table><tbody>{value.trimEnd().split(/\r?\n/u).map((row, i) => <tr key={i}>{row.split('|').map((cell, j) => i === 0 ? <th key={j} scope="col">{cell}</th> : <td key={j}>{cell}</td>)}</tr>)}</tbody></table>;
    if (e.kind === 'code') body = value ? <pre><code>{value}</code></pre> : null;
    return <div key={index} className={`${className}${focusedNode === node.nodeId ? ' is-focused' : ''}`}
      role="button" tabIndex={0} title={node.detail ?? undefined}
      data-focus-key={`reader-visual-citation:${node.nodeId}:${node.evidenceAnchorIds[0]}`}
      aria-label={`${node.label}${value ? `: ${value}` : ''}${illustrated ? ', illustrative example' : ''}. Open supporting code`}
      onClick={() => onOpenNode(node.nodeId)} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onOpenNode(node.nodeId); } }}>
      <span className="html-scene-label">{node.label}<Code2 size={12}/></span>
      {body}
      {illustrated && <small className="html-example-label">Illustrative example</small>}
    </div>;
  };
  return <section className="html-scene" aria-label={scene.title}>
    <h4>{scene.title}</h4>
    <div className="html-scene-body">{draw(0)}</div>
  </section>;
}
