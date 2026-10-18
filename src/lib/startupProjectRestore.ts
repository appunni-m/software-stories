export interface StartupProjectRestoreInput {
  persistedProjectId: string | null;
  recentProjectIds: readonly string[];
  automaticRestoreAllowed: boolean;
  currentProjectId: string | null;
  projectOpening: boolean;
}

/** Return the last project only when it is still a recent project and startup
 * has not been superseded by an explicit repository-opening action. */
export function startupProjectToRestore(input: StartupProjectRestoreInput): string | null {
  const projectId = input.persistedProjectId?.trim();
  if (!projectId || !input.automaticRestoreAllowed || input.currentProjectId || input.projectOpening) return null;
  return input.recentProjectIds.includes(projectId) ? projectId : null;
}
