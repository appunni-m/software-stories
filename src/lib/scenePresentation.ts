import type { ChapterClaim, LinkedVisualNode } from './commands';

export type ElementKind = 'row' | 'column' | 'grid' | 'panel' | 'compare' | 'stack'
  | 'card' | 'token' | 'code' | 'metric' | 'bytes' | 'matrix' | 'table' | 'text' | 'arrow' | 'divider';
export interface SceneElement {
  kind: ElementKind;
  parentIndex: number | null;
  nodeIndex: number | null;
  tone: 'neutral' | 'accent' | 'muted' | 'positive' | 'warning';
  value: string;
  valueOrigin: 'none' | 'source' | 'illustration';
}
export interface PresentationScene {
  claimIndex: number;
  title: string;
  narration: string;
  mode: 'composition' | 'graph';
  elements: SceneElement[];
}
export interface ScenePresentation { schemaVersion: 3; scenes: PresentationScene[] }
export const containerKinds = new Set<string>(['row', 'column', 'grid', 'panel', 'compare', 'stack']);
const kinds = new Set([...containerKinds, 'card', 'token', 'code', 'metric', 'bytes', 'matrix', 'table', 'text', 'arrow', 'divider']);
const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: string[]) => Object.keys(v).length === keys.length && keys.every(k => Object.hasOwn(v, k));
const bytes = (s: string) => new TextEncoder().encode(s).length;
const index = (v: unknown, bound: number): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0 && v < bound;

/** Keep the visual cue's meaning available when the composition is explored
 * with a keyboard or screen reader, even while its prose stays collapsed.
 */
export function sceneNodeAccessibleLabel(node: Pick<LinkedVisualNode, 'label' | 'detail'>, value: string, illustrated: boolean): string {
  const sentence = (text: string) => text.trim().replace(/[.!?]+$/u, '');
  return [
    sentence(node.label),
    value ? sentence(`Shown value: ${value}`) : '',
    node.detail ? sentence(`Meaning: ${node.detail}`) : '',
    illustrated ? 'Illustrative example' : '',
    'Open supporting code',
  ].filter(Boolean).join('. ');
}

/** Identity/coverage checks only; semantic faithfulness requires review against
 * the exact claim. Keep every scene and prefix inferred speech visibly. */
export function sceneNarrations(presentation: ScenePresentation, claims: ChapterClaim[]): string[] | null {
  if (presentation.schemaVersion !== 3 || presentation.scenes.length !== claims.length || !claims.length || claims.length > 20) return null;
  const scripts: string[] = [];
  for (const [i, scene] of presentation.scenes.entries()) {
    if (scene.claimIndex !== i || !claims[i].anchors.length || typeof scene.narration !== 'string' || !scene.narration.trim() || bytes(scene.narration) > 4000) return null;
    const text = scene.narration.trim().replace(/\s+/gu, ' ');
    if (claims[i].category?.kind === 'inference') {
      const label = claims[i].category.visibleLabel.trim();
      if (!label) return null;
      scripts.push(`${label}: ${text}`);
    } else scripts.push(text);
  }
  const length = bytes(scripts.join(' '));
  return length >= 20 && length <= 4000 ? scripts : null;
}

/** Called only after the whole tree is validated. Layout/tone changes cannot
 * turn identical visible content into a transformation. Values stay exact so
 * case and whitespace transformations remain expressible.
 */
function repeatsTransformation(elements: SceneElement[], nodes: LinkedVisualNode[]): boolean {
  const descendsFrom = (index: number, root: number): boolean => {
    let parent = elements[index].parentIndex;
    while (parent !== null) { if (parent === root) return true; parent = elements[parent].parentIndex; }
    return false;
  };
  const signature = (root: number) => elements.flatMap((e, i) =>
    (i === root || descendsFrom(i, root)) && !containerKinds.has(e.kind) && e.kind !== 'arrow' && e.kind !== 'divider'
      ? [[nodes[e.nodeIndex!].label.trim(), e.value]] : []);
  return elements.some((element, i) => {
    if (element.kind !== 'arrow' || element.parentIndex === null) return false;
    const siblings = elements.flatMap((e, n) => e.parentIndex === element.parentIndex ? [n] : []);
    const at = siblings.indexOf(i);
    const content = (indices: number[]) => indices.map(signature).find(s => s.length);
    const before = content(siblings.slice(0, at).reverse());
    const after = content(siblings.slice(at + 1));
    return !!before && !!after && JSON.stringify(before) === JSON.stringify(after);
  });
}

