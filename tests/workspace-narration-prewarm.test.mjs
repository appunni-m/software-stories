import test from 'node:test';
import assert from 'node:assert/strict';
import { workspaceStoreHarness } from './helpers/workspace-store-harness.mjs';

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
