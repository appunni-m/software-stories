import type { GitComparisonResult, GitFileChange } from './commands';

export interface CapturedGitSourceSnapshot {
  snapshotId: string;
  head: string | null;
  paths: readonly string[];
}

export function matchesCapturedHead(
  comparison: GitComparisonResult,
  snapshot: CapturedGitSourceSnapshot | null | undefined,
): boolean {
  return Boolean(
    snapshot?.snapshotId && snapshot.head && comparison.head
      && comparison.head.objectId.toLowerCase() === snapshot.head.toLowerCase(),
  );
}

/** Return a file path only when the selected capture is the exact comparison head. */
export function matchingCapturedHeadPath(
  change: GitFileChange,
  comparison: GitComparisonResult,
  snapshot: CapturedGitSourceSnapshot | null | undefined,
): string | null {
  const path = change.headPath ?? change.basePath;
  if (!path || !snapshot || !matchesCapturedHead(comparison, snapshot)) return null;
  return snapshot.paths.includes(path) ? path : null;
}
