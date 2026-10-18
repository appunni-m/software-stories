import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, flushLocalReads, workspaceStoreHarness } from './helpers/workspace-store-harness.mjs';

const scope = { projectId: 'project', snapshotId: 'snapshot', runId: '11111111-1111-4111-8111-111111111111' };
const section = { runId: scope.runId, key: 'entry', order: 1, generationId: 'generation:22222222-2222-4222-8222-222222222222', revisionId: 'revision', supersedes: null, contextElapsedMs: 10, runElapsedMs: 20 };
const answer = { ...scope, question: 'What happens to the input?', outcome: 'complete', sections: [section], incompleteTopics: [] };
function nativeReads(handler) {
  return (name, args) => {
    if (name === 'list_repository_guides') return Promise.resolve([]);
    if (name === 'save_workspace_answer_section') return Promise.resolve({ projectId: args.projectId, snapshotId: args.snapshotId, revisionId: args.generationId === section.generationId ? 'revision' : 'revision-two' });
    return handler(name, args);
  };
}
const turns = (h, project = 'project', snapshot = 'snapshot') => h.store.getWorkspace(project, snapshot).explorations.flatMap((e) => e.turns);

test('a section arriving during initial restoration is read before that restoration finishes', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { if (++lists === 1) { started.resolve(); return listed.promise; } return [answer]; }
    if (name === 'load_workspace_answer') return answer;
    assert.fail(`Unexpected command ${name}`);
  }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot');
  await started.promise;
  h.emit('workspace-answer-section', scope);
  await flushLocalReads();
  listed.resolve([]);
  await initial; await flushLocalReads();
  assert.equal(lists, 2);
  assert.equal(turns(h).length, 1);
  assert.equal(turns(h)[0].status, 'ready');
  assert.equal(turns(h)[0].publishedSections[0].revisionId, 'revision');
  assert.equal(h.calls.filter((c) => c.name === 'save_workspace_answer_section').length, 1);
  stop(); assert.equal(h.listenerCount(), 0);
});

test('a burst of scoped section events requires one fresh serial read', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0; let active = 0; let maxActive = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') {
      lists++; maxActive = Math.max(maxActive, ++active);
      if (lists === 1) { started.resolve(); const result = await listed.promise; active--; return result; }
      active--; return [answer];
    }
    if (name === 'load_workspace_answer') return answer;
    assert.fail(`Unexpected command ${name}`);
  }));
  const availability = [];
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => availability.push(state));
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  for (let i = 0; i < 40; i++) h.emit('workspace-answer-section', scope);
  await flushLocalReads(); listed.resolve([]);
  await initial; await flushLocalReads();
  assert.equal(lists, 2); assert.equal(maxActive, 1);
  assert.deepEqual(availability, ['loading', 'ready']);
  assert.equal(turns(h)[0].publishedSections.length, 1); stop();
});

test('finished events during section adoption preserve both sections and the reader selection', { timeout: 2000 }, async () => {
  const saving = deferred(); const started = deferred(); let saves = 0; let latest = false;
  const second = { ...section, key: 'next', order: 2, generationId: 'generation:33333333-3333-4333-8333-333333333333', revisionId: 'revision-two' };
  const partial = { ...answer, outcome: 'working' };
  const complete = { ...answer, sections: [section, second] };
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'list_repository_guides') return [];
    if (name === 'list_workspace_answers') return [latest ? complete : partial];
    if (name === 'load_workspace_answer') return latest ? complete : partial;
    if (name === 'save_workspace_answer_section') {
      if (++saves === 1) { started.resolve(); await saving.promise; }
      return { projectId: args.projectId, snapshotId: args.snapshotId, revisionId: args.generationId === section.generationId ? 'revision' : 'revision-two' };
    }
    assert.fail(`Unexpected command ${name}`);
  });
  h.store.updateWorkspace('project', 'snapshot', (s) => ({ ...s, file: 'src/main.rs', explorations: s.explorations.map((e) => ({ ...e, view: 'flow', turns: [{ id: 'question', question: answer.question, context: [], status: 'interrupted', runIds: [scope.runId], reading: { flow: { revisionId: 'revision', sectionId: 'second-scene' } } }] })) }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  latest = true; h.emit('workspace-answer-finished', scope); await flushLocalReads(); saving.resolve();
  await initial; await flushLocalReads();
  const turn = turns(h)[0];
  assert.equal(turn.status, 'ready'); assert.equal(turn.publishedSections.length, 2);
  assert.equal(turn.reading.flow.sectionId, 'second-scene');
  assert.equal(h.store.getWorkspace('project', 'snapshot').explorations[0].view, 'flow');
  assert.equal(h.store.getWorkspace('project', 'snapshot').file, 'src/main.rs'); stop();
});

