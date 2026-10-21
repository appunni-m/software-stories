import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {FLOW_DIAGRAM_LAYOUT_VERSION} from '../src/lib/readerProgress.ts';
import {createWorkspace,addTurn,decodeWorkspace,rememberCodePane,rememberFileFlow,rememberPresentation} from '../src/workspace/model.ts';
import {workspaceStoreHarness,deferred,flushLocalReads} from './helpers/workspace-store-harness.mjs';
import {rememberAnswerVersion,answerReadingPosition,resolveAnswerRevision} from '../src/workspace/answerVersions.ts';

const clone=value=>JSON.parse(JSON.stringify(value));
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
      assert.equal(args.expectedRevision,record?.revision??null,'compare-and-save prevents stale overwrite');
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
  let s=addTurn(h.store.getWorkspace('project','snapshot'),{id:'reader',question:'What happens?',context:[],status:'ready',revisionId:original.revisionId,reading:{guide:{revisionId:original.revisionId,sectionId:'section:original-last'}}},'guide');
  s=rememberAnswerVersion(s,'root','reader',original.revisionId,edit.revisionId,[original,edit]);
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
  assert.ok(h.calls.every(c=>['load_workspace_conversation','save_workspace_conversation'].includes(c.name)));
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
});

test('native restart restores owned scenes, times, viewport, pane and expanded static links without browser preferences',async()=>{
  const b=backend(),storage=new Map(),h=native(b,storage);await h.store.prepareWorkspaceConversation('project','snapshot');
  const state=addTurn(h.store.getWorkspace('project','snapshot'),{id:'reader-turn',question:'Follow the parser',context:[],status:'ready',revisionId:'revision:one'},'guide');
  const identity={revisionId:'revision:one',sectionId:'section:two',sectionRevisionId:'section-revision:two'};
  const story={...identity,sceneId:'claim:second',positions:{'claim:first':12.4,'claim:second':3.2},fullMap:true,viewport:{x:-34,y:91,zoom:.6},focusLayoutVersion:4};
  h.store.updateWorkspace('project','snapshot',()=>state);
  h.store.updateWorkspace('project','snapshot',s=>rememberCodePane(s,'root',false));
  h.store.updateWorkspace('project','snapshot',s=>rememberFileFlow(s,'root','src/font.rs',true));
  h.store.updateWorkspace('project','snapshot',s=>rememberPresentation(s,'root','reader-turn','stories',{...story,html:'must never be copied',autoPlay:true}));
  h.store.updateWorkspace('project','snapshot',s=>rememberPresentation(s,'root','reader-turn','diagrams',{...identity,viewport:{x:100,y:-500,zoom:1.8},layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION}));
  await h.store.flushWorkspaceConversation('project','snapshot');
  const metadata=JSON.stringify(b.records.get(scope));assert.doesNotMatch(metadata,/must never be copied|autoPlay|spokenText/);
  storage.clear();storage.get=()=>{throw new Error('browser unavailable');};
  const reopened=native(b,storage);await reopened.store.prepareWorkspaceConversation('project','snapshot');
  const root=reopened.store.getWorkspace('project','snapshot').explorations[0];
  assert.equal(root.codeOpen,false);assert.deepEqual(clone(root.fileFlow),{path:'src/font.rs',showAll:true});
  assert.deepEqual(clone(root.turns[0].presentation.stories),[story]);assert.equal(root.turns[0].presentation.diagrams[0].viewport.zoom,1.8);
  assert.ok(reopened.calls.every(c=>c.name==='load_workspace_conversation'),'restart reads local metadata without model, audio or indexing work');
});

test('nested conversations retain parent turns, source, sections and last views in native storage',async()=>{
  let state=createWorkspace('project','snapshot');state=addTurn(state,{id:'root-turn',question:'Read bytes',context:[{path:'src/font.rs',from:3,to:7}],status:'stopped',revisionId:'revision-one',reading:{guide:{revisionId:'revision-one',sectionId:'section:two'}}},'guide');
  let parent='root-turn';for(let n=1;n<=5;n++){state=addTurn(state,{id:`turn-${n}`,question:`Depth ${n}`,context:[],status:'stopped'},['guide','flow','history'][n%3],{originTurnId:parent,id:`child-${n}`});parent=`turn-${n}`;}
  state={...state,currentId:'root'};state=addTurn(state,{id:'sibling-turn',question:'Sibling',context:[],status:'stopped'},'flow',{originTurnId:'root-turn',id:'sibling'});
  state=decodeWorkspace(JSON.stringify(state),'project','snapshot');assert.ok(state);
  const b=backend(),h=native(b);
  await h.store.prepareWorkspaceConversation('project','snapshot');
  h.store.updateWorkspace('project','snapshot',()=>state);await h.store.flushWorkspaceConversation('project','snapshot');
  const saved=clone(h.store.getWorkspace('project','snapshot'));assert.equal(saved.explorations.length,7);assert.equal(saved.currentId,'sibling');assert.equal(saved.explorations[0].turns[0].reading.guide.sectionId,'section:two');
  const writesBeforeRestart=b.writes.length;const reloaded=native(b);await reloaded.store.prepareWorkspaceConversation('project','snapshot');assert.deepEqual(clone(reloaded.store.getWorkspace('project','snapshot')),saved);assert.equal(b.writes.length,writesBeforeRestart);
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

test('lost save acknowledgment replays the same attempt before saving edits made during the failure',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');let lost=true;
  b.controls.afterSave=()=>{if(lost){lost=false;throw new Error('acknowledgment lost');}};
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/committed.rs'}));await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'));h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/newer.rs'}));
  await h.store.retryWorkspaceConversation('project','snapshot');const attempts=b.writes.slice(1);assert.deepEqual(attempts.map(x=>x.expectedRevision),[1,1,2]);assert.equal(attempts[0].state.file,attempts[1].state.file);assert.equal(b.records.get(scope).state.file,'src/newer.rs');assert.equal(b.records.get(scope).revision,3);
});

