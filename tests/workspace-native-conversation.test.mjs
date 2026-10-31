import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {FLOW_DIAGRAM_LAYOUT_VERSION} from '../src/lib/readerProgress.ts';
import {createWorkspace,addTurn,decodeWorkspace,rememberCodePane,rememberFileFlow,rememberPresentation,rememberSourceContext} from '../src/workspace/model.ts';
import {workspaceStoreHarness,deferred,flushLocalReads} from './helpers/workspace-store-harness.mjs';
import {rememberAnswerVersion,answerReadingPosition,resolveAnswerRevision} from '../src/workspace/answerVersions.ts';

const clone=value=>JSON.parse(JSON.stringify(value));
const runId=n=>`00000000-0000-4000-8000-${String(n).padStart(12,'0')}`;
const browserWorkspaceKey=(p='project',s='snapshot')=>`code-atlas:workspace:v1:${p}:${s}`;

test('native conversation validation accepts the reader pane’s current Flow layout',async()=>{
  const native=await readFile(new URL('../src-tauri/src/workspace_conversation.rs',import.meta.url),'utf8');
  assert.match(native,new RegExp(`const FLOW_DIAGRAM_LAYOUT_VERSION: u64 = ${FLOW_DIAGRAM_LAYOUT_VERSION};`));
});

function backend(){
  const records=new Map(),writes=[];const controls={load:null,beforeSave:null,afterSave:null};
  return {records,writes,controls,invoke:async(name,args)=>{
    const key=JSON.stringify([args.projectId,args.snapshotId]);
    if(name==='load_workspace_conversation'){if(controls.load)return controls.load(args);return clone(records.get(key)??null);}
    assert.equal(name,'save_workspace_conversation','restoring conversations cannot invoke agent/indexing commands');
    writes.push(clone(args));if(controls.beforeSave)await controls.beforeSave(args);
    let record=records.get(key);
    if(!record||JSON.stringify(record.state)!==JSON.stringify(args.state)){
      if(args.expectedRevision!==(record?.revision??null))throw new Error('Another window saved this conversation. Your unsaved changes have been preserved in this window.');
      record={schemaVersion:1,revision:(record?.revision??0)+1,state:clone(args.state)};records.set(key,record);
    }
    if(controls.afterSave)await controls.afterSave(args);return clone(record);
  }};
}
const scope=JSON.stringify(['project','snapshot']);
const native=(b,storage=new Map())=>workspaceStoreHarness(b.invoke,{native:true,storage});

test('a selected accepted edit and each version position survive the actual wire snapshot and native-only restart',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');
  const original={projectId:'project',snapshotId:'snapshot',chapterId:'chapter:one',target:{targetId:'target:one',scope:{kind:'overview'}},revisionId:'revision:original',revisionNumber:1,basedOnRevisionId:null,lifecycle:'draft',validationStatus:'valid',sections:[{sectionId:'section:one'}]};
  const edit={...original,revisionId:'revision:edited',revisionNumber:2,basedOnRevisionId:original.revisionId,lifecycle:'accepted'};
  const readerId=runId(1);
  let s=addTurn(h.store.getWorkspace('project','snapshot'),{id:readerId,question:'What happens?',context:[],status:'ready',revisionId:original.revisionId,reading:{guide:{revisionId:original.revisionId,sectionId:'section:original-last'}}},'guide');
  s=rememberAnswerVersion(s,'root',readerId,original.revisionId,edit.revisionId,[original,edit]);
  s.explorations[0].turns[0].answerVersions[1].html='must not persist';
  h.store.updateWorkspace('project','snapshot',()=>s);await h.store.flushAllWorkspaceConversations();
  assert.doesNotMatch(JSON.stringify(b.records.get(scope)),/must not persist/);
  const reopened=native(b);await reopened.store.prepareWorkspaceConversation('project','snapshot');
  const turn=reopened.store.getWorkspace('project','snapshot').explorations[0].turns[0];
  assert.equal(turn.revisionId,original.revisionId);assert.equal(resolveAnswerRevision(original,turn,[original,edit]).revisionId,edit.revisionId);
  assert.equal(answerReadingPosition(turn,'guide',original.revisionId).sectionId,'section:original-last');
  assert.ok(reopened.calls.every(c=>c.name==='load_workspace_conversation'));
});

test('close waits for startup reads and saves, then resumes deferred edits only on cancellation',async()=>{
  const b=backend(),h=native(b),held=deferred();b.controls.beforeSave=()=>held.promise;
  const opening=h.store.prepareWorkspaceConversation('project','snapshot');let finished=false;
  const barrier=h.store.flushAllWorkspaceConversations().then(()=>{finished=true;});await flushLocalReads();assert.equal(finished,false);
  held.resolve();await opening;await barrier;b.controls.beforeSave=null;
  await h.store.prepareWorkspaceConversation('other','snapshot');
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/current.rs'}));
  h.store.updateWorkspace('other','snapshot',s=>({...s,file:'src/other.rs'}));
  const resume=h.store.beginWorkspaceClose();h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/late.rs'}));
  await h.store.flushAllWorkspaceConversations();assert.equal(b.records.get(scope).state.file,'src/current.rs');
  assert.equal(b.records.get(JSON.stringify(['other','snapshot'])).state.file,'src/other.rs');
  assert.equal(h.store.getWorkspace('project','snapshot').file,'src/current.rs');
  resume();resume();await h.store.flushAllWorkspaceConversations();assert.equal(b.records.get(scope).state.file,'src/late.rs');
});

test('close refuses a blocked write without retry and initial read failure without edits does not trap startup',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');
  b.controls.beforeSave=()=>{throw new Error('controlled save fault');};h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/pending.rs'}));
  await assert.rejects(h.store.flushAllWorkspaceConversations());const count=b.writes.length;
  await assert.rejects(h.store.flushAllWorkspaceConversations());assert.equal(b.writes.length,count);
  const failed=backend();failed.controls.load=()=>{throw new Error('controlled load fault');};const unopened=native(failed);
  await assert.rejects(unopened.store.prepareWorkspaceConversation('project','snapshot'));await unopened.store.flushAllWorkspaceConversations();assert.equal(failed.writes.length,0);
});

