import test from 'node:test';
import assert from 'node:assert/strict';
import { deferred, flushLocalReads, workspaceStoreHarness } from './helpers/workspace-store-harness.mjs';

const scope = { projectId: 'project', snapshotId: 'snapshot', runId: '11111111-1111-4111-8111-111111111111' };
const section = { runId: scope.runId, key: 'entry', order: 1, generationId: 'generation:22222222-2222-4222-8222-222222222222', revisionId: 'revision', supersedes: null, contextElapsedMs: 10, runElapsedMs: 20 };
const answer = { ...scope, question: 'What happens to the input?', outcome: 'complete', sections: [section], incompleteTopics: [] };
function nativeReads(handler) {
  return (name, args) => {
    if (name === 'list_repository_guides') return Promise.resolve({ guides: [], unavailableCount: 0 });
    if (name === 'save_workspace_answer_section') return Promise.resolve({ projectId: args.projectId, snapshotId: args.snapshotId, revisionId: args.generationId === section.generationId ? 'revision' : 'revision-two' });
    return handler(name, args);
  };
}
const turns = (h, project = 'project', snapshot = 'snapshot') => h.store.getWorkspace(project, snapshot).explorations.flatMap((e) => e.turns);

test('answer restoration reads only this repository snapshot job ledger', async () => {
  let jobLists = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') return [];
    if (name === 'list_workspace_generation_jobs_for_scope') return [];
    assert.fail(`Unexpected command ${name}`);
  }), { listWorkspaceGenerationJobsForScope: async (projectId,snapshotId) => {
    assert.equal(projectId,'project'); assert.equal(snapshotId,'snapshot'); jobLists++; return [];
  } });
  await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  assert.equal(jobLists, 1);
  assert.equal(h.calls.filter((call) => call.name === 'list_workspace_answers').length, 1);
  assert.equal(h.calls.filter((call) => call.name === 'list_workspace_generation_jobs_for_scope').length, 0);
});

test('restoration keeps a native worker visible after a webview reload without replaying it', async () => {
  const runId='11111111-1111-4111-8111-111111111111';
  const turnId='33333333-3333-4333-8333-333333333333';
  const job={jobId:runId,kind:'workspaceQuestion',conversationTurnId:turnId,projectId:'project',snapshotId:'snapshot',
    modelId:'gpt-6-luna',reasoningEffort:'max',sourceFingerprintSha256:'a'.repeat(64),
    requestFingerprintSha256:'b'.repeat(64),runtimeIdentitySha256:'c'.repeat(64),state:'running',workerRegistered:true,
    providerThreadId:'thread',providerTurnIds:['turn'],createdAtUnixMs:1,updatedAtUnixMs:2};
  const h=workspaceStoreHarness(nativeReads(async(name)=>{
    if(name==='list_workspace_answers') return [];
    if(name==='list_workspace_generation_jobs_for_scope') return [job];
    assert.fail(`Unexpected command ${name}`);
  }),{listWorkspaceGenerationJobsForScope:async(projectId,snapshotId)=>{
    assert.equal(projectId,'project');assert.equal(snapshotId,'snapshot');return [job];
  }});
  h.store.updateWorkspace('project','snapshot',(state)=>({...state,explorations:state.explorations.map((e)=>e.id==='root'
    ?{...e,turns:[{id:turnId,question:'Explain the caller',context:[],status:'interrupted',runIds:[runId],publishedSections:[]}]}:e)}));
  await h.store.restoreWorkspaceAnswers('project','snapshot');
  assert.equal(turns(h)[0].status,'working');
  assert.deepEqual(h.calls.filter((call)=>call.name==='generate_progressive_workspace_answer'),[]);
});

test('restoration preserves a renderer-owned queued question and never dispatches it again',async()=>{
  const runId='11111111-1111-4111-8111-111111111111';
  const turnId='33333333-3333-4333-8333-333333333333';let listedSession;
  const base={jobId:runId,kind:'workspaceQuestion',conversationTurnId:turnId,projectId:'project',snapshotId:'snapshot',
    modelId:'gpt-6-luna',reasoningEffort:'max',sourceFingerprintSha256:null,requestFingerprintSha256:null,runtimeIdentitySha256:null,
    state:'queued',workerRegistered:false,providerThreadId:null,providerTurnIds:[],createdAtUnixMs:1,updatedAtUnixMs:2};
  const h=workspaceStoreHarness(nativeReads(async(name)=>{
    if(name==='list_workspace_answers')return [];
    if(name==='list_workspace_generation_jobs_for_scope')return [];
    assert.fail(`Unexpected command ${name}`);
  }),{listWorkspaceGenerationJobsForScope:async(projectId,snapshotId,rendererSessionId)=>{
    assert.equal(projectId,'project');assert.equal(snapshotId,'snapshot');listedSession=rendererSessionId;
    assert.match(rendererSessionId,/^[0-9a-f-]{36}$/i);return [{...base,rendererSessionId}];
  }});
  h.store.updateWorkspace('project','snapshot',(state)=>({...state,explorations:state.explorations.map((e)=>e.id==='root'
    ?{...e,turns:[{id:turnId,question:'Explain the queued work',context:[],status:'queued',runIds:[runId]}]}:e)}));
  await h.store.restoreWorkspaceAnswers('project','snapshot');
  assert.ok(listedSession);assert.equal(turns(h)[0].status,'queued');
  assert.deepEqual(h.calls.filter((call)=>call.name==='generate_progressive_workspace_answer'),[]);
});

