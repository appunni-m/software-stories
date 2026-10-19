import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRefreshQueue } from '../src/workspace/refreshQueue.ts';
import { savedAnswerAvailability } from '../src/workspace/outcomes.ts';

const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};

test('chapter invalidations share one serial refresh and cannot publish a stale list', { timeout: 2000 }, async () => {
  const reads = [deferred(), deferred()];
  const started = [deferred(), deferred()];
  const values = []; let calls = 0; let idle = 0;
  const queue = createRefreshQueue({
    load: () => { const i = calls++; started[i].resolve(); return reads[i].promise; },
    onValue: (value) => values.push(value), onError: assert.fail, onIdle: () => idle++,
  });
  const first = queue.refresh(); await started[0].promise;
  for (let i = 0; i < 40; i++) assert.equal(queue.refresh(), first);
  assert.equal(calls, 1);
  reads[0].resolve(['older-revision']); await started[1].promise;
  assert.deepEqual(values, []);
  assert.equal(calls, 2);
  reads[1].resolve(['older-revision', 'new-scene']); await first;
  assert.deepEqual(values, [['older-revision', 'new-scene']]);
  assert.equal(idle, 1);
});

test('a superseded read failure cannot replace a later successful chapter refresh', { timeout: 2000 }, async () => {
  const reads = [deferred(), deferred()]; const started = [deferred(), deferred()];
  const errors = []; const values = []; let calls = 0;
  const queue = createRefreshQueue({
    load: () => { const i = calls++; started[i].resolve(); return reads[i].promise; },
    onValue: (value) => values.push(value), onError: (error) => errors.push(error),
  });
  const work = queue.refresh(); await started[0].promise; queue.refresh();
  reads[0].reject(new Error('superseded')); await started[1].promise;
  reads[1].resolve('latest'); await work;
  assert.deepEqual(values, ['latest']); assert.deepEqual(errors, []); assert.equal(calls, 2);
});

test('a genuine failure waits for an explicit invalidation instead of retrying', async () => {
  const failure = new Error('local read failed'); const errors = []; const values = []; let calls = 0;
  const queue = createRefreshQueue({
    load: async () => { if (++calls === 1) throw failure; return 'saved'; },
    onValue: (value) => values.push(value), onError: (error) => errors.push(error),
  });
  await queue.refresh(); assert.equal(calls, 1); assert.deepEqual(errors, [failure]);
  await queue.refresh(); assert.equal(calls, 2); assert.deepEqual(values, ['saved']);
});

test('leaving a workspace drops queued reads and every late UI callback', { timeout: 2000 }, async () => {
  const read = deferred(); const started = deferred(); let calls = 0; const callbacks = [];
  const queue = createRefreshQueue({
    load: () => { calls++; started.resolve(); return read.promise; },
    onValue: () => callbacks.push('value'), onError: () => callbacks.push('error'), onIdle: () => callbacks.push('idle'),
  });
  const work = queue.refresh(); await started.promise; queue.refresh(); queue.dispose();
  read.resolve('old-project'); await work; await queue.refresh();
  assert.equal(calls, 1); assert.deepEqual(callbacks, []);
});

test('a reentrant invalidation cannot start a parallel local read', async () => {
  let queue; let calls = 0; const values = [];
  queue = createRefreshQueue({
    load: async () => { if (++calls === 1) queue.refresh(); return calls; },
    onValue: (value) => values.push(value), onError: assert.fail,
  });
  await queue.refresh(); assert.equal(calls, 2); assert.deepEqual(values, [2]);
});

test('an invalidation at the completion boundary is not lost', async () => {
  let queue; let calls = 0; let idle = 0; const values = [];
  queue = createRefreshQueue({
    load: async () => ++calls,
    onValue: (value) => { values.push(value); if (value === 1) queueMicrotask(() => queue.refresh()); },
    onError: assert.fail, onIdle: () => idle++,
  });
  await queue.refresh();
  assert.equal(calls, 2); assert.deepEqual(values, [1, 2]); assert.equal(idle, 1);
});

test('the production workspace owns and disposes the tested chapter refresh queue', () => {
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.match(source, /createChapterHistory\(\{/);
  assert.match(source, /load:\(\)=>listChapterRevisions\(projectId,snapshotId\)/);
  assert.match(source, /chapterRefresh\.current=queue/);
  assert.match(source, /queue\.dispose\(\);if\(chapterRefresh\.current===queue\)chapterRefresh\.current=null/);
});

test('the Guide renders saved chapter receipts as soon as a read succeeds or a native write is acknowledged', () => {
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  const store = readFileSync(new URL('../src/workspace/store.ts', import.meta.url), 'utf8');
  assert.match(workspace, /onLoaded:\(\)=>setChaptersLoading\(false\)/);
  assert.match(workspace, /queue\.record\(revision\)/);
  assert.match(workspace, /revision\.snapshotId===snapshotId/);
  assert.match(store, /detail:\{projectId:project,revision\}/);
});

test('progressive cards stop loading when a completed history read cannot find their saved revision', () => {
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.equal(savedAnswerAvailability(true, false), 'loading');
  assert.equal(savedAnswerAvailability(false, true), 'available');
  assert.equal(savedAnswerAvailability(false, false), 'unavailable');
  assert.match(workspace, /loading=\{props\.loading\} revision=\{chapters\?\.find/);
  assert.doesNotMatch(workspace, /loading=\{props\.loading\|\|!chapters\?\.some/);
});

test('an unavailable answer stays a compact status beside its recovery action', () => {
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
  assert.match(workspace, /const unavailable=!revision&&availability==='unavailable'/);
  assert.match(workspace, /className=\{`dw-answer-card\$\{unavailable\?' dw-answer-card-unavailable':''\}`\}/);
  assert.match(workspace, /<p className="dw-unavailable-status" role="status">Saved explanation unavailable\.<\/p>/);
  assert.match(workspace, /\{!hideQuestion&&<QuestionDisclosure question=\{turn\.question\}\/>\}/);
  assert.match(workspace, /<Button size="1" variant="soft" onClick=\{onRetry\}>Explain again<\/Button>/);
  assert.match(styles, /\.dw-answer-card-unavailable\s*\{display:flex;align-items:center/);
  assert.match(styles, /\.dw-user-question summary \{display:flex;align-items:center;gap:7px;width:max-content/);
});

test('long answer questions stay collapsed until the reader asks to see them', () => {
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
  assert.equal((workspace.match(/<QuestionDisclosure question=\{turn\.question\}\/>/g) ?? []).length, 2);
  assert.match(workspace, /function QuestionDisclosure\(\{question\}:\{question:string\}\)[\s\S]*?<details className="dw-user-question"><summary><MessageSquare size=\{14\}\/><span>Question<\/span><ChevronRight size=\{13\}\/><\/summary><p>\{question\}<\/p><\/details>/);
  assert.match(styles, /\.dw-user-question summary \{display:flex;align-items:center;gap:7px;width:max-content/);
  assert.match(styles, /\.dw-user-question > p \{max-width:900px;margin:7px 0 0/);
});

test('saved chapter arrivals do not reload the immutable snapshot index or reset the reader', () => {
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  const effect = source.split('\n').find((line) => line.includes('ensureWorkspaceIndex(projectId,snapshotId)'));
  assert.ok(effect);
  assert.match(effect, /\[projectId,snapshotId\]\);$/);
  assert.doesNotMatch(effect, /chapters\.length/);
  assert.match(source, /const refreshChapters = \(\) => chapterRefresh\.current\?\.refresh\(\)/);
});
