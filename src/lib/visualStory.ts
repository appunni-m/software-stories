import type { ChapterSection, ChapterSourceAnchor, EvidenceLinkedVisualSpec, LinkedVisualEdge, LinkedVisualNode } from './commands';
import { sceneNarrations, validScenePresentation } from './scenePresentation.ts';
import { cleanStoryProgress, FOCUSED_STORY_LAYOUT_VERSION, sameSection, type FlowViewport, type SectionIdentity, type StoryProgress } from './readerProgress.ts';

export interface StoryScene {
  id: string;
  title: string;
  narration: string;
  anchorIds: string[];
  nodeIds: string[];
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
  for (const scene of visual.presentation?.scenes ?? []) {
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
  const candidate = visual ?? (section.visual?.schemaVersion === 2 ? section.visual : undefined);
  const presentation = candidate?.presentation;
  if (presentation?.schemaVersion === 3 && Array.isArray(candidate?.nodes)
    && validScenePresentation(presentation, candidate.nodes, section.claims)) {
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
export function storyLayout(visual: EvidenceLinkedVisualSpec, includedNodeIds?: ReadonlySet<string>) {
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
  const horizontal = visual.layoutHint.direction !== 'topToBottom';
  return nodes.map((node) => {
    const level = rank.get(node.nodeId) ?? 0;
    const siblings = nodes.filter((other) => rank.get(other.nodeId) === level);
    const across = (siblings.findIndex((other) => other.nodeId === node.nodeId) - (siblings.length - 1) / 2) * 132;
    return { id: node.nodeId, position: horizontal ? { x: level * 245, y: across } : { x: across * 1.8, y: level * 94 } };
  });
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

/** Saved reader viewports belong to a particular canvas size. Keep them while
 * every visible node fits, then let the player restore the scene with a useful
 * view when its pane has become narrower or shorter.
 */
export function readStoryProgress(raw: string | null, identity: SectionIdentity, scenes: StoryScene[]): StoryProgress | undefined {
  let value: unknown;
  try { value = raw && raw.length <= 16_000 ? JSON.parse(raw) : null; } catch { value = null; }
  if (!isCurrentStoryProgress(value)) return undefined;
  const clean = cleanStoryProgress(value);
  return clean && sameSection(clean, identity) && scenes.some(scene => scene.id === clean.sceneId) ? clean : undefined;
}

function sectionStoryProgress(progress: StoryProgress | undefined, scenes: StoryScene[]): { index: number; positions: [string, number][]; fullMap: boolean; viewport?: FlowViewport } {
  const positions = progress?.positions ?? {};
  return { index: Math.max(0, scenes.findIndex((scene) => scene.id === progress?.sceneId)), positions: scenes.map((scene) => {
    const position = Object.hasOwn(positions, scene.id) ? positions[scene.id] : 0;
    return [scene.id, position] as [string, number];
  }), fullMap: progress?.fullMap ?? false,
    viewport: progress?.sceneId === scenes.find((scene) => scene.id === progress?.sceneId)?.id ? progress?.viewport : undefined };
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

function isCurrentStoryProgress(value: unknown): value is StoryProgress {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  const required = ['revisionId', 'sectionId', 'sectionRevisionId', 'sceneId', 'positions', 'fullMap', 'focusLayoutVersion'];
  const allowed = [...required, 'viewport'];
  const actual = Object.keys(record);
  return actual.every(key => allowed.includes(key))
    && required.every(key => Object.hasOwn(record, key))
    && record.focusLayoutVersion === FOCUSED_STORY_LAYOUT_VERSION
    && (record.viewport === undefined || Object.hasOwn(record, 'viewport'));
}
