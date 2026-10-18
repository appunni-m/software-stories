import type { ChapterSection, EvidenceLinkedVisualSpec } from './commands';
import { sceneNarrations, validScenePresentation } from './scenePresentation.ts';
import { cleanStoryProgress, sameSection, type FlowViewport, type SectionIdentity, type StoryProgress } from './readerProgress.ts';

export interface StoryScene {
  id: string;
  narration: string;
  anchorIds: string[];
  nodeIds: string[];
}

const normalize = (text: string) => text.trim().replace(/\s+/gu, ' ');
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
        return { id: claim.claimId, narration, anchorIds,
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
    return [{ id: claim.claimId, narration: spoken.join(' '), anchorIds,
      nodeIds: visual?.nodes.filter((node) => node.evidenceAnchorIds.some((id) => anchorIds.includes(id))).map((node) => node.nodeId) ?? [] }];
  });
  const script = savedScript ?? section.spokenText;
  if (scenes.length && normalize(scenes.map((scene) => scene.narration).join(' ')) === normalize(script)) return scenes;
  return [{ id: section.sectionId, narration: script, anchorIds: section.claims.flatMap((claim) => claim.anchors.map((anchor) => anchor.anchorId)), nodeIds: visual?.nodes.map((node) => node.nodeId) ?? [] }];
}

/** A bounded layered drawing preserves the declared edges, including cycles and branches.
 * Drawing order has no runtime meaning. Unresolved cyclic nodes use authored order.
 */
export function storyLayout(visual: EvidenceLinkedVisualSpec) {
  const remaining = new Set(visual.nodes.map((node) => node.nodeId));
  const rank = new Map<string, number>();
  while (remaining.size) {
    const ready = [...remaining].filter((id) => visual.edges.filter((edge) => edge.toNodeId === id && edge.fromNodeId !== id).every((edge) => !remaining.has(edge.fromNodeId)));
    if (!ready.length) { for (const id of remaining) rank.set(id, rank.size); break; }
    for (const id of ready) {
      const parents = visual.edges.filter((edge) => edge.toNodeId === id && edge.fromNodeId !== id);
      rank.set(id, Math.max(0, ...parents.map((edge) => (rank.get(edge.fromNodeId) ?? -1) + 1)));
      remaining.delete(id);
    }
  }
  const horizontal = visual.layoutHint.direction !== 'topToBottom';
  return visual.nodes.map((node) => {
    const level = rank.get(node.nodeId) ?? 0;
    const siblings = visual.nodes.filter((other) => rank.get(other.nodeId) === level);
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
  const expected = ['revisionId', 'sectionId', 'sectionRevisionId', 'sceneId', 'positions', 'fullMap'];
  const actual = Object.keys(record);
  return actual.every(key => expected.includes(key) || key === 'viewport')
    && expected.every(key => Object.hasOwn(record, key))
    && (record.viewport === undefined || Object.hasOwn(record, 'viewport'));
}
