import type {
  ChapterClaim,
  ChapterRevision,
  ChapterSection,
  ChapterSourceAnchor,
  EvidenceLinkedVisualSpec,
  NarrationAudio,
} from './commands';
import { inspectInteractiveVisual } from './visuals.ts';

export const OFFLINE_PACKAGE_SCHEMA_VERSION = 2;
export const MAX_OFFLINE_HTML_BYTES = 25 * 1024 * 1024;
const MAX_NARRATION_AUDIO_BYTES = 12 * 1024 * 1024;

export interface OfflineExportOptions {
  selectedRevisionIds: string[];
  includeExcerpts: boolean;
  includeNarrationAudio?: boolean;
  narrationAudio?: readonly OfflineNarrationAudio[];
  exportedAtUnixMs?: number;
}

export interface OfflineNarrationAudio {
  chapterRevisionId: string;
  sectionId: string;
  audio: NarrationAudio;
}

export interface OfflineExportReference {
  anchorId: string;
  path: string;
  startLine: number;
  endLine: number;
  excerpt: string;
  redactedCredential: boolean;
}

export interface OfflineExportSectionPreview {
  sectionId: string;
  title: string;
  prose: string;
  references: OfflineExportReference[];
}

export interface OfflineExportChapterPreview {
  revisionId: string;
  snapshotId: string;
  revisionNumber: number;
  title: string;
  targetLabel: string;
  sections: OfflineExportSectionPreview[];
  sourcePaths: string[];
  hiddenReferencesOmitted: number;
  redactedCredentials: number;
}

export interface OfflineExportPreview {
  chapters: OfflineExportChapterPreview[];
  sourcePaths: string[];
  includedExcerptCount: number;
  omittedHiddenReferences: number;
  redactedCredentials: number;
  includeExcerpts: boolean;
}

export interface OfflinePackageManifest {
  schemaVersion: number;
  exportedAtUnixMs: number;
  chapters: Array<{
    revisionId: string;
    revisionNumber: number;
    snapshotId: string;
    title: string;
  }>;
  assets: Array<{
    assetId: string;
    kind: 'citedExcerpt' | 'typedVisual' | 'narrationAudio';
    mediaType: 'text/plain' | 'application/json' | 'audio/wav';
    chapterRevisionId: string;
    snapshotId: string;
    anchorId?: string;
    sectionId?: string;
    cacheFingerprint?: string;
    path?: string;
    startLine?: number;
    endLine?: number;
    sha256: string;
    byteLength: number;
  }>;
  features: {
    sectionNavigation: 'included';
    citedExcerpts: 'included' | 'omitted';
    typedVisuals: 'included-when-supported';
    narrationAudio: 'included' | 'omitted' | 'none-cached';
  };
}

export interface OfflineExportResult {
  filename: string;
  html: string;
  manifest: OfflinePackageManifest;
}

interface SafeClaim {
  claim: ChapterClaim;
  categoryLabel: string;
  prose: string;
  inferenceBasis: string | null;
  references: OfflineExportReference[];
}

interface SafeSection {
  section: ChapterSection;
  title: string;
  prose: string;
  claims: SafeClaim[];
  references: OfflineExportReference[];
  visual: EvidenceLinkedVisualSpec | null;
  visualAssetText: string | null;
  visualUnavailable: boolean;
  hotspots: ChapterSection['hotspots'];
  narrationAudio: NarrationAudio | null;
}

interface PreparedNarrationAudio extends OfflineNarrationAudio {
  bytes: Uint8Array;
}

interface SafeChapter {
  revision: ChapterRevision;
  title: string;
  targetLabel: string;
  sections: SafeSection[];
  references: OfflineExportReference[];
  hiddenReferencesOmitted: number;
  redactedCredentials: number;
}