test('an unreadable native conversation leaves source navigation usable without replacing saved bytes',async()=>{
  const b=backend(),h=native(b),preserved='unsupported conversation bytes';
  b.controls.load=()=>{throw new Error(preserved);};
  await assert.rejects(h.store.prepareWorkspaceConversation('project','snapshot'));
  h.store.updateWorkspace('project','snapshot',state=>({...state,file:'src/font.rs',explorations:state.explorations.map(e=>e.id==='root'?{...e,file:'src/font.rs',codeOpen:true}:e)}));
  const state=h.store.getWorkspace('project','snapshot');
  assert.equal(state.file,'src/font.rs');assert.equal(state.explorations[0].codeOpen,true);
  await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'));
  assert.equal(b.writes.length,0,'unreadable conversation data is never replaced by the temporary browsing state');
  assert.deepEqual(h.calls.map(call=>call.name),['load_workspace_conversation']);
});

test('a repository opening late cannot start a new read after the close barrier froze scopes',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');
  const resume=h.store.beginWorkspaceClose(),calls=h.calls.length;let opened=false;
  const opening=h.store.prepareWorkspaceConversation('other','snapshot').then(()=>{opened=true;});
  await h.store.flushAllWorkspaceConversations();await flushLocalReads();assert.equal(opened,false);assert.equal(h.calls.length,calls);
  resume();await opening;assert.equal(opened,true);assert.equal(b.records.size,2);
});

test('closing cannot dispatch a newly queued question or repository guide',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');
  const question=h.store.appendQuestion('project','snapshot','Existing question',[],'guide');
  const guide=h.store.appendRepositoryGuide('project','snapshot','Existing guide');await h.store.flushAllWorkspaceConversations();
  const resume=h.store.beginWorkspaceClose();
  assert.equal(await h.store.runQuestion('project','snapshot',question,'grant'),null);
  await h.store.runRepositoryGuide('project','snapshot',guide,'grant');
  assert.ok(h.calls.every(c=>['load_workspace_conversation','save_workspace_conversation'].includes(c.name)));
  resume();assert.ok(h.store.getWorkspace('project','snapshot').explorations[0].turns.every(t=>t.status==='interrupted'));
});

test('closing during account lookup stops dispatch and failed-close recovery leaves an explicit interrupted question',async()=>{
  const b=backend(),lookup=deferred();const h=workspaceStoreHarness(b.invoke,{native:true,ensureCodexConnection:()=>lookup.promise,readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  await h.store.prepareWorkspaceConversation('project','snapshot');const id=h.store.appendQuestion('project','snapshot','Queued before close',[],'guide');
  const running=h.store.runQuestion('project','snapshot',id,'grant');await flushLocalReads();
  const resume=h.store.beginWorkspaceClose();await h.store.flushAllWorkspaceConversations();lookup.resolve();await running;
  assert.ok(h.calls.every(c=>['load_workspace_conversation','save_workspace_conversation','load_workspace_answer'].includes(c.name)));
  resume();assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].status,'interrupted');
});

test('incremental workspace questions use the saved model and reasoning effort',async()=>{
  const b=backend();let generation;
  const h=workspaceStoreHarness(async(name,args)=>{
    if(name==='generate_progressive_workspace_answer'){
      generation=args;
      return {runId:args.input.runId,projectId:args.input.projectId,snapshotId:args.input.snapshotId,question:args.input.question,outcome:'complete',sections:[],incompleteTopics:[]};
    }
    return b.invoke(name,args);
  },{native:true,ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-5.6-luna',effort:'medium'})});
  await h.store.prepareWorkspaceConversation('project','snapshot');
  const id=h.store.appendQuestion('project','snapshot','Explain the subtotal check',[],'guide');
  await h.store.runQuestion('project','snapshot',id,'grant');
  assert.equal(generation.input.modelId,'gpt-5.6-luna');
  assert.equal(generation.input.effort,'medium');
  assert.ok(Number.isInteger(generation.input.rendererQueueWaitMs));
  assert.ok(generation.input.rendererQueueWaitMs>=0&&generation.input.rendererQueueWaitMs<=86_400_000);
});

test('questions persist native queue identity before waiting and a stopped waiter never dispatches',async()=>{
  const b=backend(),firstStarted=deferred(),firstAnswer=deferred(),queued=[],finished=[];let generationCount=0;
  const h=workspaceStoreHarness(async(name,args)=>{
    if(name==='generate_progressive_workspace_answer'){
      generationCount++;
      if(generationCount===1){firstStarted.resolve(args);return firstAnswer.promise;}
      return {runId:args.input.runId,projectId:'project',snapshotId:'snapshot',question:args.input.question,outcome:'complete',sections:[],incompleteTopics:[]};
    }
    if(name==='load_workspace_answer')throw new Error('No saved answer');
    return b.invoke(name,args);
  },{
    native:true,ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'}),
    queueWorkspaceGenerationJob:async(input)=>{queued.push(input);return {jobId:input.jobId,state:'queued'};},
    finishQueuedWorkspaceGenerationJob:async(jobId,projectId,snapshotId,state)=>{finished.push({jobId,state});return {jobId,projectId,snapshotId,state};},
    stopWorkspaceGenerationJob:async(jobId,projectId,snapshotId)=>({jobId,projectId,snapshotId,state:'stopped'}),
  });
  await h.store.prepareWorkspaceConversation('project','snapshot');
  const firstId=h.store.appendQuestion('project','snapshot','First question',[],'guide');
  const first=h.store.runQuestion('project','snapshot',firstId,'grant');await firstStarted.promise;
  const secondId=h.store.appendQuestion('project','snapshot','Second question',[],'guide');
  const second=h.store.runQuestion('project','snapshot',secondId,'grant');await flushLocalReads();
  assert.equal(queued.length,2,'both requests have durable native identities before the second waits');
  assert.notEqual(queued[0].jobId,queued[1].jobId);
  assert.equal(queued[1].conversationTurnId,secondId);
  assert.match(queued[1].rendererSessionId,/^[0-9a-f-]{36}$/i);
  assert.equal(generationCount,1,'the queued question has not reached the Codex worker');
  assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns.find(turn=>turn.id===secondId).status,'queued');
  h.store.stopQuestion('project','snapshot',secondId);await flushLocalReads();
  firstAnswer.resolve({runId:queued[0].jobId,projectId:'project',snapshotId:'snapshot',question:'First question',outcome:'complete',sections:[],incompleteTopics:[]});
  await first;await second;
  assert.equal(generationCount,1,'Stop while waiting prevents inference after the queue slot opens');
  assert.deepEqual(finished,[{jobId:queued[1].jobId,state:'stopped'}]);
  assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns.find(turn=>turn.id===secondId).status,'stopped');
});

