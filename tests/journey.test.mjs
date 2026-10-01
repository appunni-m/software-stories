import test from 'node:test';
import assert from 'node:assert/strict';
import { journeyEntries, chapterTrail, Exploration } from '../src/shared/journey.mjs';
import { searchSources } from '../src/server/repository.mjs';
import { buildPrompt } from '../src/server/agent.mjs';
test('explored chapters sit under their parent with a path back',()=>{
 const chapters=[{id:'overview'},{id:'second'},{id:'child',parentId:'overview'},{id:'detail',parentId:'child'}];
 assert.deepEqual(journeyEntries(chapters).map(({chapter,depth})=>[chapter.id,depth]),[['overview',0],['child',1],['detail',2],['second',0]]);
 assert.deepEqual(chapterTrail(chapters,'detail').map(c=>c.id),['overview','child','detail']);
 const broken=[{id:'one',parentId:'two'},{id:'two',parentId:'one'},{id:'orphan',parentId:'missing'}];
 assert.equal(journeyEntries(broken).length,3);
 assert.equal(chapterTrail(broken,'one').length,2);
});
test('exploration blocks double clicks through loading, playback, and a paused detail',()=>{
 const visit=new Exploration('overview');visit.current.progress=.43;
 const token=visit.begin('Repository');
 assert.equal(visit.begin('Second click'),null);assert.equal(visit.stack.length,1);
 assert.equal(visit.arrive(token,'repository'),true);visit.start();
 assert.equal(visit.locked,true);visit.pause();assert.equal(visit.locked,true);
 visit.complete();assert.equal(visit.locked,false);
 const nested=visit.begin('Snapshot');visit.arrive(nested,'snapshot');visit.start();
 assert.equal(visit.stack.length,2);
 assert.deepEqual(visit.back(),{id:'repository',progress:1});
 assert.deepEqual(visit.back(),{id:'overview',progress:.43});
 assert.equal(visit.locked,false);
});
test('Back detaches an outstanding request, so a late response cannot steal focus',()=>{
 const visit=new Exploration('overview');visit.current.progress=.6;
 const token=visit.begin('A slow explanation');
 assert.deepEqual(visit.back(),{id:'overview',progress:.6});
 const next=visit.begin('A different explanation');
 assert.equal(visit.arrive(token,'late-answer'),false);
 assert.equal(visit.arrive(next,'new-answer'),true);
 assert.equal(visit.current.id,'new-answer');assert.equal(visit.stack.length,1);
});
test('focused retrieval prioritizes the clicked implementation and includes import context',()=>{
 const snapshot={files:[...Array.from({length:15},(_,i)=>({path:`notes${i}.md`,kind:'documentation',text:'responsibility inputs outputs source question '.repeat(30)})),{path:'src/repository.mjs',kind:'implementation',text:"import {hash} from './identity.mjs';\nexport function captureRepository(path) { return hash(path); }"},{path:'src/identity.mjs',kind:'implementation',text:'export const hash = input => input;'}]};
 const result=searchSources(snapshot,'Explain responsibility inputs outputs source question',8,{focusLabel:'Repository'});
 assert.equal(result[0].path,'src/repository.mjs');
 assert.ok(result.some(file=>file.path==='src/identity.mjs'));
});
test('a late responsibility definition is retrieved before generic matches fill the excerpt',()=>{
 const snapshot={files:[{path:'src/render.mjs',kind:'implementation',text:('const scene = "generic scene";\n').repeat(350)+'export function renderScene() {\n return "the actual renderer";\n}'}]};
 const [source]=searchSources(snapshot,'Explain the scene in this code',8,{focusLabel:'Scene renderer'});
 assert.match(source.excerpt,/351: export function renderScene/);
 assert.match(source.excerpt,/352:  return "the actual renderer"/);
 assert.match(source.excerpt,/intervening lines omitted/);
});
test('a file walkthrough includes the requested file even when lexical ranking misses it',()=>{
 const snapshot={id:'snapshot',name:'example',files:[...Array.from({length:12},(_,i)=>({path:`hot${i}.mjs`,text:'request '.repeat(20),kind:'implementation'})),{path:'quiet.mjs',text:'export const purpose = 42;',kind:'implementation',sha256:'hash',revision:'saved'}]};
 const prompt=buildPrompt(snapshot,{query:'request',mode:'follow-up',filePath:'quiet.mjs'});
 assert.match(prompt,/FILE WALKTHROUGH of quiet.mjs/);
 assert.match(prompt,/1: export const purpose = 42;/);
 assert.throws(()=>buildPrompt(snapshot,{query:'request',filePath:'unknown.mjs'}),/not in this snapshot/);
});
