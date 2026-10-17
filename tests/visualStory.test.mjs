import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import { readStoryProgress, sceneNodeIds, storyLayout, storyScenes } from '../src/lib/visualStory.ts';

const claim = (id, prose, category = { kind: 'codeBehavior' }) => ({ claimId: id, prose, category, anchors: [{ anchorId: `a:${id}` }] });
const claims = [claim('entry', 'A request reaches handleRequest. The bytes are still untrusted.'), claim('check', 'If validate rejects those bytes, the caller receives an error. Otherwise processing continues.')];
const section = { sectionId: 'section:one', claims, spokenText: claims.map((item) => item.prose).join(' ') };
const visual = {
  schemaVersion: 2, kind: 'dataFlow', title: 'Follow a request', layoutHint: { direction: 'topToBottom' },
  nodes: ['entry', 'check', 'error', 'result'].map((id) => ({ nodeId: id, label: id, evidenceAnchorIds: [id === 'entry' ? 'a:entry' : 'a:check'] })),
  edges: [ ['entry', 'check'], ['check', 'error'], ['check', 'result'] ].map(([fromNodeId, toNodeId], i) => ({ edgeId: `e:${i}`, fromNodeId, toNodeId, label: 'branch' })),
};

describe('visual story scenes', () => {
  it('retains the entire grounded narration, including every condition and later consequence', () => {
    const scenes = storyScenes(section, visual);
    assert.equal(scenes.length, 2);
    assert.equal(scenes.map((scene) => scene.narration).join(' '), section.spokenText);
    assert.deepEqual(scenes.map((scene) => scene.nodeIds), [['entry'], ['check', 'error', 'result']]);
  });

  it('never substitutes complete claims for an older summary or edited recording script', () => {
    const summary = 'Previously reviewed, shorter narration.';
    assert.deepEqual(storyScenes({ ...section, spokenText: summary }, visual).map((scene) => scene.narration), [summary]);
    const edited = 'My edited narration, with a different scene boundary.';
    assert.deepEqual(storyScenes(section, visual, edited).map((scene) => scene.narration), [edited]);
  });

  it('keeps inference labels on every sentence and handles identifier punctuation without dropping names', () => {
    const claims = [claim('context', 'See src/file.rs and Font::memory_face.\nThis may avoid another parse!', { kind: 'inference', visibleLabel: 'Inference' })];
    const spokenText = 'Inference: See src/file.rs and Font::memory_face. Inference: This may avoid another parse!';
    assert.equal(storyScenes({ ...section, claims, spokenText })[0].narration, spokenText);
  });

  it('retains native duplicate-sentence behavior and never manufactures unanchored scenes', () => {
    const claims = [claim('a', 'A call reaches run().'), claim('b', 'A call reaches run(). The result returns.'), { ...claim('c', 'A secret extra sentence.'), anchors: [] }];
    const spokenText = 'A call reaches run(). The result returns.';
    assert.equal(storyScenes({ ...section, claims, spokenText }).map((scene) => scene.narration).join(' '), spokenText);
  });

  it('shows only declared neighbors, and keeps both rejection and success paths', () => {
    const scene = storyScenes(section, visual)[1];
    assert.deepEqual([...sceneNodeIds(visual, scene)].sort(), ['check', 'entry', 'error', 'result']);
    assert.equal(visual.edges.length, 3);
  });

  it('lays out actual branches beside one another instead of inventing a linear call sequence', () => {
    const nodes = new Map(storyLayout(visual).map((node) => [node.id, node.position]));
    assert.equal(nodes.get('error').y, nodes.get('result').y);
    assert.notEqual(nodes.get('error').x, nodes.get('result').x);
    assert.ok(nodes.get('check').y > nodes.get('entry').y);
    assert.ok(nodes.get('error').y > nodes.get('check').y);
  });

  it('terminates deterministically for cycles, self loops and disconnected diagrams', () => {
    const cyclic = { ...visual, edges: [...visual.edges, { edgeId: 'cycle', fromNodeId: 'check', toNodeId: 'entry' }, { edgeId: 'self', fromNodeId: 'result', toNodeId: 'result' }] };
    assert.deepEqual(storyLayout(cyclic), storyLayout(cyclic));
    assert.equal(storyLayout(cyclic).length, visual.nodes.length);
    assert.ok(storyLayout(cyclic).every((node) => Number.isFinite(node.position.x) && Number.isFinite(node.position.y)));
    assert.equal(new Set(storyLayout({ ...visual, edges: [] }).map((node) => `${node.position.x},${node.position.y}`)).size, 4);
  });

  it('maps all four preserved Fontdone narration scenes to their declared evidence cues without inventing timing', () => {
    const artifact = JSON.parse(readFileSync(new URL('../docs/acceptance/artifacts/2026-10-17-fontdone-complete-script-partial.json', import.meta.url)));
    const section = artifact.answer.sections[0].preview.revision.sections[0];
    const scenes = storyScenes(section, section.visual);
    assert.equal(scenes.length, 4);
    assert.equal(scenes.map((scene) => scene.narration).join(' '), section.spokenText);
    assert.ok(scenes[1].narration.includes('parse_bdf_constructor_error'));
    assert.ok(scenes[3].narration.includes('shorter inputs return that original error directly'));
    assert.equal(scenes[1].nodeIds.length, 4);
    assert.ok(scenes.every((scene) => !('startSeconds' in scene)));
  });

  it('restores the exact scene and independent audio positions without resuming playback', () => {
    const scenes = storyScenes(section, visual);
    const progress = readStoryProgress(JSON.stringify({ sceneId: 'check', positions: { entry: 12.4, check: 3.2, other: 14 }, autoPlay: true }), scenes);
    assert.deepEqual(progress, { index: 1, positions: [['entry', 12.4], ['check', 3.2]] });
    assert.equal('autoPlay' in progress, false);
  });

  it('bounds corrupt or unrelated progress and never lets it select a foreign scene', () => {
    const scenes = storyScenes(section, visual);
    for (const raw of [null, '{bad', '"text"', '0'.repeat(16_001)]) assert.deepEqual(readStoryProgress(raw, scenes), { index: 0, positions: [['entry', 0], ['check', 0]] });
    assert.deepEqual(readStoryProgress(JSON.stringify({ sceneId: 'other', positions: { entry: -10, check: '12' } }), scenes), { index: 0, positions: [['entry', 0], ['check', 0]] });
    assert.equal(readStoryProgress(JSON.stringify({ positions: { entry: 3601 } }), scenes).positions[0][1], 0);
  });
});