test('connection preparation failure finishes its persisted question as failed without dispatch',async()=>{
  const b=backend(),queued=[],finished=[];let generations=0;
  const h=workspaceStoreHarness(async(name,args)=>{
    if(name==='generate_progressive_workspace_answer'){generations++;throw new Error('must not dispatch');}
    if(name==='load_workspace_answer')throw new Error('No saved answer');
    return b.invoke(name,args);
  },{native:true,ensureCodexConnection:async()=>{throw new Error('Connection is unavailable');},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'}),
    queueWorkspaceGenerationJob:async(input)=>{queued.push(input);return {jobId:input.jobId,state:'queued'};},
    finishQueuedWorkspaceGenerationJob:async(jobId,projectId,snapshotId,state)=>{finished.push({jobId,state});return {jobId,projectId,snapshotId,state};}});
  await h.store.prepareWorkspaceConversation('project','snapshot');
  const id=h.store.appendQuestion('project','snapshot','Explain a decision',[],'guide');
  await h.store.runQuestion('project','snapshot',id,'grant');
  assert.equal(queued.length,1);assert.equal(generations,0);
  assert.deepEqual(finished,[{jobId:queued[0].jobId,state:'failed'}]);
  assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].status,'failed');
});

test('late Stop replies cannot overwrite a newer retry on the same question', { timeout: 2000 }, async () => {
  async function verifyLateStop(finishStop) {
    const b=backend(),oldStarted=deferred(),newStarted=deferred(),oldAnswer=deferred(),newAnswer=deferred(),lateStop=deferred();
    let generationCount=0,oldRunId;
    const h=workspaceStoreHarness(async(name,args)=>{
      if(name==='generate_progressive_workspace_answer'){
        generationCount++;
        if(generationCount===1){oldRunId=args.input.runId;oldStarted.resolve();return oldAnswer.promise;}
        newStarted.resolve();return newAnswer.promise;
      }
      if(name==='load_workspace_answer')throw new Error('No saved answer');
      return b.invoke(name,args);
    },{native:true,ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'}),stopWorkspaceGenerationJob:async()=>lateStop.promise});
    await h.store.prepareWorkspaceConversation('project','snapshot');
    const id=h.store.appendQuestion('project','snapshot','Explain the parser decision',[],'guide');
    const first=h.store.runQuestion('project','snapshot',id,'grant');await oldStarted.promise;
    h.store.stopQuestion('project','snapshot',id);await flushLocalReads();
    oldAnswer.resolve({runId:oldRunId,projectId:'project',snapshotId:'snapshot',question:'Explain the parser decision',outcome:'complete',sections:[],incompleteTopics:[]});
    await first;
    assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].status,'ready');
    const retry=h.store.runQuestion('project','snapshot',id,'grant');await newStarted.promise;
    const current=h.store.getWorkspace('project','snapshot').explorations[0].turns[0];
    assert.notEqual(current.runIds.at(-1),oldRunId);
    finishStop(lateStop,oldRunId);
    await flushLocalReads();
    const afterLateStop=h.store.getWorkspace('project','snapshot').explorations[0].turns[0];
    assert.equal(afterLateStop.status,'working');
    assert.equal(afterLateStop.error,undefined);
    assert.notEqual(afterLateStop.runIds.at(-1),oldRunId);
    newAnswer.resolve({runId:afterLateStop.runIds.at(-1),projectId:'project',snapshotId:'snapshot',question:'Explain the parser decision',outcome:'complete',sections:[],incompleteTopics:[]});
    await retry;
  }
  await verifyLateStop((pending,runId)=>pending.resolve({jobId:runId,projectId:'project',snapshotId:'snapshot',state:'stopped'}));
  await verifyLateStop((pending)=>pending.reject(new Error('The old Stop request failed late')));
});

