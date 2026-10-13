import type { ApplicationStatus, CaptureLimits, InterruptedOperation, PendingChapterPreviewSummary, RecentProject } from './commands.ts';
import type { Workspace } from './navigation.ts';

export const PRIMARY_WORKSPACES = [
  { id: 'atlas', label: 'Atlas' },
  { id: 'code-lens', label: 'Code Lens' },
  { id: 'walkthrough', label: 'Walkthrough' },
] as const satisfies ReadonlyArray<{ id: Workspace; label: string }>;

export const WORKSPACE_COPY: Record<Workspace, { eyebrow: string; title: string; subtitle: string }> = {
  atlas: {
    eyebrow: 'WORKSPACE 01 · ORIENT',
    title: 'See how the repository fits together.',
    subtitle: 'Browse its real files and follow relationships only where the source supports them.',
  },
  'code-lens': {
    eyebrow: 'WORKSPACE 02 · INSPECT',
    title: 'Open the code behind an explanation.',
    subtitle: 'Every claim will point to the exact captured file and lines it describes.',
  },
  walkthrough: {
    eyebrow: 'WORKSPACE 03 · UNDERSTAND',
    title: 'Learn one file, one section at a time.',
    subtitle: 'Pause, open a code reference, come back to the same point, and continue when ready.',
  },
  'guide-plan': {
    eyebrow: 'PROJECT WORKFLOW · COVERAGE',
    title: 'Know what has and has not been explained.',
    subtitle: 'Every repository path will have a visible chapter status or a clear reason it is not covered.',
  },
  'chapter-studio': {
    eyebrow: 'PROJECT WORKFLOW · REVIEW',
    title: 'Review every explanation before it becomes a guide.',
    subtitle: 'Inspect claims beside source, make edits, validate, and accept a new revision explicitly.',
  },
  changes: {
    eyebrow: 'PROJECT WORKFLOW · LOCAL GIT',
    title: 'Inspect what changed between two revisions.',
    subtitle: 'Compare local refs without checking them out, changing your working copy, or contacting a remote.',
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
