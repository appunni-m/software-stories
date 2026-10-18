import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { LatestLocalRead } from '../src/lib/latestLocalRead.ts';

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function observer() {
  const events = [];
  return { events, callbacks: {
    value: (value) => events.push(['value', value]), error: (error) => events.push(['error', error.message]),
    pending: (pending) => events.push(['pending', pending]),
  } };
}

describe('owned local notification lookups', () => {
  it('publishes one result and releases its pending state', async () => {
    const owner = new LatestLocalRead(), result = deferred(), view = observer();
    const run = owner.run(() => result.promise, view.callbacks);
    assert.deepEqual(view.events, [['pending', true]]);
    result.resolve(['saved']); await run;
    assert.deepEqual(view.events, [['pending', true], ['value', ['saved']], ['pending', false]]);
  });

  it('keeps an older result from overwriting a newer result', async () => {
    const owner = new LatestLocalRead(), old = deferred(), recent = deferred(), view = observer();
    const first = owner.run(() => old.promise, view.callbacks);
    const second = owner.run(() => recent.promise, view.callbacks);
    recent.resolve(['new']); await second;
    old.resolve(['old']); await first;
    assert.deepEqual(view.events, [['pending', true], ['pending', true], ['value', ['new']], ['pending', false]]);
  });

  it('does not let a stale error or idle callback clear the latest loading state', async () => {
    const owner = new LatestLocalRead(), old = deferred(), recent = deferred(), view = observer();
    const first = owner.run(() => old.promise, view.callbacks);
    const second = owner.run(() => recent.promise, view.callbacks);
    old.reject(new Error('stale failure')); await first;
    assert.deepEqual(view.events, [['pending', true], ['pending', true]]);
    recent.resolve([]); await second;
    assert.deepEqual(view.events.at(-1), ['pending', false]);
    assert.equal(view.events.some(([kind]) => kind === 'error'), false);
  });

  it('invalidates startup snapshots when a newer refresh begins', async () => {
    const owner = new LatestLocalRead(), initial = owner.capture(), result = deferred(), view = observer();
    assert.equal(owner.owns(initial), true);
    const run = owner.run(() => result.promise, view.callbacks);
    assert.equal(owner.owns(initial), false);
    result.resolve([]); await run;
    assert.equal(owner.owns(initial), false, 'late startup data stays stale after the newer read finishes');
  });

  it('ignores late callbacks after unmount or an explicit local discard', async () => {
    for (const reject of [false, true]) {
      const owner = new LatestLocalRead(), result = deferred(), view = observer();
      const run = owner.run(() => result.promise, view.callbacks);
      owner.invalidate();
      if (reject) result.reject(new Error('late failure')); else result.resolve(['discarded']);
      await run;
      assert.deepEqual(view.events, [['pending', true]]);
      const next = observer(); await owner.run(async () => [], next.callbacks);
      assert.deepEqual(next.events, [['pending', true], ['value', []], ['pending', false]]);
    }
  });

  it('reports a current failure once without retry or loss of previously displayed entries', async () => {
    const owner = new LatestLocalRead(), view = observer(); let calls = 0;
    await owner.run(async () => { calls++; throw new Error('lookup unavailable'); }, view.callbacks);
    assert.equal(calls, 1);
    assert.deepEqual(view.events, [['pending', true], ['error', 'lookup unavailable'], ['pending', false]]);
  });

  it('keeps independent recovery and preview owners independent', async () => {
    const recovery = new LatestLocalRead(), preview = new LatestLocalRead();
    const recoveryVersion = recovery.capture(), previewVersion = preview.capture();
    recovery.invalidate();
    assert.equal(recovery.owns(recoveryVersion), false);
    assert.equal(preview.owns(previewVersion), true);
  });
});
