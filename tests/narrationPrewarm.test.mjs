import assert from 'node:assert/strict';
import { it } from 'node:test';
import { localNarrationPrewarmInputs } from '../src/workspace/narrationPrewarm.ts';
import { createNarrationPrewarmQueue } from '../src/workspace/narrationQueue.ts';

const revision = {
  schemaVersion: 5,
  projectId: 'project:one',
  chapterId: 'chapter:one',
  revisionId: 'revision:one',
  sections: [{
    sectionId: 'section:one',
    title: 'Dispatch',
    spokenText: 'The prefix chooses a reader. That reader handles pfr data.',
    claims: [
      { claimId: 'claim:prefix', prose: 'The prefix chooses a reader.', category: { kind: 'codeBehavior' }, anchors: [{ anchorId: 'anchor:one' }] },
      { claimId: 'claim:call', prose: 'That reader receives the original bytes.', category: { kind: 'codeBehavior' }, anchors: [{ anchorId: 'anchor:two' }] },
    ],
    visual: {
      schemaVersion: 3,
      nodes: [{ nodeId: 'node:prefix', evidenceAnchorIds: ['anchor:one'] }, { nodeId: 'node:call', evidenceAnchorIds: ['anchor:two'] }],
      presentation: { schemaVersion: 3, scenes: [
        { claimIndex: 0, title: 'Choose a reader', mode: 'graph', elements: [], narration: 'The prefix chooses a reader.' },
        { claimIndex: 1, title: 'Send the bytes', mode: 'graph', elements: [], narration: 'That reader handles pfr data.' },
      ] },
    },
  }],
};
const local = { backend: 'pocket-tts' };
const read = values => key => values[key] ?? null;

it('prewarms the same authored scene scripts and saved pronunciation edits as the player', () => {
  const baseKey = 'code-atlas:narration:v2:project:one:chapter:one:revision:one:section:one';
  const saved = JSON.stringify({ backend: 'pocket-tts', spokenText: 'The prefix chooses a reader. That reader handles pfr data.', voice: 'alba', style: '', lexicon: [{ name: 'pfr', sayAs: 'P F R', include: true }] });
  const inputs = localNarrationPrewarmInputs(revision, local, read({ [baseKey]: saved }));
  assert.deepEqual(inputs.map(input => input.spokenText), ['The prefix chooses a reader.', 'That reader handles pfr data.']);
  assert.deepEqual(inputs.map(input => input.voice), ['alba', 'alba']);
  assert.deepEqual(inputs[1].lexicon, [{ name: 'pfr', sayAs: 'P F R', include: true }]);
});

it('coalesces scripts when saved pronunciation entries are disabled', async () => {
  const baseKey = 'code-atlas:narration:v2:project:one:chapter:one:revision:one:section:one';
  const disabled = localNarrationPrewarmInputs(revision, local, read({ [baseKey]: JSON.stringify({
    backend: 'pocket-tts', spokenText: revision.sections[0].spokenText, voice: 'alba', style: '',
    lexicon: [{ name: '', sayAs: '', include: false }],
  }) }));
  const plain = localNarrationPrewarmInputs(revision, local, () => null);
  assert.deepEqual(disabled, plain, 'unused pronunciation rows must not change the prepared audio identity');

  const prepared = [];
  const queue = createNarrationPrewarmQueue({
    prepare: async input => {
      prepared.push(input);
      return { status: 'ok', data: { state: 'ready', cachedAudioAvailable: true } };
    },
    status: async () => assert.fail('ready audio should not be polled'),
  });
  assert.equal(queue.enqueue(disabled), plain.length);
  assert.equal(queue.enqueue(plain), 0, 'equivalent authored scenes should not occupy duplicate queue slots');
  await queue.flush();
  assert.equal(prepared.length, plain.length);
});

it('bounds settled prewarm deduplication without blocking later scenes', async () => {
  const queue = createNarrationPrewarmQueue({
    maxPending: 48,
    maxConcurrent: 4,
    prepare: async () => ({ status: 'ok', data: { state: 'ready', cachedAudioAvailable: true } }),
    status: async () => assert.fail('ready audio should not be polled'),
  });
  const inputs = Array.from({ length: 257 }, (_, index) => ({
    spokenText: `scene ${index}`,
    voice: 'alba',
    style: '',
    lexicon: [],
  }));

  for (let offset = 0; offset < inputs.length; offset += 48) {
    const batch = inputs.slice(offset, offset + 48);
    assert.equal(queue.enqueue(batch), batch.length);
    await queue.flush();
  }

  assert.equal(queue.enqueue([inputs[0]]), 1, 'the oldest settled identity is evicted from the bounded dedupe cache');
  await queue.flush();
});

it('retries native cancellation when identity changes before prewarm admission settles', async () => {
  let settleFirstPrepare;
  const firstPrepare = new Promise(resolve => { settleFirstPrepare = resolve; });
  const prepared = [];
  const cancelled = [];
  const queue = createNarrationPrewarmQueue({
    maxConcurrent: 1,
    prepare: async candidate => {
      prepared.push(candidate);
      if (prepared.length === 1) return firstPrepare;
      return { status: 'ok', data: { state: 'ready', cachedAudioAvailable: true } };
    },
    status: async () => assert.fail('ready audio should not be polled'),
    cancelNativePending: async candidate => {
      cancelled.push(candidate);
      return true;
    },
  });

  assert.equal(queue.enqueue([localNarrationPrewarmInputs(revision, local, () => null)[0]]), 1);
  assert.equal(queue.state(prepared[0]), 'preparing');

  queue.invalidateIdentity();
  assert.equal(cancelled.length, 1, 'identity invalidation should cancel submitted native work');
  assert.equal(queue.enqueue([prepared[0]]), 1, 'the new identity must admit the same script again');
  assert.equal(queue.state(prepared[0]), 'queued');

  settleFirstPrepare({ status: 'ok', data: { state: 'preparing', cachedAudioAvailable: false } });
  await queue.flush();

  assert.deepEqual(cancelled, [prepared[0], prepared[0]], 'retry cancellation after the old admission settles');
  assert.equal(prepared.length, 2, 'the new-identity job should run after the old job exits');
});

it('never builds narration work for an unsupported connection value', () => {
  const inputs = localNarrationPrewarmInputs(revision, { backend: 'gemini' }, () => {
    throw new Error('unsupported connection values must skip storage and synthesis');
  });
  assert.deepEqual(inputs, []);
});

it('skips a section whose saved narration settings cannot identify matching audio', () => {
  const baseKey = 'code-atlas:narration:v2:project:one:chapter:one:revision:one:section:one';
  assert.deepEqual(localNarrationPrewarmInputs(revision, local, read({ [baseKey]: '{broken' })), []);
});
