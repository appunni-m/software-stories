import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { readFileSync } from 'node:fs';
import { GitComparisonController } from '../src/lib/gitComparisonController.ts';

function setup({ failCancel = false } = {}) {
  const calls = [], cancelled = [];
  let id = 0;
  const controller = new GitComparisonController({
    compare: (input) => new Promise((resolve, reject) => calls.push({ input, resolve, reject })),
    cancel: async (requestId) => { cancelled.push(requestId); if (failCancel) throw new Error('transport unavailable'); },
  }, () => `request-${++id}`);
  return { controller, calls, cancelled };
}
const ready = { status: 'ready', changes: [] };
const abort = (error) => error?.name === 'AbortError';

describe('owned local Git comparison client', () => {
  it('dispatches one identity, reports busy and releases ownership on success', async () => {
    const { controller, calls, cancelled } = setup();
    const states = [];
    const unsubscribe = controller.subscribe(() => states.push(controller.isBusy('repo')));
    const result = controller.compare('repo', 'base', 'head');
    assert.equal(controller.isBusy('repo'), true);
    assert.deepEqual(calls[0].input, { requestId: 'request-1', projectId: 'repo', baseRef: 'base', headRef: 'head' });
    calls[0].resolve(ready);
    assert.equal(await result, ready);
    assert.deepEqual(states, [true, false]);
    assert.deepEqual(cancelled, []);
    unsubscribe();
  });

  it('Stop releases busy immediately and suppresses a later native success', async () => {
    const { controller, calls, cancelled } = setup();
    const result = controller.compare('repo', 'base', 'head');
    const rejected = assert.rejects(result, abort);
    controller.cancel('repo'); controller.cancel('repo');
    assert.equal(controller.isBusy('repo'), false);
    assert.deepEqual(cancelled, ['request-1']);
    calls[0].resolve(ready);
    await rejected;
  });

  it('a superseded completion cannot publish or clear a newer request', async () => {
    const { controller, calls, cancelled } = setup();
    const first = controller.compare('repo', 'a', 'b');
    const rejected = assert.rejects(first, abort);
    const second = controller.compare('repo', 'a', 'c');
    assert.deepEqual(cancelled, ['request-1']);
    calls[0].resolve(ready); await rejected;
    assert.equal(controller.isBusy('repo'), true);
    const latest = { ...ready, comparisonId: 'latest' };
    calls[1].resolve(latest);
    assert.equal(await second, latest);
    assert.equal(controller.isBusy('repo'), false);
  });

  it('cancellation is confined to its repository', async () => {
    const { controller, calls, cancelled } = setup();
    const first = controller.compare('one', 'a', 'b');
    const rejected = assert.rejects(first, abort);
    const second = controller.compare('two', 'a', 'b');
    controller.cancel('one');
    assert.equal(controller.isBusy('two'), true);
    assert.deepEqual(cancelled, ['request-1']);
    calls[0].resolve(ready); calls[1].resolve(ready);
    await rejected; assert.equal(await second, ready);
  });

  it('a pre-aborted caller neither dispatches nor cancels existing work', async () => {
    const { controller, calls, cancelled } = setup();
    const first = controller.compare('repo', 'a', 'b');
    const external = new AbortController(); external.abort();
    await assert.rejects(controller.compare('repo', 'a', 'c', external.signal), abort);
    assert.equal(calls.length, 1); assert.deepEqual(cancelled, []);
    assert.equal(controller.isBusy('repo'), true);
    calls[0].resolve(ready); await first;
  });

  it('Stop during the busy notification prevents native dispatch', async () => {
    const { controller, calls, cancelled } = setup();
    controller.subscribe(() => { if (controller.isBusy('repo')) controller.cancel('repo'); });
    const result = controller.compare('repo', 'a', 'b');
    const rejected = assert.rejects(result, abort);
    if (calls[0]) calls[0].resolve(ready);
    await rejected;
    assert.equal(calls.length, 0);
    assert.deepEqual(cancelled, []);
    assert.equal(controller.isBusy('repo'), false);
  });

  it('external abort survives a failed cancel RPC and a late transport error', async () => {
    const { controller, calls, cancelled } = setup({ failCancel: true });
    const external = new AbortController();
    const result = controller.compare('repo', 'a', 'b', external.signal);
    const rejected = assert.rejects(result, abort);
    external.abort();
    assert.equal(controller.isBusy('repo'), false);
    calls[0].reject(new Error('late native failure'));
    await rejected;
    assert.deepEqual(cancelled, ['request-1']);
  });

  it('an ordinary failure releases ownership and never retries automatically', async () => {
    const { controller, calls, cancelled } = setup();
    const result = controller.compare('repo', 'a', 'b');
    const failure = new Error('native failure');
    const rejected = assert.rejects(result, (error) => error === failure);
    calls[0].reject(failure); await rejected;
    assert.equal(controller.isBusy('repo'), false);
    assert.equal(calls.length, 1); assert.deepEqual(cancelled, []);
  });

  it('the app cancels departed project work and renders comparison state for the selected project', () => {
    const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
    const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
    assert.match(app, /return \(\) => \{ if \(projectId\) cancelLocalGitComparison\(projectId\); \};\s*\}, \[currentProject\?\.projectId\]\);/);
    assert.match(app, /comparison=\{currentProject \? gitComparisonsByProject\[currentProject\.projectId\] \?\? null : null\}/);
    assert.match(app, /\[projectId\]: \{ baseRef, headRef, result \}/);
    assert.match(workspace, /return\(\)=>\{active=false;controller\.abort\(\);if\(gitRequest\.current===controller\)gitRequest\.current=null;\};\s*\},\[projectId,showChanges,gitReload\]\);/);
  });
});
