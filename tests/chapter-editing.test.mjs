import test from 'node:test';
import assert from 'node:assert/strict';
import {removeClaimAndReferences,replaceCitation} from '../src/lib/chapterEditing.ts';
import {sceneRevision} from './helpers/revision-history-fixture.mjs';
import {validScenePresentation} from '../src/lib/scenePresentation.ts';

function section(){
  const s=sceneRevision().sections[0];
  const first=s.claims[0],second=structuredClone(first);second.claimId='claim:second';second.prose='Another route returns.';
  second.anchors[0]={...second.anchors[0],anchorId:'anchor:second',path:'src/second.ts',symbolId:'symbol:second',excerpt:'return second();'};
  second.dependencies=[{kind:'sourceFile',path:'src/second.ts',sourceSha256:second.anchors[0].sourceSha256}];
  first.dependencies=[{kind:'sourceFile',path:first.anchors[0].path,sourceSha256:first.anchors[0].sourceSha256}];
  s.claims.push(second);s.dependencies=[...first.dependencies,...second.dependencies];
  s.visual.nodes[1]={...s.visual.nodes[1],evidenceAnchorIds:['anchor:second'],hotspotId:'hotspot:second'};
  s.visual.edges[0].evidenceAnchorIds=['anchor:route'];
  s.hotspots.push({hotspotId:'hotspot:second',label:'Open second',destination:{kind:'codeLens',anchorId:'anchor:second'}});
  s.visual.presentation.scenes.push({claimIndex:1,title:'Second step',narration:'The second route returns its own value.',mode:'composition',elements:[
    {kind:'panel',parentIndex:null,nodeIndex:null,tone:'neutral',value:'',valueOrigin:'none'},
    {kind:'code',parentIndex:0,nodeIndex:1,tone:'accent',value:'return second();',valueOrigin:'source'},
  ]});
  return s;
}

test('claim removal preserves the unrelated authored scene while rebasing every affected cue',()=>{
  const s=section(),before=JSON.stringify(s);assert.ok(validScenePresentation(s.visual.presentation,s.visual.nodes,s.claims));
  const next=removeClaimAndReferences(s,'claim:entry');
  assert.equal(next.claims.length,1);assert.equal(next.claims[0].claimId,'claim:second');
  assert.deepEqual(next.hotspots,[s.hotspots[1]]);assert.deepEqual(next.dependencies,s.claims[1].dependencies);
  assert.deepEqual(next.visual.nodes,[{...s.visual.nodes[1]}]);assert.deepEqual(next.visual.edges,[]);
  assert.equal(next.visual.presentation.scenes.length,1);assert.equal(next.visual.presentation.scenes[0].claimIndex,0);
  assert.equal(next.visual.presentation.scenes[0].elements[1].nodeIndex,0);
  assert.equal(next.visual.presentation.scenes[0].narration,s.visual.presentation.scenes[1].narration);
  assert.ok(validScenePresentation(next.visual.presentation,next.visual.nodes,next.claims));assert.equal(JSON.stringify(s),before);
});

test('the last claim, unknown claims and independently shared source evidence are preserved',()=>{
  const s=section();assert.equal(removeClaimAndReferences(s,'missing'),s);
  const one=removeClaimAndReferences(s,'claim:entry');assert.equal(removeClaimAndReferences(one,'claim:second'),one);
  s.claims[1].anchors.push({...s.claims[0].anchors[0]});
  s.claims[1].dependencies.push(...s.claims[0].dependencies);
  const next=removeClaimAndReferences(s,'claim:entry');
  assert.ok(next.hotspots.some(h=>h.hotspotId==='hotspot:route'));assert.equal(next.visual.nodes.length,2);
  assert.deepEqual(next.dependencies,s.dependencies);
});

test('citation replacement preserves stable links and narration, repairs direct dependencies, and keeps relationships explicit',()=>{
  const s=section(),old=s.claims[0].anchors[0];
  s.claims[0].dependencies.push({kind:'symbol',path:old.path,sourceSha256:old.sourceSha256,symbolId:old.symbolId});
  const relationship={kind:'relationship',path:old.path,sourceSha256:old.sourceSha256,relationshipId:'relationship:independent'};
  s.claims[0].dependencies.push(relationship);s.dependencies.push(relationship);
  const replacement={...old,path:'src/new.ts',sourceSha256:'f'.repeat(64),symbolId:null,startLine:8,endLine:9,excerpt:'new captured code'};
  const next=replaceCitation(s,old.anchorId,replacement);
  assert.equal(next.claims[0].anchors[0],replacement);assert.deepEqual(next.claims[0].dependencies,[{kind:'sourceFile',path:replacement.path,sourceSha256:replacement.sourceSha256},relationship]);
  assert.deepEqual(next.claims[1],s.claims[1]);assert.deepEqual(next.visual,s.visual);assert.deepEqual(next.hotspots,s.hotspots);
  assert.equal(next.prose,s.prose);assert.equal(next.spokenText,s.spokenText);assert.ok(next.dependencies.some(d=>d.relationshipId===relationship.relationshipId));
  assert.equal(replaceCitation(s,'missing',replacement),s);assert.equal(replaceCitation(s,old.anchorId,{...replacement,anchorId:'other'}),s);
});

test('repairing one citation does not rewrite another claim’s still-required dependency',()=>{
  const s=section(),old=s.claims[0].anchors[0];
  s.claims[1].anchors.push({...old,anchorId:'anchor:shared'});
  const replacement={...old,path:'src/new.ts',sourceSha256:'f'.repeat(64)};
  const next=replaceCitation(s,old.anchorId,replacement);
  assert.ok(next.dependencies.some(d=>d.path===old.path&&d.sourceSha256===old.sourceSha256));
  assert.ok(next.dependencies.some(d=>d.path===replacement.path&&d.sourceSha256===replacement.sourceSha256));
  assert.equal(next.claims[1].anchors.at(-1).path,old.path);
});

test('unavailable malformed visual payloads remain reviewable instead of crashing claim removal',()=>{
  for(const visual of [true,{schemaVersion:2,nodes:null,edges:[]},{schemaVersion:2,nodes:[null],edges:[]},{schemaVersion:2,nodes:[{}],edges:[]},{schemaVersion:2,nodes:[],edges:[],presentation:{scenes:[null]}}]){
    const s=section();s.visual=visual;const next=removeClaimAndReferences(s,'claim:entry');
    assert.equal(next.claims.length,1);assert.equal(next.visual,visual,'unavailable payload remains for native validation');
  }
});
