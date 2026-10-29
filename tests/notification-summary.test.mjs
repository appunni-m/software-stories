import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { briefNotificationText, groupNotifications, interruptedNotificationGroupKey, interruptedNotificationMessage, notificationAvailability, notificationBucketForOutcome, notificationGroupKey, notificationSummary, partitionNotificationGroups, previewNotificationMessage, questionNotificationText, sortInterruptedOperations, workspaceAnswerAvailability } from '../src/lib/notificationSummary.ts';

const ready = { startupComplete: true, workspaceReady: true, answers: 'ready', refreshing: false, unavailable: false };
const unread = Array.from({ length: 34 }, () => ({}));

describe('notification totals', () => {
  it('keeps question previews compact, whitespace-normalized and Unicode-safe', () => {
    assert.equal(briefNotificationText('  What\n happens\t next?  '), 'What happens next?');
    const long = `${'a'.repeat(138)}😀${'b'.repeat(8)}`;
    const shortened = briefNotificationText(long, 140);
    assert.equal(Array.from(shortened).length, 140);
    assert.equal(shortened.endsWith('…'), true);
    assert.equal(shortened.includes('😀'), true);
  });

  it('keeps the unread badge in the bell control flow', async () => {
    const css = await readFile(new URL('../src/NotificationCenter.css', import.meta.url), 'utf8');
    const triggerRule = css.match(/\.notification-trigger\s*\{([^}]+)\}/u)?.[1] ?? '';
    const countRule = css.match(/\.notification-count\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(triggerRule, /position:\s*relative/u);
    assert.match(triggerRule, /width:\s*auto/u);
    assert.match(triggerRule, /gap:\s*4px/u);
    assert.match(countRule, /position:\s*static/u);
    assert.doesNotMatch(countRule, /position:\s*absolute/u);
  });

  it('shows generated question topics in notifications without exposing request instructions', async () => {
    const prompt = 'Create exactly one fresh Guide section from the selected Fontdone snapshot about only the PFR0 branch in Font::memory_face (src/font.rs). Trace the complete implementation and preserve every error branch.';
    assert.equal(questionNotificationText(prompt), 'PFR0 branch in Font::memory_face (src/font.rs).');
    assert.equal(interruptedNotificationMessage({ operationId: 'x', kind: 'providerGeneration', projectId: 'p', snapshotId: 's', targetPath: null,
      startedAtUnixMs: 1, providerRequest: { providerId: 'codex', purpose: 'repositoryPlan', selectedPaths: [], repositoryPlan: { question: prompt } } }, 'fontdone'),
      'fontdone · PFR0 branch in Font::memory_face (src/font.rs).');
    const source = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    assert.match(source, /questionNotificationText\(outcome\.question, 140\)/);
  });

  it('keeps current-snapshot and global notifications visible and collapses older scopes without losing actions', () => {
    const currentAction = () => 'current';
    const olderAction = () => 'older';
    const otherAction = () => 'other';
    const groups = groupNotifications([
      { id: 'current', projectId: 'fontdone', snapshotId: 'main', groupKey: 'current', bucket: 'attention', action: currentAction },
      { id: 'older', projectId: 'fontdone', snapshotId: 'old', groupKey: 'older', bucket: 'attention', action: olderAction },
      { id: 'other', projectId: 'pillow-rs', snapshotId: 'main', groupKey: 'other', bucket: 'attention', action: otherAction },
      { id: 'global', groupKey: 'global', bucket: 'attention' },
    ]);
    const split = partitionNotificationGroups(groups, 'fontdone', 'main');
    assert.deepEqual(split.current.map((group) => group.id), ['current', 'global']);
    assert.deepEqual(split.earlierSnapshots.map((group) => group.id), ['older']);
    assert.deepEqual(split.otherProjects.map((group) => group.id), ['other']);
    assert.equal(split.current[0].items[0].action(), 'current');
    assert.equal(split.earlierSnapshots[0].items[0].action(), 'older');
    assert.equal(split.otherProjects[0].items[0].action(), 'other');
    assert.deepEqual(partitionNotificationGroups(groups).earlierSnapshots, []);
    assert.deepEqual(partitionNotificationGroups(groups).current, groups);
  });

  it('groups repeated question outcomes without losing attempt identities or actions', () => {
    const latest = { id: 'turn-2', groupKey: notificationGroupKey('question', 'repo', 'snap', '  Explain   cart totals '), bucket: 'attention', countLabel: 'attempts', read: false, action: () => 'latest' };
    const earlier = { id: 'turn-1', groupKey: notificationGroupKey('question', 'repo', 'snap', 'explain cart totals'), bucket: 'updates', countLabel: 'attempts', read: true, action: () => 'earlier' };
    const otherSnapshot = { id: 'turn-3', groupKey: notificationGroupKey('question', 'repo', 'other-snap', 'explain cart totals'), bucket: 'updates', countLabel: 'attempts', read: true };
    const ungroupedA = { id: 'one-off-a' }, ungroupedB = { id: 'one-off-b' };
    const groups = groupNotifications([latest, earlier, otherSnapshot, ungroupedA, ungroupedB]);
    assert.equal(groups.length, 4);
    assert.equal(groups[0].bucket, 'attention');
    assert.equal(groups[0].unreadCount, 1);
    assert.deepEqual(groups[0].items, [latest, earlier]);
    assert.equal(groups[0].items[0].action(), 'latest');
    assert.equal(groups[0].items[1].action(), 'earlier');
    assert.deepEqual(groups.slice(1).map((group) => group.items.length), [1, 1, 1]);
    const resolved = groupNotifications([
      { id: 'latest-ready', groupKey: 'same-question', bucket: 'updates', groupBucket: 'updates' },
      { id: 'older-failed', groupKey: 'same-question', bucket: 'attention', groupBucket: 'updates' },
    ]);
    assert.equal(resolved[0].bucket, 'updates');
    assert.equal(groupNotifications([
      { id: 'recovery-1', groupKey: 'recovery', bucket: 'attention' },
      { id: 'recovery-2', groupKey: 'recovery', bucket: 'attention' },
    ])[0].unreadCount, 0);
  });

  it('puts unread successful answers in Recent updates and unresolved outcomes in Needs attention', () => {
    assert.equal(notificationBucketForOutcome('info'), 'updates');
    assert.equal(notificationBucketForOutcome('warning'), 'attention');
    assert.equal(notificationBucketForOutcome('error'), 'attention');
    const grouped = groupNotifications([
      { id: 'ready', groupKey: 'ready', bucket: notificationBucketForOutcome('info'), read: false },
      { id: 'failed', groupKey: 'failed', bucket: notificationBucketForOutcome('error'), read: false },
    ]);
    assert.deepEqual(grouped.map((group) => [group.id, group.bucket, group.unreadCount]), [
      ['ready', 'updates', 1], ['failed', 'attention', 1],
    ]);
  });

  it('groups interruption recovery by exact plan question or conservative source scope', () => {
    const operation = { operationId: 'a', kind: 'providerGeneration', projectId: 'repo', snapshotId: 'snap', targetPath: null,
      startedAtUnixMs: 1, providerRequest: { providerId: 'codex', purpose: 'repositoryPlan', selectedPaths: [],
        repositoryPlan: { question: 'How does loading work?' } } };
    const sameQuestion = { ...operation, operationId: 'b', providerRequest: { ...operation.providerRequest, repositoryPlan: { question: ' How does\nloading work? ' } } };
    const differentQuestion = { ...operation, operationId: 'c', providerRequest: { ...operation.providerRequest, repositoryPlan: { question: 'How does rendering work?' } } };
    assert.equal(interruptedNotificationGroupKey(operation), interruptedNotificationGroupKey(sameQuestion));
    assert.notEqual(interruptedNotificationGroupKey(operation), interruptedNotificationGroupKey(differentQuestion));
    assert.notEqual(interruptedNotificationGroupKey(operation), interruptedNotificationGroupKey({ ...operation, snapshotId: 'new-snapshot' }));
    const scoped = { ...operation, targetPath: 'src/font.rs', providerRequest: { ...operation.providerRequest, purpose: 'fileChapter', repositoryPlan: null } };
    assert.equal(interruptedNotificationGroupKey(scoped), interruptedNotificationGroupKey({ ...scoped, operationId: 'd' }));
    assert.notEqual(interruptedNotificationGroupKey(scoped), interruptedNotificationGroupKey({ ...scoped, targetPath: 'src/outline.rs' }));
  });

  it('shows recoveries for the selected repository first without changing records', () => {
    const other = { operationId: 'z', projectId: 'other', snapshotId: 'a', startedAtUnixMs: 30 };
    const selectedOlder = { operationId: 'b', projectId: 'selected', snapshotId: 'b', startedAtUnixMs: 10 };
    const selectedNewer = { operationId: 'a', projectId: 'selected', snapshotId: 'a', startedAtUnixMs: 20 };
    const operations = [other, selectedOlder, selectedNewer];
    const before = [...operations];
    assert.deepEqual(sortInterruptedOperations(operations, 'selected'), [selectedNewer, selectedOlder, other]);
    assert.deepEqual(operations, before);
    assert.deepEqual(sortInterruptedOperations(operations).map((operation) => operation.operationId), ['z', 'a', 'b']);
  });

  it('waits for each contributor and the selected workspace before reporting a total', () => {
    for (const partial of [{ startupComplete: false }, { workspaceReady: false }, { answers: 'loading' }, { refreshing: true }]) {
      const availability = notificationAvailability({ ...ready, ...partial });
      assert.equal(availability, 'loading');
      for (const items of [[], unread.slice(0, 23), unread]) {
        const summary = notificationSummary(items, availability);
        assert.equal(summary.label, 'Notifications, loading updates');
        assert.equal(summary.badge, null);
        assert.equal(summary.marker, null);
        assert.equal(summary.announcement, '');
        assert.equal(summary.status, 'Loading updates…');
      }
    }
    const availability = notificationAvailability(ready);
    assert.equal(availability, 'ready');
    assert.equal(notificationSummary(unread, availability).label, 'Notifications, 34 unread updates');
  });

  it('does not call a failed lookup complete or announce a partial unread total', () => {
    const summary = notificationSummary(unread.slice(0, 23), notificationAvailability({ ...ready, unavailable: true }));
    assert.equal(summary.label, 'Notifications, some updates unavailable');
    assert.equal(summary.badge, null);
    assert.equal(summary.marker, 'unavailable');
    assert.match(summary.status, /try again/);
    assert.doesNotMatch(summary.announcement, /23|unread|caught up/);
    assert.equal(notificationAvailability({ ...ready, unavailable: true, refreshing: true }), 'loading');
  });

  it('waits for native answers to restore after a snapshot opens, including scope switches', () => {
    const previous = { projectId: 'old', snapshotId: 'old', availability: 'ready' };
    for (const restored of [null, previous, { projectId: 'p', snapshotId: 'old', availability: 'ready' }, { projectId: 'p', snapshotId: 's', availability: 'loading' }]) {
      const answers = workspaceAnswerAvailability(true, 'p', 's', restored);
      assert.equal(answers, 'loading');
      assert.equal(notificationSummary(unread, notificationAvailability({ ...ready, answers })).announcement, '');
    }
    assert.equal(workspaceAnswerAvailability(true, 'p', 's', { projectId: 'p', snapshotId: 's', availability: 'ready' }), 'ready');
  });

  it('exposes answer-restore failure while leaving empty or nonnative workspaces usable', () => {
    const answers = workspaceAnswerAvailability(true, 'p', 's', { projectId: 'p', snapshotId: 's', availability: 'unavailable' });
    assert.equal(notificationAvailability({ ...ready, answers }), 'unavailable');
    assert.equal(workspaceAnswerAvailability(false, 'p', 's', null), 'ready');
    assert.equal(workspaceAnswerAvailability(true, undefined, undefined, null), 'ready');
  });

  it('counts only unread entries, handles singular/empty and bounds the visible badge', () => {
    assert.deepEqual(notificationSummary([], 'ready'), {
      label: 'Notifications, no unread updates', badge: null, marker: null, announcement: '', status: null,
    });
    const one = notificationSummary([{ read: true }, { read: false }], 'ready');
    assert.equal(one.label, 'Notifications, 1 unread update');
    assert.equal(one.badge, '1');
    assert.equal(one.announcement, '1 unread update available in Notifications.');
    assert.equal(notificationSummary(unread, 'ready').badge, '9+');
    assert.equal(notificationSummary([{ read: true }], 'ready').badge, null);
  });

  it('explains how to recover a full local notification history', () => {
    assert.deepEqual(notificationSummary([], 'unavailable', 'full'), {
      label: 'Notifications, history full', badge: null, marker: 'full',
      announcement: 'Notification history is full. Dismiss an older notification to make room.',
      status: 'New notifications are waiting to save. Dismiss an older notification to make room.',
    });
  });

  it('retains entry data and action ownership while count readiness changes', () => {
    const items = [{ read: false, title: 'Saved question', run: () => assert.fail('rendering never runs an action') }];
    const before = [...items];
    for (const availability of ['loading', 'unavailable', 'ready']) notificationSummary(items, availability);
    assert.deepEqual(items, before);
  });

  it('names the repository and useful saved scope without exposing journal or provider details', () => {
    const operation = { operationId: 'private-operation-id', projectId: 'private-project-id', snapshotId: 'private-snapshot-id',
      kind: 'providerGeneration', targetPath: null, providerRequest: { providerId: 'private-provider-id', purpose: 'repositoryOverview', selectedPaths: ['private/unrelated.rs'] } };
    assert.equal(interruptedNotificationMessage(operation, 'fontdone'), 'fontdone · Repository guide');
    assert.equal(interruptedNotificationMessage({ ...operation, targetPath: 'src/font.rs' }, 'fontdone'), 'fontdone · src/font.rs');
    assert.equal(interruptedNotificationMessage({ ...operation, providerRequest: { ...operation.providerRequest, repositoryPlan: { question: 'How do\nfont bytes become a bitmap?' } } }, 'fontdone'), 'fontdone · How do font bytes become a bitmap?');
    assert.equal(interruptedNotificationMessage({ ...operation, providerRequest: null }), 'Other repository · Unfinished explanation');
    assert.equal(interruptedNotificationMessage({ ...operation, providerRequest: { ...operation.providerRequest, purpose: 'unknown-future-purpose' } }), 'Other repository · Unfinished explanation');
    assert.match(interruptedNotificationMessage({ ...operation, kind: 'capture' }, 'fontdone'), /^fontdone · You can keep reading/);
    for (const purpose of ['fileChapter', 'sectionUpdate', 'codeLensExplanation', 'prGuide', 'prChangeExplanation', 'repositoryPlan']) {
      const message = interruptedNotificationMessage({ ...operation, providerRequest: { ...operation.providerRequest, purpose } }, 'fontdone');
      assert.match(message, /^fontdone · /); assert.doesNotMatch(message, /private-|undefined/);
    }
  });

  it('keeps saved-preview context concise while preserving the full underlying metadata', () => {
    const preview = { title: 'Glyph\nloading', targetPath: 'src/font.rs', generationId: 'private-generation', modelId: 'private-model' };
    assert.equal(previewNotificationMessage(preview, 'fontdone'), 'fontdone · Glyph loading · src/font.rs');
    assert.equal(previewNotificationMessage({ ...preview, targetPath: null }), 'Other repository · Glyph loading');
    const long = { ...preview, title: '🧭'.repeat(300) };
    const message = previewNotificationMessage(long, 'fontdone');
    assert.equal(Array.from(message.split(' · ')[1]).length, 220);
    assert.equal(message.split(' · ')[1].endsWith('…'), true);
    assert.equal(long.title.length, 600);
  });

  it('binds the real nonmodal bell to loading, partial, live and empty-state semantics', async () => {
    const component = await readFile(new URL('../src/NotificationCenter.tsx', import.meta.url), 'utf8');
    const app = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    assert.match(component, /notificationSummary\(items, availability, issue\)/);
    assert.match(app, /issue=\{noticeHistoryState\.issue==='full'\?'full':null\}/);
    assert.match(component, /aria-label=\{summary.label\} aria-busy=\{availability === 'loading'\}/);
    assert.match(component, /summary\.marker && <span className=\{`notification-status-marker status-\$\{summary\.marker\}`\}/);
    assert.match(component, /role="status" aria-live="polite" aria-atomic="true">\{summary.announcement\}/);
    assert.match(component, /groups.length === 0 \? availability === 'ready' &&/);
    assert.match(component, /renderSection\('Needs attention', attentionGroups\)/);
    assert.match(component, /renderSection\('Recent updates', updateGroups\)/);
    assert.match(component, /partitionNotificationGroups\(groups, currentProjectId, currentSnapshotId\)/);
    assert.match(component, /Earlier work in this repository/);
    assert.match(component, /Other repositories/);
    assert.match(component, /Show \{earlier.length\} earlier \{group.countLabel\}/);
    assert.match(component, /latest\.groupActions/);
    assert.match(app, /Dismiss all \$\{groupedOutcomes\.length\} attempts/);
    assert.match(app, /markQuestionOutcomes\(s,groupedOutcomes\.map/);
    assert.match(app, /Dismiss all \$\{groupedNotices\.length\} notices/);
    assert.match(app, /noticeHistory\.dismissMany\(groupedNotices\.map\(notice=>notice\.id\)\)/);
    assert.match(component, /modal=\{false\}/);
    assert.match(app, /availability=\{notificationAvailability\(\{ startupComplete: startupNotificationsComplete, workspaceReady: readiness === 'ready'/);
    assert.match(app, /bucket: notificationBucketForOutcome\(outcome\.tone\)/);
    assert.match(app, /groupBucket: notificationBucketForOutcome\(outcome\.tone\)/);
    assert.match(app, /bucket: notificationBucketForOutcome\(preview\.unavailableReason \? 'warning' : 'info'\)/);
    assert.match(app, /answers: answerAvailability/);
    assert.match(app, /onAnswerRestoration=\{setAnswerRestoration\}/);
    assert.match(app, /id: 'answers-unavailable'[\s\S]*?restoreWorkspaceAnswers\(projectId, snapshotId\)/);
    const workspace = await readFile(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
    assert.match(workspace, /watchWorkspaceAnswers\(projectId,snapshotId,\(availability\)=>\{\s+if\(!closed\)onAnswerRestoration\(\{projectId,snapshotId,availability\}\)/);
    assert.doesNotMatch(workspace, /if\(!closed\)onAnswerRestoration\(\{projectId,snapshotId,availability:'ready'\}\)/);
    assert.doesNotMatch(workspace, /setChapterError\(errorMessageOr\(failure,'Saved answers could not be restored\.'/);
    assert.match(workspace, /onAnswerRestoration\(\{projectId,snapshotId,availability:'unavailable'\}\)/);
    assert.match(app, /refreshing: interruptionRefreshing \|\| previewRefreshing, unavailable: interruptedOperationsUnavailable \|\| pendingChapterPreviewsUnavailable/);
    assert.match(app, /interruptionLookup.run\(listInterruptedOperations/);
    assert.match(app, /previewLookup.run\(listPendingGeneratedChapterPreviews/);
    assert.match(app, /if \(previewLookup.owns\(previewVersion\)\)/);
    assert.match(app, /if \(interruptionLookup.owns\(recoveryVersion\)\)/);
    assert.match(app, /interruptionLookup.invalidate\(\);\s+previewLookup.invalidate\(\);/);
    assert.match(app, /interruptedNotificationMessage\(operation, knownProjects\[operation.projectId\]\?\.displayName\)/);
    assert.match(app, /interruptedNotificationGroupKey\(operation\)/);
    assert.match(app, /sortInterruptedOperations\(interruptedOperations,currentProject\?\.projectId\)/);
    assert.match(app, /label: 'Discard recovery'/);
    const css = await readFile(new URL('../src/NotificationCenter.css', import.meta.url), 'utf8');
    assert.match(css, /\.notification-panel-header \{ position: sticky; top: 0;/);
  });
});
