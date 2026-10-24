import test from 'node:test';
import assert from 'node:assert/strict';
import { createWorkspace, addTurn, canContinueRepositoryGuide, guideContinuationOperations, contextualGuideIntent, contextualGuideAction, isRepositoryGuideIntent, repositoryGuideOmissionQuestion, repositoryGuideOmissionSummary, repositoryGuideTimeoutRecovery, historyEntry, discardUnstartedGuide, mergeRepositoryGuide, parseWorkspace } from '../src/workspace/model.ts';
const guideId='ea6054f9-c7ce-4489-a928-b48697371565';
const runId='c9410fba-7b55-42c5-b6e2-061c538bfcf2';
const generationId='generation:776b022e-d466-455b-8594-d8a808e20f58';
const section={runId,key:'input',order:0,generationId,revisionId:'revision:one',supersedes:null,contextElapsedMs:15,runElapsedMs:20};
const guide=()=>({revision:1,id:guideId,projectId:'p',snapshotId:'s',question:'How does this project work?',title:'The input journey',status:'building',
  openingRunId:null,openingRunIds:[],canContinue:false,continuationOperations:0,
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
test('playback-paced guides retain their waiting state and explain when another slide is coming',()=>{
  const metadata={...guide(),status:'waitingForPlayback',canContinue:true};
  let state=mergeRepositoryGuide(createWorkspace('p','s'),metadata);
  let turn=state.explorations[0].turns[0];
  assert.equal(turn.status,'waiting');
  assert.match(historyEntry(turn,new Set()).text,/More slides will be ready as you listen/u);
  state=parseWorkspace(JSON.stringify(state),'p','s');
  turn=state.explorations[0].turns[0];
  assert.equal(turn.status,'waiting');
  assert.equal(turn.guide,undefined,'native guide state is reloaded instead of trusting a stale browser copy');
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
  assert.equal(contextualGuideIntent('   ',[],false),'How does this project work, from its entry points through its main behavior and failure paths?');
});
test('explicit project-wide guide asks use the durable pipeline from every exploration',()=>{
  for(const question of ['Explain this project','How does this repository work?','Explain how this repository works','Describe how the project is structured','Give me an overview of the codebase','Walk me through this app end to end'])
    assert.equal(isRepositoryGuideIntent(question),true,question);
  for(const question of ['Build a guide for this project','Give me a tour of the whole repository','Walk me through the app end to end','Explain this repository'])
    assert.equal(isRepositoryGuideIntent(question),true,question);
  for(const question of ['Generate it for the Fontdone project entirely','Explain every part of this project','Explain each file in the repository','Describe all of this codebase'])
    assert.equal(isRepositoryGuideIntent(question),true,question);
  for(const question of ['Explain what this project does','What does this repository do?','Tell me what the codebase provides'])
    assert.equal(isRepositoryGuideIntent(question),true,question);
  for(const question of [
    'Create exactly one new Guide section (one teaching scene, not a repository overview) about TableDirectory::find in src/tt/mod.rs.',
    'Explain only one function in this repository, not a full project guide.',
    'Create one section about a single branch from the repository.',
  ]) assert.equal(isRepositoryGuideIntent(question),false,question);
  assert.equal(isRepositoryGuideIntent('Give me a full guide for this project'),true);
  assert.equal(isRepositoryGuideIntent('Explain this function'),false);
  assert.equal(isRepositoryGuideIntent('Why does parsing default this field?'),false);
});
test('an old timed-out guide points to a fresh incremental guide without changing saved answers',()=>{
  const message='The Codex operation exceeded ten minutes, including result validation. No late response was published.';
  const recovery=repositoryGuideTimeoutRecovery({guideId,question:'How does this project work?',error:message});
  assert.match(recovery,/Start a new guide/);
  assert.match(recovery,/saved outline and continue slide by slide/);
  assert.match(recovery,/Any saved slides remain available/);
  const broadAnswerRecovery=repositoryGuideTimeoutRecovery({question:'Build a guide for this project',error:message});
  assert.match(broadAnswerRecovery,/A new guide will save an outline first/);
  assert.match(broadAnswerRecovery,/Your original question stays here/);
  assert.equal(repositoryGuideTimeoutRecovery({question:'Explain this function',error:message}),null);
  assert.equal(repositoryGuideTimeoutRecovery({question:'How does this project work?',error:'Codex account access expired.'}),null);
});
test('root Guide me always starts the incremental repository guide while scoped actions stay contextual',()=>{
  const projectQuestion='How does this project work, from its entry points through its main behavior and failure paths?';
  assert.deepEqual(contextualGuideAction('   ',[],false),{kind:'repository-guide',intent:projectQuestion});
  assert.deepEqual(contextualGuideAction('Guide me through the main project flow',[],false),{
    kind:'repository-guide',intent:'Guide me through the main project flow',
  });
  assert.deepEqual(contextualGuideAction('',[{path:'src/parser.rs',from:8,to:20}],false),{
    kind:'ask',intent:'Guide me through this selected code, its decisions and consequences.',
  });
  assert.deepEqual(contextualGuideAction('Explain bounds',[{path:'src/parser.rs',from:8,to:20}],false),{
    kind:'ask',intent:'Explain bounds',
  });
  assert.deepEqual(contextualGuideAction('',[],true),{
    kind:'ask',intent:'Guide me through this topic, its decisions and consequences.',
  });
});

test('finished guides turn bounded omissions into a focused follow-up question',()=>{
  const question=repositoryGuideOmissionQuestion(['C ABI exports and errors','WASM wrapper routing','C ABI exports and errors']);
  assert.match(question,/C ABI exports and errors; WASM wrapper routing/);
  assert.match(question,/find the relevant implementation in the captured source/);
  assert.match(question,/do not repeat sections already covered/);
  assert.equal(repositoryGuideOmissionQuestion([' ', '\n\t']),null);
  const many=repositoryGuideOmissionQuestion(Array.from({length:10},(_,index)=>`topic ${index}`));
  assert.match(many,/topic 7/);
  assert.doesNotMatch(many,/topic 8/);
});

test('partial guide coverage note names omissions concisely',()=>{
  assert.equal(repositoryGuideOmissionSummary([' C ABI exports and errors ','WASM wrapper routing','C ABI exports and errors']),
    'C ABI exports and errors · WASM wrapper routing');
  assert.equal(repositoryGuideOmissionSummary(['Not yet covered: C ABI input','Not yet covered: WASM routes']),
    'C ABI input · WASM routes');
  assert.equal(repositoryGuideOmissionSummary(['']),null);
  assert.equal(repositoryGuideOmissionSummary(Array.from({length:6},(_,index)=>`topic ${index}`)),
    'topic 0 · topic 1 · topic 2 · and 3 more');
  const long=repositoryGuideOmissionSummary([`topic ${'x'.repeat(100)}`]);
  assert.ok(long.length<=84);
  assert.ok(long.endsWith('…'));
});

test('a completed guide exposes omissions as a nested source exploration',async()=>{
  const {readFileSync}=await import('node:fs');
  const source=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  assert.match(source,/repositoryGuideOmissionQuestion\(turn\.guide\?\.omissions\?\?\[\]\)/);
  assert.match(source,/selectedGuideSlide===guideSlides\.length-1/);
  assert.match(source,/branchLabel=\{omissionQuestion\?'Explore uncovered topics'/);
  assert.match(source,/coverageSummary=\{omissionSummary\?\?undefined\}/);
  assert.match(source,/className="dw-guide-coverage" role="status"><strong>Partial guide<\/strong><span>Still uncovered:/);
  assert.match(source,/ask\(question,current\.view==='flow'\?'flow':'guide',turn\.context,turn\.id,true\)/);
  assert.match(source,/!keepQuestionScoped&&isRepositoryGuideIntent\(clean\)/);
});

test('an opening answer belongs to the guide before its outline exists and survives restart',()=>{
  const opening={...section,runId:guideId};
  const planning={...guide(),revision:0,status:'planning',openingRunId:guideId,openingRunIds:[guideId],units:[],
    answers:[{runId:guideId,projectId:'p',snapshotId:'s',question:'How does this project work?',outcome:'working',sections:[opening],incompleteTopics:[]}]};
  let state=mergeRepositoryGuide(createWorkspace('p','s'),planning);
  const turn=state.explorations[0].turns[0];
  assert.deepEqual(turn.runIds,[guideId]);assert.deepEqual(turn.publishedSections,[opening]);
  assert.equal(turn.status,'working');assert.equal(turn.guide.units.length,0);
  state=parseWorkspace(JSON.stringify(state),'p','s');
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening]);
  const outline={...guide(),openingRunId:guideId,openingRunIds:[guideId],answers:[...planning.answers,...guide().answers]};
  state=mergeRepositoryGuide(state,outline);
  assert.equal(state.explorations[0].turns.length,1);
  assert.deepEqual(state.explorations[0].turns[0].runIds,[guideId,runId]);
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening,section]);
  const stopped={...outline,revision:3,status:'stopped'};
  state=mergeRepositoryGuide(state,stopped);
  assert.equal(mergeRepositoryGuide(state,planning),state);
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening,section]);
});

