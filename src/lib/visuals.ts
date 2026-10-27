import type {
  ChapterRevision,
  ChapterSection,
  ChapterSourceAnchor,
  EvidenceLinkedVisualSpec,
  HotspotDestination,
} from './commands';
import { validStoredScenePresentation } from './scenePresentation.ts';

const MAX_NODES = 500;
const MAX_EDGES = 1_000;
const MAX_TITLE_BYTES = 160;
const MAX_LABEL_BYTES = 240;
const MAX_DETAIL_BYTES = 512;
const MAX_EVIDENCE_ANCHORS = 8;
const SAFE_ID = /^[A-Za-z0-9][A-Za-z0-9:_-]{0,127}$/;

export type InteractiveVisualResult =
  | { status: 'none' }
  | { status: 'unavailable'; reason: 'malformed' | 'unsupportedVersion' }
  | {
      status: 'available';
      visual: EvidenceLinkedVisualSpec;
      anchorsById: Map<string, ChapterSourceAnchor>;
      hotspotsById: Map<string, ChapterSection['hotspots'][number]>;
    };

export function inspectInteractiveVisual(
  revision: ChapterRevision,
  section: ChapterSection,
): InteractiveVisualResult {
  const raw: unknown = section.visual;
  if (raw == null) return { status: 'none' };
  if (!isRecord(raw)) return { status: 'unavailable', reason: 'malformed' };
  if (typeof raw.schemaVersion === 'number' && Number.isSafeInteger(raw.schemaVersion)
    && raw.schemaVersion > 0 && raw.schemaVersion !== 3) {
    return { status: 'unavailable', reason: 'unsupportedVersion' };
  }
  if (raw.schemaVersion !== 3 || !isCurrentVisual(raw)) return { status: 'unavailable', reason: 'malformed' };

  const anchorsById = new Map<string, ChapterSourceAnchor>();
  for (const claim of section.claims) {
    for (const anchor of claim.anchors) {
      if (anchorsById.has(anchor.anchorId)) return { status: 'unavailable', reason: 'malformed' };
      anchorsById.set(anchor.anchorId, anchor);
    }
  }
  const allRevisionAnchorIds = new Set(
    revision.sections.flatMap((item) => item.claims.flatMap((claim) => claim.anchors.map((anchor) => anchor.anchorId))),
  );
  const sectionIds = new Set(revision.sections.map((item) => item.sectionId));
  const hotspotsById = new Map<string, ChapterSection['hotspots'][number]>();
  for (const hotspot of section.hotspots) {
    if (!isRecord(hotspot) || typeof hotspot.hotspotId !== 'string'
      || typeof hotspot.label !== 'string' || !isRecord(hotspot.destination)
      || !SAFE_ID.test(hotspot.hotspotId) || hotspotsById.has(hotspot.hotspotId)) {
      return { status: 'unavailable', reason: 'malformed' };
    }
    hotspotsById.set(hotspot.hotspotId, hotspot);
  }

  const ids = new Set<string>();
  const nodeIds = new Set<string>();
  for (const node of raw.nodes) {
    if (!SAFE_ID.test(node.nodeId) || ids.has(node.nodeId)) return { status: 'unavailable', reason: 'malformed' };
    ids.add(node.nodeId);
    nodeIds.add(node.nodeId);
    if (!hasBoundedText(node.label, MAX_LABEL_BYTES)
      || (node.detail !== null && !hasBoundedText(node.detail, MAX_DETAIL_BYTES, false))
      || (node.hotspotId !== null && (!SAFE_ID.test(node.hotspotId) || !hotspotsById.has(node.hotspotId)))
      || !hasValidEvidence(node.evidenceAnchorIds, anchorsById)) {
      return { status: 'unavailable', reason: 'malformed' };
    }
    if (node.sourceAnchorId !== undefined || node.sourceValue !== undefined) {
      const sourceAnchor = typeof node.sourceAnchorId === 'string'
        ? anchorsById.get(node.sourceAnchorId)
        : undefined;
      if (typeof node.sourceAnchorId !== 'string'
        || typeof node.sourceValue !== 'string'
        || !node.evidenceAnchorIds.includes(node.sourceAnchorId)
        || !hasBoundedText(node.sourceValue, 240)
        || !sourceAnchor
        || sourceAnchor.excerpt.split(node.sourceValue).length !== 2) {
        return { status: 'unavailable', reason: 'malformed' };
      }
    }
    if (node.hotspotId !== null) {
      const hotspot = hotspotsById.get(node.hotspotId);
      if (!hotspot || !hasBoundedText(hotspot.label, MAX_LABEL_BYTES)
        || !hasValidDestination(hotspot.destination, sectionIds, allRevisionAnchorIds)) {
        return { status: 'unavailable', reason: 'malformed' };
      }
    }
  }
  for (const edge of raw.edges) {
    if (!SAFE_ID.test(edge.edgeId) || ids.has(edge.edgeId)) return { status: 'unavailable', reason: 'malformed' };
    ids.add(edge.edgeId);
    if (!nodeIds.has(edge.fromNodeId) || !nodeIds.has(edge.toNodeId)
      || !hasBoundedText(edge.label, MAX_LABEL_BYTES)
      || !hasValidEvidence(edge.evidenceAnchorIds, anchorsById)) {
      return { status: 'unavailable', reason: 'malformed' };
    }
  }

  if (!validStoredScenePresentation(raw.presentation, raw.nodes, section.claims)) {
    return { status: 'unavailable', reason: 'malformed' };
  }
  return { status: 'available', visual: raw, anchorsById, hotspotsById };
}

