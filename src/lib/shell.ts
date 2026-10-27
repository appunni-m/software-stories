import type { ApplicationStatus, CaptureLimits, InterruptedOperation, PendingChapterPreviewSummary, RecentProject } from './commands.ts';
import type { Workspace } from './navigation.ts';

export const WORKSPACE_COPY: Record<Workspace, { eyebrow: string; title: string; subtitle: string }> = {
  atlas: {
    eyebrow: 'EXPLORER · SOURCE AND EXPLANATION',
    title: 'Choose a file. See what it does and where the code proves it.',
    subtitle: 'Browse the captured snapshot, read an explanation, and open its cited lines beside the source.',
  },
  'code-lens': {
    eyebrow: 'EXPLORER · SOURCE',
    title: 'Read the source behind this explanation.',
    subtitle: 'Open the exact captured lines, inspect the whole file, or create an explanation for it.',
  },
  'guide-plan': {
    eyebrow: 'CODE EXPLORATION',
    title: 'Ask about this codebase.',
    subtitle: 'Open a file, ask a question, or build a short Guide from the captured source.',
  },
  'chapter-studio': {
    eyebrow: 'GUIDE · EDIT',
    title: 'Review and edit an explanation.',
    subtitle: 'Check each claim against its source, revise the section you choose, and save a new version for review.',
  },
  changes: {
    eyebrow: 'CHANGES · COMPARE',
    title: 'See what changed between two versions.',
    subtitle: 'Compare local revisions, inspect the changed source, and follow the evidence behind each explanation.',
  },
};

const EXPLORATION_COPY = {
  eyebrow: 'CODE EXPLORATION',
  title: 'Explore this codebase.',
  subtitle: 'Ask a question, follow a guide or explore the flow with source beside it.',
};

/** The captured conversation view replaces these former standalone pages. */
export function workspaceHeading(workspace: Workspace, hasCapturedSnapshot: boolean) {
  return hasCapturedSnapshot && ['atlas', 'code-lens', 'guide-plan'].includes(workspace)
    ? EXPLORATION_COPY : WORKSPACE_COPY[workspace];
}

export interface ShellStartupCommands {
  getApplicationStatus: () => Promise<ApplicationStatus>;
  getCaptureLimits: () => Promise<CaptureLimits>;
  listRecentProjects: () => Promise<RecentProject[]>;
  listInterruptedOperations: () => Promise<InterruptedOperation[]>;
  listPendingGeneratedChapterPreviews: () => Promise<PendingChapterPreviewSummary[]>;
}

export interface ShellStartupData {
  applicationStatus: ApplicationStatus | null;
  applicationStatusUnavailable: boolean;
  captureLimits: CaptureLimits | null;
  recentProjects: RecentProject[] | null;
  recentProjectsUnavailable: boolean;
  interruptedOperations: InterruptedOperation[] | null;
  interruptedOperationsUnavailable: boolean;
  pendingChapterPreviews: PendingChapterPreviewSummary[] | null;
  pendingChapterPreviewsUnavailable: boolean;
}

type Settled<T> = { ok: true; value: T } | { ok: false };

async function settle<T>(load: () => Promise<T>): Promise<Settled<T>> {
  try {
    return { ok: true, value: await load() };
  } catch {
    return { ok: false };
  }
}

/**
 * Load only the local data needed to open the shell. This command contract does
 * not include repository inspection, credentials, AI providers, or narration.
 */
export async function loadShellStartupData(
  commands: ShellStartupCommands,
  onRecentProjects?: (projects: RecentProject[] | null) => void,
): Promise<ShellStartupData> {
  const [status, captureLimits, recentProjects, interruptedOperations, pendingChapterPreviews] = await Promise.all([
    settle(commands.getApplicationStatus),
    settle(commands.getCaptureLimits),
    settle(commands.listRecentProjects).then((result) => {
      // Opening a saved workspace must not wait for recovery/preview enumeration.
      onRecentProjects?.(result.ok ? result.value : null);
      return result;
    }),
    settle(commands.listInterruptedOperations),
    settle(commands.listPendingGeneratedChapterPreviews),
  ]);
  return {
    applicationStatus: status.ok ? status.value : null,
    applicationStatusUnavailable: !status.ok,
    captureLimits: captureLimits.ok ? captureLimits.value : null,
    recentProjects: recentProjects.ok ? recentProjects.value : null,
    recentProjectsUnavailable: !recentProjects.ok,
    interruptedOperations: interruptedOperations.ok ? interruptedOperations.value : null,
    interruptedOperationsUnavailable: !interruptedOperations.ok,
    pendingChapterPreviews: pendingChapterPreviews.ok ? pendingChapterPreviews.value : null,
    pendingChapterPreviewsUnavailable: !pendingChapterPreviews.ok,
  };
}
