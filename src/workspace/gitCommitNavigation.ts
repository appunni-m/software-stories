import type { GitCommit } from '../lib/gitHistory';

/** Move inside the commit list without adding every history row to Tab order. */
export function nextFocusableCommitIndex(
  commits: readonly Pick<GitCommit, 'parents'>[],
  currentIndex: number,
  direction: -1 | 1,
): number {
  if (!Number.isInteger(currentIndex) || currentIndex < 0 || currentIndex >= commits.length) return currentIndex;
  for (let index = currentIndex + direction; index >= 0 && index < commits.length; index += direction) {
    if (commits[index].parents.length > 0) return index;
  }
  return currentIndex;
}