function isCurrentVisual(value: Record<string, unknown>): value is Record<string, unknown> & EvidenceLinkedVisualSpec {
  if (!hasExactKeys(value, ['schemaVersion', 'kind', 'title', 'layoutHint', 'nodes', 'edges', 'presentation'])
    || !['dataFlow', 'sequence', 'hierarchy'].includes(String(value.kind))
    || !hasBoundedText(value.title, MAX_TITLE_BYTES)
    || !isRecord(value.layoutHint)
    || !hasExactKeys(value.layoutHint, ['direction'])
    || !['auto', 'leftToRight', 'topToBottom'].includes(String(value.layoutHint.direction))
    || !Array.isArray(value.nodes)
    || value.nodes.length < 1 || value.nodes.length > MAX_NODES
    || !Array.isArray(value.edges)
    || value.edges.length > MAX_EDGES) return false;
  return value.nodes.every((item) => isRecord(item)
    && (hasExactKeys(item, ['nodeId', 'label', 'detail', 'hotspotId', 'evidenceAnchorIds'])
      || hasExactKeys(item, ['nodeId', 'label', 'detail', 'hotspotId', 'evidenceAnchorIds', 'sourceAnchorId', 'sourceValue']))
    && typeof item.nodeId === 'string'
    && typeof item.label === 'string'
    && (item.detail === null || typeof item.detail === 'string')
    && (item.hotspotId === null || typeof item.hotspotId === 'string')
    && Array.isArray(item.evidenceAnchorIds)
    && item.evidenceAnchorIds.every((anchorId) => typeof anchorId === 'string'))
    && value.edges.every((item) => isRecord(item)
      && hasExactKeys(item, ['edgeId', 'fromNodeId', 'toNodeId', 'label', 'evidenceAnchorIds'])
      && typeof item.edgeId === 'string'
      && typeof item.fromNodeId === 'string'
      && typeof item.toNodeId === 'string'
      && typeof item.label === 'string'
      && Array.isArray(item.evidenceAnchorIds)
      && item.evidenceAnchorIds.every((anchorId) => typeof anchorId === 'string'));
}

function hasValidEvidence(anchorIds: string[], anchorsById: Map<string, ChapterSourceAnchor>): boolean {
  return anchorIds.length >= 1
    && anchorIds.length <= MAX_EVIDENCE_ANCHORS
    && new Set(anchorIds).size === anchorIds.length
    && anchorIds.every((anchorId) => SAFE_ID.test(anchorId) && anchorsById.has(anchorId));
}

function hasValidDestination(
  destination: HotspotDestination,
  sectionIds: Set<string>,
  anchorIds: Set<string>,
): boolean {
  if (!isRecord(destination)) return false;
  switch (destination.kind) {
    case 'section': return hasExactKeys(destination, ['kind', 'sectionId'])
      && typeof destination.sectionId === 'string' && sectionIds.has(destination.sectionId);
    case 'codeLens': return hasExactKeys(destination, ['kind', 'anchorId'])
      && typeof destination.anchorId === 'string' && anchorIds.has(destination.anchorId);
    case 'atlasFile': return hasExactKeys(destination, ['kind', 'path'])
      && isSafeSourcePath(destination.path);
    case 'atlasSymbol': return hasExactKeys(destination, ['kind', 'path', 'symbolId'])
      && isSafeSourcePath(destination.path)
      && typeof destination.symbolId === 'string' && SAFE_ID.test(destination.symbolId);
    case 'relationship': return hasExactKeys(destination, ['kind', 'relationshipId'])
      && typeof destination.relationshipId === 'string' && SAFE_ID.test(destination.relationshipId);
    default: return false;
  }
}

function isSafeSourcePath(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && !value.startsWith('/')
    && !value.includes('\\') && !value.includes('\0')
    && value.split('/').every((part) => part !== '' && part !== '.' && part !== '..');
}

function hasExactKeys(value: Record<string, unknown>, expected: string[]): boolean {
  const actual = Object.keys(value);
  return actual.length === expected.length && expected.every((key) => Object.hasOwn(value, key));
}

function hasBoundedText(value: unknown, maximumBytes: number, requireVisibleText = true): value is string {
  return typeof value === 'string'
    && (!requireVisibleText || value.trim().length > 0)
    && new TextEncoder().encode(value).byteLength <= maximumBytes;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
