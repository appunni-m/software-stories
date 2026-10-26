export interface SavedFileLocation { snapshotId: string; path?: string }
export type SavedFileLocationRead =
  | { status: 'missing' }
  | { status: 'available'; location: SavedFileLocation }
  | { status: 'unavailable' };

const MAX_FILE_LOCATION_BYTES = 8_192;
const MAX_EXPLORER_FOLDERS = 1_000;
const MAX_EXPLORER_STATE_BYTES = 64 * 1_024;
const MAX_PANEL_LAYOUT_BYTES = 8_192;
const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f-\u009f]/u;

function isRelativeCapturedPath(value: unknown): value is string {
  return typeof value === 'string'
    && value.length > 0
    && value.length <= 4096
    && !value.startsWith('/')
    && !CONTROL_CHARACTERS.test(value)
    && value.split('/').every((part) => part.length > 0 && part !== '.' && part !== '..');
}

function isSnapshotId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value === value.trim()
    && value.length <= 256 && !CONTROL_CHARACTERS.test(value);
}

function isSavedFileLocation(value: unknown): value is SavedFileLocation {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record);
  return keys.every((key) => key === 'snapshotId' || key === 'path')
    && Object.hasOwn(record, 'snapshotId') && isSnapshotId(record.snapshotId)
    && (!Object.hasOwn(record, 'path') || isRelativeCapturedPath(record.path));
}

export function readFileLocation(projectId: string): SavedFileLocationRead {
  try {
    const serialized = localStorage.getItem(`code-atlas:last-file:${projectId}`);
    if (serialized === null) return { status: 'missing' };
    if (new TextEncoder().encode(serialized).byteLength > MAX_FILE_LOCATION_BYTES) return { status: 'unavailable' };
    const value: unknown = JSON.parse(serialized);
    if (!isSavedFileLocation(value)) return { status: 'unavailable' };
    return { status: 'available', location: value };
  } catch { /* Preserve unreadable optional UI state; browsing remains available. */ }
  return { status: 'unavailable' };
}

export function rememberFileLocation(projectId: string, location: SavedFileLocation, replaceUnavailable = false): boolean {
  const saved = readFileLocation(projectId);
  if (saved.status === 'unavailable' && !replaceUnavailable) return false;
  const normalized = location.path === undefined ? { snapshotId: location.snapshotId } : location;
  if (!isSavedFileLocation(normalized)) return false;
  try {
    const serialized = JSON.stringify(normalized);
    if (typeof serialized !== 'string') return false;
    if (new TextEncoder().encode(serialized).byteLength > MAX_FILE_LOCATION_BYTES) return false;
    localStorage.setItem(`code-atlas:last-file:${projectId}`, serialized);
    return true;
  } catch { /* Optional UI state never blocks repository browsing. */ }
  return false;
}

/** Explorer disclosure state is scoped to an immutable snapshot and stays small. */
export function readExpandedExplorerFolders(projectId: string, snapshotId: string): string[] {
  if (!isSnapshotId(projectId) || !isSnapshotId(snapshotId)) return [];
  try {
    const serialized = localStorage.getItem(`code-atlas:explorer-folders:${projectId}:${snapshotId}`);
    if (serialized === null || new TextEncoder().encode(serialized).byteLength > MAX_EXPLORER_STATE_BYTES) return [];
    const value: unknown = JSON.parse(serialized);
    if (!value || typeof value !== 'object' || Array.isArray(value)) return [];
    const record = value as Record<string, unknown>;
    if (Object.keys(record).some((key) => !['version', 'folders'].includes(key))
      || record.version !== 1 || !Array.isArray(record.folders) || record.folders.length > MAX_EXPLORER_FOLDERS
      || !record.folders.every(isRelativeCapturedPath)) return [];
    return [...new Set(record.folders)];
  } catch { /* Optional Explorer state never blocks browsing. */ }
  return [];
}

export function rememberExpandedExplorerFolders(projectId: string, snapshotId: string, folders: readonly string[]): boolean {
  if (!isSnapshotId(projectId) || !isSnapshotId(snapshotId)) return false;
  const normalized = [...new Set(folders.filter(isRelativeCapturedPath))].slice(-MAX_EXPLORER_FOLDERS);
  try {
    const serialized = JSON.stringify({ version: 1, folders: normalized });
    if (new TextEncoder().encode(serialized).byteLength > MAX_EXPLORER_STATE_BYTES) return false;
    localStorage.setItem(`code-atlas:explorer-folders:${projectId}:${snapshotId}`, serialized);
    return true;
  } catch { /* Optional Explorer state never blocks browsing. */ }
  return false;
}

export interface WorkspacePanelLayout {
  sidebar?: number;
  source?: number;
}

function isPanelSize(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 5 && value <= 95;
}

/** Split-pane proportions are optional, project/snapshot-local UI preferences. */
export function readWorkspacePanelLayout(projectId: string, snapshotId: string): WorkspacePanelLayout {
  if (!isSnapshotId(projectId) || !isSnapshotId(snapshotId)) return {};
  try {
    const serialized = localStorage.getItem(`code-atlas:workspace-panels:${projectId}:${snapshotId}`);
    if (serialized === null || new TextEncoder().encode(serialized).byteLength > MAX_PANEL_LAYOUT_BYTES) return {};
    const value: unknown = JSON.parse(serialized);
    if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
    const record = value as Record<string, unknown>;
    if (Object.keys(record).some((key) => !['version', 'sidebar', 'source'].includes(key)) || record.version !== 1) return {};
    return {
      ...(isPanelSize(record.sidebar) ? { sidebar: record.sidebar } : {}),
      ...(isPanelSize(record.source) ? { source: record.source } : {}),
    };
  } catch { /* Optional UI state never blocks repository browsing. */ }
  return {};
}

export function rememberWorkspacePanelLayout(
  projectId: string,
  snapshotId: string,
  layout: WorkspacePanelLayout,
): boolean {
  if (!isSnapshotId(projectId) || !isSnapshotId(snapshotId)) return false;
  const saved = readWorkspacePanelLayout(projectId, snapshotId);
  const next = {
    ...(isPanelSize(layout.sidebar) ? { sidebar: layout.sidebar } : saved.sidebar !== undefined ? { sidebar: saved.sidebar } : {}),
    ...(isPanelSize(layout.source) ? { source: layout.source } : saved.source !== undefined ? { source: saved.source } : {}),
  };
  try {
    const serialized = JSON.stringify({ version: 1, ...next });
    if (new TextEncoder().encode(serialized).byteLength > MAX_PANEL_LAYOUT_BYTES) return false;
    localStorage.setItem(`code-atlas:workspace-panels:${projectId}:${snapshotId}`, serialized);
    return true;
  } catch { /* Optional UI state never blocks repository browsing. */ }
  return false;
}