test('an independently scoped opening preserves the original guide card question and section ownership',()=>{
  const original='How do font bytes become bitmap pixels, including alternate formats and errors?';
  const opening={...section,runId:guideId};
  const planning={...guide(),question:original,revision:0,status:'planning',openingRunId:guideId,openingRunIds:[guideId],units:[],
    answers:[{runId:guideId,projectId:'p',snapshotId:'s',question:'How does the actual first input transition return or propagate a result?',outcome:'partial',sections:[opening],incompleteTopics:['A guard remains unresolved.']}]};
  let state=mergeRepositoryGuide(createWorkspace('p','s'),planning);
  assert.equal(state.explorations[0].turns[0].question,original);
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening]);
  state=parseWorkspace(JSON.stringify(state),'p','s');
  assert.equal(state.explorations[0].turns[0].question,original);
  assert.deepEqual(state.explorations[0].turns[0].runIds,[guideId]);
  const deeper={...guide(),question:original,openingRunId:guideId,openingRunIds:[guideId],answers:[...planning.answers,...guide().answers]};
  state=mergeRepositoryGuide(state,deeper);
  assert.equal(state.explorations[0].turns.length,1);
  assert.equal(state.explorations[0].turns[0].question,original);
  assert.deepEqual(state.explorations[0].turns[0].publishedSections,[opening,section]);
});

