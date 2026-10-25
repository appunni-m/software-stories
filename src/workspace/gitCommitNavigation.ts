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

/** Explain why no first-parent comparison can start after history has loaded. */
export function commitHistoryEmptyMessage(
  commits: readonly Pick<GitCommit, 'parents'>[],
  state: { loaded: boolean; loading: boolean; stopped: boolean; error: string | null },
): string | null {
  if (!state.loaded || state.loading || state.stopped || state.error) return null;
  if (commits.length === 0) return 'No commits yet. Create the first commit, then refresh to compare changes.';
  if (!commits.some((commit) => commit.parents.length > 0)) {
    return 'Only the initial commit is available. Create another commit to compare changes.';
  }
  return null;
}
