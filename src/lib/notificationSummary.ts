import type { InterruptedOperation, PendingChapterPreviewSummary } from './commands.ts';
import { questionSummaryText } from '../workspace/outcomes.ts';

export type NotificationAvailability = 'loading' | 'ready' | 'unavailable';
export type NotificationBucket = 'attention' | 'updates';
export function notificationBucketForOutcome(tone: 'info' | 'warning' | 'error'): NotificationBucket {
  return tone === 'info' ? 'updates' : 'attention';
}
export interface NotificationGroupingItem {
  id: string;
  projectId?: string;
  snapshotId?: string;
  groupKey?: string;
  bucket?: NotificationBucket;
  /** Overrides mixed historical severities when the caller knows the group's current state. */
  groupBucket?: NotificationBucket;
  countLabel?: string;
  read?: boolean;
}
export interface NotificationGroup<T extends NotificationGroupingItem> {
  id: string;
  bucket: NotificationBucket;
  countLabel: string;
  unreadCount: number;
  items: T[];
}

/** Stable, in-memory key for grouping the same question within the same scope. */
export function notificationGroupKey(...parts: string[]): string {
  return JSON.stringify(parts.map((part) => part.normalize('NFC').replace(/\s+/g, ' ').trim().toLowerCase()));
}

/** Keep notification rows scannable while leaving the full question available from its action. */
export function briefNotificationText(value: string, limit = 220): string {
  const text = value.replace(/\s+/g, ' ').trim();
  const characters = Array.from(text);
  return characters.length > limit ? `${characters.slice(0, limit - 1).join('')}…` : text;
}

/** Show the code topic in notifications without surfacing generated request scaffolding. */
export function questionNotificationText(value: string, limit = 140): string {
  return briefNotificationText(questionSummaryText(value, limit), limit);
}

/** Group display rows without changing their persisted identities or actions. */
export function groupNotifications<T extends NotificationGroupingItem>(items: readonly T[]): NotificationGroup<T>[] {
  const grouped = new Map<string, T[]>();
  for (const item of items) {
    const key = item.groupKey ?? `item:${item.id}`;
    const group = grouped.get(key);
    if (group) group.push(item);
    else grouped.set(key, [item]);
  }
  return [...grouped].map(([id, entries]) => ({
    id,
    bucket: entries.find((item) => item.groupBucket)?.groupBucket
      ?? (entries.some((item) => item.bucket === 'attention') ? 'attention' : 'updates'),
    countLabel: entries[0]?.countLabel ?? 'updates',
    unreadCount: entries.filter((item) => item.read === false).length,
    items: entries,
  }));
}

/** Keep the selected repository's work visible and retain all other
 * repository notifications under one collapsed panel section. Global items
 * stay visible because they describe the app rather than another repository. */
export function partitionNotificationGroups<T extends NotificationGroupingItem>(
  groups: readonly NotificationGroup<T>[],
  selectedProjectId?: string,
  selectedSnapshotId?: string,
): { current: NotificationGroup<T>[]; earlierSnapshots: NotificationGroup<T>[]; otherProjects: NotificationGroup<T>[] } {
  if (!selectedProjectId) return { current: [...groups], earlierSnapshots: [], otherProjects: [] };
  const current: NotificationGroup<T>[] = [];
  const earlierSnapshots: NotificationGroup<T>[] = [];
  const otherProjects: NotificationGroup<T>[] = [];
  for (const group of groups) {
    const belongsToOtherProject = group.items.length > 0
      && group.items.every((item) => item.projectId !== undefined && item.projectId !== selectedProjectId);
    const belongsToEarlierSnapshot = !belongsToOtherProject && Boolean(selectedSnapshotId) && group.items.length > 0
      && group.items.every((item) => item.projectId === selectedProjectId && item.snapshotId !== undefined && item.snapshotId !== selectedSnapshotId);
    (belongsToOtherProject ? otherProjects : belongsToEarlierSnapshot ? earlierSnapshots : current).push(group);
  }
  return { current, earlierSnapshots, otherProjects };
}

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

export function notificationSummary(items: readonly { read?: boolean }[], availability: NotificationAvailability, issue?: 'full'|null) {
  if (availability === 'loading') return {
    label: 'Notifications, loading updates', badge: null, announcement: '', status: 'Loading updates…',
  };
  if (availability === 'unavailable') return {
    label: issue==='full'?'Notifications, history full':'Notifications, some updates unavailable', badge: null,
    announcement: issue==='full'?'Notification history is full. Dismiss an older notification to make room.':'Some updates could not be loaded. Open Notifications to try again.',
    status: issue==='full'?'New notifications are waiting to save. Dismiss an older notification to make room.':'Some updates could not be loaded. You can try again below.',
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
/** Describe saved work with user context; never render journal/provider identifiers. */
export function interruptedNotificationMessage(operation: InterruptedOperation, projectName?: string): string {
  const repository = briefNotificationText(projectName?.trim() || 'Other repository', 80);
  if (operation.kind === 'capture') return `${repository} · You can keep reading the previous version or try the update again.`;
  const question = operation.providerRequest?.repositoryPlan?.question;
  const purpose = operation.providerRequest?.purpose;
  const context = question?.trim() || operation.targetPath?.trim() || (purpose && interruptedPurpose[purpose]) || 'Unfinished explanation';
  return `${repository} · ${question ? questionNotificationText(context) : briefNotificationText(context)}`;
}

/** Prefer the exact question when the recovery record retains it; otherwise
 * keep a safer file/operation-scope group instead of merging unrelated work. */
export function interruptedNotificationGroupKey(operation: InterruptedOperation): string {
  const request = operation.providerRequest;
  const question = request?.repositoryPlan?.question?.trim();
  if (question) return notificationGroupKey('interrupted-question', operation.projectId, operation.snapshotId, question);
  const scope = operation.targetPath || request?.selectedPaths.join(', ') || request?.purpose || operation.kind;
  return notificationGroupKey('interrupted-scope', operation.projectId, operation.snapshotId, scope);
}

/** Keep recoveries for the currently open repository ahead of other projects. */
export function sortInterruptedOperations(
  operations: readonly InterruptedOperation[], selectedProjectId?: string,
): InterruptedOperation[] {
  return [...operations].sort((a, b) => {
    const projectOrder = Number(a.projectId !== selectedProjectId) - Number(b.projectId !== selectedProjectId);
    return projectOrder || b.startedAtUnixMs - a.startedAtUnixMs || a.operationId.localeCompare(b.operationId);
  });
}

export function previewNotificationMessage(preview: PendingChapterPreviewSummary, projectName?: string): string {
  return [briefNotificationText(projectName?.trim() || 'Other repository', 80), briefNotificationText(preview.title), preview.targetPath ? briefNotificationText(preview.targetPath) : ''].filter(Boolean).join(' · ');
}
