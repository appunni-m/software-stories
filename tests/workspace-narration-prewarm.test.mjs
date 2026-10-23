import test from 'node:test';
import assert from 'node:assert/strict';
import { flushLocalReads, workspaceStoreHarness } from './helpers/workspace-store-harness.mjs';

const input = {
  spokenText: 'The prefix chooses a reader.',
  voice: 'alba',
  style: '',
  lexicon: [],
};

test('newly published explanation sections enqueue local narration without waiting for audio', async () => {
  const calls = [];
  let releaseWarmup;
  let warmupReleased = false;
  const warmup = new Promise(resolve => {
    releaseWarmup = () => {
      warmupReleased = true;
      resolve({ status: 'ok', data: { state: 'ready', cachedAudioAvailable: true } });
    };
  });
  const revision = {
    projectId: 'project',
    snapshotId: 'snapshot',
    chapterId: 'chapter',
    revisionId: 'revision',
    revisionNumber: 1,
    basedOnRevisionId: null,
    lifecycle: 'draft',
    validationStatus: 'valid',
    target: { targetId: 'target', scope: { kind: 'overview' } },
    sections: [{ sectionId: 'section', title: 'Dispatch', spokenText: input.spokenText, claims: [] }],
  };
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'load_workspace_conversation') return null;
    if (name === 'save_workspace_conversation') {
      return { schemaVersion: 1, revision: (args.expectedRevision ?? 0) + 1, state: args.state };
    }
    if (name === 'generate_progressive_workspace_answer') {
      return {
        projectId: args.input.projectId,
        snapshotId: args.input.snapshotId,
        runId: args.input.runId,
        question: args.input.question,
        outcome: 'complete',
        sections: [{
          runId: args.input.runId,
          key: 'dispatch',
          order: 1,
          generationId: 'generation:11111111-1111-4111-8111-111111111111',
          revisionId: 'revision',
          supersedes: null,
          contextElapsedMs: 10,
          runElapsedMs: 20,
        }],
        incompleteTopics: [],
      };
    }
    if (name === 'save_workspace_answer_section') return revision;
    assert.fail(`Unexpected native command: ${name}`);
  }, {
    native: true,
    ensureCodexConnection: async () => {},
    readCodexSelection: () => ({ modelId: 'gpt-6-luna', effort: 'max' }),
    readVoiceConnection: () => ({ backend: 'pocket-tts', googleVoice: 'Kore', googleStyle: '' }),
    localNarrationPrewarmInputs: (candidate, connection) => {
      assert.equal(candidate.revisionId, revision.revisionId);
      assert.equal(connection.backend, 'pocket-tts');
      return [input];
    },
    prewarmLocalNarration: (candidate) => {
      calls.push(candidate);
      return warmup;
    },
  });

  await h.store.prepareWorkspaceConversation('project', 'snapshot');
  const question = h.store.appendQuestion('project', 'snapshot', 'Follow the prefix check', [], 'guide');
  const run = h.store.runQuestion('project', 'snapshot', question, 'grant');
  let timeout;
  const publication = await Promise.race([
    run.then(value => ({ value })),
    new Promise(resolve => { timeout = setTimeout(() => resolve({ timedOut: true }), 2_000); }),
  ]);
  clearTimeout(timeout);
  assert.equal(publication.timedOut, undefined, 'answer publication must not wait for Pocket TTS');
  const saved = publication.value;

  assert.equal(saved.revisionId, revision.revisionId);
  assert.deepEqual(calls, [input]);
  assert.equal(warmupReleased, false, 'the answer should be ready while the audio worker is still pending');
  const turn = h.store.getWorkspace('project', 'snapshot').explorations
    .flatMap(item => item.turns).find(item => item.id === question);
  assert.equal(turn.publishedSections?.[0]?.revisionId, revision.revisionId);
  releaseWarmup();
  assert.equal(warmupReleased, true);
});

test('a progressive repository-guide section is adopted and warmed before its investigation unit finishes', async () => {
  const warmups = [];
  const revision = {
    projectId: 'project', snapshotId: 'snapshot', chapterId: 'chapter', revisionId: 'guide-revision',
    revisionNumber: 1, basedOnRevisionId: null, lifecycle: 'draft', validationStatus: 'valid',
    target: { targetId: 'target', scope: { kind: 'overview' } },
    sections: [{ sectionId: 'guide-section', title: 'Dispatch', spokenText: 'The guide section is ready.', claims: [] }],
  };
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'load_workspace_conversation') return null;
    if (name === 'save_workspace_conversation') return { schemaVersion: 1, revision: (args.expectedRevision ?? 0) + 1, state: args.state };
    if (name === 'save_workspace_answer_section') return revision;
    assert.fail(`Unexpected native command: ${name}`);
  }, {
    native: true,
    localNarrationPrewarmInputs: candidate => [{ spokenText: candidate.revisionId, voice: 'alba', style: '', lexicon: [] }],
    prewarmLocalNarration: input => { warmups.push(input); return Promise.resolve({ status: 'ok', data: { state: 'preparing', cachedAudioAvailable: false } }); },
  });

  await h.store.prepareWorkspaceConversation('project', 'snapshot');
  const turnId = h.store.appendRepositoryGuide('project', 'snapshot', 'Follow this repository feature');
  const turn = h.store.getWorkspace('project', 'snapshot').explorations.flatMap(item => item.turns).find(item => item.id === turnId);
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  const section = {
    projectId: 'project', snapshotId: 'snapshot', runId: '11111111-1111-4111-8111-111111111111', key: 'dispatch', order: 1,
    generationId: 'generation:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', revisionId: revision.revisionId,
    supersedes: null, contextElapsedMs: 10, runElapsedMs: 20,
  };

  h.emit('repository-guide-section-published', { guideId: turn.guideId, section });
  await flushLocalReads();

  assert.equal(h.calls.filter(call => call.name === 'save_workspace_answer_section').length, 1);
  assert.deepEqual(warmups, [{ spokenText: revision.revisionId, voice: 'alba', style: '', lexicon: [] }]);
  assert.equal(h.store.getWorkspace('project', 'snapshot').explorations[0].turns[0].publishedSections[0].revisionId, revision.revisionId);
  stop();
  assert.equal(h.listenerCount(), 0);
});

