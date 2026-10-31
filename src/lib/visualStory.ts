import type { ChapterSection, ChapterSourceAnchor, EvidenceLinkedVisualSpec, LinkedVisualEdge, LinkedVisualNode } from './commands';
import { sceneNarrations, validStoredScenePresentation } from './scenePresentation.ts';
import { cleanStoryProgress, FOCUSED_STORY_LAYOUT_VERSION, sameSection, type FlowViewport, type SectionIdentity, type StoryProgress } from './readerProgress.ts';

export interface StoryScene {
  id: string;
  title: string;
  narration: string;
  anchorIds: string[];
  nodeIds: string[];
}

export interface StoryLayoutResult {
  direction: 'horizontal' | 'vertical';
  nodes: { id: string; position: { x: number; y: number } }[];
}

/** A full graph is an authored view only when a current presentation includes a graph scene. */
export function supportsStoryFullMap(visual: EvidenceLinkedVisualSpec | undefined): boolean {
  return Boolean(visual?.presentation?.scenes.some((scene) => scene.mode === 'graph'));
}

export const STORY_NODE_WIDTH = 195;
export const STORY_EDGE_LABEL_MAX_WIDTH = 120;
export const STORY_EDGE_LABEL_HEIGHT = 40;
export const STORY_EDGE_LABEL_GAP = 12;

/** Restore only a focused node or edge that still exists in this section's visual. */
export function restoreStoryFocus(visual:EvidenceLinkedVisualSpec|undefined,nodeId?:string,edgeId?:string):{nodeId:string|null;edgeId:string|null} {
  const node=visual?.nodes.find((item)=>item.nodeId===nodeId);
  const edge=visual?.edges.find((item)=>item.edgeId===edgeId);
  const focusedNodeId=node?.nodeId??edge?.fromNodeId??null;
  return {nodeId:focusedNodeId,edgeId:edge?.fromNodeId===focusedNodeId?edge.edgeId:null};
}

const normalize = (text: string) => text.trim().replace(/\s+/gu, ' ');

/** Narrow a visual node's source link only when its visible code expression is
 * a unique, byte-exact match inside the already verified chapter citation.
 * Natural-language descriptions and ambiguous matches keep the full citation.
 */
