import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createWorkspace,addTurn,decodeWorkspace} from '../src/workspace/model.ts';
import {workspaceStoreHarness,deferred,flushLocalReads} from './helpers/workspace-store-harness.mjs';

const clone=value=>JSON.parse(JSON.stringify(value));
const legacyKey=(p='project',s='snapshot')=>`code-atlas:workspace:v1:${p}:${s}`;
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

test('migrates five nested levels and siblings once, retaining parent turns, source, sections and last views',async()=>{
  let state=createWorkspace('project','snapshot');state=addTurn(state,{id:'root-turn',question:'Read bytes',context:[{path:'src/font.rs',from:3,to:7}],status:'stopped',revisionId:'revision-one',reading:{guide:{revisionId:'revision-one',sectionId:'section:two'}}},'guide');
  let parent='root-turn';for(let n=1;n<=5;n++){state=addTurn(state,{id:`turn-${n}`,question:`Depth ${n}`,context:[],status:'stopped'},['guide','flow','history'][n%3],{originTurnId:parent,id:`child-${n}`});parent=`turn-${n}`;}
  state={...state,currentId:'root'};state=addTurn(state,{id:'sibling-turn',question:'Sibling',context:[],status:'stopped'},'flow',{originTurnId:'root-turn',id:'sibling'});
  const raw=JSON.stringify(state),storage=new Map([[legacyKey(),raw]]),b=backend(),h=native(b,storage);
  await h.store.prepareWorkspaceConversation('project','snapshot');assert.equal(b.writes.length,1);assert.equal(storage.get(legacyKey()),raw,'migration preserves the original preference');
  const migrated=clone(h.store.getWorkspace('project','snapshot'));assert.equal(migrated.explorations.length,7);assert.equal(migrated.currentId,'sibling');assert.equal(migrated.explorations[0].turns[0].reading.guide.sectionId,'section:two');
  storage.clear();const reloaded=native(b,storage);await reloaded.store.prepareWorkspaceConversation('project','snapshot');assert.deepEqual(clone(reloaded.store.getWorkspace('project','snapshot')),migrated);assert.equal(b.writes.length,1,'existing native history is never imported again');
});

