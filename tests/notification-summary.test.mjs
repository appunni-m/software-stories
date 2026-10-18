import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { interruptedNotificationMessage, notificationAvailability, notificationSummary, previewNotificationMessage, workspaceAnswerAvailability } from '../src/lib/notificationSummary.ts';

const ready = { startupComplete: true, workspaceReady: true, answers: 'ready', refreshing: false, unavailable: false };
const unread = Array.from({ length: 34 }, () => ({}));

describe('notification totals', () => {
  it('waits for each contributor and the selected workspace before reporting a total', () => {
    for (const partial of [{ startupComplete: false }, { workspaceReady: false }, { answers: 'loading' }, { refreshing: true }]) {
      const availability = notificationAvailability({ ...ready, ...partial });
      assert.equal(availability, 'loading');
      for (const items of [[], unread.slice(0, 23), unread]) {
        const summary = notificationSummary(items, availability);
        assert.equal(summary.label, 'Notifications, loading updates');
        assert.equal(summary.badge, null);
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
      label: 'Notifications, no unread updates', badge: null, announcement: '', status: null,
    });
    const one = notificationSummary([{ read: true }, { read: false }], 'ready');
    assert.equal(one.label, 'Notifications, 1 unread update');
    assert.equal(one.badge, '1');
    assert.equal(one.announcement, '1 unread update available in Notifications.');
    assert.equal(notificationSummary(unread, 'ready').badge, '9+');
    assert.equal(notificationSummary([{ read: true }], 'ready').badge, null);
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
    assert.match(component, /notificationSummary\(items, availability\)/);
    assert.match(component, /aria-label=\{summary.label\} aria-busy=\{availability === 'loading'\}/);
    assert.match(component, /role="status" aria-live="polite" aria-atomic="true">\{summary.announcement\}/);
    assert.match(component, /items.length === 0 \? availability === 'ready' &&/);
    assert.match(component, /modal=\{false\}/);
    assert.match(app, /availability=\{notificationAvailability\(\{ startupComplete: startupNotificationsComplete, workspaceReady: readiness === 'ready'/);
    assert.match(app, /answers: answerAvailability/);
    assert.match(app, /onAnswerRestoration=\{setAnswerRestoration\}/);
    assert.match(app, /id: 'answers-unavailable'[\s\S]*?setWorkspaceReload/);
    const workspace = await readFile(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
    assert.match(workspace, /await restoreWorkspaceAnswers\(projectId,snapshotId\);\s+if\(!closed\)onAnswerRestoration\(\{projectId,snapshotId,availability:'ready'\}\)/);
    assert.match(workspace, /onAnswerRestoration\(\{projectId,snapshotId,availability:'unavailable'\}\)/);
    assert.match(app, /refreshing: interruptionRefreshing \|\| previewRefreshing, unavailable: interruptedOperationsUnavailable \|\| pendingChapterPreviewsUnavailable/);
    assert.match(app, /interruptionLookup.run\(listInterruptedOperations/);
    assert.match(app, /previewLookup.run\(listPendingGeneratedChapterPreviews/);
    assert.match(app, /if \(previewLookup.owns\(previewVersion\)\)/);
    assert.match(app, /if \(interruptionLookup.owns\(recoveryVersion\)\)/);
    assert.match(app, /interruptionLookup.invalidate\(\);\s+previewLookup.invalidate\(\);/);
    assert.match(app, /interruptedNotificationMessage\(operation, knownProjects\[operation.projectId\]\?\.displayName\)/);
    const css = await readFile(new URL('../src/NotificationCenter.css', import.meta.url), 'utf8');
    assert.match(css, /\.notification-panel-header \{ position: sticky; top: 0;/);
  });
});
