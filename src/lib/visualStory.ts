import type { ChapterSection, EvidenceLinkedVisualSpec } from './commands';
import { sceneNarrations, validScenePresentation } from './scenePresentation.ts';

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

export function readStoryProgress(raw: string | null, scenes: StoryScene[]): { index: number; positions: [string, number][] } {
  let value: unknown;
  try { value = raw && raw.length <= 16_000 ? JSON.parse(raw) : null; } catch { value = null; }
  const record = value && typeof value === 'object' ? value as { sceneId?: unknown; positions?: unknown } : {};
  const positions = record.positions && typeof record.positions === 'object' ? record.positions as Record<string, unknown> : {};
  return { index: Math.max(0, scenes.findIndex((scene) => scene.id === record.sceneId)), positions: scenes.map((scene) => {
    const position = Object.hasOwn(positions, scene.id) ? positions[scene.id] : 0;
    return [scene.id, typeof position === 'number' && Number.isFinite(position) && position >= 0 && position <= 3600 ? position : 0];
  }) };
}
