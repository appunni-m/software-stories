import type { SourceDiscoveryResult, SourceDiscoveryScope } from './sourceDiscovery';

const KEY_PREFIX = 'code-atlas:source-discovery:v2:';
const MAX_ID_CHARS = 1024;
const MAX_SNAPSHOT_CHARS = 256;
const MAX_QUESTION_CHARS = 300;
const MAX_TARGET_PATH_CHARS = 1024;
const MAX_FILES = 12;
const MAX_PATH_CHARS = 1024;
const MAX_REASON_CHARS = 1200;
const MAX_LIMITATION_CHARS = 2000;
const MAX_RECORD_BYTES = 32 * 1024;
const MAX_SAVED_RESULTS = 8;
const MAX_COLLECTION_BYTES = 128 * 1024;

export const SOURCE_DISCOVERY_SAVED_EVENT = 'code-atlas:source-discovery-saved';

export type SourceDiscoveryRead =
  | { status: 'missing' }
  | { status: 'available'; result: SourceDiscoveryResult }
  | { status: 'unavailable'; issue: 'storage' | 'oversized' | 'unreadable' | 'unsupported' | 'invalid' | 'stale-path' };

interface SavedSourceDiscovery {
  schemaVersion: 1;
  projectId: string;
  snapshotId: string;
  question: string;
  targetPath: string | null;
  selectedFiles: { path: string; reason: string }[];
  limitation: string;
  savedAtUnixMs: number;
}

interface SavedSourceDiscoveryCollection {
  schemaVersion: 2;
  projectId: string;
  snapshotId: string;
  records: SavedSourceDiscovery[];
}

function hasExactKeys(value: Record<string, unknown>, keys: string[]): boolean {
  return Object.keys(value).length === keys.length && keys.every((key) => Object.hasOwn(value, key));
}

