import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readGitHistory } from '../src/lib/gitHistory.ts';

const stopped = (error) => error?.name === 'AbortError';
function fixture({ cancelFails = false } = {}) {
  const calls = [], cancellations = [];
  let id = 0;
  const transport = {
    read: (input) => new Promise((resolve, reject) => calls.push({ input, resolve, reject })),
    cancel: async (request) => { cancellations.push(request); if (cancelFails) throw new Error('cancel unavailable'); },
  };
  return { calls, cancellations, load: (project, signal) => readGitHistory(transport, project, signal, () => `history-${++id}`) };
}

test('history owns its request and removes the cancellation listener after success', async () => {
  const f = fixture(), abort = new AbortController();
  const pending = f.load('one', abort.signal);
  assert.deepEqual(f.calls[0].input, { projectId: 'one', requestId: 'history-1' });
  const commits = [{ oid: 'a'.repeat(40), parents: [], subject: 'Synthetic commit', author: 'Fixture', timestamp: '' }];
  f.calls[0].resolve(commits);
  assert.equal(await pending, commits);
  abort.abort();
  assert.deepEqual(f.cancellations, []);
});

test('pre-cancelled history never dispatches or allocates a request identity', async () => {
  const f = fixture(), abort = new AbortController();abort.abort();
  await assert.rejects(f.load('one', abort.signal), stopped);
  assert.deepEqual(f.calls, []);
  assert.deepEqual(f.cancellations, []);
});

test('leaving a history view cancels only its own request and suppresses late success', async () => {
  const f = fixture(), first = new AbortController();
  const old = f.load('one', first.signal), rejected = assert.rejects(old, stopped);
  const other = f.load('two');
  first.abort();first.abort();
  assert.deepEqual(f.cancellations, ['history-1']);
  f.calls[0].resolve([{ oid: 'old' }]);
  await rejected;
  const current = [{ oid: 'current' }];f.calls[1].resolve(current);
  assert.equal(await other, current);
});

test('a cancelled refresh cannot replace a newer refresh for the same repository', async () => {
  const f = fixture(), abort = new AbortController();
  const old = f.load('one', abort.signal), rejected = assert.rejects(old, stopped);
  abort.abort();
  const fresh = f.load('one');
  f.calls[1].resolve([]);assert.deepEqual(await fresh, []);
  f.calls[0].resolve([{ oid: 'late' }]);await rejected;
  assert.deepEqual(f.cancellations, ['history-1']);
});

test('failed cancellation transport still suppresses the old native result', async () => {
  const f = fixture({ cancelFails: true }), abort = new AbortController();
  const pending = f.load('one', abort.signal), rejected = assert.rejects(pending, stopped);
  abort.abort();await Promise.resolve();
  f.calls[0].reject(new Error('native failure after cancellation'));
  await rejected;
});

test('native errors propagate unchanged and release the abort listener', async () => {
  const f = fixture(), abort = new AbortController();
  const pending = f.load('one', abort.signal), error = new Error('Git history took too long. Try again.');
  const rejected = assert.rejects(pending, (value) => value === error);
  f.calls[0].reject(error);await rejected;
  abort.abort();assert.deepEqual(f.cancellations, []);
});

test('the Walkthrough ignores late history success, failure, and cleanup from a departed request', () => {
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.match(workspace, /listWorkspaceCommits\(projectId,controller\.signal\)\.then\(\(value\)=>\{if\(active\)setCommits\(value\);\}\)\.catch\(\(failure\)=>\{\s*if\(active&&!controller\.signal\.aborted\)setGitError/);
  assert.match(workspace, /finally\(\(\)=>\{if\(active\)setGitBusy\(false\);if\(gitRequest\.current===controller\)gitRequest\.current=null;\}\);\s*return\(\)=>\{active=false;controller\.abort\(\);if\(gitRequest\.current===controller\)gitRequest\.current=null;\};\s*\},\[projectId,showChanges,gitReload\]\);/);
});
