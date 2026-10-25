import type { CapturedFile, ChapterRevision, IndexedSymbol, SnapshotIndex, SnapshotManifest, SourceAnchor } from './commands';
import { projectProvenRelationshipGraph } from './atlas';

const MAX_PLANS = 8;
const MAX_STEPS = 24;
const MAX_BOUNDARIES = 48;
const MAX_STORAGE_BYTES = 500_000;
const MAX_REQUEST_BYTES = 250_000;

export interface JourneyStep {
  stepId: string;
  question: string;
  path: string;
  sourceSha256: string;
  symbolId: string | null;
  symbolName: string | null;
  anchor: SourceAnchor | null;
  prerequisiteStepId: string | null;
  call: { relationshipId: string; evidence: SourceAnchor } | null;
}

export interface RepositoryJourney {
  schemaVersion: 1;
  journeyId: string;
  projectId: string;
  snapshotId: string;
  question: string;
  createdAtUnixMs: number;
  indexerVersion: string | null;
  steps: JourneyStep[];
  boundaries: string[];
  truncated: boolean;
  selectedStepId: string;
  readStepIds: string[];
}

export interface JourneyStore {
  journeys: RepositoryJourney[];
  token: string | null;
}

export function journeyStorageKey(projectId: string, snapshotId: string): string {
  return `code-atlas:journeys:v1:${encodeURIComponent(projectId)}:${encodeURIComponent(snapshotId)}`;
}

export function journeySourceFiles(manifest: SnapshotManifest): CapturedFile[] {
  return manifest.files.filter((file) => file.kind === 'file' && !file.secretLike
    && (file.contentKind === 'text' || file.contentKind === 'unknown'));
}

function text(value: unknown, max: number): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= max
    && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

function relativePath(value: unknown, max = 4096): value is string {
  return text(value, max) && !value.startsWith('/') && !value.includes('\\')
    && value.split('/').every((part) => part.length > 0 && part !== '.' && part !== '..');
}

function exactKeys(value: unknown, keys: readonly string[]): value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  return Object.keys(record).length === keys.length && keys.every((key) => Object.hasOwn(record, key));
}

function anchor(value: unknown): value is SourceAnchor {
  if (!exactKeys(value, ['path', 'sourceSha256', 'startByte', 'endByte', 'startLine', 'endLine', 'startColumn', 'endColumn'])) return false;
  const item = value as unknown as SourceAnchor;
  return relativePath(item.path) && /^[a-f0-9]{64}$/.test(item.sourceSha256)
    && [item.startByte, item.endByte, item.startLine, item.endLine, item.startColumn, item.endColumn]
      .every((number) => Number.isSafeInteger(number) && number >= 0)
    && item.startLine > 0 && item.endLine >= item.startLine && item.endByte > item.startByte;
}

function symbolLabel(symbol: IndexedSymbol): string {
  return symbol.qualifiedName.length > 300 ? `${symbol.qualifiedName.slice(0, 297)}…` : symbol.qualifiedName;
}

function isJourney(value: unknown, projectId: string, snapshotId: string): value is RepositoryJourney {
  if (!exactKeys(value, ['schemaVersion', 'journeyId', 'projectId', 'snapshotId', 'question', 'createdAtUnixMs', 'indexerVersion', 'steps', 'boundaries', 'truncated', 'selectedStepId', 'readStepIds'])) return false;
  const item = value as unknown as RepositoryJourney;
  if (item.schemaVersion !== 1 || item.projectId !== projectId || item.snapshotId !== snapshotId
    || !text(item.journeyId, 100) || !text(item.question, 300)
    || !Number.isSafeInteger(item.createdAtUnixMs) || item.createdAtUnixMs < 0
    || !(item.indexerVersion === null || text(item.indexerVersion, 200))
    || typeof item.truncated !== 'boolean' || !Array.isArray(item.steps) || item.steps.length < 1 || item.steps.length > MAX_STEPS
    || !Array.isArray(item.boundaries) || item.boundaries.length > MAX_BOUNDARIES || !item.boundaries.every((note) => text(note, 1500))
    || !Array.isArray(item.readStepIds) || item.readStepIds.length > item.steps.length) return false;
  const seen = new Set<string>();
  for (const rawStep of item.steps) {
    if (!exactKeys(rawStep, ['stepId', 'question', 'path', 'sourceSha256', 'symbolId', 'symbolName', 'anchor', 'prerequisiteStepId', 'call'])) return false;
    const step = rawStep as unknown as JourneyStep;
    if (!text(step.stepId, 160) || seen.has(step.stepId) || !text(step.question, 1000)
      || !relativePath(step.path) || !/^[a-f0-9]{64}$/.test(step.sourceSha256)
      || !(step.symbolId === null || text(step.symbolId, 500)) || !(step.symbolName === null || text(step.symbolName, 500))
      || !(step.anchor === null || anchor(step.anchor))
      || !(step.prerequisiteStepId === null || seen.has(step.prerequisiteStepId))
      || !(step.call === null || (exactKeys(step.call, ['relationshipId', 'evidence'])
        && text(step.call.relationshipId, 500) && anchor(step.call.evidence)))) return false;
    if ((step.symbolId === null) !== (step.anchor === null) || (step.symbolId === null) !== (step.symbolName === null)
      || (step.call === null) !== (step.prerequisiteStepId === null)
      || (seen.size === 0 ? step.prerequisiteStepId !== null : step.prerequisiteStepId === null)
      || (step.anchor !== null && (step.anchor.path !== step.path || step.anchor.sourceSha256 !== step.sourceSha256))
      || (step.symbolId === null && item.steps.length !== 1)
      || (step.symbolId !== null && item.indexerVersion === null)) return false;
    seen.add(step.stepId);
  }
  return seen.has(item.selectedStepId) && new Set(item.readStepIds).size === item.readStepIds.length
    && item.readStepIds.every((id) => seen.has(id));
}

