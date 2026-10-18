import type { InterruptedOperation, PendingChapterPreviewSummary } from './commands.ts';

export type NotificationAvailability = 'loading' | 'ready' | 'unavailable';
export interface WorkspaceAnswerRestoration {
  projectId: string; snapshotId: string; availability: NotificationAvailability;
}

/** A previously restored scope cannot stand in for the newly selected answers. */
export function workspaceAnswerAvailability(
  native: boolean, projectId: string | undefined, snapshotId: string | undefined,
  restored: WorkspaceAnswerRestoration | null,
): NotificationAvailability {
  if (!native || !projectId || !snapshotId) return 'ready';
  return restored?.projectId === projectId && restored.snapshotId === snapshotId ? restored.availability : 'loading';
}

/** A complete unread total requires both global lookups and the selected workspace. */
export function notificationAvailability(input: {
  startupComplete: boolean; workspaceReady: boolean; answers: NotificationAvailability; notices?: NotificationAvailability; refreshing: boolean; unavailable: boolean;
}): NotificationAvailability {
  if (!input.startupComplete || !input.workspaceReady || input.answers === 'loading' || input.notices === 'loading' || input.refreshing) return 'loading';
  return input.unavailable || input.answers === 'unavailable' || input.notices === 'unavailable' ? 'unavailable' : 'ready';
}

export function notificationSummary(items: readonly { read?: boolean }[], availability: NotificationAvailability) {
  if (availability === 'loading') return {
    label: 'Notifications, loading updates', badge: null, announcement: '', status: 'Loading updates…',
  };
  if (availability === 'unavailable') return {
    label: 'Notifications, some updates unavailable', badge: null,
    announcement: 'Some updates could not be loaded. Open Notifications to try again.',
    status: 'Some updates could not be loaded. You can try again below.',
  };
  const unread = items.filter((item) => !item.read).length;
  const count = `${unread} unread update${unread === 1 ? '' : 's'}`;
  return {
    label: `Notifications, ${unread ? count : 'no unread updates'}`,
    badge: unread > 0 ? (unread > 9 ? '9+' : String(unread)) : null,
    announcement: unread > 0 ? `${count} available in Notifications.` : '', status: null,
  };
}
const interruptedPurpose = {
  fileChapter: 'File explanation', repositoryOverview: 'Repository guide', sectionUpdate: 'Section update',
  codeLensExplanation: 'Selected code', prGuide: 'Change guide', prChangeExplanation: 'Changed code', repositoryPlan: 'Repository question',
};
const brief = (value: string, limit = 220) => {
  const text = value.replace(/\s+/g, ' ').trim();
  const characters = Array.from(text);
  return characters.length > limit ? `${characters.slice(0, limit - 1).join('')}…` : text;
};

/** Describe saved work with user context; never render journal/provider identifiers. */
export function interruptedNotificationMessage(operation: InterruptedOperation, projectName?: string): string {
  const repository = brief(projectName?.trim() || 'Other repository', 80);
  if (operation.kind === 'capture') return `${repository} · You can keep reading the previous version or try the update again.`;
  const question = operation.providerRequest?.repositoryPlan?.question;
  const purpose = operation.providerRequest?.purpose;
  const context = question?.trim() || operation.targetPath?.trim() || (purpose && interruptedPurpose[purpose]) || 'Unfinished explanation';
  return `${repository} · ${brief(context)}`;
}

export function previewNotificationMessage(preview: PendingChapterPreviewSummary, projectName?: string): string {
  return [brief(projectName?.trim() || 'Other repository', 80), brief(preview.title), preview.targetPath ? brief(preview.targetPath) : ''].filter(Boolean).join(' · ');
}
