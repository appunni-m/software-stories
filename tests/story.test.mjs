import { test } from 'node:test';
import assert from 'node:assert/strict';
import sharp from 'sharp';
import { activeStep, sceneTypes, validateStory } from '../src/shared/story.mjs';
import { renderScene } from '../src/shared/render.mjs';
import { cueIndex, timelineAt } from '../src/shared/timeline.mjs';
import { isCurrentNarrative, narrativeVersion } from '../src/shared/narrative.mjs';

const snapshot = { files: [{ path: 'src/cache.js', kind: 'implementation', text: 'function get(id) {\n  return cache.get(id);\n}' }, { path: 'README.md', kind: 'documentation', text: 'The cache is fast.' }] };
const chapter = { title: 'Read the cache', summary: 'A result is retrieved.', narration: 'The lookup reads a saved result from the cache.', claims: [{ text: 'get delegates to the cache.', basis: 'implementation', evidence: [{ path: 'src/cache.js', startLine: 1, endLine: 3, quote: 'return cache.get(id);' }] }], scene: { type: 'flow', nodes: [{ id: 'a', label: 'Caller', detail: 'Supplies a key' }, { id: 'b', label: 'Cache', detail: 'Returns a saved result' }], edges: [{ from: 'a', to: 'b', label: 'key' }], steps: [{ label: 'Supply a key', detail: 'The caller supplies a key.', nodeId: 'a' }, { label: 'Retrieve a result', detail: 'The cache returns a saved result.', nodeId: 'b' }] }, follows: [] };
const story = () => ({ title: 'A cache lookup', chapters: [structuredClone(chapter)] });
test('a takeaway does not make old animation commentary a current software story',()=>{
 assert.equal(isCurrentNarrative({...chapter,takeaway:'Reuse the cached result'}),false);
 assert.equal(isCurrentNarrative({...chapter,narrativeVersion}),true);
 for(const narration of ['Watch the blue path.','Look at the fork.','Watch this handoff.']) {
   const input=story();Object.assign(input.chapters[0],{narrativeVersion,takeaway:'Reuse the cached result',narration});
   assert.throws(()=>validateStory(input,snapshot),/Explain the software/);
 }
});
test('a software explanation may take the space needed for purpose and mechanism',()=>{
 const input=story();Object.assign(input.chapters[0],{narrativeVersion,takeaway:'The caller delegates retrieval to the cache.',narration:'Suppose a caller needs a previously saved result. It supplies an identifier to the lookup function, which asks the cache for the corresponding value. The function does not own the storage details; its role is to delegate retrieval. That boundary gives the caller a small operation to use without inspecting how the cache keeps its values. This excerpt does not establish what happens when the identifier is missing, so that case needs a deeper source lookup.'});
 assert.ok(input.chapters[0].narration.split(/\s+/).length>65);
 assert.equal(validateStory(input,snapshot),input);
});
test('rejects fabricated source quotes and out-of-range citations', () => {
  for (const change of [citation => citation.quote = 'return database.get(id);', citation => citation.endLine = 900, citation => citation.path = '../../secrets']) {
    const input = story(); change(input.chapters[0].claims[0].evidence[0]); assert.throws(() => validateStory(input, snapshot));
  }
});
test('visual timing follows known spoken timestamps instead of evenly spaced steps',()=>{
 const input=structuredClone(chapter);
 input.narration='Supply a key. Wait for work. Retrieve a result.';
 input.scene.steps[0].cue='Supply a key';input.scene.steps[1].cue='Retrieve a result';
 input.audio={duration:20,alignment:{words:Array.from({length:10},(_,i)=>({start:i===6?16:i,end:i+1}))}};
 assert.equal(cueIndex(input.narration,'Retrieve a result'),6);
 assert.equal(timelineAt(input,.7).index,0);assert.equal(timelineAt(input,.85).index,1);
 assert.equal(timelineAt(input,.85).aligned,true);
 input.audio.alignment.words[6]=null;assert.equal(timelineAt(input,.85).aligned,false);
});
test('spoken cues, referenced claims, and comparison evidence must be valid',()=>{
 for(const change of [input=>input.scene.steps[0].cue='Words that were never spoken',input=>input.scene.steps[0].claimIndex=7,input=>input.scene.steps[0].fromNodeId='invented',input=>{input.scene.steps[0].fromNodeId='b';},input=>{input.takeaway='Keep the explanation focused';input.narration='word '.repeat(170);},input=>{input.scene.type='diff';input.scene.rationale='Compare the versions';}]){
   const input=story();change(input.chapters[0]);assert.throws(()=>validateStory(input,snapshot));
 }
});
test('templates use different visual structures and motion changes within a step',()=>{
 const structures=new Set();
 for(const type of sceneTypes){const input=structuredClone(chapter);input.scene.type=type;const a=renderScene(input,.1),b=renderScene(input,.2);structures.add(a.match(/data-visual="([^"]+)"/)[1]);assert.notEqual(a,b);}
 assert.equal(structures.size,sceneTypes.length);
 const locked=renderScene(chapter,.2,{locked:true});assert.match(locked,/aria-disabled="true"/);assert.match(locked,/tabindex="-1"/);
});
test('documentation cannot establish implementation, but may support a labeled inference', () => {
  const input = story(), claim = input.chapters[0].claims[0];
  claim.evidence = [{ path: 'README.md', startLine: 1, endLine: 1, quote: 'The cache is fast.' }];
  assert.throws(() => validateStory(input, snapshot), /documentation source/);
  claim.basis = 'inference'; assert.equal(validateStory(input, snapshot), input);
});
test('invalid edges, duplicate nodes, and missing step references are rejected', () => {
  for (const change of [scene => scene.edges[0].to = 'unknown', scene => scene.nodes[1].id = 'a', scene => scene.steps[1].nodeId = 'unknown']) {
    const input = story(); change(input.chapters[0].scene); assert.throws(() => validateStory(input, snapshot));
  }
});
test('timeline handles both boundaries and escaped scene content is not executable markup', () => {
  assert.equal(activeStep(chapter.scene, 0), 0); assert.equal(activeStep(chapter.scene, 1), 1);
  const input = structuredClone(chapter); input.scene.nodes[0].label = '<script>alert(1)</script>';
  const svg = renderScene(input, .5); assert.ok(!svg.includes('<script>')); assert.ok(svg.includes('&lt;script&gt;'));
  assert.equal(svg, renderScene(input, .5)); assert.notEqual(svg, renderScene(input, 0));
});
test('every scene template produces a valid rasterizable image in desktop and compact layouts', async () => {
  for (const type of sceneTypes) for (const compact of [false, true]) {
    const input = structuredClone(chapter); input.scene.type = type;
    const png = await sharp(Buffer.from(renderScene(input, .5, { compact }))).resize(640).png().toBuffer();
    assert.ok(png.length > 500);
  }
});