/** Reject damaged/newer data without replacing it with an empty plan. */
export function loadJourneyStore(projectId: string, snapshotId: string, storage: Pick<Storage, 'getItem'> = localStorage): JourneyStore {
  const token = storage.getItem(journeyStorageKey(projectId, snapshotId));
  if (token === null) return { journeys: [], token };
  if (new TextEncoder().encode(token).byteLength > MAX_STORAGE_BYTES) throw new Error('The saved journey plan exceeds the local size limit. Its stored data was preserved.');
  let parsed: unknown;
  try { parsed = JSON.parse(token); } catch { throw new Error('The saved journey plan could not be read. Its stored data was preserved.'); }
  if (!Array.isArray(parsed) || parsed.length > MAX_PLANS || !parsed.every((item) => isJourney(item, projectId, snapshotId))
    || new Set(parsed.map((item) => item.journeyId)).size !== parsed.length) {
    throw new Error('The saved journey plan uses an invalid or unsupported format. Its stored data was preserved.');
  }
  return { journeys: parsed, token };
}

/** Compare the last loaded value before writing so a second window cannot silently erase progress. */
export function saveJourneyStore(projectId: string, snapshotId: string, journeys: RepositoryJourney[], expectedToken: string | null,
  storage: Pick<Storage, 'getItem' | 'setItem'> = localStorage): JourneyStore {
  if (journeys.length > MAX_PLANS) throw new Error(`This snapshot can keep up to ${MAX_PLANS} journeys. Remove a plan before creating another.`);
  if (!journeys.every((item) => isJourney(item, projectId, snapshotId))
    || new Set(journeys.map((item) => item.journeyId)).size !== journeys.length) throw new Error('The journey plan could not be validated. The saved plan was preserved.');
  const token = JSON.stringify(journeys);
  if (new TextEncoder().encode(token).byteLength > MAX_STORAGE_BYTES) throw new Error('The journey plan exceeds the local size limit. The saved plan was preserved.');
  if (storage.getItem(journeyStorageKey(projectId, snapshotId)) !== expectedToken) {
    throw new Error('The journey plan changed in another window. Reload the saved plans before editing.');
  }
  storage.setItem(journeyStorageKey(projectId, snapshotId), token);
  return { journeys, token };
}

function sameAnchor(left: SourceAnchor, right: SourceAnchor): boolean {
  return left.path === right.path && left.sourceSha256 === right.sourceSha256
    && left.startByte === right.startByte && left.endByte === right.endByte
    && left.startLine === right.startLine && left.endLine === right.endLine
    && left.startColumn === right.startColumn && left.endColumn === right.endColumn;
}

export function journeyStartingSymbols(manifest: SnapshotManifest, index: SnapshotIndex | null, path: string): IndexedSymbol[] {
  if (!index || index.projectId !== manifest.summary.projectId || index.snapshotId !== manifest.summary.snapshotId) return [];
  const file = journeySourceFiles(manifest).find((item) => item.path === path);
  if (!file || !index.files.some((item) => item.path === path && item.sourceSha256 === file.sha256)) return [];
  return index.symbols.filter((symbol) => symbol.path === path && ['function', 'method'].includes(symbol.kind)
    && symbol.anchor.path === path && symbol.anchor.sourceSha256 === file.sha256 && anchor(symbol.anchor)
    && symbol.anchor.endByte <= file.sizeBytes)
    .sort((left, right) => Number(right.isExported) - Number(left.isExported) || left.anchor.startByte - right.anchor.startByte);
}