test('repository guide start, resume and refill carry renderer queue wait to native commands',async()=>{
  const b=backend(),held=deferred(),generationStarted=deferred();let now=0,startArgs,resumeArgs,advanceArgs;
  const h=workspaceStoreHarness(async(name,args)=>{
    if(name==='generate_progressive_workspace_answer'){generationStarted.resolve();return held.promise;}
    if(name==='load_workspace_answer')throw new Error('No saved answer');
    if(name==='start_repository_guide'){startArgs=clone(args);throw new Error('controlled after capturing start timing');}
    if(name==='resume_repository_guide'){resumeArgs=clone(args);throw new Error('controlled after capturing resume timing');}
    if(name==='advance_repository_guide'){advanceArgs=clone(args);throw new Error('controlled after capturing refill timing');}
    if(name==='list_repository_guides')return {guides:[],unavailableCount:0};
    return b.invoke(name,args);
  },{native:true,performance:{now:()=>now},ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  await h.store.prepareWorkspaceConversation('project','snapshot');
  const questionId=h.store.appendQuestion('project','snapshot','Hold the shared question queue',[],'guide');
  const question=h.store.runQuestion('project','snapshot',questionId,'grant');await generationStarted.promise;
  const guideTurnId=h.store.appendRepositoryGuide('project','snapshot','Explain the repository journey');
  const starting=h.store.runRepositoryGuide('project','snapshot',guideTurnId,'grant');await flushLocalReads();
  now=125;held.reject(new Error('release controlled queue holder'));await question;await starting;
  assert.equal(startArgs.input.rendererQueueWaitMs,125,'the queued Guide start records time spent behind an active question');
  await h.store.runRepositoryGuide('project','snapshot',guideTurnId,'grant',true);
  assert.ok(Number.isInteger(resumeArgs.rendererQueueWaitMs)&&resumeArgs.rendererQueueWaitMs>=0&&resumeArgs.rendererQueueWaitMs<=86_400_000);
  h.store.updateWorkspace('project','snapshot',state=>({...state,explorations:state.explorations.map(exploration=>({...exploration,turns:exploration.turns.map(turn=>turn.id===guideTurnId?{...turn,guide:{canContinue:true}}:turn)}))}));
  await h.store.advanceRepositoryGuide('project','snapshot',guideTurnId,'consumed-revision');
  assert.ok(Number.isInteger(advanceArgs.rendererQueueWaitMs)&&advanceArgs.rendererQueueWaitMs>=0&&advanceArgs.rendererQueueWaitMs<=86_400_000);
});

test('playing a Guide slide during an active run saves its receipt before the queued refill',async()=>{
  const b=backend(),started=deferred(),startResult=deferred(),receiptSaved=deferred(),refillStarted=deferred();
  let receiptArgs,refillArgs;
  const h=workspaceStoreHarness(async(name,args)=>{
    if(name==='start_repository_guide'){started.resolve();return startResult.promise;}
    if(name==='record_repository_guide_consumption'){
      receiptArgs=clone(args);receiptSaved.resolve();return guideSummary(args.id,args.consumedRevisionId);
    }
    if(name==='advance_repository_guide'){
      refillArgs=clone(args);refillStarted.resolve();return guideSummary(args.id,args.consumedRevisionId);
    }
    if(name==='list_repository_guides')return {guides:[],unavailableCount:0};
    return b.invoke(name,args);
  },{native:true,ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  function guideSummary(id,consumedRevisionId){
    return {revision:1,id,projectId:'project',snapshotId:'snapshot',question:'Explain the repository journey',title:'Repository journey',
      status:'building',openingRunId:null,openingRunIds:[],canContinue:true,canExtend:false,continuationOperations:1,
      consumedSlideRevisions:consumedRevisionId?[consumedRevisionId]:[],units:[],answers:[],omissions:[],capturedFiles:1,startingFiles:1};
  }
  await h.store.prepareWorkspaceConversation('project','snapshot');
  const turnId=h.store.appendRepositoryGuide('project','snapshot','Explain the repository journey');
  const guideId=h.store.getWorkspace('project','snapshot').explorations[0].turns.find(turn=>turn.id===turnId).guideId;
  const running=h.store.runRepositoryGuide('project','snapshot',turnId,'grant');await started.promise;
  h.store.updateWorkspace('project','snapshot',state=>({...state,explorations:state.explorations.map(exploration=>({...exploration,
    turns:exploration.turns.map(turn=>turn.id===turnId?{...turn,guide:{canContinue:true}}:turn)}))}));

  await h.store.advanceRepositoryGuide('project','snapshot',turnId,'slide:one');await receiptSaved.promise;
  assert.deepEqual(receiptArgs,{id:guideId,projectId:'project',snapshotId:'snapshot',consumedRevisionId:'slide:one'});
  assert.equal(refillArgs,undefined,'the active Guide worker owns refill scheduling');

  startResult.resolve(guideSummary(guideId));await running;await refillStarted.promise;
  assert.equal(refillArgs.id,guideId);assert.equal(refillArgs.consumedRevisionId,'slide:one');
});

test('an uncovered-topic Guide is a nested, durable guide under the original question',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');
  const parentId=h.store.appendRepositoryGuide('project','snapshot','How does this project work?');
  const childId=h.store.appendRepositoryGuide('project','snapshot','Explore uncovered topics',parentId);
  const state=h.store.getWorkspace('project','snapshot');
  const parent=state.explorations[0].turns.find(turn=>turn.id===parentId);
  const childExploration=state.explorations.find(exploration=>exploration.originTurnId===parentId);
  assert.ok(parent?.guideId);assert.ok(childExploration);
  assert.notEqual(childExploration.id,'root');assert.equal(childExploration.parentId,'root');
  assert.equal(childExploration.turns.length,1);assert.equal(childExploration.turns[0].id,childId);
  assert.ok(childExploration.turns[0].guideId,'the follow-up uses the outline-and-slide Guide path');
  assert.equal(state.currentId,childExploration.id);
  await h.store.flushAllWorkspaceConversations();
  const saved=b.records.get(scope).state;
  assert.equal(saved.currentId,childExploration.id);
  assert.equal(saved.explorations.find(exploration=>exploration.originTurnId===parentId).turns[0].id,childId);
  assert.ok(h.calls.every(call=>['load_workspace_conversation','save_workspace_conversation'].includes(call.name)),
    'creating the nested child does not start inference until the app dispatches the owned Guide');
});

test('native restart restores owned scenes, times, viewport, pane and expanded static links without browser preferences',async()=>{
  const b=backend(),storage=new Map(),h=native(b,storage);await h.store.prepareWorkspaceConversation('project','snapshot');
  const readerId=runId(2);
  const state=addTurn(h.store.getWorkspace('project','snapshot'),{id:readerId,question:'Follow the parser',context:[],status:'ready',revisionId:'revision:one'},'guide');
  const identity={revisionId:'revision:one',sectionId:'section:two',sectionRevisionId:'section-revision:two'};
  const story={...identity,sceneId:'claim:second',positions:{'claim:first':12.4,'claim:second':3.2},fullMap:true,viewport:{x:-34,y:91,zoom:.6},focusLayoutVersion:4};
  h.store.updateWorkspace('project','snapshot',()=>state);
  h.store.updateWorkspace('project','snapshot',s=>rememberCodePane(s,'root',false));
  h.store.updateWorkspace('project','snapshot',s=>rememberFileFlow(s,'root','src/font.rs',true));
  h.store.updateWorkspace('project','snapshot',s=>rememberPresentation(s,'root',readerId,'stories',story));
  h.store.updateWorkspace('project','snapshot',s=>rememberPresentation(s,'root',readerId,'diagrams',{...identity,viewport:{x:100,y:-500,zoom:1.8},focusedEdgeId:'edge:subtotal',layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION}));
  h.store.updateWorkspace('project','snapshot',s=>({...s,explorations:s.explorations.map(e=>{
    if(e.id!=='root')return e;
    return {...e,turns:e.turns.map(t=>t.id!==readerId?t:{...t,presentation:{...t.presentation,
      stories:t.presentation.stories.map(item=>({...item,html:'must never be copied',autoPlay:true}))}})};
  })}));
  await h.store.flushWorkspaceConversation('project','snapshot');
  const metadata=JSON.stringify(b.records.get(scope));assert.doesNotMatch(metadata,/must never be copied|autoPlay|spokenText/);
  storage.clear();storage.get=()=>{throw new Error('browser unavailable');};
  const reopened=native(b,storage);await reopened.store.prepareWorkspaceConversation('project','snapshot');
  const root=reopened.store.getWorkspace('project','snapshot').explorations[0];
  assert.equal(root.codeOpen,false);assert.deepEqual(clone(root.fileFlow),{path:'src/font.rs',showAll:true});
  assert.equal(root.focusedTurnId,readerId,'restart retains the answer the reader last opened');
  assert.deepEqual(clone(root.turns[0].presentation.stories),[story]);assert.equal(root.turns[0].presentation.diagrams[0].viewport.zoom,1.8);
  assert.equal(root.turns[0].presentation.diagrams[0].focusedEdgeId,'edge:subtotal','native restart restores the selected relationship with the diagram viewport');
  assert.ok(reopened.calls.every(c=>c.name==='load_workspace_conversation'),'restart reads local metadata without model, audio or indexing work');
});

test('the exact Code Lens selection is the saved source reveal after native restart',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');
  const selection={path:'src/tt/glyf.rs',sha256:'a'.repeat(64),from:564,to:583};
  h.store.updateWorkspace('project','snapshot',state=>rememberSourceContext(state,'root',selection));
  await h.store.flushWorkspaceConversation('project','snapshot');
  const reopened=native(b);await reopened.store.prepareWorkspaceConversation('project','snapshot');
  const state=reopened.store.getWorkspace('project','snapshot');
  assert.equal(state.file,selection.path);
  assert.deepEqual(clone(state.explorations[0].reveal),selection);
  assert.ok(reopened.calls.every(call=>call.name==='load_workspace_conversation'),'restoring a selected range must not dispatch generation');
});

test('invalid Code Lens source context cannot replace the current saved reveal',()=>{
  const initial=createWorkspace('project','snapshot');
  const saved=rememberSourceContext(initial,'root',{path:'src/font.rs',sha256:'b'.repeat(64),from:4,to:8});
  for(const invalid of [
    {path:'../outside.rs',from:1,to:2},
    {path:'src/tt/glyf.rs',from:0,to:2},
    {path:'src/tt/glyf.rs',from:4,to:3},
    {path:'src/tt/glyf.rs',from:4,to:5,sha256:'stale'},
  ]) assert.equal(rememberSourceContext(saved,'root',invalid),saved);
});

test('nested conversations retain parent turns and last views while composer drafts stay transient',async()=>{
  const rootTurnId=runId(10);
  let state=createWorkspace('project','snapshot');state=addTurn(state,{id:rootTurnId,question:'Read bytes',context:[{path:'src/font.rs',from:3,to:7}],status:'stopped',failureCode:'stopped',revisionId:'revision-one',reading:{guide:{revisionId:'revision-one',sectionId:'section:two'}}},'guide');
  let parent=rootTurnId;for(let n=1;n<=5;n++){const turnId=runId(10+n);state=addTurn(state,{id:turnId,question:`Depth ${n}`,context:[],status:'stopped'},['guide','flow','history'][n%3],{originTurnId:parent,id:runId(100+n)});parent=turnId;}
  const siblingId=runId(200);
  state={...state,currentId:'root'};state=addTurn(state,{id:runId(20),question:'Sibling',context:[],status:'stopped'},'flow',{originTurnId:rootTurnId,id:siblingId});
  state={...state,explorations:state.explorations.map(e=>e.id===siblingId?{...e,draft:{question:'Explain this later',context:[]}}:e)};
  state=decodeWorkspace(JSON.stringify(state),'project','snapshot');assert.ok(state);
  const b=backend(),h=native(b);
  await h.store.prepareWorkspaceConversation('project','snapshot');
  h.store.updateWorkspace('project','snapshot',()=>state);await h.store.flushWorkspaceConversation('project','snapshot');
  const saved=clone(h.store.getWorkspace('project','snapshot'));assert.equal(saved.explorations.length,7);assert.equal(saved.currentId,siblingId);assert.equal(saved.explorations[0].turns[0].reading.guide.sectionId,'section:two');
  assert.equal(saved.explorations[0].turns[0].failureCode,'stopped');assert.equal(saved.explorations.find(e=>e.id===siblingId).draft.question,'Explain this later');
  const expectedAfterRestart=clone(saved);delete expectedAfterRestart.explorations.find(e=>e.id===siblingId).draft;
  const writesBeforeRestart=b.writes.length;const reloaded=native(b);await reloaded.store.prepareWorkspaceConversation('project','snapshot');assert.deepEqual(clone(reloaded.store.getWorkspace('project','snapshot')),expectedAfterRestart);assert.equal(b.writes.length,writesBeforeRestart);
});

test('native startup ignores browser workspace and last-file preferences',async()=>{
  const oldState=createWorkspace('project','snapshot');oldState.file='src/old.rs';oldState.explorations[0].turns.push({id:'old-turn',question:'Old browser question',context:[],status:'stopped'});
  const oldRaw=JSON.stringify(oldState),lastFile=JSON.stringify({snapshotId:'snapshot',path:'src/old.rs'});
  const storage=new Map([[browserWorkspaceKey(),oldRaw],['code-atlas:last-file:project',lastFile]]),b=backend(),h=native(b,storage);
  await h.store.prepareWorkspaceConversation('project','snapshot');
  const current=h.store.getWorkspace('project','snapshot');
  assert.equal(current.file,null);assert.deepEqual(clone(current.explorations[0].turns),[]);
  assert.equal(b.records.get(scope).state.file,null);assert.equal(b.records.get(scope).state.explorations[0].turns.length,0);
  assert.equal(storage.get(browserWorkspaceKey()),oldRaw);assert.equal(storage.get('code-atlas:last-file:project'),lastFile);
});

test('native history survives corrupt and unavailable browser preferences without reading them',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');h.store.appendQuestion('project','snapshot','Saved question',[],'history');await h.store.flushWorkspaceConversation('project','snapshot');
  const storage=new Map([[browserWorkspaceKey(),'{broken']]);storage.get=()=>{throw new Error('Browser preferences unavailable');};storage.set=()=>assert.fail('native updates must not write browser preferences');
  const restart=native(b,storage);await restart.store.prepareWorkspaceConversation('project','snapshot');assert.equal(restart.store.getWorkspace('project','snapshot').explorations[0].turns[0].question,'Saved question');
  restart.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/new.rs'}));await restart.store.flushWorkspaceConversation('project','snapshot');assert.equal(b.records.get(scope).state.file,'src/new.rs');
});

test('native startup ignores browser workspace when localStorage throws',async()=>{
  const storage=new Map([[browserWorkspaceKey(),'{broken']]);storage.get=()=>{throw new Error('Browser preferences unavailable');};
  storage.set=()=>assert.fail('native startup and updates cannot write browser preferences');
  const b=backend(),h=native(b,storage);await h.store.prepareWorkspaceConversation('project','snapshot');
  assert.equal(h.store.getWorkspace('project','snapshot').file,null);
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/current.rs'}));await h.store.flushWorkspaceConversation('project','snapshot');
  assert.equal(b.records.get(scope).state.file,'src/current.rs');
});

test('coalesces startup reads and prevents edits before the authoritative conversation opens',async()=>{
  const b=backend(),read=deferred(),h=native(b);b.controls.load=()=>read.promise;
  const first=h.store.prepareWorkspaceConversation('project','snapshot'),second=h.store.prepareWorkspaceConversation('project','snapshot');assert.equal(first,second);
  assert.throws(()=>h.store.updateWorkspace('project','snapshot',s=>({...s,file:'too-soon.rs'})),/still opening/);
  assert.equal(h.calls.length,1);read.resolve(null);await first;assert.equal(b.writes.length,1);assert.equal(b.records.get(scope).state.file,null);
});

test('serializes a held save and collapses a burst to the latest state without replacing the live reader',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');const held=deferred();let saves=0;b.controls.beforeSave=()=>++saves===1?held.promise:undefined;
  for(let n=0;n<40;n++)h.store.updateWorkspace('project','snapshot',s=>({...s,file:`src/file-${n}.rs`}));
  assert.equal(saves,1);assert.equal(h.store.getWorkspace('project','snapshot').file,'src/file-39.rs');held.resolve();await h.store.flushWorkspaceConversation('project','snapshot');
  assert.equal(saves,2);assert.equal(b.records.get(scope).state.file,'src/file-39.rs');assert.equal(h.store.getWorkspace('project','snapshot').file,'src/file-39.rs');
});