test('a different newer native writer is preserved while the current window retains its unsaved edits',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');const other=clone(b.records.get(scope));other.revision++;other.state.file='src/other-window.rs';b.records.set(scope,other);
  h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/this-window.rs'}));await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'));await assert.rejects(h.store.retryWorkspaceConversation('project','snapshot'));
  assert.equal(b.records.get(scope).state.file,'src/other-window.rs');assert.equal(h.store.getWorkspace('project','snapshot').file,'src/this-window.rs');
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
  const state=createWorkspace('project','snapshot');state.explorations[0].revealTurnId='turn-one';state.explorations[0].turns=[{id:'turn-one',question:'Queued work',context:[],status:'queued',guide:{privateCopy:true}}];
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');h.store.updateWorkspace('project','snapshot',()=>state);await h.store.flushWorkspaceConversation('project','snapshot');const saved=b.records.get(scope).state;
  assert.equal(saved.explorations[0].turns[0].status,'interrupted');assert.equal('revealTurnId' in saved.explorations[0],false);assert.equal('guide' in saved.explorations[0].turns[0],false);assert.equal(h.calls.some(c=>/generate|start_repository|resume_repository|authorize/.test(c.name)),false);assert.equal(decodeWorkspace('{broken','project','snapshot'),null);
  h.store.updateWorkspace('project','snapshot',s=>({...s,explorations:s.explorations.map(e=>({...e,revealTurnId:'turn-one',turns:e.turns.map(t=>({...t,guide:{privateCopy:true},context:[{path:'src/font.rs',sourceText:'not metadata'}]}))}))}));
  assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].guide.privateCopy,true);await h.store.flushWorkspaceConversation('project','snapshot');
  assert.doesNotMatch(JSON.stringify(b.records.get(scope)),/privateCopy|sourceText|revealTurnId/);
});

test('a real queued question cannot dispatch inference when its native run reference cannot be saved',async()=>{
  const b=backend();let connections=0;const h=workspaceStoreHarness((name,args)=>name==='load_workspace_answer'?Promise.reject(new Error('No saved run')):b.invoke(name,args),{native:true,ensureCodexConnection:async()=>{connections++;},readCodexSelection:()=>({modelId:'gpt-6-luna',effort:'max'})});
  await h.store.prepareWorkspaceConversation('project','snapshot');b.controls.beforeSave=()=>{throw new Error('controlled unavailable storage');};
  const id=h.store.appendQuestion('project','snapshot','Explain this decision',[],'guide');const result=await h.store.runQuestion('project','snapshot',id,'existing-grant');
  assert.equal(result,null);assert.equal(connections,1);assert.equal(h.store.getWorkspace('project','snapshot').explorations[0].turns[0].status,'failed');
  assert.deepEqual([...new Set(h.calls.map(c=>c.name))].sort(),['load_workspace_answer','load_workspace_conversation','save_workspace_conversation']);assert.equal(b.records.get(scope).state.explorations[0].turns.length,0);
});

test('an unavailable conversation leaves the production source workspace readable and gates generation',async()=>{
  const [store,workspace,app,native]=await Promise.all(['../src/workspace/store.ts','../src/workspace/Workspace.tsx','../src/App.tsx','../src-tauri/src/lib.rs'].map(p=>readFile(new URL(p,import.meta.url),'utf8')));
  assert.match(workspace,/return availability==='loading'\?[\s\S]*?:<DeveloperWorkspace \{\.\.\.props\} conversationAvailability=\{availability\}/);assert.match(workspace,/prepareWorkspaceConversation\(projectId,snapshotId\)/);assert.match(app,/retryWorkspaceConversation\(projectId,snapshotId\)/);
  assert.match(workspace,/Saved conversation unavailable\./);assert.match(workspace,/disabled=\{conversationUnavailable\}/);assert.match(workspace,/This session is read-only/);assert.match(workspace,/!conversationUnavailable&&<>[\s\S]*?Build repository guide/);
  assert.match(store,/if\(persistence!\.availability==='ready'\)persistence!\.enqueue\(next\)/);
  assert.match(store,/await prepareWorkspaceConversation\(project,snapshot\)/);
  for(const [start,end,command] of [['export async function runRepositoryGuide','export async function runQuestion',"'start_repository_guide'"],['export async function runQuestion',null,"'generate_progressive_workspace_answer'"]]){const code=store.slice(store.indexOf(start),end?store.indexOf(end):undefined);const save=code.indexOf('await flushWorkspaceConversation'),dispatch=code.indexOf(command);assert.ok(save>=0&&dispatch>save,'the question/run reference must be durable before dispatch');}
  assert.match(native,/workspace_conversation::load_workspace_conversation/);assert.match(native,/workspace_conversation::save_workspace_conversation/);
  assert.doesNotMatch(app,/relativePath: saved\?\.snapshotId === snapshot\.snapshotId \? saved\.path/,'a stale browser hint cannot override the native selection after hydration');
});
