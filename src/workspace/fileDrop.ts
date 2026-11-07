export const WORKSPACE_FILE_CONTEXT_MIME = 'application/x-code-atlas-context';

export interface ActiveWorkspaceFileDrag {
  path: string;
  token: string;
}

export function workspaceFileDragPayload(path: string, token: string): string {
  return JSON.stringify({ path, token });
}

/** Accept a file drop only from the matching live Explorer drag and snapshot inventory. */
export function acceptedWorkspaceFileDrop(
  payload: string,
  active: ActiveWorkspaceFileDrag | null,
  capturedPaths: ReadonlySet<string>,
): string | null {
  if (!active) return null;
  try {
    const candidate: unknown = JSON.parse(payload);
    if (candidate === null || typeof candidate !== 'object' || Array.isArray(candidate)) return null;
    const value = candidate as Record<string, unknown>;
    if (Object.keys(value).length !== 2 || typeof value.path !== 'string' || typeof value.token !== 'string') return null;
    if (value.path !== active.path || value.token !== active.token || !capturedPaths.has(value.path)) return null;
    return value.path;
  } catch {
    return null;
  }
}