/** Reject the entire presentation on unknown fields, foreign evidence or an
 * oversized tree. Strings are always text; the renderer never interprets HTML.
 */
export function validScenePresentation(raw: unknown, nodes: LinkedVisualNode[], claims: ChapterClaim[]): raw is ScenePresentation {
  if (!record(raw) || !exact(raw, ['schemaVersion', 'scenes']) || raw.schemaVersion !== 3 || !Array.isArray(raw.scenes)
    || raw.scenes.length !== claims.length || raw.scenes.length < 1 || raw.scenes.length > 20) return false;
  const valid = raw.scenes.every((scene, claimIndex) => {
    if (!record(scene) || !exact(scene, ['claimIndex', 'title', 'mode', 'elements', 'narration']) || scene.claimIndex !== claimIndex
      || typeof scene.title !== 'string' || !scene.title.trim() || bytes(scene.title) > 120
      || !['composition', 'graph'].includes(String(scene.mode)) || !Array.isArray(scene.elements)) return false;
    if (scene.mode === 'graph') return scene.elements.length === 0;
    const elements = scene.elements;
    if (!elements.length || elements.length > 48) return false;
    const depths: number[] = [];
    const counts = elements.map(() => 0);
    let leaves = 0;
    for (let i = 0; i < elements.length; i++) {
      const e = elements[i];
      if (!record(e) || !exact(e, ['kind', 'parentIndex', 'nodeIndex', 'tone', 'value', 'valueOrigin'])
        || !kinds.has(String(e.kind)) || !['neutral', 'accent', 'muted', 'positive', 'warning'].includes(String(e.tone))
        || typeof e.value !== 'string' || bytes(e.value) > 1024 || !['none', 'source', 'illustration'].includes(String(e.valueOrigin))) return false;
      if (i === 0 && e.parentIndex === null) depths.push(0);
      else if (index(e.parentIndex, i) && containerKinds.has(String(elements[e.parentIndex].kind))) {
        if (++counts[e.parentIndex] > 12) return false;
        depths.push(depths[e.parentIndex] + 1);
      } else return false;
      if (depths[i] > 6) return false;
      if (containerKinds.has(String(e.kind)) || e.kind === 'arrow' || e.kind === 'divider') {
        if (e.nodeIndex !== null || e.value !== '' || e.valueOrigin !== 'none') return false;
        continue;
      }
      leaves++;
      if (!index(e.nodeIndex, nodes.length)) return false;
      const anchors = claims[claimIndex].anchors.filter(a => nodes[e.nodeIndex as number].evidenceAnchorIds.includes(a.anchorId));
      if (!anchors.length) return false;
      if (e.valueOrigin === 'none' ? e.value !== '' : !e.value.trim()) return false;
      if (e.valueOrigin === 'source' && !anchors.some(a => a.excerpt.includes(e.value as string))) return false;
      if (e.kind === 'matrix') {
        const rows = e.value.split(/\r?\n/u); if (rows.at(-1) === '') rows.pop();
        if (!rows.length || rows.length > 16 || rows.some(r => !r.length || r.length > 24 || r.length !== rows[0].length || !/^[01.]+$/u.test(r))) return false;
      }
      if (e.kind === 'table') {
        const rows = e.value.split(/\r?\n/u); if (rows.at(-1) === '') rows.pop();
        if (!rows.length || rows.length > 8 || rows.some(r => r.split('|').length > 6 || r.split('|').length !== rows[0].split('|').length)) return false;
      }
      if (e.kind === 'bytes' && (!e.value.trim() || e.value.trim().split(/\s+/u).length > 32)) return false;
    }
    return leaves >= 1 && leaves <= 12 && elements.every((e, i) => !containerKinds.has(e.kind) || counts[i] > 0)
      && !repeatsTransformation(elements as SceneElement[], nodes);
  });
  return valid && sceneNarrations(raw as unknown as ScenePresentation, claims) !== null;
}
