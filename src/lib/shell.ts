import type { ApplicationStatus, CaptureLimits, InterruptedOperation, PendingChapterPreviewSummary, RecentProject } from './commands.ts';
import type { Workspace } from './navigation.ts';

export const PRIMARY_WORKSPACES: ReadonlyArray<{ id: Workspace; label: string; activeWorkspaces: readonly Workspace[] }> = [
  { id: 'guide-plan', label: 'Guide', activeWorkspaces: ['guide-plan', 'walkthrough', 'chapter-studio'] },
  { id: 'atlas', label: 'Files', activeWorkspaces: ['atlas', 'code-lens'] },
  { id: 'changes', label: 'Changes', activeWorkspaces: ['changes'] },
];

export const WORKSPACE_COPY: Record<Workspace, { eyebrow: string; title: string; subtitle: string }> = {
  atlas: {
    eyebrow: 'FILES · SOURCE AND EXPLANATION',
    title: 'Choose a file. See what it does and where the code proves it.',
    subtitle: 'Browse one captured snapshot, read its explanation, and open any cited lines without leaving Files.',
  },
  'code-lens': {
    eyebrow: 'FILES · SOURCE',
    title: 'Read the source behind this explanation.',
    subtitle: 'Open the exact captured lines, inspect the whole file, or create an explanation for it.',
  },
  walkthrough: {
    eyebrow: 'GUIDE · READ',
    title: 'Follow an explanation at your own pace.',
    subtitle: 'Move one section at a time. Open any source link, then return to where you were.',
  },
  'guide-plan': {
    eyebrow: 'YOUR REPOSITORY GUIDE',
    title: 'Understand this codebase.',
    subtitle: 'Start with a short accepted guide, continue where you left off, or open a real file to see what it does.',
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
export async function loadShellStartupData(commands: ShellStartupCommands): Promise<ShellStartupData> {
  const [status, captureLimits, recentProjects, interruptedOperations, pendingChapterPreviews] = await Promise.all([
    settle(commands.getApplicationStatus),
    settle(commands.getCaptureLimits),
    settle(commands.listRecentProjects),
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