function boundedText(value: unknown, max: number, allowEmpty = false): value is string {
  return typeof value === 'string' && value.length <= max
    && (allowEmpty || value.trim().length > 0)
    && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

function boundedCharacters(value: unknown, max: number): value is string {
  return boundedText(value, max * 2) && [...value].length <= max;
}

function relativePath(value: unknown, max: number): value is string {
  if (!boundedText(value, max)) return false;
  return !value.startsWith('/') && !value.includes('\\')
    && value.split('/').every((part) => part !== '' && part !== '.' && part !== '..');
}

function scopeIsValid(scope: SourceDiscoveryScope): boolean {
  return boundedText(scope.projectId, MAX_ID_CHARS)
    && boundedText(scope.snapshotId, MAX_SNAPSHOT_CHARS)
    && boundedCharacters(scope.question, MAX_QUESTION_CHARS)
    && (scope.targetPath == null || relativePath(scope.targetPath, MAX_TARGET_PATH_CHARS));
}

function targetPath(scope: SourceDiscoveryScope): string | null {
  return scope.targetPath ?? null;
}

function storageKey(scope: SourceDiscoveryScope): string | null {
  if (!scopeIsValid(scope)) return null;
  try { return `${KEY_PREFIX}${encodeURIComponent(scope.projectId)}:${encodeURIComponent(scope.snapshotId)}`; }
  catch { return null; }
}

function byteLength(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

function validResult(result: unknown): result is SourceDiscoveryResult {
  if (!result || typeof result !== 'object' || Array.isArray(result)) return false;
  const value = result as Record<string, unknown>;
  if (!hasExactKeys(value, ['selectedFiles', 'limitation']) || !Array.isArray(value.selectedFiles)
    || value.selectedFiles.length < 1 || value.selectedFiles.length > MAX_FILES
    || !boundedText(value.limitation, MAX_LIMITATION_CHARS, true)) return false;
  const paths = new Set<string>();
  for (const candidate of value.selectedFiles) {
    if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) return false;
    const file = candidate as Record<string, unknown>;
    if (!hasExactKeys(file, ['path', 'reason']) || !relativePath(file.path, MAX_PATH_CHARS)
      || !boundedText(file.reason, MAX_REASON_CHARS)) return false;
    if (paths.has(file.path)) return false;
    paths.add(file.path);
  }
  return true;
}

function validRecord(value: unknown): value is SavedSourceDiscovery {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  if (!hasExactKeys(record, ['schemaVersion', 'projectId', 'snapshotId', 'question', 'targetPath', 'selectedFiles', 'limitation', 'savedAtUnixMs'])
    || record.schemaVersion !== 1 || !boundedText(record.projectId, MAX_ID_CHARS)
    || !boundedText(record.snapshotId, MAX_SNAPSHOT_CHARS) || !boundedCharacters(record.question, MAX_QUESTION_CHARS)
    || !(record.targetPath === null || relativePath(record.targetPath, MAX_TARGET_PATH_CHARS))
    || !Number.isSafeInteger(record.savedAtUnixMs) || Number(record.savedAtUnixMs) <= 0) return false;
  if (!validResult({ selectedFiles: record.selectedFiles, limitation: record.limitation })) return false;
  try { return byteLength(JSON.stringify(record)) <= MAX_RECORD_BYTES; }
  catch { return false; }
}

function validCollection(value: unknown): value is SavedSourceDiscoveryCollection {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const collection = value as Record<string, unknown>;
  if (!hasExactKeys(collection, ['schemaVersion', 'projectId', 'snapshotId', 'records'])
    || collection.schemaVersion !== 2 || !boundedText(collection.projectId, MAX_ID_CHARS)
    || !boundedText(collection.snapshotId, MAX_SNAPSHOT_CHARS) || !Array.isArray(collection.records)
    || collection.records.length < 1 || collection.records.length > MAX_SAVED_RESULTS) return false;

  const scopes = new Set<string>();
  for (const record of collection.records) {
    if (!validRecord(record) || record.projectId !== collection.projectId || record.snapshotId !== collection.snapshotId) return false;
    const identity = JSON.stringify([record.question, record.targetPath]);
    if (scopes.has(identity)) return false;
    scopes.add(identity);
  }
  return true;
}

function recordMatchesScope(record: SavedSourceDiscovery, scope: SourceDiscoveryScope): boolean {
  return record.projectId === scope.projectId && record.snapshotId === scope.snapshotId
    && record.question === scope.question && record.targetPath === targetPath(scope);
}

function readSavedRecords(serialized: string, scope: SourceDiscoveryScope): SavedSourceDiscovery[] | null {
  if (byteLength(serialized) > MAX_COLLECTION_BYTES) return null;
  try {
    const parsed: unknown = JSON.parse(serialized);
    if (validCollection(parsed) && parsed.projectId === scope.projectId && parsed.snapshotId === scope.snapshotId) {
      return parsed.records;
    }
    return null;
  } catch { return null; }
}

/** Save only a bounded code-selection result. Reviews, consent and source contents are never retained. */
export function saveSourceDiscoveryResult(scope: SourceDiscoveryScope, result: SourceDiscoveryResult): boolean {
  const key = storageKey(scope);
  if (!key || !validResult(result)) return false;
  const record: SavedSourceDiscovery = {
    schemaVersion: 1,
    projectId: scope.projectId,
    snapshotId: scope.snapshotId,
    question: scope.question,
    targetPath: targetPath(scope),
    selectedFiles: result.selectedFiles.map(({ path, reason }) => ({ path, reason })),
    limitation: result.limitation,
    savedAtUnixMs: Date.now(),
  };
  try {
    const current = localStorage.getItem(key);
    const previousRecords = current === null ? [] : readSavedRecords(current, scope);
    // Do not replace a malformed or foreign record silently.
    if (previousRecords === null) return false;

    const previousNewest = Math.max(0, ...previousRecords.map((candidate) => candidate.savedAtUnixMs));
    record.savedAtUnixMs = Math.max(record.savedAtUnixMs, previousNewest);
    const identity = JSON.stringify([record.question, record.targetPath]);
    let records = [
      ...previousRecords.filter((candidate) => JSON.stringify([candidate.question, candidate.targetPath]) !== identity),
      record,
    ].sort((left, right) => right.savedAtUnixMs - left.savedAtUnixMs
      || Number(right === record) - Number(left === record));
    let serialized = '';
    while (records.length > 0) {
      const collection: SavedSourceDiscoveryCollection = {
        schemaVersion: 2,
        projectId: scope.projectId,
        snapshotId: scope.snapshotId,
        records,
      };
      serialized = JSON.stringify(collection);
      if (records.length <= MAX_SAVED_RESULTS && byteLength(serialized) <= MAX_COLLECTION_BYTES) break;
      records = records.slice(0, -1);
    }
    if (records.length === 0 || byteLength(serialized) > MAX_COLLECTION_BYTES) return false;
    localStorage.setItem(key, serialized);
    return true;
  } catch { return false; }
}

/** Read only for the same question, snapshot and target, and only while every path remains in its manifest. */
export function readSourceDiscoveryResult(
  scope: SourceDiscoveryScope,
  availablePaths: readonly string[],
): SourceDiscoveryRead {
  const key = storageKey(scope);
  if (!key) return { status: 'missing' };
  let serialized: string | null;
  try {
    serialized = localStorage.getItem(key);
  } catch {
    return { status: 'unavailable', issue: 'storage' };
  }
  if (serialized === null) return { status: 'missing' };
  if (byteLength(serialized) > MAX_COLLECTION_BYTES) return { status: 'unavailable', issue: 'oversized' };

  let parsed: unknown;
  try { parsed = JSON.parse(serialized); }
  catch { return { status: 'unavailable', issue: 'unreadable' }; }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return { status: 'unavailable', issue: 'invalid' };
  const collection = parsed as Record<string, unknown>;
  if (collection.schemaVersion !== 2) return { status: 'unavailable', issue: 'unsupported' };
  if (Array.isArray(collection.records) && collection.records.some((candidate) => candidate && typeof candidate === 'object'
    && !Array.isArray(candidate) && (candidate as Record<string, unknown>).schemaVersion !== 1)) {
    return { status: 'unavailable', issue: 'unsupported' };
  }
  if (!validCollection(parsed)) return { status: 'unavailable', issue: 'invalid' };
  if (collection.projectId !== scope.projectId || collection.snapshotId !== scope.snapshotId) {
    return { status: 'unavailable', issue: 'invalid' };
  }
  const records = collection.records as SavedSourceDiscovery[];
  try {
    const saved = records.find((record) => recordMatchesScope(record, scope));
    if (!saved) return { status: 'missing' };
    const allowed = new Set(availablePaths);
    if (saved.selectedFiles.some(({ path }) => !allowed.has(path))) return { status: 'unavailable', issue: 'stale-path' };
    return { status: 'available', result: {
      selectedFiles: saved.selectedFiles.map(({ path, reason }) => ({ path, reason })),
      limitation: saved.limitation,
    } };
  } catch { return { status: 'unavailable', issue: 'invalid' }; }
}
