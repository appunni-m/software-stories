import assert from 'node:assert/strict';
import { it } from 'node:test';
import { localNarrationPrewarmInputs } from '../src/workspace/narrationPrewarm.ts';

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
      schemaVersion: 2,
      nodes: [{ nodeId: 'node:prefix', evidenceAnchorIds: ['anchor:one'] }, { nodeId: 'node:call', evidenceAnchorIds: ['anchor:two'] }],
      presentation: { schemaVersion: 3, scenes: [
        { claimIndex: 0, title: 'Choose a reader', mode: 'graph', elements: [], narration: 'The prefix chooses a reader.' },
        { claimIndex: 1, title: 'Send the bytes', mode: 'graph', elements: [], narration: 'That reader handles pfr data.' },
      ] },
    },
  }],
};
const local = { backend: 'pocket-tts', googleVoice: 'Kore', googleStyle: '' };
const cloud = { backend: 'gemini', googleVoice: 'Charon', googleStyle: 'Calm' };
const read = values => key => values[key] ?? null;

it('prewarms the same authored scene scripts and saved pronunciation edits as the player', () => {
  const baseKey = 'code-atlas:narration:v2:project:one:chapter:one:revision:one:section:one';
  const saved = JSON.stringify({ backend: 'pocket-tts', spokenText: 'The prefix chooses a reader. That reader handles pfr data.', voice: 'alba', style: '', lexicon: [{ name: 'pfr', sayAs: 'P F R', include: true }] });
  const inputs = localNarrationPrewarmInputs(revision, local, read({ [baseKey]: saved }));
  assert.deepEqual(inputs.map(input => input.spokenText), ['The prefix chooses a reader.', 'That reader handles pfr data.']);
  assert.deepEqual(inputs.map(input => input.voice), ['alba', 'alba']);
  assert.deepEqual(inputs[1].lexicon, [{ name: 'pfr', sayAs: 'P F R', include: true }]);
});

it('never builds automatic narration work for a cloud Voice connection', () => {
  const inputs = localNarrationPrewarmInputs(revision, cloud, () => {
    throw new Error('cloud selection must skip local storage and synthesis');
  });
  assert.deepEqual(inputs, []);
});

it('skips a section whose saved narration settings cannot identify matching audio', () => {
  const baseKey = 'code-atlas:narration:v2:project:one:chapter:one:revision:one:section:one';
  assert.deepEqual(localNarrationPrewarmInputs(revision, local, read({ [baseKey]: '{broken' })), []);
});
