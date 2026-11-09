import test from 'node:test';
import assert from 'node:assert/strict';
import { flushLocalReads, workspaceStoreHarness } from './helpers/workspace-store-harness.mjs';

const revisionFor = (revisionId, spokenText) => ({
  projectId: 'project',
  snapshotId: 'snapshot',
  chapterId: 'chapter',
  revisionId,
  revisionNumber: 1,
  basedOnRevisionId: null,
  lifecycle: 'draft',
  validationStatus: 'valid',
  target: { targetId: 'target', scope: { kind: 'overview' } },
  sections: [{ sectionId: 'section', title: 'Dispatch', spokenText, claims: [] }],
});

const narrationHooks = (calls) => ({
  native: true,
  localNarrationPrewarmInputs: (revision) => [{ spokenText: revision.revisionId, voice: 'alba', style: '', lexicon: [] }],
  prewarmLocalNarration: (input) => {
    calls.push(input);
    return Promise.resolve({ status: 'ok', data: { state: 'ready', cachedAudioAvailable: true } });
  },
});

test('publishing a validated answer does not synthesize narration', async () => {
  const prewarmCalls = [];
  const revision = revisionFor('answer-revision', 'The prefix chooses a reader.');
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
          revisionId: revision.revisionId,
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
    ...narrationHooks(prewarmCalls),
    ensureCodexConnection: async () => {},
    readCodexSelection: () => ({ modelId: 'gpt-6-luna', effort: 'max' }),
  });

  await h.store.prepareWorkspaceConversation('project', 'snapshot');
  const turnId = h.store.appendQuestion('project', 'snapshot', 'Follow the prefix check', [], 'guide');
  const saved = await h.store.runQuestion('project', 'snapshot', turnId, 'grant');

  assert.equal(saved.revisionId, revision.revisionId);
  assert.equal(h.store.getWorkspace('project', 'snapshot').explorations[0].turns[0].publishedSections[0].revisionId, revision.revisionId);
  assert.deepEqual(prewarmCalls, []);
});

test('progressive Guide sections publish without starting Pocket audio', async () => {
  const prewarmCalls = [];
  const revision = revisionFor('guide-revision', 'The guide section is ready.');
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'load_workspace_conversation') return null;
    if (name === 'save_workspace_conversation') {
      return { schemaVersion: 1, revision: (args.expectedRevision ?? 0) + 1, state: args.state };
    }
    if (name === 'save_workspace_answer_section') return revision;
    assert.fail(`Unexpected native command: ${name}`);
  }, narrationHooks(prewarmCalls));

  await h.store.prepareWorkspaceConversation('project', 'snapshot');
  const turnId = h.store.appendRepositoryGuide('project', 'snapshot', 'Follow this repository feature');
  const turn = h.store.getWorkspace('project', 'snapshot').explorations[0].turns.find(item => item.id === turnId);
  const stop = await h.store.watchWorkspaceAnswers('project', 'snapshot');
  h.emit('repository-guide-section-published', {
    guideId: turn.guideId,
    section: {
      projectId: 'project',
      snapshotId: 'snapshot',
      runId: '11111111-1111-4111-8111-111111111111',
      key: 'dispatch',
      order: 1,
      generationId: 'generation:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
      revisionId: revision.revisionId,
      supersedes: null,
      contextElapsedMs: 10,
      runElapsedMs: 20,
    },
  });
  await flushLocalReads();

  assert.equal(h.calls.filter(call => call.name === 'save_workspace_answer_section').length, 1);
  assert.equal(h.store.getWorkspace('project', 'snapshot').explorations[0].turns[0].publishedSections[0].revisionId, revision.revisionId);
  assert.deepEqual(prewarmCalls, []);
  stop();
  assert.equal(h.listenerCount(), 0);
});

test('restoring saved Guides does not synthesize audio or replay the agent', async () => {
  const prewarmCalls = [];
  const runId = '11111111-1111-4111-8111-111111111111';
  const section = {
    runId,
    key: 'entry',
    order: 1,
    generationId: 'generation:22222222-2222-4222-8222-222222222222',
    revisionId: 'saved-revision',
    supersedes: null,
    contextElapsedMs: 10,
    runElapsedMs: 20,
  };
  const guideId = '99999999-9999-4999-8999-999999999999';
  const guide = {
    revision: 1,
    id: guideId,
    projectId: 'project',
    snapshotId: 'snapshot',
    question: 'How does the repository work?',
    title: 'Follow the request',
    status: 'partial',
    openingRunId: null,
    openingRunIds: [],
    canContinue: false,
    canExtend: false,
    continuationOperations: 0,
    units: [],
    answers: [{ projectId: 'project', snapshotId: 'snapshot', runId, question: 'What happens to the input?', outcome: 'partial', sections: [section], incompleteTopics: [] }],
    omissions: [],
    capturedFiles: 1,
    startingFiles: 1,
  };
  const revision = revisionFor('saved-revision', 'The saved section remains readable.');
  const h = workspaceStoreHarness(async (name, args) => {
    if (name === 'load_workspace_conversation') return null;
    if (name === 'save_workspace_conversation') {
      return { schemaVersion: 1, revision: (args.expectedRevision ?? 0) + 1, state: args.state };
    }
    if (name === 'list_workspace_answers') return [];
    if (name === 'list_repository_guides') return { guides: [guide], unavailableCount: 0, unavailableGuides: [] };
    assert.fail(`Restoring local Guide content must not dispatch AI or synthesize audio: ${name}`);
  }, {
    ...narrationHooks(prewarmCalls),
    listChapterRevisions: async () => [revision],
  });

  await h.store.restoreWorkspaceAnswers('project', 'snapshot');

  assert.deepEqual(prewarmCalls, []);
  assert.equal(h.calls.some(call => /generate|start_repository_guide|resume_repository_guide/.test(call.name)), false);
  assert.equal(h.calls.some(call => call.name === 'list_repository_guides'), true);
});