function sourceAnchorForVisualText(anchor: ChapterSourceAnchor, label: string, detail?: string | null, sourceValue?: string): ChapterSourceAnchor {
  const excerptBytes = new TextEncoder().encode(anchor.excerpt).byteLength;
  if (!Number.isSafeInteger(anchor.startByte) || !Number.isSafeInteger(anchor.endByteInclusive)
    || anchor.endByteInclusive < anchor.startByte
    || excerptBytes !== anchor.endByteInclusive - anchor.startByte + 1) return anchor;

  const candidates = new Set<string>();
  if (sourceValue !== undefined) {
    if (sourceValue.length >= 1 && sourceValue.length <= 240) candidates.add(sourceValue);
  } else {
    const codeLike = /(?:\b[A-Za-z_$][\w$]*(?:(?:::|\.|->)[A-Za-z_$][\w$]*)*\s*\([^()\r\n]{0,240}\)|\b0[xX][0-9a-fA-F]+\b)/gu;
    for (const description of [label, detail ?? '']) {
      for (const match of description.matchAll(/`([^`\r\n]{1,240})`/gu)) candidates.add(match[1]);
      for (const match of description.matchAll(codeLike)) candidates.add(match[0]);
    }
  }

  const exactMatches = [...candidates].flatMap((candidate) => {
    if (candidate.length < 1) return [];
    const start = anchor.excerpt.indexOf(candidate);
    if (start < 0 || anchor.excerpt.indexOf(candidate, start + 1) !== -1) return [];
    const end = start + candidate.length;
    const before = anchor.excerpt.slice(0, start);
    const priorLines = before.match(/\r\n|\r|\n/gu)?.length ?? 0;
    const lineStart = Math.max(before.lastIndexOf('\n'), before.lastIndexOf('\r')) + 1;
    const startLine = anchor.startLine + priorLines;
    const endLine = startLine + (anchor.excerpt.slice(start, end).match(/\r\n|\r|\n/gu)?.length ?? 0);
    if (startLine !== endLine) return [];
    const columnBase = priorLines === 0 ? anchor.startColumn : 1;
    const startColumn = columnBase + new TextEncoder().encode(before.slice(lineStart)).byteLength;
    const endColumn = startColumn + new TextEncoder().encode(candidate).byteLength - 1;
    return [{ candidate, start, end, startLine, endLine, startColumn, endColumn }];
  });

  if (exactMatches.length === 0) return anchor;
  const lines = new Set(exactMatches.map((match) => `${match.startLine}:${match.endLine}`));
  if (lines.size !== 1) return anchor;
  const match = exactMatches.sort((left, right) => right.candidate.length - left.candidate.length)[0];
  const startByte = anchor.startByte + new TextEncoder().encode(anchor.excerpt.slice(0, match.start)).byteLength;
  const endByteInclusive = startByte + new TextEncoder().encode(match.candidate).byteLength - 1;
  return {
    ...anchor,
    startByte,
    endByteInclusive,
    startLine: match.startLine,
    endLine: match.endLine,
    startColumn: match.startColumn,
    endColumn: match.endColumn,
    excerpt: match.candidate,
  };
}

export function sourceAnchorForVisualNode(anchor: ChapterSourceAnchor, node: Pick<LinkedVisualNode, 'label' | 'detail'>, sourceValue?: string): ChapterSourceAnchor {
  return sourceAnchorForVisualText(anchor, node.label, node.detail, sourceValue);
}

/** Narrow a visual edge's source link when its label names one unique code expression. */
export function sourceAnchorForVisualEdge(
  anchor: ChapterSourceAnchor,
  edge: Pick<LinkedVisualEdge, 'label'>,
  targetAnchor?: ChapterSourceAnchor,
  sourceAnchor?: ChapterSourceAnchor,
): ChapterSourceAnchor {
  const byLabel = sourceAnchorForVisualText(anchor, edge.label);
  if (byLabel !== anchor) return byLabel;

  // When a natural-language edge points at a function node, use the declared
  // target symbol to locate its unique call site inside the edge's own evidence.
  // If either side is ambiguous, keep the validated citation broad.
  if (targetAnchor) {
    const declarations = [...targetAnchor.excerpt.matchAll(/\b(?:fn|function|def|func)\s+([A-Za-z_$][\w$]*)\s*\(/gu)];
    const names = new Set(declarations.map((match) => match[1]));
    if (names.size === 1) {
      const name = [...names][0].replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
      const callPattern = new RegExp(`(?:\\b[A-Za-z_$][\\w$]*::)*${name}\\s*\\([^()\\r\\n]{0,240}\\)`, 'gu');
      const calls = [...anchor.excerpt.matchAll(callPattern)].filter((match) => {
        const lineStart = Math.max(anchor.excerpt.lastIndexOf('\n', match.index - 1), anchor.excerpt.lastIndexOf('\r', match.index - 1)) + 1;
        return !/\b(?:fn|function|def|func)\s+$/u.test(anchor.excerpt.slice(lineStart, match.index));
      });
      if (calls.length === 1) return sourceAnchorForVisualText(anchor, edge.label, undefined, calls[0][0]);
    }
  }

  // A natural-language edge can still be exact when its source node is bound
  // to a single branch condition. Find that condition in the edge's own
  // validated excerpt and narrow only to its single direct return call.
  if (sourceAnchor?.path === anchor.path && sourceAnchor.sourceSha256 === anchor.sourceSha256) {
    const context = sourceAnchor.excerpt.trim();
    const contextStart = anchor.excerpt.indexOf(context);
    if (context && contextStart >= 0 && anchor.excerpt.indexOf(context, contextStart + context.length) === -1) {
      const contextOpenBrace = context.indexOf('{');
      const openBrace = contextOpenBrace >= 0
        ? contextStart + contextOpenBrace
        : anchor.excerpt.indexOf('{', contextStart + context.length);
      const closeBrace = openBrace >= 0 ? anchor.excerpt.indexOf('}', openBrace + 1) : -1;
      if (openBrace >= 0 && closeBrace > openBrace) {
        const headerLineStart = Math.max(anchor.excerpt.lastIndexOf('\n', openBrace - 1), anchor.excerpt.lastIndexOf('\r', openBrace - 1)) + 1;
        const headerLineEnd = [anchor.excerpt.indexOf('\n', openBrace), anchor.excerpt.indexOf('\r', openBrace)]
          .filter((position) => position >= 0).reduce((end, position) => Math.min(end, position), anchor.excerpt.length);
        const header = anchor.excerpt.slice(headerLineStart, headerLineEnd).trim();
        if (!/^\s*(?:\}\s*)?(?:else\s+)?(?:if|while|for)\b[\s\S]*\{\s*$/u.test(header)) return anchor;
        if (contextStart < headerLineStart || contextStart > openBrace
          || (!context.includes('{') && contextStart + context.length > openBrace)) return anchor;
        if (anchor.excerpt.slice(contextStart, openBrace).includes('\n') || anchor.excerpt.slice(contextStart, openBrace).includes('\r')) return anchor;
        const body = anchor.excerpt.slice(openBrace + 1, closeBrace);
        // Nested blocks and embedded braces are intentionally left broad.
        if (!body.includes('{')) {
          const returns = [...body.matchAll(/\breturn\s+([^;{}]+);/gu)];
          const expression = returns[0]?.[1]?.trim();
          if (returns.length === 1 && expression && !/[\r\n]/u.test(expression)
            && /^(?:[A-Za-z_$][\w$]*::)*[A-Za-z_$][\w$]*\s*\([^()\r\n]{0,240}\)$/u.test(expression)) {
            const narrowed = sourceAnchorForVisualText(anchor, edge.label, undefined, expression);
            if (narrowed !== anchor) return narrowed;
          }
        }
      }
    }
  }

  return anchor;
}

/** Select a node's validated source binding before falling back to its cited anchors. */
export function sourceEvidenceAnchorForVisualNode(
  anchors: readonly ChapterSourceAnchor[],
  node: Pick<LinkedVisualNode, 'evidenceAnchorIds' | 'sourceAnchorId'>,
  sourceValue?: string,
): ChapterSourceAnchor | undefined {
  const allowed = new Set(node.evidenceAnchorIds);
  if (node.sourceAnchorId) {
    return allowed.has(node.sourceAnchorId)
      ? anchors.find((anchor) => anchor.anchorId === node.sourceAnchorId)
      : undefined;
  }
  if (sourceValue) {
    const matching = anchors.find((anchor) => allowed.has(anchor.anchorId) && anchor.excerpt.includes(sourceValue));
    if (matching) return matching;
  }
  return anchors.find((anchor) => allowed.has(anchor.anchorId));
}

/** Prefer evidence from the claim that authored this scene. This matters when
 * the same literal appears in several claims and the visual node is reused.
 */
export function sourceEvidenceAnchorForVisualNodeInScene(
  anchors: readonly ChapterSourceAnchor[],
  node: Pick<LinkedVisualNode, 'evidenceAnchorIds' | 'sourceAnchorId'>,
  sourceValue?: string,
  sceneAnchorIds?: readonly string[],
): ChapterSourceAnchor | undefined {
  const allowed = new Set(node.evidenceAnchorIds);
  if (sceneAnchorIds) {
    const sceneAllowed = new Set(sceneAnchorIds);
    const preferred = anchors.filter((anchor) => sceneAllowed.has(anchor.anchorId) && allowed.has(anchor.anchorId));
    const boundSceneAnchor = preferred.find((anchor) => anchor.anchorId === node.sourceAnchorId);
    if (boundSceneAnchor && (!sourceValue || boundSceneAnchor.excerpt.includes(sourceValue))) return boundSceneAnchor;
    const exactSceneValueAnchor = sourceValue ? preferred.find((anchor) => anchor.excerpt.includes(sourceValue)) : undefined;
    const preferredAnchor = exactSceneValueAnchor ?? boundSceneAnchor ?? preferred[0];
    if (preferredAnchor) return preferredAnchor;
  }
  const boundAnchor = node.sourceAnchorId && allowed.has(node.sourceAnchorId)
    ? anchors.find((anchor) => anchor.anchorId === node.sourceAnchorId)
    : undefined;
  if (boundAnchor && (!sourceValue || boundAnchor.excerpt.includes(sourceValue))) return boundAnchor;
  const exactValueAnchor = sourceValue
    ? anchors.find((anchor) => allowed.has(anchor.anchorId) && anchor.excerpt.includes(sourceValue))
    : undefined;
  return exactValueAnchor ?? boundAnchor ?? sourceEvidenceAnchorForVisualNode(anchors, node);
}

/** Resolve an edge only through its validated evidence bindings. */
export function sourceEvidenceAnchorForVisualEdge(
  anchors: readonly ChapterSourceAnchor[],
  edge: Pick<LinkedVisualEdge, 'evidenceAnchorIds'>,
): ChapterSourceAnchor | undefined {
  const allowed = new Set(edge.evidenceAnchorIds);
  return anchors.find((anchor) => allowed.has(anchor.anchorId));
}

/** Resolve code values authored for each composition node so the graph view can
 * open the same exact source line as the HTML composition view. If one graph
 * node has conflicting source values across authored scenes, leave it broad.
 */
export function sourceValuesByVisualNode(visual: EvidenceLinkedVisualSpec): Map<string, string> {
  const values = new Map<string, Set<string>>();
  for (const scene of visual.presentation.scenes) {
    if (scene.mode !== 'composition') continue;
    for (const element of scene.elements) {
      if (element.valueOrigin !== 'source' || element.nodeIndex === null) continue;
      const node = visual.nodes[element.nodeIndex];
      if (!node) continue;
      const nodeValues = values.get(node.nodeId) ?? new Set<string>();
      nodeValues.add(element.value);
      values.set(node.nodeId, nodeValues);
    }
  }
  return new Map([...values].flatMap(([nodeId, nodeValues]) =>
    nodeValues.size === 1 ? [[nodeId, [...nodeValues][0]] as const] : []));
}

function sentences(text: string): string[] {
  const result: string[] = [];
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    if (text[i] === '\n' || ('.!?'.includes(text[i]) && (i + 1 === text.length || /\s/u.test(text[i + 1])))) {
      const part = text.slice(start, text[i] === '\n' ? i : i + 1).trim();
      if (part) result.push(part);
      start = i + 1;
    }
  }
  if (text.slice(start).trim()) result.push(text.slice(start).trim());
  return result;
}

/** Scene boundaries are authored claims, never estimated word timestamps or runtime traces.
 * Older summaries and edited scripts stay whole unless every spoken sentence is accounted for.
 */
export function storyScenes(section: ChapterSection, visual?: EvidenceLinkedVisualSpec, savedScript?: string): StoryScene[] {
  // Export and audio scheduling call without a visual argument. Read the same
  // validated presentation, so they never fall back to technical claim prose.
  const candidate = visual ?? (section.visual?.schemaVersion === 3 ? section.visual : undefined);
  const presentation = candidate?.presentation;
  if (presentation?.schemaVersion === 3 && Array.isArray(candidate?.nodes)
    && validStoredScenePresentation(presentation, candidate.nodes, section.claims)) {
    const scripts = sceneNarrations(presentation, section.claims)!;
    if (normalize(scripts.join(' ')) === normalize(savedScript ?? section.spokenText)) {
      return scripts.map((narration, i) => {
        const claim = section.claims[i];
        const anchorIds = claim.anchors.map(a => a.anchorId);
        return { id: claim.claimId, title: presentation.scenes[i].title, narration, anchorIds,
          nodeIds: candidate.nodes.filter(n => n.evidenceAnchorIds.some(id => anchorIds.includes(id))).map(n => n.nodeId) };
      });
    }
  }
  const seen = new Set<string>();
  const scenes = section.claims.filter((claim) => claim.anchors.length > 0).flatMap((claim) => {
    const spoken = sentences(claim.prose)
      .map(normalize).filter(Boolean).map((sentence) => claim.category.kind === 'inference'
        ? `${claim.category.visibleLabel.trim()}: ${sentence}` : sentence)
      .filter((sentence) => { if (seen.has(sentence)) return false; seen.add(sentence); return true; });
    if (!spoken.length) return [];
    const anchorIds = claim.anchors.map((anchor) => anchor.anchorId);
    return [{ id: claim.claimId, title: section.title, narration: spoken.join(' '), anchorIds,
      nodeIds: visual?.nodes.filter((node) => node.evidenceAnchorIds.some((id) => anchorIds.includes(id))).map((node) => node.nodeId) ?? [] }];
  });
  const script = savedScript ?? section.spokenText;
  if (scenes.length && normalize(scenes.map((scene) => scene.narration).join(' ')) === normalize(script)) return scenes;
  return [{ id: section.sectionId, title: section.title, narration: script, anchorIds: section.claims.flatMap((claim) => claim.anchors.map((anchor) => anchor.anchorId)), nodeIds: visual?.nodes.map((node) => node.nodeId) ?? [] }];
}

/** A bounded layered drawing preserves the declared edges, including cycles and branches.
 * Drawing order has no runtime meaning. Unresolved cyclic nodes use authored order.
 */
export function storyLayoutWithDirection(visual: EvidenceLinkedVisualSpec, includedNodeIds?: ReadonlySet<string>): StoryLayoutResult {
  // Focused scenes must be laid out from their own visible subgraph. Laying
  // out the full chapter and filtering afterward leaves invisible branches
  // contributing to the viewport bounds, which shrinks the narrated content.
  const nodes = includedNodeIds
    ? visual.nodes.filter((node) => includedNodeIds.has(node.nodeId))
    : visual.nodes;
  const nodeIds = new Set(nodes.map((node) => node.nodeId));
  const edges = visual.edges.filter((edge) => nodeIds.has(edge.fromNodeId) && nodeIds.has(edge.toNodeId));
  const remaining = new Set(nodeIds);
  const rank = new Map<string, number>();
  while (remaining.size) {
    const ready = [...remaining].filter((id) => edges.filter((edge) => edge.toNodeId === id && edge.fromNodeId !== id).every((edge) => !remaining.has(edge.fromNodeId)));
    if (!ready.length) { for (const id of remaining) rank.set(id, rank.size); break; }
    for (const id of ready) {
      const parents = edges.filter((edge) => edge.toNodeId === id && edge.fromNodeId !== id);
      rank.set(id, Math.max(0, ...parents.map((edge) => (rank.get(edge.fromNodeId) ?? -1) + 1)));
      remaining.delete(id);
    }
  }
  const preferredHorizontal = visual.layoutHint.direction !== 'topToBottom';
  const deepestRank = Math.max(0, ...rank.values());
  // A long single-lane route becomes unreadable when fit into the Guide's
  // bounded-width canvas. Preserve the authored relationships, but turn deep
  // routes into a vertical reading path so labels remain legible at the
  // default zoom. Shorter diagrams keep their requested direction.
  const horizontal = preferredHorizontal && !(nodes.length >= 6 && deepestRank >= 4);
  return {
    direction: horizontal ? 'horizontal' : 'vertical',
    nodes: nodes.map((node) => {
      const level = rank.get(node.nodeId) ?? 0;
      const siblings = nodes.filter((other) => rank.get(other.nodeId) === level);
      const across = (siblings.findIndex((other) => other.nodeId === node.nodeId) - (siblings.length - 1) / 2) * 132;
      return { id: node.nodeId, position: horizontal ? { x: level * 340, y: across } : { x: across * 1.8, y: level * 94 } };
    }),
  };
}

/** Preserve the established position-only helper for callers that only draw nodes. */
export function storyLayout(visual: EvidenceLinkedVisualSpec, includedNodeIds?: ReadonlySet<string>) {
  return storyLayoutWithDirection(visual, includedNodeIds).nodes;
}

export function sceneNodeIds(visual: EvidenceLinkedVisualSpec, scene: StoryScene): Set<string> {
  const ids = new Set(scene.nodeIds);
  for (const edge of visual.edges) {
    if (scene.nodeIds.includes(edge.fromNodeId) || scene.nodeIds.includes(edge.toNodeId)) {
      ids.add(edge.fromNodeId); ids.add(edge.toNodeId);
    }
  }
  return ids;
}

/** Keep dense edge labels scannable; the complete label remains available to assistive tech and hover text. */
export function visibleStoryEdgeLabel(label: string, maxCharacters = 36): string {
  const text = label.trim();
  if (text.length <= maxCharacters || maxCharacters < 2) return text;
  const prefix = text.slice(0, maxCharacters - 1);
  const lastSpace = prefix.lastIndexOf(' ');
  const boundary = lastSpace >= Math.floor(maxCharacters * 0.6) ? lastSpace : prefix.length;
  return `${prefix.slice(0, boundary).trimEnd()}…`;
}

/** Put vertical-path labels beside the cards, not over the short connector gap. */
export function storyEdgeLabelPosition(
  sourceX: number,
  targetX: number,
  labelX: number,
  labelY: number,
  vertical: boolean,
  preferredSide: number,
): { x: number; y: number } {
  if (!vertical) return { x: labelX, y: labelY };
  const side = Math.sign(targetX - sourceX) || (preferredSide < 0 ? -1 : 1);
  const cardAndLabelHalfWidths = (STORY_NODE_WIDTH + STORY_EDGE_LABEL_MAX_WIDTH) / 2;
  const branchClearance = Math.abs(targetX - sourceX) / 2;
  return {
    x: labelX + side * (cardAndLabelHalfWidths + STORY_EDGE_LABEL_GAP + branchClearance),
    y: labelY,
  };
}

/** Include custom edge labels when framing a Guide graph. React Flow's built-in
 * fitView measures node cards only, so labels rendered in the edge layer can
 * otherwise disappear outside a narrow or restored viewport.
 */
export function storyViewportBounds(
  bounds: { x: number; y: number; width: number; height: number },
  edgeLabels: { x: number; y: number; width: number; height: number }[],
): { x: number; y: number; width: number; height: number } {
  let left = bounds.x;
  let top = bounds.y;
  let right = bounds.x + bounds.width;
  let bottom = bounds.y + bounds.height;
  for (const label of edgeLabels) {
    if (![label.x, label.y, label.width, label.height].every(Number.isFinite)
      || label.width <= 0 || label.height <= 0) continue;
    left = Math.min(left, label.x);
    top = Math.min(top, label.y);
    right = Math.max(right, label.x + label.width);
    bottom = Math.max(bottom, label.y + label.height);
  }
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/** Saved reader viewports belong to a particular canvas size. Keep them while
 * every visible node fits, then let the player restore the scene with a useful
 * view when its pane has become narrower or shorter.
 */
export type StoryProgressRead =
  | { status: 'missing' }
  | { status: 'available'; progress: StoryProgress }
  | { status: 'unavailable' };

export function readStoryProgressRecord(raw: string | null, identity: SectionIdentity, scenes: StoryScene[]): StoryProgressRead {
  if (raw === null) return { status: 'missing' };
  if (new TextEncoder().encode(raw).byteLength > 64_000) return { status: 'unavailable' };
  let value: unknown;
  try { value = JSON.parse(raw); } catch { return { status: 'unavailable' }; }
  if (!isCurrentStoryProgress(value)) return { status: 'unavailable' };
  const clean = cleanStoryProgress(value);
  if (!clean || !sameSection(clean, identity) || !scenes.some(scene => scene.id === clean.sceneId)) return { status: 'unavailable' };
  return { status: 'available', progress: clean };
}

function sectionStoryProgress(progress: StoryProgress | undefined, scenes: StoryScene[]): { index: number; positions: [string, number][]; fullMap: boolean; viewport?: FlowViewport; focusedNodeId?: string; focusedEdgeId?: string } {
  const positions = progress?.positions ?? {};
  return { index: Math.max(0, scenes.findIndex((scene) => scene.id === progress?.sceneId)), positions: scenes.map((scene) => {
    const position = Object.hasOwn(positions, scene.id) ? positions[scene.id] : 0;
    return [scene.id, position] as [string, number];
  }), fullMap: progress?.fullMap ?? false,
    viewport: progress?.sceneId === scenes.find((scene) => scene.id === progress?.sceneId)?.id ? progress?.viewport : undefined,
    ...(progress?.focusedNodeId?{focusedNodeId:progress.focusedNodeId}:{}),
    ...(progress?.focusedEdgeId?{focusedEdgeId:progress.focusedEdgeId}:{}) };
}

/** Restore only the one current, section-scoped record. Unknown or previous
 * shapes start at the first scene without carrying guessed playback state. */
export function readSectionStoryProgress(progress:StoryProgress|undefined,identity:SectionIdentity,scenes:StoryScene[],fallbackPosition=0) {
  const clean = cleanStoryProgress(progress);
  const matching = clean && sameSection(clean, identity) && scenes.some(scene => scene.id === clean.sceneId) ? clean : undefined;
  const result = sectionStoryProgress(matching, scenes);
  if(scenes.length===1&&(!matching||!Object.hasOwn(matching.positions,scenes[0].id))&&Number.isFinite(fallbackPosition)&&fallbackPosition>0&&fallbackPosition<=3600)result.positions[0][1]=fallbackPosition;
  result.viewport = matching?.sceneId===scenes[result.index]?.id?matching.viewport:undefined;
  return result;
}

function isCurrentStoryProgress(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  const required = ['revisionId', 'sectionId', 'sectionRevisionId', 'sceneId', 'positions', 'fullMap', 'focusLayoutVersion'];
  const allowed = [...required, 'viewport', 'focusedNodeId', 'focusedEdgeId'];
  const actual = Object.keys(record);
  return actual.every(key => allowed.includes(key))
    && required.every(key => Object.hasOwn(record, key))
    && (record.focusLayoutVersion === FOCUSED_STORY_LAYOUT_VERSION || record.focusLayoutVersion === 4 || record.focusLayoutVersion === 5)
    && (record.viewport === undefined || Object.hasOwn(record, 'viewport'));
}