test('a native invalidation supersedes a failing initial read without replaying inference', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { if (++lists === 1) { started.resolve(); return listed.promise; } return [answer]; }
    if (name === 'load_workspace_answer') return answer;
    assert.fail(`Unexpected command ${name}`);
  }));
  const availability = [];
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => availability.push(state));
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  h.emit('workspace-answer-finished', scope); await flushLocalReads(); listed.reject(new Error('superseded read failed'));
  await initial; await flushLocalReads();
  assert.equal(lists, 2); assert.equal(turns(h)[0].status, 'ready');
  assert.deepEqual(availability, ['loading', 'ready']); stop();
});

test('a genuine restoration failure rejects without automatic retry and an explicit read can recover', async () => {
  let lists = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { if (++lists === 1) throw new Error('local read failed'); return [answer]; }
    if (name === 'load_workspace_answer') return answer;
    assert.fail(`Unexpected command ${name}`);
  }));
  await assert.rejects(h.store.restoreWorkspaceAnswers('project', 'snapshot'), /local read failed/);
  await flushLocalReads(); assert.equal(lists, 1);
  await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  assert.equal(lists, 2); assert.equal(turns(h)[0].status, 'ready');
});

test('wrong-scope events and events after disposal cannot restore another workspace', async () => {
  const h = workspaceStoreHarness(() => assert.fail('Unexpected native read'));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  h.emit('workspace-answer-section', { ...scope, projectId: 'other' });
  h.emit('workspace-answer-finished', { ...scope, snapshotId: 'other' });
  h.emit('repository-guide-changed', { projectId: 'other', snapshotId: 'snapshot' });
  await flushLocalReads(); assert.equal(h.calls.length, 0);
  stop(); h.emit('workspace-answer-section', scope); h.emit('workspace-answer-finished', scope);
  await flushLocalReads(); assert.equal(h.calls.length, 0); assert.equal(h.listenerCount(), 0);
});

test('guide events invalidate the saved guide instead of publishing their older payload', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0;
  const guide = { projectId: 'project', snapshotId: 'snapshot', id: '44444444-4444-4444-8444-444444444444', revision: 2,
    question: answer.question, title: 'Current saved guide', status: 'ready', units: [], openingRunId: scope.runId,
    answers: [answer], omissions: [], capturedFiles: 1, startingFiles: 1 };
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'list_repository_guides') { if (++lists === 1) { started.resolve(); return listed.promise; } return [guide]; }
    if (name === 'list_workspace_answers') return [];
    if (name === 'save_workspace_answer_section') return { projectId: args.projectId, snapshotId: args.snapshotId, revisionId: 'revision' };
    assert.fail(`Unexpected command ${name}`);
  });
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  h.emit('repository-guide-changed', { ...guide, revision: 1, title: 'Old event payload', answers: [] });
  await flushLocalReads(); listed.resolve([]); await initial; await flushLocalReads();
  assert.equal(lists, 2);
  assert.equal(turns(h)[0].guide.title, 'Current saved guide');
  assert.equal(turns(h)[0].guide.revision, 2);
  assert.equal(turns(h)[0].publishedSections.length, 1); stop();
});