test('an opening follow-up retains both owners, earlier sections and the reader position',()=>{
  const followUp='5af0e9f9-b14a-450e-8156-277b9c4b7632';
  const opening={...section,runId:guideId};
  const original={...guide(),openingRunId:guideId,openingRunIds:[guideId],canContinue:true,continuationOperations:1,
    answers:[{...guide().answers[0],runId:guideId,sections:[opening]}]};
  let state=mergeRepositoryGuide(createWorkspace('p','s'),original);
  const position={revisionId:opening.revisionId,sectionId:'section:kept'};
  state.explorations[0].turns[0].reading={guide:position};
  state.explorations[0].view='flow';
  const continued={...original,revision:8,openingRunId:followUp,openingRunIds:[guideId,followUp],canContinue:false,continuationOperations:0,
    answers:[...original.answers,{...guide().answers[0],runId:followUp,sections:[]}]};
  state=mergeRepositoryGuide(state,continued);
  const turn=state.explorations[0].turns[0];
  assert.deepEqual(turn.runIds,[guideId,followUp,runId]);
  assert.deepEqual(turn.publishedSections,[opening]);
  assert.deepEqual(turn.reading.guide,position);
  assert.equal(state.explorations[0].view,'flow');
  assert.equal(turn.guide.canContinue,false);
  const reopened=parseWorkspace(JSON.stringify(state),'p','s');
  assert.deepEqual(reopened.explorations[0].turns[0].runIds,[guideId,followUp,runId]);
  assert.deepEqual(reopened.explorations[0].turns[0].publishedSections,[opening]);
  assert.deepEqual(reopened.explorations[0].turns[0].reading.guide,position);
});

test('continuation uses the explicit current opening allowance after all deeper units complete',()=>{
  const complete=guide();complete.units.forEach((unit)=>{unit.status='complete';});
  assert.equal(canContinueRepositoryGuide({...complete,canContinue:true,continuationOperations:1}),true);
  assert.equal(guideContinuationOperations({...complete,continuationOperations:1}),1);
  assert.equal(canContinueRepositoryGuide({...complete,canContinue:false,continuationOperations:0}),false);
  assert.equal(canContinueRepositoryGuide(complete),false);
  assert.equal(guideContinuationOperations({...guide(),continuationOperations:99}),0);
  assert.equal(canContinueRepositoryGuide({...complete,canContinue:undefined}),false);
  assert.equal(canContinueRepositoryGuide(undefined),false);
});