/** Plan a reading order over resolved call edges. This is not a runtime execution trace. */
export function createRepositoryJourney(manifest: SnapshotManifest, index: SnapshotIndex | null,
  input: { question: string; path: string; symbolId: string | null }): RepositoryJourney {
  const question = input.question.trim();
  if (question.length < 8 || question.length > 300) throw new Error('Describe the learning question in 8–300 characters.');
  const files = new Map(journeySourceFiles(manifest).map((file) => [file.path, file]));
  const startFile = files.get(input.path);
  if (!startFile) throw new Error('Choose a captured text file from this snapshot.');
  const start = input.symbolId ? journeyStartingSymbols(manifest, index, input.path).find((item) => item.symbolId === input.symbolId) : null;
  if (input.symbolId && !start) throw new Error('The starting declaration is no longer verified by this snapshot index. Choose it again.');
  const journeyId = `journey:${crypto.randomUUID()}`;
  const root: JourneyStep = {
    stepId: `${journeyId}:0`, question: start ? `What happens inside ${symbolLabel(start)}?` : `What does ${input.path.slice(0, 900)} do?`,
    path: input.path, sourceSha256: startFile.sha256, symbolId: start?.symbolId ?? null,
    symbolName: start ? symbolLabel(start) : null, anchor: start?.anchor ?? null, prerequisiteStepId: null, call: null,
  };
  const plan: RepositoryJourney = {
    schemaVersion: 1, journeyId, projectId: manifest.summary.projectId, snapshotId: manifest.summary.snapshotId,
    question, createdAtUnixMs: Date.now(), indexerVersion: start ? index!.indexerVersion : null,
    steps: [root], boundaries: [], truncated: false, selectedStepId: root.stepId, readStepIds: [],
  };
  const boundary = (note: string) => {
    if (plan.boundaries.includes(note)) return;
    if (plan.boundaries.length < MAX_BOUNDARIES) plan.boundaries.push(note.slice(0, 1500));
    else plan.truncated = true;
  };
  if (!start || !index) {
    boundary('This is a file reading step. Choose a verified declaration to follow static call links; unknown language, framework, or protocol boundaries are not inferred from filenames.');
    return plan;
  }
  const graph = projectProvenRelationshipGraph(index, manifest, new Set(files.keys()));
  const symbols = new Map(index.symbols.map((symbol) => [symbol.symbolId, symbol]));
  const outgoing = new Map<string, typeof graph.edges>();
  const provenCalls = new Set<string>();
  for (const edge of graph.edges) {
    if (edge.relationship.kind !== 'calls' || !anchor(edge.relationship.evidence)
      || edge.relationship.evidence.path !== symbols.get(edge.sourceId)?.path) continue;
    const entries = outgoing.get(edge.sourceId) ?? [];
    entries.push(edge);
    outgoing.set(edge.sourceId, entries);
    provenCalls.add(edge.relationship.relationshipId);
  }
  for (const entries of outgoing.values()) entries.sort((left, right) => left.evidence.startByte - right.evidence.startByte
    || left.targetId.localeCompare(right.targetId));
  const unresolved = new Map<string, string[]>();
  for (const edge of index.relationships) {
    if (edge.kind !== 'calls' || provenCalls.has(edge.relationshipId)) continue;
    const notes = unresolved.get(edge.sourceId) ?? [];
    if (notes.length < MAX_BOUNDARIES) notes.push(`${symbols.get(edge.sourceId)?.qualifiedName ?? edge.evidence.path}: ${edge.unresolvedReason || 'the call lacks exact captured endpoint evidence'}`);
    unresolved.set(edge.sourceId, notes);
  }
  const visited = new Set([start.symbolId]);
  for (let cursor = 0; cursor < plan.steps.length; cursor += 1) {
    const step = plan.steps[cursor];
    for (const note of unresolved.get(step.symbolId!) ?? []) boundary(note);
    const fileIndex = index.files.find((file) => file.path === step.path);
    if (fileIndex?.status !== 'indexed') boundary(`${step.path}: local analysis is ${fileIndex?.status ?? 'unavailable'}; this plan cannot establish all calls or branches.`);
    for (const edge of outgoing.get(step.symbolId!) ?? []) {
      if (visited.has(edge.targetId)) continue;
      const target = symbols.get(edge.targetId);
      const targetFile = target ? files.get(target.path) : undefined;
      if (!target || !targetFile || !anchor(target.anchor) || target.anchor.path !== target.path
        || target.anchor.sourceSha256 !== targetFile.sha256 || target.anchor.endByte > targetFile.sizeBytes) { boundary(`${step.path}: a target declaration lacks a valid source range.`); continue; }
      if (plan.steps.length >= MAX_STEPS) { plan.truncated = true; continue; }
      visited.add(target.symbolId);
      plan.steps.push({
        stepId: `${journeyId}:${plan.steps.length}`, question: `How does ${step.symbolName} reach ${symbolLabel(target)}?`,
        path: target.path, sourceSha256: targetFile.sha256, symbolId: target.symbolId,
        symbolName: symbolLabel(target), anchor: target.anchor, prerequisiteStepId: step.stepId,
        call: { relationshipId: edge.relationship.relationshipId, evidence: edge.relationship.evidence },
      });
    }
  }
  boundary('Reading order follows static call links, with repeated or cyclic destinations shown once. Branch conditions, timing, state changes, success/failure paths, and dynamic or cross-language dispatch need their own source evidence; this plan does not establish them.');
  return plan;
}