it('authored narration follows all claim scenes for the reader and export without speaking identifiers', () => {
  const scripts = ['For example, a request arrives with untrusted bytes.', 'If the check rejects them, the caller gets an error. Otherwise, processing continues.'];
  const authored = { ...visual, presentation: { schemaVersion: 3, scenes: scripts.map((narration, claimIndex) => ({ claimIndex, title: 'Follow the request', narration, mode: 'graph', elements: [] })) } };
  const current = { ...section, spokenText: scripts.join(' '), visual: authored };
  const fromReader = storyScenes(current, authored);
  const fromExport = storyScenes(current);
  assert.deepEqual(fromReader, fromExport);
  assert.deepEqual(fromReader.map(s => s.narration), scripts);
  assert.deepEqual(fromReader.map(s => s.id), claims.map(c => c.claimId));
  assert.deepEqual(fromReader.map(s => s.anchorIds), [['a:entry'], ['a:check']]);
  assert.equal(fromReader[1].nodeIds.length, 3);
  assert.ok(current.claims[0].prose.includes('handleRequest'));
  assert.ok(!fromReader[0].narration.includes('handleRequest'));
  assert.deepEqual(storyScenes(current, authored, 'My edited whole script.').map(s => s.narration), ['My edited whole script.']);
  const bad = structuredClone(current); bad.visual.presentation.scenes.pop();
  assert.deepEqual(storyScenes(bad).map(s => s.narration), [current.spokenText]);
});

it('new authored inferred scenes preserve their visible uncertainty label in speech', () => {
  const current = structuredClone(section);
  current.claims[1].category = { kind: 'inference', visibleLabel: 'Inference' };
  current.visual = { ...visual, presentation: { schemaVersion: 3, scenes: [
    { claimIndex: 0, title: 'Arrival', narration: 'The bytes arrive before any trust decision.', mode: 'graph', elements: [] },
    { claimIndex: 1, title: 'Possible effect', narration: 'This may avoid work on rejected input. This is not a runtime guarantee.', mode: 'graph', elements: [] },
  ] } };
  current.spokenText = 'The bytes arrive before any trust decision. Inference: This may avoid work on rejected input. This is not a runtime guarantee.';
  assert.deepEqual(storyScenes(current).map(s => s.narration), ['The bytes arrive before any trust decision.', 'Inference: This may avoid work on rejected input. This is not a runtime guarantee.']);
});
