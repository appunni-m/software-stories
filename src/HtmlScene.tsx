import type { ReactNode } from 'react';
import { ArrowDown, ArrowRight, Code2 } from 'lucide-react';
import type { EvidenceLinkedVisualSpec } from './lib/commands';
import { containerKinds, sceneNodeAccessibleLabel, type PresentationScene } from './lib/scenePresentation';
import './html-scene.css';

/** The AI composes HTML from bounded layout/data primitives. React owns every
 * tag, style and event; model strings never become executable DOM or CSS.
 */
export function HtmlScene({ scene, visual, onOpenNode, focusedNode }: {
  scene: PresentationScene; visual: EvidenceLinkedVisualSpec;
  onOpenNode: (id: string, sourceValue?: string) => void; focusedNode?: string | null;
}) {
  const draw = (index: number): ReactNode => {
    const e = scene.elements[index];
    const childIndices = scene.elements.flatMap((child, i) => child.parentIndex === index ? [i] : []);
    const children = childIndices.map(draw);
    const hasArrows = e.kind === 'row' && childIndices.some(childIndex => scene.elements[childIndex]?.kind === 'arrow');
    const className = `html-scene-element html-scene-${e.kind} html-tone-${e.tone}${hasArrows ? ' has-arrows' : ''}`;
    if (containerKinds.has(e.kind)) {
      const labels: Record<string, string> = {
        timeline: 'Ordered timeline', lanes: 'Independent parallel lanes',
        branch: 'Decision and outcomes', beforeAfter: 'Before and after states',
        transform: 'Input, operation and result', tree: 'Nested data hierarchy',
        queue: 'First in, first out queue, ordered front to back',
        selector: 'Ordered first-match selector; checks are evaluated in source order',
      };
      const label = labels[e.kind];
      if (e.kind === 'selector') {
        return <div key={index} className={className} role="group" aria-label={label}>
          {childIndices.map((check, checkIndex) => <div className="html-scene-selector-step" key={check}>
            <span className="html-scene-selector-label" role="img" aria-label={`Probe ${checkIndex + 1}`} title={`Probe ${checkIndex + 1}`}>{String(checkIndex + 1).padStart(2, '0')}</span>
            {draw(check)}
            {checkIndex < childIndices.length - 1 && <div className="html-scene-selector-handoff">
              <ArrowDown aria-hidden="true" size={18}/><span>No match · continue</span>
            </div>}
          </div>)}
        </div>;
      }
      if (e.kind === 'transform') {
        const slots = ['Input', 'Operation', 'Result'];
        return <div key={index} className={className} role="group" aria-label={label}>
          {children.flatMap((child, childIndex) => [
            <div key={`slot-${childIndex}`} className="html-scene-transform-slot">
              <small>{slots[childIndex]}</small>{child}
            </div>,
            ...(childIndex < 2 ? [<ArrowRight key={`arrow-${childIndex}`} aria-hidden="true" className="html-scene-transform-arrow" size={22}/>] : []),
          ])}
        </div>;
      }
      if (e.kind === 'branch') {
        return <div key={index} className={className} role="group" aria-label={label}>
          {children.map((child, childIndex) => <div key={`branch-item-${childIndices[childIndex]}`} className="html-scene-branch-item">{child}</div>)}
        </div>;
      }
      return <div key={index} className={className} role={label ? 'group' : undefined} aria-label={label}>{children}</div>;
    }
    if (e.kind === 'arrow') return <span key={index} className={className} aria-hidden="true"><ArrowDown size={22}/></span>;
    if (e.kind === 'divider') return <hr key={index} className={className}/>;
    const node = visual.nodes[e.nodeIndex!];
    const originLabel = e.valueOrigin === 'source' ? 'Source value' : e.valueOrigin === 'illustration' ? 'Illustrative example' : null;
    const value = e.value;
    const sourceValue = e.valueOrigin === 'source' ? value : undefined;
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
      aria-label={sceneNodeAccessibleLabel(node, value, e.valueOrigin)}
      onClick={() => onOpenNode(node.nodeId, sourceValue)} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onOpenNode(node.nodeId, sourceValue); } }}>
      <span className="html-scene-label">{node.label}<Code2 size={12}/></span>
      {body}
      {originLabel && <small className="html-origin-label">{originLabel}</small>}
    </div>;
  };
  return <section className="html-scene" aria-label={scene.title}>
    <h4>{scene.title}</h4>
    <div className="html-scene-body">{draw(0)}</div>
  </section>;
}