test('a guide update warms only new revisions already adopted by the native reader', async () => {
  const firstRunId = '11111111-1111-4111-8111-111111111111';
  const nextRunId = '22222222-2222-4222-8222-222222222222';
  const firstSection = { runId: firstRunId, key: 'first', order: 1, generationId: 'generation:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', revisionId: 'first-revision', supersedes: null, contextElapsedMs: 10, runElapsedMs: 20 };
  const nextSection = { runId: nextRunId, key: 'next', order: 1, generationId: 'generation:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb', revisionId: 'next-revision', supersedes: null, contextElapsedMs: 30, runElapsedMs: 40 };
  const answerFor = (runId, sections) => ({ runId, projectId: 'project', snapshotId: 'snapshot', question: `Question for ${runId}`, outcome: 'complete', sections, incompleteTopics: [] });
  const oldAnswer = answerFor(firstRunId, [firstSection]);
  const nextAnswer = answerFor(nextRunId, [nextSection]);
  let guideId = '';
  const updatedGuide = {
    projectId: 'project', snapshotId: 'snapshot', revision: 2, id: '', question: 'Follow this repository feature', title: 'Feature path', status: 'building',
    openingRunId: firstRunId, openingRunIds: [firstRunId], canContinue: true, continuationOperations: 0,
    units: [{ key: 'next', question: 'What happens next?', prerequisites: [], status: 'complete', runIds: [nextRunId] }],
    answers: [oldAnswer, nextAnswer], omissions: [], capturedFiles: 2, startingFiles: 2,
  };
  const revisions = [firstSection, nextSection].map(section => ({
    projectId: 'project', snapshotId: 'snapshot', chapterId: section.key, revisionId: section.revisionId,
    revisionNumber: 1, basedOnRevisionId: null, lifecycle: 'draft', validationStatus: 'valid',
    target: { targetId: 'target', scope: { kind: 'overview' } },
    sections: [{ sectionId: section.key, title: section.key, spokenText: section.revisionId, claims: [] }],
  }));
  const warmups = [];
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'load_workspace_conversation') return null;
    if (name === 'save_workspace_conversation') return { schemaVersion: 1, revision: (args.expectedRevision ?? 0) + 1, state: args.state };
    if (name === 'list_repository_guides') return { guides: [{ ...updatedGuide, id: guideId }], unavailableCount: 0 };
    if (name === 'list_workspace_answers') return [];
    assert.fail(`Unexpected native command: ${name}`);
  }, {
    native: true,
    listChapterRevisions: async () => revisions,
    localNarrationPrewarmInputs: candidate => [{ spokenText: candidate.revisionId, voice: 'alba', style: '', lexicon: [] }],
    prewarmLocalNarration: input => { warmups.push(input); return Promise.resolve({ status: 'ok', data: { state: 'preparing', cachedAudioAvailable: false } }); },
  });

  await h.store.prepareWorkspaceConversation('project', 'snapshot');
  const turnId = h.store.appendRepositoryGuide('project', 'snapshot', updatedGuide.question);
  const turn = h.store.getWorkspace('project', 'snapshot').explorations.flatMap(item => item.turns).find(item => item.id === turnId);
  guideId = turn.guideId;
  updatedGuide.id = guideId;
  h.store.updateWorkspace('project', 'snapshot', state => ({
    ...state,
    explorations: state.explorations.map(exploration => ({ ...exploration, turns: exploration.turns.map(item => item.id === turnId
      ? { ...item, guide: { ...updatedGuide, answers: undefined, revision: 1 }, publishedSections: [firstSection] }
      : item) })),
  }));
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');

  h.emit('repository-guide-changed', { ...updatedGuide, revision: 1, title: 'Stale event payload', answers: [] });
  await flushLocalReads();
  await flushLocalReads();

  assert.deepEqual(warmups, [{ spokenText: 'next-revision', voice: 'alba', style: '', lexicon: [] }]);
  const updatedTurn = h.store.getWorkspace('project', 'snapshot').explorations[0].turns[0];
  assert.equal(updatedTurn.guide.title, 'Feature path');
  assert.equal(JSON.stringify(updatedTurn.publishedSections.map(section => section.revisionId)), JSON.stringify(['first-revision', 'next-revision']));
  stop();
});