test('a failed native write keeps one scoped episode and all newer edits until explicit local retry',async()=>{
  const b=backend(),h=native(b),notices=[],resolved=[];h.onWindowEvent('code-atlas:workspace-notice',e=>notices.push(clone(e.detail)));h.onWindowEvent('code-atlas:workspace-notice-resolved',e=>resolved.push(clone(e.detail)));
  await h.store.prepareWorkspaceConversation('project','snapshot');b.controls.beforeSave=()=>{throw new Error('private filesystem diagnostic');};
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/first.rs'}));await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'),/could not be saved/);
  for(let n=0;n<40;n++)h.store.updateWorkspace('project','snapshot',s=>({...s,file:`src/latest-${n}.rs`}));await flushLocalReads();
  assert.equal(b.writes.length,2);assert.equal(notices.length,1);assert.deepEqual(notices[0].target,{kind:'retryConversation'});assert.equal(notices[0].message,'Some conversation changes could not be saved. Existing explanations are preserved.');assert.doesNotMatch(JSON.stringify(notices),/private filesystem/);assert.equal(resolved.length,0);
  b.controls.beforeSave=null;await h.store.retryWorkspaceConversation('project','snapshot');assert.equal(b.records.get(scope).state.file,'src/latest-39.rs');assert.equal(resolved.length,1);assert.equal(resolved[0].occurrenceId,notices[0].occurrenceId);
  b.controls.beforeSave=()=>{throw new Error('another failure');};h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/again.rs'}));await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'));assert.equal(notices.length,2);assert.notEqual(notices[0].occurrenceId,notices[1].occurrenceId);
});