const HIDDEN_OR_SECRET_PATH_PART = /^(?:\.env(?:\..*)?|\.ssh|\.aws|\.config|\.git|\.svn|\.hg|\.npmrc|\.netrc|\.pypirc|secrets?|credentials?|private[-_]?keys?|id_(?:rsa|ed25519|ecdsa)|.*\.(?:pem|p12|pfx|key))$/i;
const CREDENTIAL_PATTERNS = [
  /\bAIza[0-9A-Za-z_-]{20,}\b/g,
  /\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{20,}\b/g,
  /\bgithub_pat_[A-Za-z0-9_]{20,}\b/g,
  /\bglpat-[A-Za-z0-9_-]{20,}\b/g,
  /\bnpm_[A-Za-z0-9]{20,}\b/g,
  /\bxox[baprs]-[A-Za-z0-9-]{20,}\b/g,
  /\b(?:sk|rk)_(?:live|test)_[A-Za-z0-9]{12,}\b/g,
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{8,}\b/g,
  /\bAKIA[0-9A-Z]{16}\b/g,
  /\b(?:sk|pk)-[A-Za-z0-9]{20,}\b/g,
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/g,
];
const ASSIGNED_SECRET = /(\b[A-Za-z][A-Za-z0-9_-]*(?:key|secret|token|password|passwd|credential|authorization|jwt)[A-Za-z0-9_-]*\s*[:=]\s*)(?:"[^"\r\n]*"|'[^'\r\n]*'|[^\s,;]{1,})/gi;
const AUTHORIZATION_SECRET = /(\b(?:authorization|bearer)\s*[:=]\s*(?:bearer\s+)?)[A-Za-z0-9._~+/-]{1,}/gi;
const BEARER_SECRET = /\bbearer\s+[A-Za-z0-9._~+/-]{1,}/gi;
const ABSOLUTE_LOCAL_PATH = /(?:\/(?:Users|home|private|tmp|var)\/[^\s"'`<>),;]+|(?<![A-Za-z0-9:/])\/(?:[A-Za-z0-9._-]+\/)+[A-Za-z0-9._-]*|\b[A-Za-z]:\\(?:[^\s"'`<>),;]+\\?)+)/g;
const HIDDEN_PATH_IN_COPY = /(^|[\s("'`])((?:\.\.\/|\.\/)?(?:[A-Za-z0-9_-]+\/)*(?:\.env(?:\.[A-Za-z0-9._-]+)?|\.ssh(?:\/[A-Za-z0-9._-]+)*|\.aws(?:\/[A-Za-z0-9._-]+)*|\.config(?:\/[A-Za-z0-9._-]+)*|\.git(?:\/[A-Za-z0-9._-]+)*|\.svn(?:\/[A-Za-z0-9._-]+)*|\.hg(?:\/[A-Za-z0-9._-]+)*|\.npmrc|\.netrc|\.pypirc|(?:secrets?|credentials?)(?:\/[A-Za-z0-9._-]+)*|private[-_]?keys?(?:\/[A-Za-z0-9._-]+)*|id_(?:rsa|ed25519|ecdsa)(?:\.[A-Za-z0-9_-]+)?))/gi;

export function isSafeOfflineSourcePath(path: string): boolean {
  if (!path || path.startsWith('/') || path.includes('\\') || path.includes('\0') || path.includes(':')) return false;
  const parts = path.split('/');
  return parts.every((part) => part.length > 0 && part !== '.' && part !== '..' && !part.startsWith('.') && !HIDDEN_OR_SECRET_PATH_PART.test(part));
}

export function isOfflineExportableRevision(revision: ChapterRevision): boolean {
  if (revision.lifecycle !== 'accepted' || !Array.isArray(revision.sections) || revision.sections.length === 0) return false;
  const paths = targetPaths(revision);
  return paths.every(isSafeOfflineSourcePath);
}

export function buildOfflineExportPreview(
  revisions: readonly ChapterRevision[],
  selectedRevisionIds: readonly string[],
  includeExcerpts = true,
): OfflineExportPreview {
  const chapters = selectExportableRevisions(revisions, selectedRevisionIds).map((revision) => createSafeChapter(revision));
  const sourcePaths = [...new Set(chapters.flatMap((chapter) => chapter.references.map((reference) => reference.path)))].sort();
  const references = chapters.flatMap((chapter) => chapter.references);
  return {
    chapters: chapters.map((chapter) => ({
      revisionId: chapter.revision.revisionId,
      snapshotId: chapter.revision.snapshotId,
      revisionNumber: chapter.revision.revisionNumber,
      title: chapter.title,
      targetLabel: chapter.targetLabel,
      sections: chapter.sections.map((section) => ({
        sectionId: section.section.sectionId,
        title: section.title,
        prose: section.prose,
        references: section.references,
      })),
      sourcePaths: [...new Set(chapter.references.map((reference) => reference.path))].sort(),
      hiddenReferencesOmitted: chapter.hiddenReferencesOmitted,
      redactedCredentials: chapter.redactedCredentials,
    })),
    sourcePaths,
    includedExcerptCount: includeExcerpts ? references.length : 0,
    omittedHiddenReferences: chapters.reduce((total, chapter) => total + chapter.hiddenReferencesOmitted, 0),
    redactedCredentials: chapters.reduce((total, chapter) => total + chapter.redactedCredentials, 0),
    includeExcerpts,
  };
}

export async function buildOfflineGuideHtml(
  revisions: readonly ChapterRevision[],
  options: OfflineExportOptions,
): Promise<OfflineExportResult> {
  if (new Set(options.selectedRevisionIds).size !== options.selectedRevisionIds.length) {
    throw new Error('A chapter revision was selected more than once.');
  }
  if (options.selectedRevisionIds.length === 0) throw new Error('Select at least one accepted chapter with exportable source paths.');
  const selected = selectExportableRevisions(revisions, options.selectedRevisionIds);
  if (selected.length !== options.selectedRevisionIds.length) throw new Error('Only accepted, exportable chapter revisions can be included in an offline guide.');
  if (selected.length === 0) throw new Error('Select at least one accepted chapter with exportable source paths.');

  const chapters = selected.map((revision) => createSafeChapter(revision));
  const narrationAudio = options.includeNarrationAudio
    ? validateNarrationAudio(selected, options.narrationAudio ?? [])
    : new Map<string, PreparedNarrationAudio>();
  for (const chapter of chapters) {
    for (const section of chapter.sections) {
      section.narrationAudio = narrationAudio.get(sectionKey(chapter.revision.revisionId, section.section.sectionId))?.audio ?? null;
    }
  }
  const assets: OfflinePackageManifest['assets'] = [];
  for (const chapter of chapters) {
    for (const section of chapter.sections) {
      if (section.visualAssetText) {
        const bytes = new TextEncoder().encode(section.visualAssetText);
        assets.push({
          assetId: `visual:${chapter.revision.revisionId}:${section.section.sectionId}`,
          kind: 'typedVisual',
          mediaType: 'application/json',
          chapterRevisionId: chapter.revision.revisionId,
          snapshotId: chapter.revision.snapshotId,
          sectionId: section.section.sectionId,
          sha256: await sha256(bytes),
          byteLength: bytes.byteLength,
        });
      }
      const narration = narrationAudio.get(sectionKey(chapter.revision.revisionId, section.section.sectionId));
      if (narration) {
        assets.push({
          assetId: `narration:${chapter.revision.revisionId}:${section.section.sectionId}`,
          kind: 'narrationAudio',
          mediaType: 'audio/wav',
          chapterRevisionId: chapter.revision.revisionId,
          snapshotId: chapter.revision.snapshotId,
          sectionId: section.section.sectionId,
          cacheFingerprint: narration.audio.fingerprint,
          sha256: await sha256(narration.bytes),
          byteLength: narration.bytes.byteLength,
        });
      }
    }
    if (options.includeExcerpts) {
      for (const reference of chapter.references) {
        const bytes = new TextEncoder().encode(reference.excerpt);
        assets.push({
          assetId: `excerpt:${chapter.revision.revisionId}:${reference.anchorId}`,
          kind: 'citedExcerpt',
          mediaType: 'text/plain',
          chapterRevisionId: chapter.revision.revisionId,
          snapshotId: chapter.revision.snapshotId,
          anchorId: reference.anchorId,
          path: reference.path,
          startLine: reference.startLine,
          endLine: reference.endLine,
          sha256: await sha256(bytes),
          byteLength: bytes.byteLength,
        });
      }
    }
  }
  const manifest: OfflinePackageManifest = {
    schemaVersion: OFFLINE_PACKAGE_SCHEMA_VERSION,
    exportedAtUnixMs: options.exportedAtUnixMs ?? Date.now(),
    chapters: chapters.map((chapter) => ({
      revisionId: chapter.revision.revisionId,
      revisionNumber: chapter.revision.revisionNumber,
      snapshotId: chapter.revision.snapshotId,
      title: chapter.title,
    })),
    assets,
    features: {
      sectionNavigation: 'included',
      citedExcerpts: options.includeExcerpts ? 'included' : 'omitted',
      typedVisuals: 'included-when-supported',
      narrationAudio: narrationAudio.size > 0
        ? 'included'
        : options.includeNarrationAudio ? 'none-cached' : 'omitted',
    },
  };
  const html = renderPackage(chapters, manifest, options.includeExcerpts);
  if (new TextEncoder().encode(html).byteLength > MAX_OFFLINE_HTML_BYTES) {
    throw new Error('This offline guide would exceed the 25 MB local export limit. Select fewer chapters, omit cited excerpts, or leave cached audio out.');
  }
  return { filename: filenameFor(chapters), html, manifest };
}

function selectExportableRevisions(
  revisions: readonly ChapterRevision[],
  selectedRevisionIds: readonly string[],
): ChapterRevision[] {
  const selected = new Set(selectedRevisionIds);
  return revisions.filter((revision) => selected.has(revision.revisionId) && isOfflineExportableRevision(revision));
}

function targetPaths(revision: ChapterRevision): string[] {
  switch (revision.target.scope.kind) {
    case 'file': return [revision.target.scope.path];
    case 'group': return revision.target.scope.paths;
    case 'overview': return [];
  }
}

function createSafeChapter(revision: ChapterRevision): SafeChapter {
  const hiddenPaths = new Set<string>();
  for (const path of targetPaths(revision)) if (!isSafeOfflineSourcePath(path)) hiddenPaths.add(path);
  const anchors = uniqueAnchors(revision);
  for (const anchor of anchors) if (!isSafeOfflineSourcePath(anchor.path)) hiddenPaths.add(anchor.path);
  const hiddenPathList = [...hiddenPaths].sort((left, right) => right.length - left.length);
  const sanitizer = createTextSanitizer(hiddenPathList);
  const safeAnchors = new Map<string, OfflineExportReference>();
  const hiddenReferencesOmitted = anchors.filter((anchor) => !isSafeOfflineSourcePath(anchor.path)).length;
  for (const anchor of anchors) {
    if (!isSafeOfflineSourcePath(anchor.path)) continue;
    const sanitizedPath = sanitizer.clean(anchor.path);
    const sanitizedExcerpt = sanitizer.clean(anchor.excerpt);
    safeAnchors.set(anchor.anchorId, {
      anchorId: anchor.anchorId,
      path: sanitizedPath.text,
      startLine: anchor.startLine,
      endLine: anchor.endLine,
      excerpt: sanitizedExcerpt.text,
      redactedCredential: sanitizedPath.redactedCredential || sanitizedExcerpt.redactedCredential,
    });
  }
  const sections = [...revision.sections]
    .sort((left, right) => left.order - right.order || left.sectionId.localeCompare(right.sectionId))
    .map((section) => {
      const claims = section.claims.map((claim): SafeClaim => {
        const references = claim.anchors.flatMap((anchor) => {
          const safe = safeAnchors.get(anchor.anchorId);
          return safe ? [safe] : [];
        });
        const prose = sanitizer.clean(claim.prose);
        const inferenceBasis = claim.category.kind === 'inference'
          ? sanitizer.clean(claim.category.basis)
          : null;
        return {
          claim,
          categoryLabel: claim.category.kind === 'inference'
            ? sanitizer.clean(claim.category.visibleLabel).text
            : claim.category.kind === 'repositoryContext' ? 'Repository context' : 'Code behavior',
          prose: prose.text,
          inferenceBasis: inferenceBasis?.text ?? null,
          references,
        };
      });
      const references = uniqueReferences(claims.flatMap((claim) => claim.references));
      const inspection = inspectInteractiveVisual(revision, section);
      const visual = inspection.status === 'available'
        && inspection.visual.nodes.every((node) => node.evidenceAnchorIds.every((id) => safeAnchors.has(id)))
        && inspection.visual.edges.every((edge) => edge.evidenceAnchorIds.every((id) => safeAnchors.has(id)))
        ? sanitizeVisual(inspection.visual, sanitizer.clean)
        : null;
      const visualAssetText = visual ? JSON.stringify(visual) : null;
      const hotspots = section.hotspots.map((hotspot) => ({
        ...hotspot,
        label: sanitizer.clean(hotspot.label).text,
        destination: hotspot.destination.kind === 'atlasFile'
          ? { ...hotspot.destination, path: sanitizer.clean(hotspot.destination.path).text }
          : hotspot.destination.kind === 'atlasSymbol'
            ? { ...hotspot.destination, path: sanitizer.clean(hotspot.destination.path).text }
            : hotspot.destination,
      }));
      return {
        section,
        title: sanitizer.clean(section.title).text,
        prose: sanitizer.clean(section.prose).text,
        claims,
        references,
        visual,
        visualAssetText,
        visualUnavailable: section.visual !== null && visual === null,
        hotspots,
        narrationAudio: null,
      };
    });
  const references = uniqueReferences(sections.flatMap((section) => section.references));
  const targetLabel = targetLabelFor(revision, sanitizer.clean);
  const safeTitle = sanitizer.clean(revision.title);
  return {
    revision,
    title: safeTitle.text || 'Untitled chapter',
    targetLabel,
    sections,
    references,
    hiddenReferencesOmitted,
    redactedCredentials: sanitizer.redactedCredentialCount,
  };
}

function validateNarrationAudio(
  revisions: readonly ChapterRevision[],
  candidates: readonly OfflineNarrationAudio[],
): Map<string, PreparedNarrationAudio> {
  const selectedKeys = new Set(revisions.flatMap((revision) => revision.sections.map((section) => sectionKey(revision.revisionId, section.sectionId))));
  const prepared = new Map<string, PreparedNarrationAudio>();
  for (const candidate of candidates) {
    const key = sectionKey(candidate.chapterRevisionId, candidate.sectionId);
    if (!selectedKeys.has(key)) continue;
    if (prepared.has(key)) throw new Error('Cached narration was supplied more than once for one chapter section.');
    if (!/^[a-f0-9]{64}$/i.test(candidate.audio.fingerprint) || candidate.audio.mimeType !== 'audio/wav') {
      throw new Error('A selected narration asset has an unsupported format or cache identity.');
    }
    const bytes = decodeAndValidateNarrationWav(candidate.audio.audioBase64);
    prepared.set(key, { ...candidate, bytes });
  }
  return prepared;
}

function decodeAndValidateNarrationWav(encoded: string): Uint8Array {
  if (!encoded || encoded.length % 4 !== 0 || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) {
    throw new Error('A cached narration asset is not valid base64 WAV data.');
  }
  if (encoded.length > Math.ceil(MAX_NARRATION_AUDIO_BYTES / 3) * 4) {
    throw new Error('A cached narration asset exceeds the 12 MB per-section limit.');
  }
  const binary = atob(encoded);
  if (binary.length > MAX_NARRATION_AUDIO_BYTES) throw new Error('A cached narration asset exceeds the 12 MB per-section limit.');
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  if (!isSupportedNarrationWav(bytes)) throw new Error('A cached narration asset is not the supported 24 kHz mono PCM WAV format.');
  return bytes;
}

function isSupportedNarrationWav(bytes: Uint8Array): boolean {
  if (bytes.length < 44 || ascii(bytes, 0, 4) !== 'RIFF' || ascii(bytes, 8, 12) !== 'WAVE') return false;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (view.getUint32(4, true) + 8 !== bytes.length) return false;
  let cursor = 12;
  let hasPcmFormat = false;
  let hasAudioData = false;
  while (cursor + 8 <= bytes.length) {
    const chunkId = ascii(bytes, cursor, cursor + 4);
    const chunkSize = view.getUint32(cursor + 4, true);
    const dataStart = cursor + 8;
    const dataEnd = dataStart + chunkSize;
    if (!Number.isSafeInteger(dataEnd) || dataEnd > bytes.length) return false;
    if (chunkId === 'fmt ') {
      if (chunkSize < 16 || hasPcmFormat) return false;
      hasPcmFormat = view.getUint16(dataStart, true) === 1
        && view.getUint16(dataStart + 2, true) === 1
        && view.getUint32(dataStart + 4, true) === 24_000
        && view.getUint32(dataStart + 8, true) === 48_000
        && view.getUint16(dataStart + 12, true) === 2
        && view.getUint16(dataStart + 14, true) === 16;
      if (!hasPcmFormat) return false;
    } else if (chunkId === 'data') {
      if (hasAudioData || chunkSize === 0 || chunkSize % 2 !== 0) return false;
      hasAudioData = true;
    }
    const next = dataStart + chunkSize + (chunkSize % 2);
    if (!Number.isSafeInteger(next) || next > bytes.length) return false;
    cursor = next;
  }
  return cursor === bytes.length && hasPcmFormat && hasAudioData;
}

function ascii(bytes: Uint8Array, start: number, end: number): string {
  return String.fromCharCode(...bytes.subarray(start, end));
}

function sectionKey(revisionId: string, sectionId: string): string {
  return `${revisionId}\u0000${sectionId}`;
}

function sanitizeVisual(visual: EvidenceLinkedVisualSpec, clean: (text: string) => SanitizedText): EvidenceLinkedVisualSpec {
  return {
    ...visual,
    title: clean(visual.title).text,
    nodes: visual.nodes.map((node) => ({
      ...node,
      label: clean(node.label).text,
      detail: node.detail === null ? null : clean(node.detail).text,
    })),
    edges: visual.edges.map((edge) => ({ ...edge, label: clean(edge.label).text })),
  };
}

function targetLabelFor(revision: ChapterRevision, clean: (text: string) => SanitizedText): string {
  switch (revision.target.scope.kind) {
    case 'overview': return 'Repository overview';
    case 'file': return clean(revision.target.scope.path).text;
    case 'group': return revision.target.scope.paths.map((path) => clean(path).text).join(' · ');
  }
}

interface SanitizedText {
  text: string;
  redacted: boolean;
  redactedCredential: boolean;
}

function createTextSanitizer(hiddenPaths: string[]): {
  clean: (text: string) => SanitizedText;
  redactedCredentialCount: number;
} {
  let redactedCredentialCount = 0;
  const clean = (input: string): SanitizedText => {
    let text = String(input ?? '');
    let redacted = false;
    let redactedCredential = false;
    const replace = (pattern: RegExp, value: string) => {
      text = text.replace(pattern, () => {
        redacted = true;
        return value;
      });
    };
    for (const hiddenPath of hiddenPaths) {
      if (!hiddenPath) continue;
      const escaped = hiddenPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      replace(new RegExp(escaped, 'gi'), '[hidden source path omitted]');
    }
    replace(ABSOLUTE_LOCAL_PATH, '[local path omitted]');
    text = text.replace(HIDDEN_PATH_IN_COPY, (_match, prefix: string) => {
      redacted = true;
      return `${prefix}[hidden source path omitted]`;
    });
    for (const pattern of CREDENTIAL_PATTERNS) {
      const prior = text;
      replace(pattern, '[redacted credential]');
      if (text !== prior) redactedCredential = true;
    }
    const priorAuthorization = text;
    text = text.replace(AUTHORIZATION_SECRET, (_match, prefix: string) => {
      redacted = true;
      redactedCredential = true;
      return `${prefix}[redacted credential]`;
    });
    text = text.replace(BEARER_SECRET, () => {
      redacted = true;
      redactedCredential = true;
      return 'Bearer [redacted credential]';
    });
    if (text !== priorAuthorization) redactedCredential = true;
    text = text.replace(ASSIGNED_SECRET, (_match, prefix: string) => {
      redacted = true;
      redactedCredential = true;
      return `${prefix}[redacted credential]`;
    });
    if (redactedCredential) redactedCredentialCount += 1;
    return { text, redacted, redactedCredential };
  };
  return { clean, get redactedCredentialCount() { return redactedCredentialCount; } };
}

function uniqueAnchors(revision: ChapterRevision): ChapterSourceAnchor[] {
  const anchors = new Map<string, ChapterSourceAnchor>();
  for (const section of revision.sections) {
    for (const claim of section.claims) {
      for (const anchor of claim.anchors) if (!anchors.has(anchor.anchorId)) anchors.set(anchor.anchorId, anchor);
    }
  }
  return [...anchors.values()];
}

function uniqueReferences(references: OfflineExportReference[]): OfflineExportReference[] {
  const unique = new Map<string, OfflineExportReference>();
  for (const reference of references) if (!unique.has(reference.anchorId)) unique.set(reference.anchorId, reference);
  return [...unique.values()];
}

async function sha256(bytes: Uint8Array): Promise<string> {
  if (!globalThis.crypto?.subtle) throw new Error('This app cannot calculate SHA-256 hashes in the current runtime, so the offline guide was not created.');
  const ownedBytes = new Uint8Array(bytes.byteLength);
  ownedBytes.set(bytes);
  const digest = await globalThis.crypto.subtle.digest('SHA-256', ownedBytes);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function renderPackage(
  chapters: SafeChapter[],
  manifest: OfflinePackageManifest,
  includeExcerpts: boolean,
): string {
  const chapterIds = chapters.map((_chapter, index) => `chapter-${index + 1}`);
  const allSections = chapters.flatMap((chapter, chapterIndex) => chapter.sections.map((section, sectionIndex) => ({
    chapter,
    chapterIndex,
    section,
    sectionIndex,
    id: `section-${chapterIndex + 1}-${sectionIndex + 1}`,
  })));
  const evidenceIds = new Map<string, string>();
  for (const [chapterIndex, chapter] of chapters.entries()) {
    for (const [referenceIndex, reference] of chapter.references.entries()) {
      evidenceIds.set(`${chapter.revision.revisionId}:${reference.anchorId}`, `evidence-${chapterIndex + 1}-${referenceIndex + 1}`);
    }
  }
  const sectionIds = new Map<string, string>();
  for (const item of allSections) sectionIds.set(`${item.chapter.revision.revisionId}:${item.section.section.sectionId}`, item.id);

  const contents = chapters.map((chapter, chapterIndex) => {
    const chapterId = chapterIds[chapterIndex];
    return `<li><a href="#${chapterId}">${escapeHtml(chapter.title)}</a><span>${escapeHtml(chapter.targetLabel)}</span><ol>${chapter.sections.map((section, sectionIndex) => `<li><a href="#section-${chapterIndex + 1}-${sectionIndex + 1}">${escapeHtml(section.title)}</a></li>`).join('')}</ol></li>`;
  }).join('');

  const renderedChapters = chapters.map((chapter, chapterIndex) => {
    const chapterId = chapterIds[chapterIndex];
    const renderedSections = chapter.sections.map((section, sectionIndex) => {
      const globalIndex = allSections.findIndex((item) => item.chapterIndex === chapterIndex && item.sectionIndex === sectionIndex);
      const previous = globalIndex > 0 ? allSections[globalIndex - 1] : null;
      const next = globalIndex + 1 < allSections.length ? allSections[globalIndex + 1] : null;
      const sectionId = `section-${chapterIndex + 1}-${sectionIndex + 1}`;
      const claims = section.claims.map((claim) => {
        const category = claim.categoryLabel;
        const citations = claim.references.length > 0
          ? `<ul class="citations">${claim.references.map((reference) => {
            const evidenceId = evidenceIds.get(`${chapter.revision.revisionId}:${reference.anchorId}`);
            return evidenceId
              ? `<li><a href="#${evidenceId}">${escapeHtml(reference.path)}:${reference.startLine}-${reference.endLine}</a></li>`
              : '';
          }).join('')}</ul>`
          : '<p class="muted">No exportable source reference is included for this claim.</p>';
        const basis = claim.inferenceBasis ? `<p class="inference"><strong>Inference basis:</strong> ${escapeHtml(claim.inferenceBasis)}</p>` : '';
        return `<article class="claim"><p class="eyebrow">${escapeHtml(category)}</p><p>${escapeHtml(claim.prose)}</p>${basis}${citations}</article>`;
      }).join('');
      const visual = renderVisual(
        chapter.revision,
        section,
        sectionIds,
        evidenceIds,
      );
      const previousLink = previous
        ? `<a class="secondary" href="#${previous.id}">Previous: ${escapeHtml(previous.section.title)}</a>`
        : `<a class="secondary" href="#${chapterId}">Chapter contents</a>`;
      const nextLink = next
        ? `<a class="primary" href="#${next.id}">Continue: ${escapeHtml(next.section.title)}</a>`
        : '<span class="muted">End of guide</span>';
      const narration = section.narrationAudio
        ? `<section class="narration-audio" aria-label="Cached narration"><h4>Cached narration</h4><audio controls preload="none" aria-label="Narration for ${escapeHtml(section.title)}" src="data:audio/wav;base64,${section.narrationAudio.audioBase64}">This browser cannot play the bundled narration audio.</audio></section>`
        : '';
      return `<section class="section" id="${sectionId}"><p class="eyebrow">SECTION ${sectionIndex + 1} OF ${chapter.sections.length}</p><h3>${escapeHtml(section.title)}</h3><p class="prose">${escapeHtml(section.prose)}</p>${narration}${claims ? `<section class="evidence"><h4>What supports this explanation?</h4>${claims}</section>` : ''}${visual}${renderEvidenceAppendix(chapter, section, chapterIndex, evidenceIds, includeExcerpts)}<nav class="section-nav" aria-label="Section navigation">${previousLink}${nextLink}</nav></section>`;
    }).join('');
    const omitted = chapter.hiddenReferencesOmitted > 0
      ? `<p class="notice">${chapter.hiddenReferencesOmitted} source reference${chapter.hiddenReferencesOmitted === 1 ? '' : 's'} to hidden or sensitive paths were omitted. Some claims may have less bundled evidence.</p>`
      : '';
    return `<article class="chapter" id="${chapterId}"><header><p class="eyebrow">ACCEPTED REVISION ${chapter.revision.revisionNumber} · SNAPSHOT ${escapeHtml(chapter.revision.snapshotId)}</p><h2>${escapeHtml(chapter.title)}</h2><p class="muted">${escapeHtml(chapter.targetLabel)}</p></header>${omitted}${renderedSections}</article>`;
  }).join('');

  const manifestText = escapeHtml(JSON.stringify(manifest, null, 2));
  const excerptMessage = includeExcerpts
    ? 'Cited excerpts are bundled below their sections. Review the manifest to verify each excerpt hash.'
    : 'Cited source excerpts were omitted. Source path and line references remain as metadata.';
  const narrationMessage = manifest.features.narrationAudio === 'included'
    ? 'Selected cached narration is bundled in its matching sections and can be played offline.'
    : manifest.features.narrationAudio === 'none-cached'
      ? 'Narration was requested, but no cached section audio was available; no audio is bundled.'
      : 'Narration audio was not selected for this package.';
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; media-src data:; base-uri 'none'; form-action 'none'">
<title>Code Atlas · Offline guide</title>
<style>
:root{color-scheme:light;--ink:#18231c;--muted:#56665b;--line:#d8e0d9;--paper:#f7f9f6;--panel:#fff;--green:#315e3e;--soft:#edf4ed}*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--paper);color:var(--ink);font:16px/1.65 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}a{color:var(--green);text-underline-offset:3px}a:focus-visible{outline:3px solid #b36b00;outline-offset:3px}header.page{padding:42px max(22px,calc((100vw - 1080px)/2));background:#17251b;color:#f4f8f2}header.page h1{max-width:780px;margin:8px 0;font-size:clamp(28px,5vw,48px);line-height:1.1}header.page p{max-width:760px;color:#c8d4c9}.eyebrow{margin:0;color:var(--muted);font-size:12px;font-weight:700;letter-spacing:.09em;text-transform:uppercase}.page .eyebrow{color:#a8c5a9}.wrap{width:min(1080px,calc(100% - 36px));margin:0 auto}.notice{padding:12px 14px;border:1px solid #c28a35;border-radius:8px;background:#fff7e8;color:#614619}.guide-layout{display:grid;grid-template-columns:minmax(220px,280px) minmax(0,1fr);gap:26px;padding:28px 0 60px}.contents{align-self:start;position:sticky;top:16px;max-height:calc(100vh - 32px);overflow:auto;padding:16px;border:1px solid var(--line);border-radius:12px;background:var(--panel)}.contents h2{margin:0 0 10px;font-size:17px}.contents ol{padding-left:20px}.contents li{margin:6px 0}.contents span{display:block;color:var(--muted);font-size:12px;overflow-wrap:anywhere}.chapter{margin:0 0 30px;padding:24px;border:1px solid var(--line);border-radius:14px;background:var(--panel)}.chapter>header{padding-bottom:15px;border-bottom:1px solid var(--line)}.chapter h2{margin:5px 0;font-size:clamp(23px,3vw,34px);line-height:1.2}.chapter h3{margin:8px 0 14px;font-size:clamp(20px,2.5vw,27px);line-height:1.25}.section{padding:22px 0 26px;border-bottom:1px solid var(--line);scroll-margin-top:16px}.section:last-child{border-bottom:0}.prose{font-size:17px}.evidence,.visual,.excerpt,.narration-audio{margin:18px 0;padding:16px;border:1px solid var(--line);border-radius:10px;background:#fbfcfa}.evidence h4,.visual h4,.excerpt h4,.narration-audio h4{margin:0 0 12px;font-size:16px}.narration-audio audio{display:block;width:min(100%,520px)}.claim{margin:10px 0;padding:12px;border-left:3px solid #8ca98e;background:var(--soft)}.claim p{margin:4px 0}.citations{margin:6px 0;padding-left:20px}.inference{color:#6a501e}.muted{color:var(--muted)}.diagram{display:grid;gap:10px;list-style:none;padding:0}.diagram-node{padding:13px;border:1px solid #bfd0c1;border-radius:8px;background:#f3f8f2}.diagram-node strong{display:block}.diagram-node small{display:block;color:var(--muted)}.diagram-edge{padding:8px 12px;color:var(--muted);border-left:2px solid #9ab39c}.diagram-links{margin:7px 0 0;padding-left:18px;font-size:14px}.hotspot{display:inline-block;margin:8px 5px 0 0;padding:3px 8px;border:1px solid #c7d5c8;border-radius:999px;background:#fff;font-size:13px}.excerpt pre,.manifest pre,.visual-source pre{max-width:100%;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}.reference-meta{color:var(--muted);font-size:13px}.visual-source{margin-top:12px;border-top:1px solid var(--line);padding-top:10px}.visual-source summary{cursor:pointer;color:var(--muted);font-size:13px}.visual-source pre{max-height:360px;margin-top:8px;padding:10px;border-radius:6px;background:#f1f4f0}.section-nav{display:flex;justify-content:space-between;align-items:center;gap:12px;margin-top:22px;padding-top:14px;border-top:1px solid var(--line)}.section-nav a{display:inline-block;padding:8px 12px;border:1px solid var(--line);border-radius:8px;text-decoration:none}.section-nav .primary{background:var(--green);color:#fff;border-color:var(--green)}.manifest{margin:30px 0;padding:18px;border:1px solid var(--line);border-radius:10px;background:#fff}.manifest summary{cursor:pointer;font-weight:700}.manifest pre{max-height:480px}.page-footer{padding:22px 0;color:var(--muted);font-size:13px;border-top:1px solid var(--line)}
@media(max-width:760px){.guide-layout{grid-template-columns:1fr}.contents{position:static;max-height:none}.chapter{padding:17px}.section-nav{align-items:stretch;flex-direction:column}.section-nav a{width:100%}}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}}
</style>
</head>
<body>
<header class="page"><div class="wrap"><p class="eyebrow">Code Atlas · Offline guide</p><h1>Read the code story at your own pace.</h1><p>This file works without Code Atlas or a network connection. ${escapeHtml(excerptMessage)} ${escapeHtml(narrationMessage)}</p><p class="notice">Privacy review: this package includes selected chapter explanations and the source paths shown below${includeExcerpts ? ', plus the cited excerpts listed in the export preview' : ''}${manifest.features.narrationAudio === 'included' ? ', plus cached narration audio for the sections listed in the export preview' : ''}. Source excerpts can contain sensitive code. Provider configuration and the local repository checkout are not part of this package. Common credential patterns and hidden paths are filtered; review the exported text before sharing.</p></div></header>
<main class="wrap"><div class="guide-layout"><nav class="contents" aria-label="Guide contents"><h2>Chapters and sections</h2><ol>${contents}</ol></nav><div class="guide">${renderedChapters}<details class="manifest"><summary>Package manifest · schema ${manifest.schemaVersion}</summary><p>Revision IDs, evidence snapshots, and hashes for included excerpts, typed diagram data, and narration audio.</p><pre>${manifestText}</pre></details></div></div><footer class="page-footer">Exported by Code Atlas. This static package does not execute chapter content or open local source files.</footer></main>
</body>
</html>`;
}

function renderVisual(
  revision: ChapterRevision,
  section: SafeSection,
  sectionIds: Map<string, string>,
  evidenceIds: Map<string, string>,
): string {
  if (!section.section.visual) return '<p class="muted">No visual is attached to this section.</p>';
  if (section.visualUnavailable || !section.visual) {
    return '<p class="notice">This diagram is unavailable in the offline format. The explanation and cited evidence are still included.</p>';
  }
  if (typeof document !== 'undefined') {
    return renderVisualWithDom(revision, section, sectionIds, evidenceIds);
  }
  const typedVisual = section.visual;
  const anchorsFor = (ids: string[]) => ids.map((anchorId) => {
    const id = evidenceIds.get(`${revision.revisionId}:${anchorId}`);
    const reference = section.references.find((item) => item.anchorId === anchorId);
    if (!id || !reference) return '';
    return `<a href="#${id}">${escapeHtml(reference.path)}:${reference.startLine}-${reference.endLine}</a>`;
  }).filter(Boolean).join(' · ');
  const nodes = typedVisual.nodes.map((node, nodeIndex) => {
    const references = anchorsFor(node.evidenceAnchorIds);
    const hotspot = node.hotspotId ? section.hotspots.find((item) => item.hotspotId === node.hotspotId) : undefined;
    const hotspotMarkup = hotspot ? renderOfflineHotspot(hotspot, revision, sectionIds, evidenceIds) : '';
    const outgoing = typedVisual.edges.filter((edge) => edge.fromNodeId === node.nodeId).map((edge) => {
      const target = typedVisual.nodes.find((candidate) => candidate.nodeId === edge.toNodeId);
      return `<div class="diagram-edge"><strong>${escapeHtml(edge.label)}</strong> → ${escapeHtml(target?.label ?? 'Unavailable target')} ${anchorsFor(edge.evidenceAnchorIds) ? `<ul class="diagram-links"><li>${anchorsFor(edge.evidenceAnchorIds)}</li></ul>` : ''}</div>`;
    }).join('');
    return `<li class="diagram-node"><p class="eyebrow">STEP ${nodeIndex + 1}</p><strong>${escapeHtml(node.label)}</strong>${node.detail ? `<small>${escapeHtml(node.detail)}</small>` : ''}${references ? `<p class="reference-meta">Evidence: ${references}</p>` : '<p class="reference-meta">Evidence omitted.</p>'}${hotspotMarkup}${outgoing}</li>`;
  }).join('');
  const type = typedVisual.kind === 'dataFlow' ? 'Data flow' : typedVisual.kind === 'sequence' ? 'Sequence' : 'Hierarchy';
  const visualData = section.visualAssetText ? `<details class="visual-source"><summary>Typed diagram data · JSON</summary><pre>${escapeHtml(section.visualAssetText)}</pre></details>` : '';
  return `<figure class="visual"><figcaption><p class="eyebrow">${type} diagram</p><h4>${escapeHtml(typedVisual.title)}</h4><p class="muted">App-rendered from the accepted typed diagram. No chapter HTML, script, or CSS is executed.</p></figcaption><ol class="diagram">${nodes}</ol>${visualData}</figure>`;
}

function renderVisualWithDom(
  revision: ChapterRevision,
  section: SafeSection,
  sectionIds: Map<string, string>,
  evidenceIds: Map<string, string>,
): string {
  const visual = section.visual;
  if (!visual) return '';
  const element = <K extends keyof HTMLElementTagNameMap>(tag: K, className?: string, text?: string): HTMLElementTagNameMap[K] => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const type = visual.kind === 'dataFlow' ? 'Data flow' : visual.kind === 'sequence' ? 'Sequence' : 'Hierarchy';
  const figure = element('figure', 'visual');
  const caption = element('figcaption');
  caption.append(element('p', 'eyebrow', `${type} diagram`));
  caption.append(element('h4', undefined, visual.title));
  caption.append(element('p', 'muted', 'App-rendered from the accepted typed diagram. No chapter HTML, script, or CSS is executed.'));
  figure.append(caption);
  const list = element('ol', 'diagram');
  for (const [nodeIndex, node] of visual.nodes.entries()) {
    const item = element('li', 'diagram-node');
    item.append(element('p', 'eyebrow', `STEP ${nodeIndex + 1}`));
    item.append(element('strong', undefined, node.label));
    if (node.detail) item.append(element('small', undefined, node.detail));
    const evidenceLinks = node.evidenceAnchorIds.flatMap((anchorId) => {
      const id = evidenceIds.get(`${revision.revisionId}:${anchorId}`);
      const reference = section.references.find((entry) => entry.anchorId === anchorId);
      return id && reference ? [{ id, text: `${reference.path}:${reference.startLine}-${reference.endLine}` }] : [];
    });
    if (evidenceLinks.length > 0) {
      const evidence = element('p', 'reference-meta', 'Evidence: ');
      appendLinks(evidence, evidenceLinks);
      item.append(evidence);
    } else item.append(element('p', 'reference-meta', 'Evidence omitted.'));

    const hotspot = node.hotspotId ? section.hotspots.find((entry) => entry.hotspotId === node.hotspotId) : undefined;
    if (hotspot) {
      const destination = hotspot.destination;
      const label = element(
        destination.kind === 'section' || destination.kind === 'codeLens' ? 'a' : 'span',
        'hotspot',
        hotspot.label,
      );
      if (destination.kind === 'section') {
        const target = sectionIds.get(`${revision.revisionId}:${destination.sectionId}`);
        if (target) label.setAttribute('href', `#${target}`);
        else label.textContent += ' · section unavailable offline';
      } else if (destination.kind === 'codeLens') {
        const target = evidenceIds.get(`${revision.revisionId}:${destination.anchorId}`);
        if (target) {
          label.setAttribute('href', `#${target}`);
          label.textContent += ' · bundled evidence';
        } else label.textContent += ' · source path omitted';
      } else {
        label.classList.add('muted');
        label.textContent += destination.kind === 'relationship'
          ? ' · Atlas relationship unavailable offline'
          : ' · Atlas destination unavailable offline';
      }
      item.append(label);
    }

    const outgoing = visual.edges.filter((edge) => edge.fromNodeId === node.nodeId);
    for (const edge of outgoing) {
      const edgeNode = element('div', 'diagram-edge');
      edgeNode.append(element('strong', undefined, edge.label));
      const target = visual.nodes.find((candidate) => candidate.nodeId === edge.toNodeId);
      edgeNode.append(document.createTextNode(` → ${target?.label ?? 'Unavailable target'} `));
      const edgeEvidence = edge.evidenceAnchorIds.flatMap((anchorId) => {
        const id = evidenceIds.get(`${revision.revisionId}:${anchorId}`);
        const reference = section.references.find((entry) => entry.anchorId === anchorId);
        return id && reference ? [{ id, text: `${reference.path}:${reference.startLine}-${reference.endLine}` }] : [];
      });
      if (edgeEvidence.length > 0) {
        const evidenceList = element('ul', 'diagram-links');
        const evidenceItem = element('li');
        appendLinks(evidenceItem, edgeEvidence);
        evidenceList.append(evidenceItem);
        edgeNode.append(evidenceList);
      }
      item.append(edgeNode);
    }
    list.append(item);
  }
  figure.append(list);
  if (section.visualAssetText) {
    const source = element('details', 'visual-source');
    source.append(element('summary', undefined, 'Typed diagram data · JSON'));
    source.append(element('pre', undefined, section.visualAssetText));
    figure.append(source);
  }
  return figure.outerHTML;

  function appendLinks(parent: HTMLElement, links: Array<{ id: string; text: string }>) {
    links.forEach((link, index) => {
      if (index > 0) parent.append(document.createTextNode(' · '));
      const anchor = element('a', undefined, link.text);
      anchor.setAttribute('href', `#${link.id}`);
      parent.append(anchor);
    });
  }
}

function renderOfflineHotspot(
  hotspot: ChapterSection['hotspots'][number],
  revision: ChapterRevision,
  sectionIds: Map<string, string>,
  evidenceIds: Map<string, string>,
): string {
  const destination = hotspot.destination;
  let destinationMarkup = '';
  if (destination.kind === 'section') {
    const sectionId = sectionIds.get(`${revision.revisionId}:${destination.sectionId}`);
    destinationMarkup = sectionId ? `<a class="hotspot" href="#${sectionId}">${escapeHtml(hotspot.label)}</a>` : '';
  } else if (destination.kind === 'codeLens') {
    const target = evidenceIds.get(`${revision.revisionId}:${destination.anchorId}`);
    destinationMarkup = target
      ? `<a class="hotspot" href="#${target}">${escapeHtml(hotspot.label)} · bundled evidence</a>`
      : `<span class="hotspot muted">${escapeHtml(hotspot.label)} · source path omitted</span>`;
  } else if (destination.kind === 'atlasFile' || destination.kind === 'atlasSymbol') {
    destinationMarkup = `<span class="hotspot muted">${escapeHtml(hotspot.label)} · Atlas destination unavailable offline</span>`;
  } else {
    destinationMarkup = `<span class="hotspot muted">${escapeHtml(hotspot.label)} · Atlas relationship unavailable offline</span>`;
  }
  return destinationMarkup;
}

function renderEvidenceAppendix(
  chapter: SafeChapter,
  section: SafeSection,
  chapterIndex: number,
  evidenceIds: Map<string, string>,
  includeExcerpts: boolean,
): string {
  const references = section.references;
  if (references.length === 0) return '';
  return `<section class="evidence"><h4>Source references</h4>${references.map((reference) => {
    const id = evidenceIds.get(`${chapter.revision.revisionId}:${reference.anchorId}`);
    const excerpt = includeExcerpts
      ? `<pre>${escapeHtml(reference.excerpt)}</pre>`
      : '<p class="muted">Source excerpt omitted from this package.</p>';
    const redaction = reference.redactedCredential ? '<p class="notice">A likely credential was redacted from this citation.</p>' : '';
    return `<article class="excerpt" id="${id ?? `evidence-${chapterIndex + 1}-omitted`}"><h4>${escapeHtml(reference.path)}:${reference.startLine}-${reference.endLine}</h4><p class="reference-meta">Snapshot ${escapeHtml(chapter.revision.snapshotId)}</p>${redaction}${excerpt}</article>`;
  }).join('')}</section>`;
}

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
  })[character] ?? character);
}

function filenameFor(chapters: SafeChapter[]): string {
  if (chapters.length !== 1) return 'code-atlas-offline-guide.html';
  const safe = chapters[0].title.normalize('NFKD').replace(/[^A-Za-z0-9]+/g, '-').replace(/^-+|-+$/g, '').toLowerCase().slice(0, 48);
  return `${safe || 'code-atlas'}-offline-guide.html`;
}