/** Recheck saved references against current immutable inventory and parser semantics before using them. */
export function journeyStepIssue(plan: RepositoryJourney, step: JourneyStep, manifest: SnapshotManifest, index: SnapshotIndex | null): string | null {
  if (plan.projectId !== manifest.summary.projectId || plan.snapshotId !== manifest.summary.snapshotId) return 'This plan belongs to another snapshot.';
  const file = journeySourceFiles(manifest).find((item) => item.path === step.path && item.sha256 === step.sourceSha256);
  if (!file) return 'This step does not match a captured text file in the selected snapshot.';
  if (!step.symbolId) return null;
  if (!index || index.projectId !== plan.projectId || index.snapshotId !== plan.snapshotId || index.indexerVersion !== plan.indexerVersion) return 'Reload the matching local index to verify this saved call plan.';
  const symbol = index.symbols.find((item) => item.symbolId === step.symbolId);
  if (!symbol || !step.anchor || symbol.path !== step.path || symbolLabel(symbol) !== step.symbolName
    || symbol.anchor.path !== step.path || symbol.anchor.sourceSha256 !== file.sha256 || symbol.anchor.endByte > file.sizeBytes
    || !sameAnchor(symbol.anchor, step.anchor)
    || !index.files.some((item) => item.path === step.path && item.sourceSha256 === file.sha256)) return 'The saved declaration no longer matches the local index. Create a new plan.';
  if (step.call) {
    const parent = plan.steps.find((item) => item.stepId === step.prerequisiteStepId);
    const caller = index.symbols.find((item) => item.symbolId === parent?.symbolId);
    const edge = index.relationships.find((item) => item.relationshipId === step.call!.relationshipId);
    const evidenceFile = manifest.files.find((item) => item.path === step.call!.evidence.path);
    if (!parent || !caller || caller.path !== parent.path || !parent.anchor || !anchor(caller.anchor)
      || caller.anchor.path !== parent.path || caller.anchor.sourceSha256 !== parent.sourceSha256 || !sameAnchor(caller.anchor, parent.anchor)
      || !edge || edge.kind !== 'calls' || edge.confidence !== 'high' || edge.unresolvedReason !== null
      || edge.sourceId !== parent.symbolId || edge.targetId !== step.symbolId || (edge.targetPath !== null && edge.targetPath !== step.path)
      || !sameAnchor(edge.evidence, step.call.evidence) || edge.evidence.path !== parent.path || !evidenceFile
      || evidenceFile.sha256 !== step.call.evidence.sourceSha256 || parent.sourceSha256 !== evidenceFile.sha256
      || step.call.evidence.endByte > evidenceFile.sizeBytes || caller.anchor.endByte > evidenceFile.sizeBytes
      || !index.files.some((item) => item.path === evidenceFile.path && item.sourceSha256 === evidenceFile.sha256)) return 'The saved transition no longer has matching static call evidence. Create a new plan.';
  }
  return null;
}

export function journeyGenerationPaths(plan: RepositoryJourney, step: JourneyStep, manifest: SnapshotManifest): string[] {
  const files = new Map(journeySourceFiles(manifest).map((file) => [file.path, file]));
  const neighbours = plan.steps.filter((item) => item.stepId === step.prerequisiteStepId || item.prerequisiteStepId === step.stepId);
  const candidates = [...new Set([step.path, ...neighbours.map((item) => item.path)])];
  const selected: string[] = [];
  let bytes = 0;
  for (const path of candidates) {
    const file = files.get(path);
    if (!file || file.sizeBytes > MAX_REQUEST_BYTES || bytes + file.sizeBytes > MAX_REQUEST_BYTES || selected.length >= 12) continue;
    selected.push(path);
    bytes += file.sizeBytes;
  }
  return selected[0] === step.path ? selected : [];
}

export function journeyFileRevision(plan: RepositoryJourney, step: JourneyStep, revisions: ChapterRevision[]): ChapterRevision | null {
  return revisions.filter((revision) => revision.projectId === plan.projectId && revision.snapshotId === plan.snapshotId
    && revision.target.scope.kind === 'file' && revision.target.scope.path === step.path)
    .sort((left, right) => right.revisionNumber - left.revisionNumber || right.createdAtUnixMs - left.createdAtUnixMs)[0] ?? null;
}