test('a safe save contention reason reaches the retry notice',async()=>{
  const b=backend(),h=native(b),notices=[];h.onWindowEvent('code-atlas:workspace-notice',e=>notices.push(clone(e.detail)));
  await h.store.prepareWorkspaceConversation('project','snapshot');
  b.controls.beforeSave=()=>{throw new Error('A conversation save is still in progress. Wait a moment, then retry.');};
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/pending.rs'}));
  await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'),/Wait a moment/);
  assert.equal(notices.length,1);assert.match(notices[0].message,/Wait a moment/);
  assert.doesNotMatch(JSON.stringify(notices),/src\/pending\.rs/);
});

test('specific local-storage recovery guidance reaches the notice without exposing details',async()=>{
  for(const message of [
    'The Mac reported insufficient local storage space. Check available disk space, then retry saving; your changes remain open in this window.',
    'A local storage quota was reached. Check the app or filesystem storage limit, then retry saving; your changes remain open in this window.',
    'Code Atlas cannot write conversation data. Check local storage permissions, then retry; your changes remain open in this window.',
  ]){
    const b=backend(),h=native(b),notices=[];h.onWindowEvent('code-atlas:workspace-notice',e=>notices.push(clone(e.detail)));
    await h.store.prepareWorkspaceConversation('project','snapshot');
    b.controls.beforeSave=()=>{throw new Error(message);};
    h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/private.rs'}));
    await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'),new RegExp(message.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')));
    assert.equal(notices.length,1);assert.equal(notices[0].message,message);
    assert.doesNotMatch(JSON.stringify(notices),/src\/private\.rs|private filesystem|ErrorKind/);
  }
});

test('lost save acknowledgment is recognized before newer local edits are saved',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');let lost=true;
  b.controls.afterSave=()=>{if(lost){lost=false;throw new Error('acknowledgment lost');}};
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/committed.rs'}));await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'));h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/newer.rs'}));
  await h.store.retryWorkspaceConversation('project','snapshot');const attempts=b.writes.slice(1);assert.deepEqual(attempts.map(x=>x.expectedRevision),[1,2]);assert.equal(attempts[0].state.file,'src/committed.rs');assert.equal(attempts[1].state.file,'src/newer.rs');assert.equal(b.records.get(scope).state.file,'src/newer.rs');assert.equal(b.records.get(scope).revision,3);
});

test('automatically merges independent window edits and preserves both windows\' turns',async()=>{
  const b=backend(),local=native(b),remote=native(b);
  await Promise.all([local.store.prepareWorkspaceConversation('project','snapshot'),remote.store.prepareWorkspaceConversation('project','snapshot')]);
  remote.store.updateWorkspace('project','snapshot',s=>({...s,explorations:s.explorations.map(e=>e.id==='root'?{...e,view:'flow'}:e)}));
  const remoteTurn=remote.store.appendQuestion('project','snapshot','Remote window question',[],'flow');
  await remote.store.flushWorkspaceConversation('project','snapshot');
  local.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/local-window.rs'}));
  const localTurn=local.store.appendQuestion('project','snapshot','Current window question',[],'guide');
  await local.store.flushWorkspaceConversation('project','snapshot');
  const saved=b.records.get(scope).state,root=saved.explorations.find(e=>e.id==='root');
  assert.equal(saved.file,'src/local-window.rs');assert.equal(root.view,'flow');
  assert.deepEqual(root.turns.map(t=>t.id),[remoteTurn,localTurn]);
  assert.equal(root.focusedTurnId,localTurn,'the retrying window keeps its active reader position');
  assert.deepEqual(clone(local.store.getWorkspace('project','snapshot')),saved);
  const reopened=native(b);await reopened.store.prepareWorkspaceConversation('project','snapshot');
  assert.deepEqual(clone(reopened.store.getWorkspace('project','snapshot')),saved);
});

