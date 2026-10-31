import type { GitCommit } from '../lib/gitHistory';

const commitDateFormatter = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric' });

/** Keep commit dates readable in the narrow Changes rail without changing the stored ISO instant. */
export function commitDateLabel(timestamp: string): string | null {
  const date = new Date(timestamp);
  return Number.isFinite(date.getTime()) ? commitDateFormatter.format(date) : null;
}

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
  commits: readonly Pick<GitCommit, 'parents' | 'isHead'>[],
  state: { loaded: boolean; loading: boolean; stopped: boolean; error: string | null },
): string | null {
  if (!state.loaded || state.loading || state.stopped || state.error) return null;
  if (commits.length === 0) return 'No commits yet. Create the first commit in your Git tool, then select Refresh here.';
  const head = commits.find((commit) => commit.isHead);
  if (!head) return 'The current commit is unavailable in this history. Refresh to try again.';
  if (head.parents.length === 0) return 'This is the initial commit. Create another commit in your Git tool, then select Refresh to compare changes.';
  if (!commits.some((commit) => commit.parents.length > 0)) {
    return 'Only the initial commit is available. Create another commit in your Git tool, then select Refresh to compare changes.';
  }
  return null;
}