test('native history survives missing, corrupt and unavailable browser preferences',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project','snapshot');h.store.appendQuestion('project','snapshot','Saved question',[],'history');await h.store.flushWorkspaceConversation('project','snapshot');
  const storage=new Map([[legacyKey(),'{broken']]);storage.get=()=>{throw new Error('Browser preferences unavailable');};storage.set=()=>assert.fail('native updates must not write browser preferences');
  const restart=native(b,storage);await restart.store.prepareWorkspaceConversation('project','snapshot');assert.equal(restart.store.getWorkspace('project','snapshot').explorations[0].turns[0].question,'Saved question');
  restart.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/new.rs'}));await restart.store.flushWorkspaceConversation('project','snapshot');assert.equal(b.records.get(scope).state.file,'src/new.rs');
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
  assert.equal(b.writes.length,2);assert.equal(notices.length,1);assert.deepEqual(notices[0].target,{kind:'retryConversation'});assert.doesNotMatch(JSON.stringify(notices),/private filesystem/);assert.equal(resolved.length,0);
  b.controls.beforeSave=null;await h.store.retryWorkspaceConversation('project','snapshot');assert.equal(b.records.get(scope).state.file,'src/latest-39.rs');assert.equal(resolved.length,1);assert.equal(resolved[0].occurrenceId,notices[0].occurrenceId);
  b.controls.beforeSave=()=>{throw new Error('another failure');};h.store.updateWorkspace('project','snapshot',s=>({...s,file:'src/again.rs'}));await assert.rejects(h.store.flushWorkspaceConversation('project','snapshot'));assert.equal(notices.length,2);assert.notEqual(notices[0].occurrenceId,notices[1].occurrenceId);
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

test('malformed initial preferences and foreign/future native records are preserved without writing a fallback',async()=>{
  for(const raw of ['{broken',JSON.stringify({...createWorkspace('project','snapshot'),version:2})]){
    const b=backend(),storage=new Map([[legacyKey(),raw]]),h=native(b,storage);await assert.rejects(h.store.prepareWorkspaceConversation('project','snapshot'));assert.equal(b.writes.length,0);assert.equal(storage.get(legacyKey()),raw);
  }
  for(const record of [{schemaVersion:2,revision:1,state:createWorkspace('project','snapshot')},{schemaVersion:1,revision:1,state:createWorkspace('foreign','snapshot')}]){
    const b=backend();b.records.set(scope,record);const h=native(b);await assert.rejects(h.store.prepareWorkspaceConversation('project','snapshot'));assert.deepEqual(b.records.get(scope),record);assert.equal(b.writes.length,0);
  }
});

test('preparation failures retry only on request and scopes cannot replace each other',async()=>{
  const b=backend(),h=native(b);b.controls.load=()=>{throw new Error('unavailable');};await assert.rejects(h.store.prepareWorkspaceConversation('project','snapshot'));assert.equal(h.calls.length,1);await flushLocalReads();assert.equal(h.calls.length,1);
  b.controls.load=null;await h.store.prepareWorkspaceConversation('project','snapshot');await h.store.prepareWorkspaceConversation('other','snapshot');h.store.updateWorkspace('other','snapshot',s=>({...s,file:'other.rs'}));await h.store.flushWorkspaceConversation('other','snapshot');assert.equal(h.store.getWorkspace('project','snapshot').file,null);assert.equal(h.store.getWorkspace('other','snapshot').file,'other.rs');
});

test('tuple scopes stay independent even when colon-joined legacy keys collide',async()=>{
  const b=backend(),h=native(b);await h.store.prepareWorkspaceConversation('project:other','one');await h.store.prepareWorkspaceConversation('project','other:one');
  h.store.updateWorkspace('project:other','one',s=>({...s,file:'first.rs'}));h.store.updateWorkspace('project','other:one',s=>({...s,file:'second.rs'}));
  await Promise.all([h.store.flushWorkspaceConversation('project:other','one'),h.store.flushWorkspaceConversation('project','other:one')]);
  assert.equal(h.store.getWorkspace('project:other','one').file,'first.rs');assert.equal(h.store.getWorkspace('project','other:one').file,'second.rs');assert.equal(b.records.size,2);
});

test('wire persistence excludes one-shot focus and native guide copies and restart never resends queued work',async()=>{
  const state=createWorkspace('project','snapshot');state.explorations[0].revealTurnId='turn-one';state.explorations[0].turns=[{id:'turn-one',question:'Queued work',context:[],status:'queued',guide:{privateCopy:true}}];
  const b=backend(),h=native(b,new Map([[legacyKey(),JSON.stringify(state)]]));await h.store.prepareWorkspaceConversation('project','snapshot');const saved=b.records.get(scope).state;
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

test('the production workspace gates mounting and generation on durable conversation preparation',async()=>{
  const [store,workspace,app,native]=await Promise.all(['../src/workspace/store.ts','../src/workspace/Workspace.tsx','../src/App.tsx','../src-tauri/src/lib.rs'].map(p=>readFile(new URL(p,import.meta.url),'utf8')));
  assert.match(workspace,/availability==='ready'\?<DeveloperWorkspace/);assert.match(workspace,/prepareWorkspaceConversation\(projectId,snapshotId\)/);assert.match(app,/retryWorkspaceConversation\(projectId,snapshotId\)/);
  assert.match(store,/await prepareWorkspaceConversation\(project,snapshot\)/);
  for(const [start,end,command] of [['export async function runRepositoryGuide','export async function runQuestion',"'start_repository_guide'"],['export async function runQuestion',null,"'generate_progressive_workspace_answer'"]]){const code=store.slice(store.indexOf(start),end?store.indexOf(end):undefined);const save=code.indexOf('await flushWorkspaceConversation'),dispatch=code.indexOf(command);assert.ok(save>=0&&dispatch>save,'the question/run reference must be durable before dispatch');}
  assert.match(native,/workspace_conversation::load_workspace_conversation/);assert.match(native,/workspace_conversation::save_workspace_conversation/);
});