test('a new question merges disjoint window edits before dispatching generation',async()=>{
  const b=backend(),remote=native(b);let generationArgs;
  const local=workspaceStoreHarness(async(name,args)=>{
    if(name==='generate_progressive_workspace_answer'){
      generationArgs=args;
      const saved=b.records.get(scope).state.explorations.find(e=>e.id==='root').turns.map(t=>t.id);
      assert.ok(saved.includes(remoteTurn),'the other window is durable before generation starts');
      assert.ok(saved.includes(localTurn),'the new question is durable before generation starts');
      return {runId:args.input.runId,projectId:args.input.projectId,snapshotId:args.input.snapshotId,
        question:args.input.question,outcome:'complete',sections:[],incompleteTopics:[]};
    }
    return b.invoke(name,args);
  },{native:true,ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  await local.store.prepareWorkspaceConversation('project','snapshot');
  await remote.store.prepareWorkspaceConversation('project','snapshot');
  const remoteTurn=remote.store.appendQuestion('project','snapshot','Remote window question',[],'flow');
  await remote.store.flushWorkspaceConversation('project','snapshot');
  const localTurn=local.store.appendQuestion('project','snapshot','Explain this selected range',[],'guide');
  await local.store.runQuestion('project','snapshot',localTurn,'grant');
  await local.store.flushWorkspaceConversation('project','snapshot');
  assert.ok(generationArgs,'the failed local save was reconciled instead of turning the question into a no-op');
  assert.equal(local.store.getWorkspace('project','snapshot').explorations[0].turns.some(t=>t.id===remoteTurn),true);
  assert.equal(local.store.getWorkspace('project','snapshot').explorations[0].turns.some(t=>t.id===localTurn),true);
});

test('a new attempt reconciles a previously blocked window conflict before generation',async()=>{
  const b=backend(),remote=native(b);let generationArgs;let remoteTurn;let localTurn;
  const local=workspaceStoreHarness(async(name,args)=>{
    if(name==='generate_progressive_workspace_answer'){
      generationArgs=args;
      const saved=b.records.get(scope).state.explorations.find(e=>e.id==='root').turns.map(t=>t.id);
      assert.ok(saved.includes(remoteTurn),'the other window is retained before generation starts');
      assert.ok(saved.includes(localTurn),'the retried question is saved before generation starts');
      return {runId:args.input.runId,projectId:args.input.projectId,snapshotId:args.input.snapshotId,
        question:args.input.question,outcome:'complete',sections:[],incompleteTopics:[]};
    }
    return b.invoke(name,args);
  },{native:true,ensureCodexConnection:async()=>{},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  await local.store.prepareWorkspaceConversation('project','snapshot');
  await remote.store.prepareWorkspaceConversation('project','snapshot');
  remote.store.updateWorkspace('project','snapshot',s=>({...s,grantId:runId(301)}));
  remoteTurn=remote.store.appendQuestion('project','snapshot','Remote window question',[],'flow');
  await remote.store.flushWorkspaceConversation('project','snapshot');
  local.store.updateWorkspace('project','snapshot',s=>({...s,grantId:runId(302)}));
  await assert.rejects(local.store.flushWorkspaceConversation('project','snapshot'),/conversation changed in another window/);
  remote.store.updateWorkspace('project','snapshot',s=>({...s,grantId:runId(302)}));
  await remote.store.flushWorkspaceConversation('project','snapshot');
  localTurn=local.store.appendQuestion('project','snapshot','Retry this focused question',[],'guide');
  await local.store.runQuestion('project','snapshot',localTurn,'grant');
  await local.store.flushWorkspaceConversation('project','snapshot');
  assert.ok(generationArgs,'a safe merge clears the prior blocked save before the user’s next generation attempt');
});

test('same-field window conflicts keep both copies intact until the current window resolves its choice',async()=>{
  const b=backend(),local=native(b),remote=native(b);
  await Promise.all([local.store.prepareWorkspaceConversation('project','snapshot'),remote.store.prepareWorkspaceConversation('project','snapshot')]);
  remote.store.updateWorkspace('project','snapshot',s=>({...s,grantId:runId(401)}));await remote.store.flushWorkspaceConversation('project','snapshot');
  local.store.updateWorkspace('project','snapshot',s=>({...s,grantId:runId(402)}));
  await assert.rejects(local.store.flushWorkspaceConversation('project','snapshot'));
  await assert.rejects(local.store.retryWorkspaceConversation('project','snapshot'),/This conversation changed in another window/);
  assert.equal(b.records.get(scope).state.grantId,runId(401));
  assert.equal(local.store.getWorkspace('project','snapshot').grantId,runId(402));
  local.store.updateWorkspace('project','snapshot',s=>({...s,grantId:runId(401)}));
  await local.store.retryWorkspaceConversation('project','snapshot');
  assert.equal(b.records.get(scope).state.grantId,runId(401));
  assert.equal(local.store.getWorkspace('project','snapshot').grantId,runId(401));
});

test('concurrent code-context changes stay separate instead of broadening one question',async()=>{
  const b=backend(),seed=native(b);await seed.store.prepareWorkspaceConversation('project','snapshot');
  const turnId=seed.store.appendQuestion('project','snapshot','Explain this selection',[{path:'src/base.rs',from:2,to:4}],'guide');
  await seed.store.flushWorkspaceConversation('project','snapshot');
  const local=native(b),remote=native(b);
  await Promise.all([local.store.prepareWorkspaceConversation('project','snapshot'),remote.store.prepareWorkspaceConversation('project','snapshot')]);
  const setContext=(path)=>state=>({...state,explorations:state.explorations.map(e=>({...e,turns:e.turns.map(t=>t.id===turnId?{...t,context:[{path,from:9,to:11}]}:t)}))});
  remote.store.updateWorkspace('project','snapshot',setContext('src/remote.rs'));await remote.store.flushWorkspaceConversation('project','snapshot');
  local.store.updateWorkspace('project','snapshot',setContext('src/local.rs'));await assert.rejects(local.store.flushWorkspaceConversation('project','snapshot'));
  await assert.rejects(local.store.retryWorkspaceConversation('project','snapshot'),/This conversation changed in another window/);
  const savedTurn=b.records.get(scope).state.explorations.flatMap(e=>e.turns).find(t=>t.id===turnId);
  const localTurn=local.store.getWorkspace('project','snapshot').explorations.flatMap(e=>e.turns).find(t=>t.id===turnId);
  assert.deepEqual(savedTurn.context,[{path:'src/remote.rs',from:9,to:11}]);
  assert.deepEqual(localTurn.context,[{path:'src/local.rs',from:9,to:11}]);
  local.store.updateWorkspace('project','snapshot',setContext('src/remote.rs'));
  await local.store.retryWorkspaceConversation('project','snapshot');
  assert.deepEqual(b.records.get(scope).state.explorations.flatMap(e=>e.turns).find(t=>t.id===turnId).context,[{path:'src/remote.rs',from:9,to:11}]);
});

test('malformed or foreign native conversation records are preserved without replacement',async()=>{
  for(const record of [
    {schemaVersion:1,revision:1,state:{...createWorkspace('project','snapshot'),version:2}},
    {schemaVersion:2,revision:1,state:createWorkspace('project','snapshot')},
    {schemaVersion:1,revision:1,state:createWorkspace('foreign','snapshot')},
  ]){
    const b=backend();b.records.set(scope,record);const h=native(b);await assert.rejects(h.store.prepareWorkspaceConversation('project','snapshot'));assert.deepEqual(b.records.get(scope),record);assert.equal(b.writes.length,0);
  }
});

test('preparation failures retry only on request and scopes cannot replace each other',async()=>{
  const b=backend(),h=native(b);b.controls.load=()=>{throw new Error('unavailable');};await assert.rejects(h.store.prepareWorkspaceConversation('project','snapshot'));assert.equal(h.calls.length,1);await flushLocalReads();assert.equal(h.calls.length,1);
  b.controls.load=null;await h.store.prepareWorkspaceConversation('project','snapshot');await h.store.prepareWorkspaceConversation('other','snapshot');h.store.updateWorkspace('other','snapshot',s=>({...s,file:'other.rs'}));await h.store.flushWorkspaceConversation('other','snapshot');assert.equal(h.store.getWorkspace('project','snapshot').file,null);assert.equal(h.store.getWorkspace('other','snapshot').file,'other.rs');
});

test('tuple scopes stay independent even when project and snapshot names contain colons',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project:other','one');await h.store.prepareWorkspaceConversation('project','other:one');
  h.store.updateWorkspace('project:other','one',s=>({...s,file:'first.rs'}));h.store.updateWorkspace('project','other:one',s=>({...s,file:'second.rs'}));
  await Promise.all([h.store.flushWorkspaceConversation('project:other','one'),h.store.flushWorkspaceConversation('project','other:one')]);
  assert.equal(h.store.getWorkspace('project:other','one').file,'first.rs');assert.equal(h.store.getWorkspace('project','other:one').file,'second.rs');assert.equal(b.records.size,2);
});

test('wire persistence excludes one-shot focus and native guide copies and restart never resends queued work',async()=>{
  const turnId=runId(501);
  const state=createWorkspace('project','snapshot');state.explorations[0].revealTurnId=turnId;state.explorations[0].turns=[{id:turnId,question:'Queued work',context:[],status:'queued',guide:{privateCopy:true}}];
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');h.store.updateWorkspace('project','snapshot',()=>state);await h.store.flushWorkspaceConversation('project','snapshot');const saved=b.records.get(scope).state;
  assert.equal(saved.explorations[0].turns[0].status,'interrupted');assert.equal('revealTurnId' in saved.explorations[0],false);assert.equal('guide' in saved.explorations[0].turns[0],false);assert.equal(h.calls.some(c=>/generate|start_repository|resume_repository|authorize/.test(c.name)),false);assert.equal(decodeWorkspace('{broken','project','snapshot'),null);
  h.store.updateWorkspace('project','snapshot',s=>({...s,explorations:s.explorations.map(e=>({...e,revealTurnId:turnId,turns:e.turns.map(t=>({...t,guide:{privateCopy:true},context:[{path:'src/font.rs',sourceText:'not metadata'}]}))}))}));
  assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].guide.privateCopy,true);await h.store.flushWorkspaceConversation('project','snapshot');
  assert.doesNotMatch(JSON.stringify(b.records.get(scope)),/privateCopy|sourceText|revealTurnId/);
});

test('a real queued question cannot dispatch inference when its native run reference cannot be saved',async()=>{
  const b=backend();let connections=0;const h=workspaceStoreHarness((name,args)=>name==='load_workspace_answer'?Promise.reject(new Error('No saved run')):b.invoke(name,args),{native:true,ensureCodexConnection:async()=>{connections++;},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  await h.store.prepareWorkspaceConversation('project','snapshot');b.controls.beforeSave=()=>{throw new Error('controlled unavailable storage');};
  const id=h.store.appendQuestion('project','snapshot','Explain this decision',[],'guide');const result=await h.store.runQuestion('project','snapshot',id,'existing-grant');
  assert.equal(result,null);assert.equal(connections,0,'unreadable conversation state prevents both queue registration and connection preparation');assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].status,'failed');
  assert.deepEqual([...new Set(h.calls.map(c=>c.name))].sort(),['load_workspace_answer','load_workspace_conversation','save_workspace_conversation']);assert.equal(b.records.get(scope).state.explorations[0].turns.length,0);
});

test('an unavailable conversation leaves the production source workspace readable and gates generation',async()=>{
  const [store,workspace,app,native]=await Promise.all(['../src/workspace/store.ts','../src/workspace/Workspace.tsx','../src/App.tsx','../src-tauri/src/lib.rs'].map(p=>readFile(new URL(p,import.meta.url),'utf8')));
  assert.match(workspace,/return availability==='loading'\?[\s\S]*?:<DeveloperWorkspace \{\.\.\.props\} conversationAvailability=\{availability\}/);assert.match(workspace,/prepareWorkspaceConversation\(projectId,snapshotId\)/);assert.match(app,/retryWorkspaceConversation\(projectId,snapshotId\)/);
  assert.match(workspace,/Saved conversation unavailable\./);assert.match(workspace,/disabled=\{conversationUnavailable\}/);assert.match(workspace,/new questions cannot be saved/);
  assert.match(workspace,/<BookOpen size=\{14\}\/>Guide me/);assert.doesNotMatch(workspace,/Build repository guide/);
  assert.match(workspace,/const action=contextualGuideAction\(query,attachments,!!current\.parentId\);if\(action\.kind==='repository-guide'\)startRepositoryGuide\(action\.intent\);else ask\(action\.intent,'guide'\)/);
  assert.match(store,/if\(persistence!\.availability==='ready'\)persistence!\.enqueue\(next\)/);
  assert.match(store,/await prepareWorkspaceConversation\(project,snapshot\)/);
  for(const [start,end,command] of [['export async function runRepositoryGuide','export async function runQuestion',"'start_repository_guide'"],['export async function runQuestion',null,"'generate_progressive_workspace_answer'"]]){const code=store.slice(store.indexOf(start),end?store.indexOf(end):undefined);const save=code.indexOf('await flushWorkspaceConversation'),dispatch=code.indexOf(command);assert.ok(save>=0&&dispatch>save,'the question/run reference must be durable before dispatch');}
  assert.match(native,/workspace_conversation::load_workspace_conversation/);assert.match(native,/workspace_conversation::save_workspace_conversation/);
  assert.doesNotMatch(app,/relativePath: saved\?\.snapshotId === snapshot\.snapshotId \? saved\.path/,'a stale browser hint cannot override the native selection after hydration');
});