test('different repository scopes restore independently and cannot share answers', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred();
  const firstAnswer = { ...answer, projectId: 'one' };
  const h = workspaceStoreHarness(nativeReads(async (name, args) => {
    if (name === 'list_workspace_answers') {
      if (args.projectId === 'one') { started.resolve(); return listed.promise; }
      assert.equal(args.projectId, 'two'); return [];
    }
    if (name === 'load_workspace_answer') { assert.equal(args.projectId, 'one'); return firstAnswer; }
    assert.fail(`Unexpected command ${name}`);
  }));
  const firstStatus = []; const secondStatus = [];
  const stopFirst = await h.store.watchWorkspaceAnswers('one', 'snapshot', (state) => firstStatus.push(state));
  const stopSecond = await h.store.watchWorkspaceAnswers('two', 'snapshot', (state) => secondStatus.push(state));
  const first = h.store.restoreWorkspaceAnswers('one', 'snapshot'); await started.promise;
  await h.store.restoreWorkspaceAnswers('two', 'snapshot');
  assert.deepEqual(firstStatus, ['loading']);
  assert.deepEqual(secondStatus, ['loading', 'ready']);
  assert.equal(turns(h, 'one').length, 0); assert.equal(turns(h, 'two').length, 0);
  listed.resolve([firstAnswer]); await first;
  assert.equal(turns(h, 'one')[0].publishedSections.length, 1);
  assert.equal(turns(h, 'two').length, 0);
  assert.deepEqual(firstStatus, ['loading', 'ready']); stopFirst(); stopSecond();
});

test('a background read failure is visible, preserves answers and clears only after a fresh local read', async () => {
  let lists = 0; let fail = false; const availability = [];
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { lists++; if (fail) throw new Error('private local diagnostic'); return [answer]; }
    if (name === 'load_workspace_answer') return answer;
    assert.fail(`Unexpected command ${name}`);
  }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => availability.push(state));
  await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  assert.deepEqual(availability, ['loading', 'ready']); availability.length = 0;
  const saved = JSON.stringify(turns(h));
  fail = true; h.emit('workspace-answer-section', scope); await flushLocalReads();
  assert.deepEqual(availability, ['loading', 'unavailable']);
  assert.equal(JSON.stringify(turns(h)), saved); assert.equal(lists, 2);
  await flushLocalReads(); assert.equal(lists, 2);
  fail = false; await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  assert.deepEqual(availability, ['loading', 'unavailable', 'loading', 'ready']);
  assert.equal(lists, 3); assert.equal(JSON.stringify(turns(h)), saved); stop();
});

test('a previous completion cannot replace a newer loading state for another observer', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0; let next = null;
  const firstStatus = []; const secondStatus = [];
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { if (++lists === 2) { started.resolve(); return listed.promise; } return []; }
    assert.fail(`Unexpected command ${name}`);
  }));
  const stopFirst = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => {
    firstStatus.push(state);
    if (state === 'ready' && lists === 1) next = h.store.restoreWorkspaceAnswers('project', 'snapshot');
  });
  const stopSecond = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => secondStatus.push(state));
  await h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  assert.deepEqual(firstStatus, ['loading', 'ready', 'loading']);
  assert.deepEqual(secondStatus, ['loading', 'loading']);
  listed.resolve([]); await next;
  assert.equal(secondStatus.at(-1), 'ready'); stopFirst(); stopSecond();
});

test('disposing a watcher during a failed read suppresses every late availability callback', async () => {
  const listed = deferred(); const started = deferred(); const availability = [];
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { started.resolve(); return listed.promise; }
    assert.fail(`Unexpected command ${name}`);
  }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => availability.push(state));
  const pending = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  stop(); listed.reject(new Error('local failure after navigation'));
  await assert.rejects(pending, /local failure after navigation/);
  assert.deepEqual(availability, ['loading']); assert.equal(h.listenerCount(), 0);
});

test('a late failure in one scope cannot change another workspace availability', async () => {
  const listed = deferred(); const started = deferred(); const firstStatus = []; const secondStatus = [];
  const h = workspaceStoreHarness(nativeReads(async (name, args) => {
    if (name === 'list_workspace_answers') { if (args.projectId === 'one') { started.resolve(); return listed.promise; } return []; }
    assert.fail(`Unexpected command ${name}`);
  }));
  const stopFirst = await h.store.watchWorkspaceAnswers('one', 'snapshot', (state) => firstStatus.push(state));
  const stopSecond = await h.store.watchWorkspaceAnswers('two', 'snapshot', (state) => secondStatus.push(state));
  const first = h.store.restoreWorkspaceAnswers('one', 'snapshot'); await started.promise;
  await h.store.restoreWorkspaceAnswers('two', 'snapshot');
  listed.reject(new Error('old scope failure')); await assert.rejects(first, /old scope failure/);
  assert.deepEqual(firstStatus, ['loading', 'unavailable']);
  assert.deepEqual(secondStatus, ['loading', 'ready']); stopFirst(); stopSecond();
});
