import assert from 'node:assert/strict';
import { it } from 'node:test';
import { narrationStorageKey, scopedNarrationPreferences, sectionNarrationScopes } from '../src/lib/narrationScope.ts';

const section = () => ({
  sectionId: 'section:one', title: 'Dispatch',
  spokenText: 'memory_face checks the prefix. pfr_face receives the original data.',
  claims: [
    { claimId: 'claim:prefix', prose: 'memory_face checks the prefix.', category: { kind: 'codeBehavior' }, anchors: [{ anchorId: 'anchor:one' }] },
    { claimId: 'claim:call', prose: 'pfr_face receives the original data.', category: { kind: 'codeBehavior' }, anchors: [{ anchorId: 'anchor:two' }] },
  ],
});
const revision = { projectId: 'project:one', chapterId: 'chapter:one', revisionId: 'revision:one' };
const baseKey = narrationStorageKey(revision.projectId, revision.chapterId, revision.revisionId, 'section:one');
const read = values => key => values[key] ?? null;

it('export uses the player scene keys and inherits delivery without a whole-section script', () => {
  const parent = { backend: 'pocket-tts', spokenText: 'memory face checks the prefix. pfr face receives the original data.',
    lexicon: [{ name: 'pfr', sayAs: 'P F R', include: true }] };
  const scopes = sectionNarrationScopes(revision, section(), read({ [baseKey]: JSON.stringify(parent) }));
  assert.deepEqual(scopes.map(scope => scope.sceneId), ['claim:prefix', 'claim:call']);
  assert.equal(scopes[0].preferences.spokenText, 'memory face checks the prefix.');
  assert.equal(scopes[1].preferences.spokenText, 'pfr face receives the original data.');
  assert.deepEqual(scopes[1].preferences.lexicon, parent.lexicon);
});

it('saved scene scripts remain independent and a custom whole script stays whole', () => {
  const values = { [`${baseKey}:scene:claim:call`]: JSON.stringify({ backend: 'pocket-tts', spokenText: 'My reviewed second scene.' }) };
  const scopes = sectionNarrationScopes(revision, section(), read(values));
  assert.equal(scopes[1].preferences.spokenText, 'My reviewed second scene.');
  values[baseKey] = JSON.stringify({ backend: 'pocket-tts', spokenText: 'My complete custom story.' });
  const whole = sectionNarrationScopes(revision, section(), read(values));
  assert.equal(whole.length, 1);
  assert.equal(whole[0].sceneId, undefined);
  assert.equal(whole[0].preferences.spokenText, 'My complete custom story.');
});

it('legacy summaries are not split and unreadable storage cannot guess cache settings', () => {
  const legacy = section(); legacy.spokenText = 'A short summary.';
  assert.equal(sectionNarrationScopes(revision, legacy, read({})).length, 1);
  assert.throws(() => sectionNarrationScopes(revision, section(), () => { throw new Error('Storage unavailable'); }), /Storage unavailable/);
  assert.equal(scopedNarrationPreferences(read({}), 'scene', 'Scene fallback', baseKey).spokenText, 'Scene fallback');
});

it('natural scene scripts use identical scopes in player and export and retain per-scene edits', () => {
  const current = section();
  const scripts = ['A prefix check decides which reader should handle these bytes.', 'The chosen reader gets the original input, including its remaining bytes.'];
  current.spokenText = scripts.join(' ');
  current.visual = { schemaVersion: 2, nodes: [{ nodeId: 'n1', evidenceAnchorIds: ['anchor:one'] }, { nodeId: 'n2', evidenceAnchorIds: ['anchor:two'] }],
    presentation: { schemaVersion: 3, scenes: scripts.map((narration, claimIndex) => ({ claimIndex, title: 'Read the input', narration, mode: 'graph', elements: [] })) } };
  const scopes = sectionNarrationScopes(revision, current, read({}));
  assert.deepEqual(scopes.map(s => s.sceneId), ['claim:prefix', 'claim:call']);
  assert.deepEqual(scopes.map(s => s.preferences.spokenText), scripts);
  const edited = sectionNarrationScopes(revision, current, read({ [`${baseKey}:scene:claim:call`]: JSON.stringify({ backend: 'pocket-tts', spokenText: 'My second-scene script.' }) }));
  assert.equal(edited[1].preferences.spokenText, 'My second-scene script.');
  assert.equal(edited[0].preferences.spokenText, scripts[0]);
});