test('restoration never maps a guide slide or a mismatched turn onto a question',async()=>{
  const runId='11111111-1111-4111-8111-111111111111';
  const guideRunId='55555555-5555-4555-8555-555555555555';
  const turnId='33333333-3333-4333-8333-333333333333';
  const wrongTurnId='44444444-4444-4444-8444-444444444444';
  const base={jobId:runId,projectId:'project',snapshotId:'snapshot',modelId:'gpt-6-luna',reasoningEffort:'max',
    sourceFingerprintSha256:'a'.repeat(64),requestFingerprintSha256:'b'.repeat(64),runtimeIdentitySha256:'c'.repeat(64),
    state:'running',workerRegistered:true,providerThreadId:null,providerTurnIds:[],createdAtUnixMs:1,updatedAtUnixMs:2};
  const guide={...base,jobId:guideRunId,kind:'repositoryGuideSlide',conversationTurnId:turnId};
  const wrongQuestion={...base,kind:'workspaceQuestion',conversationTurnId:wrongTurnId};
  const jobs=[guide,wrongQuestion];
  const h=workspaceStoreHarness(nativeReads(async(name)=>{
    if(name==='list_workspace_answers') return [];
    if(name==='list_workspace_generation_jobs_for_scope') return jobs;
    assert.fail(`Unexpected command ${name}`);
  }),{listWorkspaceGenerationJobsForScope:async()=>jobs});
  h.store.updateWorkspace('project','snapshot',(state)=>({...state,explorations:state.explorations.map((e)=>e.id==='root'
    ?{...e,turns:[{id:turnId,question:'Explain the caller',context:[],status:'interrupted',runIds:[runId]}]}:e)}));
  await h.store.restoreWorkspaceAnswers('project','snapshot');
  assert.equal(turns(h)[0].status,'interrupted');
});

test('job reconciliation keeps the saved failure detail for the reader',async()=>{
  const runId='11111111-1111-4111-8111-111111111111';
  const turnId='33333333-3333-4333-8333-333333333333';
  const job={jobId:runId,kind:'workspaceQuestion',conversationTurnId:turnId,projectId:'project',snapshotId:'snapshot',
    modelId:'gpt-6-luna',reasoningEffort:'max',sourceFingerprintSha256:'a'.repeat(64),
    requestFingerprintSha256:'b'.repeat(64),runtimeIdentitySha256:'c'.repeat(64),state:'failed',workerRegistered:true,
    providerThreadId:null,providerTurnIds:[],createdAtUnixMs:1,updatedAtUnixMs:2};
  const h=workspaceStoreHarness(nativeReads(async(name)=>{
    if(name==='list_workspace_answers') return [];
    if(name==='list_workspace_generation_jobs_for_scope') return [job];
    assert.fail(`Unexpected command ${name}`);
  }),{listWorkspaceGenerationJobsForScope:async()=>[job]});
  const savedError='Codex could not read the captured source.';
  h.store.updateWorkspace('project','snapshot',(state)=>({...state,explorations:state.explorations.map((e)=>e.id==='root'
    ?{...e,turns:[{id:turnId,question:'Explain the caller',context:[],status:'interrupted',error:savedError,runIds:[runId]}]}:e)}));
  await h.store.restoreWorkspaceAnswers('project','snapshot');
  assert.equal(turns(h)[0].status,'failed');
  assert.equal(turns(h)[0].error,savedError);
});

