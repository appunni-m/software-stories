import test from 'node:test';
import assert from 'node:assert/strict';
import { createWorkspace, addTurn, contextualGuideIntent, discardUnstartedGuide, mergeRepositoryGuide, parseWorkspace } from '../src/workspace/model.ts';
const guideId='ea6054f9-c7ce-4489-a928-b48697371565';
const runId='c9410fba-7b55-42c5-b6e2-061c538bfcf2';
const generationId='generation:776b022e-d466-455b-8594-d8a808e20f58';
const section={runId,key:'input',order:0,generationId,revisionId:'revision:one',supersedes:null,contextElapsedMs:15,runElapsedMs:20};
const guide=()=>({revision:1,id:guideId,projectId:'p',snapshotId:'s',question:'How does this project work?',title:'The input journey',status:'building',
  units:[{key:'input',question:'How does input arrive?',prerequisites:[],status:'working',runIds:[runId]},
    {key:'output',question:'How does output leave?',prerequisites:['input'],status:'pending',runIds:[]}],
  answers:[{runId,projectId:'p',snapshotId:'s',question:'How does input arrive?',outcome:'working',sections:[section],incompleteTopics:[]}],
  omissions:['Execution was not observed.'],capturedFiles:3500,startingFiles:4});
test('independent guide runs form one durable card without moving the reader',()=>{
  let state=createWorkspace('p','s');state=addTurn(state,{id:'question',guideId,question:'How does this project work?',context:[],status:'queued'},'flow');
  const position={revisionId:'revision:one',sectionId:'section:one'};state.explorations[0].turns[0].reading={guide:position};
  state=mergeRepositoryGuide(state,guide());state=mergeRepositoryGuide(state,guide());
  assert.equal(state.explorations[0].turns.length,1);assert.equal(state.explorations[0].view,'flow');
  const turn=state.explorations[0].turns[0];assert.equal(turn.id,'question');assert.equal(turn.status,'working');
  assert.deepEqual(turn.reading.guide,position);assert.deepEqual(turn.publishedSections,[section]);assert.deepEqual(turn.notes,[]);
  const completed=guide();completed.status='ready';state=mergeRepositoryGuide(state,completed);assert.equal(state.explorations[0].turns[0].status,'ready');
});
test('native guide recovery reconstructs a lost UI card without dispatching or changing a nested tab',()=>{
  let state=createWorkspace('p','s');state=addTurn(state,{id:'other',question:'Another question',context:[],status:'ready'},'history');
  state=addTurn(state,{id:'child-turn',question:'A nested topic',context:[],status:'ready'},'flow',{id:'child',originTurnId:'other'});
  state=mergeRepositoryGuide(state,guide());assert.equal(state.currentId,'child');assert.equal(state.explorations[1].view,'flow');
  assert.equal(state.explorations[0].turns.length,2);assert.equal(state.explorations[0].turns[1].guideId,guideId);
  assert.equal(mergeRepositoryGuide(state,{...guide(),projectId:'wrong'}),state);
});
test('restart keeps guide identity and saved cards while discarding stale job metadata',()=>{
  const state=mergeRepositoryGuide(createWorkspace('p','s'),guide());
  const restored=parseWorkspace(JSON.stringify(state),'p','s').explorations[0].turns[0];
  assert.equal(restored.guideId,guideId);assert.equal(restored.status,'interrupted');assert.equal(restored.guide,undefined);
  assert.deepEqual(restored.publishedSections,[section]);
});
test('a unit finishing does not mark the whole guide ready or duplicate prior sections',()=>{
  let state=mergeRepositoryGuide(createWorkspace('p','s'),guide());const next=guide();
  const secondRun='dd5c1a91-b35f-4d97-a841-21321e2499d1';
  next.units[0].status='complete';next.units[1].status='working';next.units[1].runIds=[secondRun];
  next.answers[0].outcome='complete';next.answers.push({runId:secondRun,projectId:'p',snapshotId:'s',question:'How does output leave?',outcome:'working',sections:[],incompleteTopics:[]});
  state=mergeRepositoryGuide(state,next);const turn=state.explorations[0].turns[0];
  assert.equal(turn.status,'working');assert.deepEqual(turn.runIds,[runId,secondRun]);assert.equal(turn.publishedSections.length,1);
});
test('late progress cannot replace a newer stop or completion receipt',()=>{
  let state=mergeRepositoryGuide(createWorkspace('p','s'),{...guide(),revision:4,status:'ready'});
  const previous=state;state=mergeRepositoryGuide(state,guide());assert.equal(state,previous);
  assert.equal(state.explorations[0].turns[0].status,'ready');
});
test('canceling an unstarted guide review removes only its empty card',()=>{
  let state=createWorkspace('p','s');state=addTurn(state,{id:'old',question:'Earlier question',context:[],status:'ready'},'flow');
  state=addTurn(state,{id:'new',question:'New guide',guideId,context:[],status:'queued'},'guide');
  const canceled=discardUnstartedGuide(state,'new');assert.deepEqual(canceled.explorations[0].turns.map((t)=>t.id),['old']);
  assert.equal(canceled.currentId,state.currentId);
  state.explorations[0].turns[1].status='working';assert.equal(discardUnstartedGuide(state,'new'),state);
  state.explorations[0].turns[1].status='queued';state.explorations[0].turns[1].runIds=[runId];assert.equal(discardUnstartedGuide(state,'new'),state);
});
test('contextual guide keeps explicit source and nested intent',()=>{
  assert.equal(contextualGuideIntent('Explain bounds',[],false),'Explain bounds');
  assert.match(contextualGuideIntent('',[{path:'src/parser.rs',from:8,to:20}],false),/selected code/);
  assert.match(contextualGuideIntent('',[],true),/this topic/);
  assert.equal(contextualGuideIntent('   ',[],false),null);
});
test('an opening answer belongs to the guide before its outline exists and survives restart',()=>{
  const opening={...section,runId:guideId};
  const planning={...guide(),revision:0,status:'planning',openingRunId:guideId,units:[],
    answers:[{runId:guideId,projectId:'p',snapshotId:'s',question:'How does this project work?',outcome:'working',sections:[opening],incompleteTopics:[]}]};
  let state=mergeRepositoryGuide(createWorkspace('p','s'),planning);
  const turn=state.explorations[0].turns[0];
  assert.deepEqual(turn.runIds,[guideId]);assert.deepEqual(turn.publishedSections,[opening]);
  assert.equal(turn.status,'working');assert.equal(turn.guide.units.length,0);
  state=parseWorkspace(JSON.stringify(state),'p','s');
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening]);
  const outline={...guide(),openingRunId:guideId,answers:[...planning.answers,...guide().answers]};
  state=mergeRepositoryGuide(state,outline);
  assert.equal(state.explorations[0].turns.length,1);
  assert.deepEqual(state.explorations[0].turns[0].runIds,[guideId,runId]);
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening,section]);
  const stopped={...outline,revision:3,status:'stopped'};
  state=mergeRepositoryGuide(state,stopped);
  assert.equal(mergeRepositoryGuide(state,planning),state);
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening,section]);
});
