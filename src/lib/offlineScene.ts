import type { EvidenceLinkedVisualSpec } from './commands';
import { containerKinds, type PresentationScene } from './scenePresentation.ts';

export const escapeSceneText = (value: string) => value.replace(/[&<>"']/gu, character => ({
  '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
})[character]!);

/** Render an already validated/sanitized composition using app-owned tags.
 * Destinations are generated package anchors; no model HTML, CSS or URL runs.
 */
export function renderOfflineScene(scene: PresentationScene, visual: EvidenceLinkedVisualSpec, evidenceHref: (nodeIndex: number) => string | undefined): string {
  const draw = (index: number): string => {
    const element = scene.elements[index];
    const cls = `scene-element scene-${element.kind} scene-tone-${element.tone}`;
    if (containerKinds.has(element.kind)) {
      const childIndices = scene.elements.flatMap((child, i) => child.parentIndex === index ? [i] : []);
      const labels: Record<string, string> = {
        timeline: 'Ordered timeline', lanes: 'Independent parallel lanes',
        branch: 'Decision and outcomes', beforeAfter: 'Before and after states',
        transform: 'Input, operation and result', tree: 'Nested data hierarchy',
        queue: 'First in, first out queue, ordered front to back',
        selector: 'Ordered first-match selector; checks are evaluated in source order',
      };
      const label = labels[element.kind];
      const children = childIndices.map(draw);
      if (element.kind === 'selector') {
        return `<div class="${cls}" role="group" aria-label="${label}">${childIndices.map((check, checkIndex) => `<div class="scene-selector-step"><span class="scene-selector-label">Check ${checkIndex + 1}</span>${draw(check)}${checkIndex < childIndices.length - 1 ? '<div class="scene-selector-handoff"><span aria-hidden="true">↓</span><span>No match · continue</span></div>' : ''}</div>`).join('')}</div>`;
      }
      if (element.kind === 'transform') {
        const slots = ['Input', 'Operation', 'Result'];
        return `<div class="${cls}" role="group" aria-label="${label}">${children.map((child, childIndex) => `<div class="scene-transform-slot"><small>${slots[childIndex]}</small>${child}</div>${childIndex < 2 ? '<span class="scene-transform-arrow" aria-hidden="true">→</span>' : ''}`).join('')}</div>`;
      }
      if (element.kind === 'branch') {
        return `<div class="${cls}" role="group" aria-label="${label}">${children.map(child => `<div class="scene-branch-item">${child}</div>`).join('')}</div>`;
      }
      return `<div class="${cls}"${label ? ` role="group" aria-label="${label}"` : ''}>${children.join('')}</div>`;
    }
    if (element.kind === 'arrow') return `<span class="${cls}" aria-hidden="true">↓</span>`;
    if (element.kind === 'divider') return `<hr class="${cls}">`;
    const node = visual.nodes[element.nodeIndex!];
    const value = escapeSceneText(element.value);
    let body = value ? `<span class="scene-value">${value}</span>` : '';
    if (element.kind === 'code' && value) body = `<pre><code>${value}</code></pre>`;
    if (element.kind === 'bytes') {
      body = `<span class="scene-bytes">${element.value.trim().split(/\s+/u).map((cell, i) => `<span><b>${escapeSceneText(cell)}</b><small>${i}</small></span>`).join('')}</span>`;
    }
    if (element.kind === 'matrix') {
      const rows = element.value.trimEnd().split(/\r?\n/u);
      const matrixLabel = [
        `${rows.length} by ${rows[0].length} cell representation`,
        ...rows.map((row, rowIndex) => `row ${rowIndex + 1}: ${row}`),
      ].join('; ');
      const cells = rows.flatMap(row => [...row].map(cell => `<i class="${cell === '1' ? 'filled' : 'empty'}"></i>`)).join('');
      body = `<span class="scene-matrix" role="img" aria-label="${escapeSceneText(matrixLabel)}" style="grid-template-columns:repeat(${rows[0].length},1fr)">${cells}</span>`;
    }
    if (element.kind === 'table') {
      body = `<table><tbody>${element.value.trimEnd().split(/\r?\n/u).map((row, i) => `<tr>${row.split('|').map(cell => i === 0 ? `<th scope="col">${escapeSceneText(cell)}</th>` : `<td>${escapeSceneText(cell)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
    }
    const originLabel = element.valueOrigin === 'source' ? 'Source value' : element.valueOrigin === 'illustration' ? 'Illustrative example' : '';
    const origin = originLabel ? `<small class="scene-origin">${originLabel}</small>` : '';
    const content = `<span class="scene-label">${escapeSceneText(node.label)}</span>${body}${origin}`;
    const href = evidenceHref(element.nodeIndex!);
    // Even a programming error in the destination callback cannot introduce an
    // external/local URL, attribute or executable link into the package.
    return href && /^#[a-zA-Z0-9-]+$/u.test(href)
      ? `<a class="${cls} scene-content" href="${href}" aria-label="${escapeSceneText(node.label)}. Open supporting code">${content}</a>`
      : `<div class="${cls} scene-content">${content}<small>Supporting source omitted.</small></div>`;
  };
  return `<div class="offline-scene" role="group" aria-label="${escapeSceneText(scene.title)}"><h4>${escapeSceneText(scene.title)}</h4>${draw(0)}</div>`;
}

/** The offline reader has a separate paper palette and no animated transitions.
 * Layout primitives mirror HtmlScene; all style tokens are app-owned constants.
 */
export const OFFLINE_SCENE_CSS = `
.offline-scene{container-type:inline-size;min-height:280px;padding:20px;border:1px solid var(--line);border-radius:12px;background:var(--panel);display:flex;flex-direction:column;justify-content:center;gap:24px}
.offline-scene h4{text-align:center;margin:0;font-size:19px}.scene-element{min-width:0;--scene-color:var(--line)}.scene-tone-accent{--scene-color:#416ba6}.scene-tone-positive{--scene-color:#3e7952}.scene-tone-warning{--scene-color:#a56c18}.scene-tone-muted{color:var(--muted)}
.scene-row,.scene-compare{display:flex;flex-wrap:wrap;justify-content:center;align-items:center;gap:18px}.scene-row>:not(.scene-arrow),.scene-compare>*{flex:1 1 130px}.scene-column{display:flex;flex-direction:column;gap:16px}.scene-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,140px),1fr));gap:12px}.scene-panel{display:flex;flex-direction:column;gap:12px;padding:20px;border:1px solid var(--scene-color);border-radius:12px;background:var(--soft)}.scene-stack{display:flex;flex-direction:column-reverse;gap:6px;padding:12px;border-inline:2px solid var(--scene-color);border-bottom:2px solid var(--scene-color)}
.scene-timeline{position:relative;display:grid;grid-auto-flow:column;grid-auto-columns:minmax(0,1fr);align-items:stretch;gap:14px;padding-top:30px}.scene-timeline:before{content:'';position:absolute;top:11px;left:4%;right:4%;height:2px;background:var(--line)}.scene-timeline>*{position:relative}.scene-timeline>*:before{content:'';position:absolute;z-index:1;top:-25px;left:12px;width:12px;height:12px;border:2px solid var(--scene-color);border-radius:50%;background:var(--panel)}.scene-lanes{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,210px),1fr));gap:12px;align-items:stretch}.scene-lanes>*{border-inline-start:3px solid var(--scene-color);padding-inline-start:12px}.scene-branch{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;align-items:stretch}.scene-branch-item{box-sizing:border-box;min-width:0;width:100%;container:scene-branch-item/inline-size}.scene-branch>:first-child{grid-column:1/-1;justify-self:center;width:min(100%,540px);border-bottom:2px solid var(--scene-color)}.scene-beforeAfter{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px;align-items:stretch}.scene-beforeAfter>*{position:relative;padding-top:34px}.scene-beforeAfter>:first-child:before,.scene-beforeAfter>:last-child:before{position:absolute;top:10px;left:18px;color:var(--muted);font:600 10px/1.2 system-ui;letter-spacing:.1em}.scene-beforeAfter>:first-child:before{content:'BEFORE'}.scene-beforeAfter>:last-child:before{content:'AFTER'}
.scene-transform{box-sizing:border-box;width:min(100%,960px);margin-inline:auto;display:grid;grid-template-columns:minmax(0,1fr) 24px minmax(0,1fr) 24px minmax(0,1fr);align-items:center;gap:12px}.scene-transform-slot{min-width:0;display:grid;align-content:center;gap:9px}.scene-transform-slot>small{color:var(--muted);text-align:center;font:600 10px/1.2 system-ui;letter-spacing:.1em;text-transform:uppercase}.scene-transform-slot>.scene-element{max-width:none}.scene-transform-arrow{color:#416ba6;justify-self:center}.scene-tree{display:flex;flex-direction:column;align-items:stretch;gap:10px;margin-inline-start:12px;padding-inline-start:20px;border-inline-start:1px solid var(--scene-color)}.scene-tree>*{position:relative}.scene-tree>*:before{content:'';position:absolute;top:24px;inset-inline-start:-20px;width:12px;border-top:1px solid var(--scene-color)}.scene-queue{position:relative;display:flex;flex-wrap:wrap;justify-content:center;align-items:stretch;gap:8px;padding:30px 48px;border-block:2px solid var(--scene-color)}.scene-queue:before,.scene-queue:after{position:absolute;top:7px;color:var(--muted);font:600 10px/1.2 system-ui;letter-spacing:.1em}.scene-queue:before{content:'FRONT';inset-inline-start:4px}.scene-queue:after{content:'BACK';inset-inline-end:4px}
.scene-selector{width:min(100%,760px);margin-inline:auto;display:flex;flex-direction:column;gap:4px}.scene-selector-step{display:grid;grid-template-columns:72px minmax(0,1fr);align-items:start;gap:12px}.scene-selector-label{justify-self:end;margin-top:18px;padding:5px 9px;border:1px solid var(--scene-color);border-radius:999px;background:var(--panel);color:var(--muted);font:600 10px/1.2 system-ui;letter-spacing:.08em;text-transform:uppercase;white-space:nowrap}.scene-selector-step>.scene-branch{padding:14px;border:1px solid var(--scene-color);border-radius:12px;background:var(--panel)}.scene-selector-handoff{grid-column:2;display:flex;align-items:center;gap:7px;padding:5px 0 9px 20px;color:var(--muted);font-size:12px}.scene-selector-handoff>span:first-child{color:var(--scene-color)}
.scene-content{display:flex;flex-direction:column;justify-content:center;gap:14px;padding:18px;min-height:78px;border:1px solid var(--scene-color);border-radius:10px;background:var(--soft);color:var(--ink);text-decoration:none}.scene-content:focus-visible,.scene-content:hover{outline:2px solid var(--green);outline-offset:3px}.scene-label{font-size:13px;color:var(--muted)}.scene-value{white-space:pre-wrap;overflow-wrap:anywhere;font:500 19px/1.45 ui-monospace,monospace}.scene-code pre{margin:0;white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.7 ui-monospace,monospace}.scene-token{text-align:center;align-items:center;border-radius:999px;padding:12px 20px;min-height:44px}.scene-token .scene-label{color:var(--ink)}.scene-text{min-height:0;border:0;background:transparent;padding:4px}.scene-metric{text-align:center;align-items:center;border:0;background:transparent}.scene-metric .scene-value{font:600 clamp(32px,7cqw,58px)/1.1 system-ui;color:var(--scene-color)}
.scene-bytes{display:flex;flex-wrap:wrap;justify-content:center;gap:4px}.scene-bytes>span{display:flex;flex-direction:column;align-items:center;gap:7px}.scene-bytes b{padding:10px 9px;border:1px solid var(--scene-color);border-radius:4px;font:15px ui-monospace,monospace}.scene-bytes small{font:10px ui-monospace,monospace}.scene-matrix{display:grid;gap:3px;width:min(100%,220px);align-self:center}.scene-matrix i{aspect-ratio:1;background:#fff;border:1px solid var(--line)}.scene-matrix .filled{background:var(--green);border-color:var(--green)}.scene-content table{width:100%;border-collapse:collapse;font-size:12px}.scene-content th,.scene-content td{padding:9px;text-align:left;border-bottom:1px solid var(--line);overflow-wrap:anywhere}.scene-origin{font-size:11px;color:var(--muted);font-style:italic}.scene-arrow{display:grid;place-items:center;flex:0 0 24px;color:var(--muted)}.scene-row>.scene-arrow{transform:rotate(-90deg)}.scene-divider{border:0;border-top:1px solid var(--line);width:100%}
.offline-story-scene{padding:20px 0;scroll-margin-top:16px}.scene-nav{display:flex;justify-content:space-between;gap:12px;margin:12px 0}.scene-nav a{font-size:14px}.scene-transcript{margin:12px 0}.scene-transcript summary{cursor:pointer}.scene-transcript p{white-space:pre-wrap}
@container(max-width:560px){.scene-row,.scene-compare{flex-direction:column;align-items:stretch}.scene-row>:not(.scene-arrow),.scene-compare>*{flex-basis:auto}.scene-row>.scene-arrow{transform:none}.scene-beforeAfter{grid-template-columns:1fr}}
@container(max-width:520px){.scene-timeline{grid-auto-flow:row;grid-auto-columns:auto;gap:12px;padding:0 0 0 24px}.scene-timeline:before{top:4px;bottom:4px;left:6px;right:auto;width:2px;height:auto}.scene-timeline>*{padding-inline-start:8px}.scene-timeline>*:before{top:12px;left:-24px}.scene-transform{grid-template-columns:minmax(0,1fr);gap:8px}.scene-transform-arrow{transform:rotate(90deg)}.scene-queue{padding:30px 8px 14px}}
@container scene-branch-item(max-width:720px){.scene-transform{grid-template-columns:minmax(0,1fr);gap:8px}.scene-transform-arrow{transform:rotate(90deg)}}
@container(max-width:480px){.scene-branch{grid-template-columns:1fr}.scene-branch>:first-child{grid-column:auto;width:100%}}
@container(max-width:440px){.scene-selector-step{grid-template-columns:1fr;gap:6px}.scene-selector-label{justify-self:start;margin:0}.scene-selector-handoff{grid-column:1;padding:4px 0 8px 12px}}
`;