test('a section arriving during initial restoration is read before that restoration finishes', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { if (++lists === 1) { started.resolve(); return listed.promise; } return [answer]; }
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
  assert.equal(h.calls.filter((c) => c.name === 'load_workspace_answer').length, 0);
  assert.equal(h.calls.filter((c) => c.name === 'save_workspace_answer_section').length, 0);
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

test('finished events during restoration preserve both sections and the reader selection', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0; let latest = false;
  const second = { ...section, key: 'next', order: 2, generationId: 'generation:33333333-3333-4333-8333-333333333333', revisionId: 'revision-two' };
  const partial = { ...answer, outcome: 'working' };
  const complete = { ...answer, sections: [section, second] };
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'list_repository_guides') return { guides: [], unavailableCount: 0 };
    if (name === 'list_workspace_answers') { if (++lists === 1) { started.resolve(); return listed.promise; } return [latest ? complete : partial]; }
    assert.fail(`Unexpected command ${name}`);
  });
  h.store.updateWorkspace('project', 'snapshot', (s) => ({ ...s, file: 'src/main.rs', explorations: s.explorations.map((e) => ({ ...e, view: 'flow', turns: [{ id: 'question', question: answer.question, context: [], status: 'interrupted', runIds: [scope.runId], reading: { flow: { revisionId: 'revision', sectionId: 'second-scene' } } }] })) }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  latest = true; h.emit('workspace-answer-finished', scope); await flushLocalReads(); listed.resolve([partial]);
  await initial; await flushLocalReads();
  const turn = turns(h)[0];
  assert.equal(lists, 2);
  assert.equal(turn.status, 'ready'); assert.equal(turn.publishedSections.length, 2);
  assert.equal(turn.reading.flow.sectionId, 'second-scene');
  assert.equal(h.store.getWorkspace('project', 'snapshot').explorations[0].view, 'flow');
  assert.equal(h.store.getWorkspace('project', 'snapshot').file, 'src/main.rs'); stop();
});

test('a native invalidation supersedes a failing initial read without replaying inference', { timeout: 2000 }, async () => {
  const listed = deferred(); const started = deferred(); let lists = 0;
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') { if (++lists === 1) { started.resolve(); return listed.promise; } return [answer]; }
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
    assert.fail(`Unexpected command ${name}`);
  }));
  await assert.rejects(h.store.restoreWorkspaceAnswers('project', 'snapshot'), /local read failed/);
  await flushLocalReads(); assert.equal(lists, 1);
  await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  assert.equal(lists, 2); assert.equal(turns(h)[0].status, 'ready');
});

test('startup merges every native-verified answer without replaying individual receipts', async () => {
  const secondRun = '55555555-5555-4555-8555-555555555555';
  const secondSection = { ...section, runId: secondRun, generationId: 'generation:66666666-6666-4666-8666-666666666666', revisionId: 'revision-two' };
  const secondAnswer = { ...answer, runId: secondRun, question: 'What uses the result?', sections: [secondSection] };
  const availability = [];
  const h = workspaceStoreHarness(nativeReads(async (name, args) => {
    if (name === 'list_workspace_answers') return [answer, secondAnswer];
    if (name === 'load_workspace_answer') throw new Error('startup must not replay individual receipts');
    assert.fail(`Unexpected command ${name}`);
  }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot', (state) => availability.push(state));
  await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  assert.deepEqual(availability, ['loading', 'ready']);
  assert.equal(turns(h).length, 2);
  assert.equal(turns(h)[0].status, 'ready');
  assert.equal(turns(h)[0].publishedSections.length, 1);
  assert.equal(turns(h)[1].status, 'ready');
  assert.equal(turns(h)[1].publishedSections[0].revisionId, 'revision-two');
  assert.equal(h.calls.filter((call) => call.name === 'load_workspace_answer').length, 0);
  stop();
});

test('a failed native answer restores its finite reason for the reader', async () => {
  const failed = { ...answer, outcome: 'failed', failureCode: 'noPublishedSections', sections: [], incompleteTopics: [] };
  const h = workspaceStoreHarness(nativeReads(async (name) => {
    if (name === 'list_workspace_answers') return [failed];
    assert.fail(`Unexpected command ${name}`);
  }));
  await h.store.restoreWorkspaceAnswers('project', 'snapshot');
  const restored = turns(h)[0];
  assert.equal(restored.status, 'failed');
  assert.equal(restored.failureCode, 'noPublishedSections');
  assert.equal(restored.publishedSections.length, 0);
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
    openingRunIds: [scope.runId], canContinue: false, canExtend: false, continuationOperations: 0,
    answers: [answer], omissions: [], capturedFiles: 1, startingFiles: 1 };
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'list_repository_guides') { if (++lists === 1) { started.resolve(); return listed.promise; } return { guides: [guide], unavailableCount: 0 }; }
    if (name === 'list_workspace_answers') return [];
    assert.fail(`Unexpected command ${name}`);
  });
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  const initial = h.store.restoreWorkspaceAnswers('project', 'snapshot'); await started.promise;
  h.emit('repository-guide-changed', { ...guide, revision: 1, title: 'Old event payload', answers: [] });
  await flushLocalReads(); listed.resolve({ guides: [], unavailableCount: 0 }); await initial; await flushLocalReads();
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
