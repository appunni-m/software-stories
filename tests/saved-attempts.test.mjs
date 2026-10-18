import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { SavedAttempts } from '../src/lib/savedAttempts.ts';

const entry = (id = 'a', reviewToken = 'token-a') => ({ id, reviewToken, projectId: 'repository', snapshotId: 'snapshot', startedAtUnixMs: 1, bytes: 100, candidateCount: 1, available: true });
const inventory = (entries = [entry()], extra = {}) => ({ entries, operationLimit: 8, limited: false, busy: false, ...extra });
const review = item => ({ entry: item, attempts: [{ ordinal: 1, responseText: '<script>untrusted()</script>', rejection: null, findings: [] }] });
function deferred() { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
function fixture(overrides = {}) {
  const calls = [];
  const controller = new SavedAttempts({ list: async () => { calls.push('list'); return inventory(); },
    read: async (id, token) => { calls.push(['read', id, token]); return review(entry(id, token)); },
    trash: async (id, token) => { calls.push(['trash', id, token]); }, ...overrides });
  return { controller, calls };
}

describe('saved attempt recovery', () => {
  it('does no work until opened and reviews only the selected exact token', async () => {
    const { controller, calls } = fixture();
    assert.deepEqual(calls, []);
    await controller.refresh();
    assert.deepEqual(calls, ['list']);
    await controller.select(entry());
    assert.deepEqual(calls, ['list', ['read', 'a', 'token-a']]);
    assert.equal(controller.getSnapshot().review.attempts[0].responseText, '<script>untrusted()</script>');
    assert.equal(controller.getSnapshot().reviewing, false);
  });
  it('ignores an older list result or failure while the newest lookup owns loading', async () => {
    for (const fail of [false, true]) {
      const old = deferred(), next = deferred(); let calls = 0;
      const { controller } = fixture({ list: () => ++calls === 1 ? old.promise : next.promise });
      const first = controller.refresh(), second = controller.refresh();
      if (fail) old.reject(new Error('private native detail')); else old.resolve(inventory([entry('old')]));
      await first;
      assert.equal(controller.getSnapshot().loading, true); assert.equal(controller.getSnapshot().error, null);
      next.resolve(inventory([entry('new')])); await second;
      assert.equal(controller.getSnapshot().inventory.entries[0].id, 'new');
    }
  });
  it('keeps old review callbacks from replacing a newer selection or refreshed list', async () => {
    for (const refresh of [false, true]) {
      const old = deferred();
      const { controller } = fixture({ list: async () => inventory([entry(), entry('b', 'token-b')]),
        read: (id, token) => id === 'a' ? old.promise : Promise.resolve(review(entry(id, token))) });
      await controller.refresh(); const first = controller.select(entry());
      if (refresh) await controller.refresh(); else await controller.select(entry('b', 'token-b'));
      old.resolve(review(entry())); await first;
      assert.equal(controller.getSnapshot().selectedId, refresh ? null : 'b');
      assert.equal(controller.getSnapshot().review?.entry.id ?? null, refresh ? null : 'b');
    }
  });
  it('refuses stale/unknown selections and removal while native generation is busy', async () => {
    const { controller, calls } = fixture({ list: async () => inventory([entry()], { busy: true }) });
    await controller.refresh(); await controller.select(entry('a', 'old')); await controller.select(entry('unknown'));
    assert.equal(controller.canRemove(entry()), false);
    await controller.remove(entry()); assert.deepEqual(calls, []);
  });
  it('requires explicit refresh after review identity mismatch without exposing native errors', async () => {
    const { controller, calls } = fixture({ read: async () => review(entry('a', 'changed')) });
    await controller.refresh(); await controller.select(entry());
    assert.equal(controller.getSnapshot().review, null); assert.equal(controller.getSnapshot().requiresRefresh, true);
    await controller.remove(entry()); assert.deepEqual(calls, ['list']);
    await controller.refresh(); assert.equal(controller.canRemove(entry()), true);
  });
  it('moves only after explicit removal, refreshes once and retains no removed review', async () => {
    const calls = []; let listed = 0;
    const { controller } = fixture({ list: async () => { calls.push('list'); return inventory(listed++ ? [] : [entry()]); },
      trash: async (id, token) => { calls.push(['trash', id, token]); } });
    await controller.refresh(); await controller.select(entry());
    assert.deepEqual(calls, ['list']); await controller.remove(entry());
    assert.deepEqual(calls, ['list', ['trash', 'a', 'token-a'], 'list']);
    assert.equal(controller.getSnapshot().review, null); assert.equal(controller.getSnapshot().selectedId, null);
    assert.equal(controller.getSnapshot().inventory.entries.length, 0); assert.match(controller.getSnapshot().message, /Moved to Trash/);
  });
  it('keeps entries after a failed move and never automatically retries it', async () => {
    let moved = 0;
    const { controller, calls } = fixture({ trash: async () => { moved++; throw new Error('/private/account/secret'); } });
    await controller.refresh(); await controller.remove(entry());
    assert.equal(moved, 1); assert.equal(controller.getSnapshot().inventory.entries.length, 1);
    assert.equal(controller.getSnapshot().requiresRefresh, true); assert.doesNotMatch(controller.getSnapshot().error, /secret/);
    await controller.remove(entry()); assert.equal(moved, 1); assert.deepEqual(calls, ['list']);
    await controller.refresh(); assert.equal(controller.canRemove(entry()), true); assert.equal(moved, 1);
  });
  it('reports success separately if the follow-up list fails and never shows stale removed entries', async () => {
    let lists = 0;
    const { controller } = fixture({ list: async () => { if (lists++) throw new Error('offline'); return inventory(); } });
    await controller.refresh(); await controller.remove(entry());
    assert.equal(controller.getSnapshot().inventory, null); assert.match(controller.getSnapshot().message, /Moved to Trash/);
    assert.equal(controller.getSnapshot().requiresRefresh, true); assert.equal(controller.getSnapshot().removing, false);
  });
  it('permits explicit removal of damaged regular output without trying to read it as an answer', async () => {
    const damaged = { ...entry(), available: false, candidateCount: 0 };
    const { controller, calls } = fixture({ list: async () => inventory([damaged]) });
    await controller.refresh(); await controller.select(damaged);
    assert.equal(controller.getSnapshot().review, null); assert.equal(controller.canRemove(damaged), true);
    await controller.remove(damaged); assert.deepEqual(calls, [['trash', 'a', 'token-a']]);
    const noToken = { ...damaged, reviewToken: null }; assert.equal(controller.canRemove(noToken), false);
  });
  it('serializes removal through refresh, blocks repeat clicks and waits at app close', async () => {
    const moving = deferred(); let moves = 0, lists = 0;
    const { controller } = fixture({ list: async () => { lists++; return inventory(); }, trash: () => { moves++; return moving.promise; } });
    await controller.refresh(); const run = controller.remove(entry());
    await controller.remove(entry()); await controller.refresh();
    const unfreeze = controller.beginClose(); let flushed = false;
    const flush = controller.flush().then(() => { flushed = true; });
    await Promise.resolve(); assert.equal(flushed, false); assert.equal(moves, 1); assert.equal(lists, 1);
    moving.resolve(); await run; await flush; assert.equal(flushed, true); assert.equal(lists, 2);
    await controller.refresh(); assert.equal(lists, 2); unfreeze(); await controller.refresh(); assert.equal(lists, 3);
  });
  it('ignores late reads and move completion after unmount without starting a follow-up lookup', async () => {
    for (const operation of ['read', 'move', 'list']) {
      const pending = deferred(); let lists = 0;
      const { controller } = fixture({ list: () => { lists++; return operation === 'list' ? pending.promise : Promise.resolve(inventory()); },
        read: () => pending.promise, trash: () => pending.promise });
      let run;
      if (operation === 'list') run = controller.refresh();
      else { await controller.refresh(); run = operation === 'read' ? controller.select(entry()) : controller.remove(entry()); }
      const before = controller.getSnapshot(); controller.dispose();
      pending.resolve(operation === 'list' ? inventory() : review(entry())); await run;
      assert.equal(controller.getSnapshot(), before); assert.equal(lists, 1);
      await controller.refresh(); await controller.remove(entry()); assert.equal(lists, 1);
    }
  });
  it('uses standard dialogs, plain text and Notifications without a model execution path', async () => {
    const ui = await readFile(new URL('../src/SavedAttemptsDialog.tsx', import.meta.url), 'utf8');
    const notifications = await readFile(new URL('../src/NotificationCenter.tsx', import.meta.url), 'utf8');
    const native = await readFile(new URL('../src-tauri/src/rejected_answers.rs', import.meta.url), 'utf8');
    assert.match(ui, /AlertDialog\.Title>Move this attempt to Trash/);
    assert.match(ui, /<pre className="saved-attempt-text">\{expanded \? text/);
    assert.match(ui, /appCloseCoordinator\.register/); assert.match(ui, /data-focus-key="notifications"/);
    assert.match(notifications, /onReviewSavedAttempts\(\)/);
    assert.doesNotMatch(ui, /dangerouslySetInnerHTML|iframe|innerHTML|eval\(/);
    assert.doesNotMatch(native, /\.start\(|\.status\(|thread\/|turn\/|save_retrieval_grant|reset/);
  });
});
