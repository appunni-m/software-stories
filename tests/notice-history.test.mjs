import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {NoticeHistory,emptyNoticeHistory,normalizeNotice,noticeKey,projectNotices} from '../src/lib/noticeHistory.ts';
import {notificationAvailability} from '../src/lib/notificationSummary.ts';

const input={title:'Saved update',message:'Font bytes → pixels',projectId:'project-one',snapshotId:'snap-one',target:{kind:'openExploration',explorationId:'root'}};
test('conversation retry is a scoped local action and requires both repository and snapshot',()=>{
  const action={kind:'retryConversation'};
  assert.deepEqual(normalizeNotice({...input,target:action}).target,action);
  assert.throws(()=>normalizeNotice({title:'Save failed',message:'Try again',target:action}));
  assert.throws(()=>normalizeNotice({title:'Save failed',message:'Try again',projectId:'project',target:action}));
});
const flush=()=>new Promise(resolve=>setImmediate(resolve));
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
function settled(history){
  if(history.getSnapshot().availability!=='loading')return Promise.resolve();
  return new Promise((resolve,reject)=>{
    const timeout=setTimeout(()=>{stop();reject(new Error('Local history did not settle'));},2000);
    const stop=history.subscribe(()=>{if(history.getSnapshot().availability!=='loading'){clearTimeout(timeout);stop();resolve();}});
  });
}
function fixture({load,update}={}){
  let data=emptyNoticeHistory();const calls=[];
  const api={load:async()=>{calls.push({kind:'load'});return load?load():structuredClone(data);},update:async operation=>{
    calls.push(operation);if(update)return update(operation);
    const item=operation.kind==='publish'?{id:await noticeKey(operation.input),input:operation.input,createdAtUnixMs:Date.now(),read:false}:undefined;
    data=projectNotices(data,[{operation,...(item?{item}:{})}]);return structuredClone(data);
  }};
  return {api,calls,history:new NoticeHistory(api),saved:()=>structuredClone(data)};
}

test('canonical Unicode identity matches the independent native golden value',async()=>{
  assert.equal(await noticeKey(normalizeNotice(input)),'d18911fbf06bfa438a39042ba1c956a52025741c0dd8cfab89be057b27879517');
  assert.deepEqual(Object.keys(normalizeNotice({title:'A',message:'B'})),['title','message','tone','projectId','snapshotId','target','occurrenceId']);
  assert.equal(normalizeNotice({title:'  A\n B ',message:' C\t D '}).message,'C D');
  const value=normalizeNotice({...input,projectId:'/Users/dev/work/Font App/字型',message:'🧭'.repeat(701)});
  assert.equal(Array.from(value.message).length,700);assert.equal(value.projectId,'/Users/dev/work/Font App/字型');
});

test('unsupported targets, orphan scopes and invalid identities cannot become stored actions',()=>{
  for(const invalid of [{...input,projectId:null},{...input,snapshotId:'../snapshot'},{...input,target:{kind:'sendModelRequest'}},{...input,message:'unsafe\0text'},{...input,projectId:'x'.repeat(4097)}])assert.throws(()=>normalizeNotice(invalid));
  const clean=normalizeNotice({...input,target:{kind:'openExploration',explorationId:'root',command:'do not retain'}});
  assert.equal(JSON.stringify(clean).includes('command'),false);
});

test('multiple outcomes persist and a new store restores them without republishing',async()=>{
  const f=fixture();await f.history.publish(input);await settled(f.history);await f.history.publish({...input,message:'Second'});await settled(f.history);
  assert.equal(f.history.getSnapshot().availability,'ready');assert.equal(f.history.getSnapshot().items.length,2);
  const restored=new NoticeHistory(f.api);restored.start();await settled(restored);
  assert.deepEqual(restored.getSnapshot(),f.history.getSnapshot());assert.deepEqual(f.calls.map(c=>c.kind),['load','publish','publish','load']);
});

test('repeat events preserve read acknowledgments and dismissal survives a store restart',async()=>{
  const f=fixture();await f.history.publish(input);await settled(f.history);const id=f.history.getSnapshot().items[0].id;
  f.history.read(id);await settled(f.history);await f.history.publish(input);await settled(f.history);assert.equal(f.history.getSnapshot().items[0].read,true);
  f.history.dismiss(id);await settled(f.history);const stored=f.saved();assert.deepEqual(stored.dismissed,[id]);
  const restored=new NoticeHistory(f.api);restored.start();await settled(restored);await restored.publish(input);await settled(restored);
  assert.deepEqual(restored.getSnapshot().items,[]);assert.deepEqual(f.saved(),stored);
});

test('equal text in another repository or a changed navigation target remains distinct',async()=>{
  const f=fixture();for(const value of [input,{...input,projectId:'project-two'},{...input,target:{kind:'retryRecents'}}]){await f.history.publish(value);await settled(f.history);}
  assert.equal(f.history.getSnapshot().items.length,3);assert.equal(new Set(f.history.getSnapshot().items.map(i=>i.id)).size,3);
});

test('a delayed initial read cannot replace a newer notice or a queued acknowledgment',async()=>{
  const gate=deferred();let saved=emptyNoticeHistory();let active=0,maxActive=0;
  const f=fixture({load:async()=>{active++;maxActive=Math.max(maxActive,active);await gate.promise;active--;return saved;},update:async operation=>{
    active++;maxActive=Math.max(maxActive,active);
    const item=operation.kind==='publish'?{id:await noticeKey(operation.input),input:operation.input,createdAtUnixMs:1,read:false}:undefined;
    saved=projectNotices(saved,[{operation,...(item?{item}:{})}]);active--;return structuredClone(saved);
  }});
  await f.history.publish(input);const id=f.history.getSnapshot().items[0].id;f.history.read(id);
  assert.equal(f.history.getSnapshot().availability,'loading');assert.equal(f.history.getSnapshot().items[0].read,true);
  gate.resolve();await settled(f.history);assert.equal(maxActive,1);assert.equal(f.history.getSnapshot().items[0].read,true);assert.equal(f.history.getSnapshot().availability,'ready');
});

test('a failed write retains the notice and retries only after the explicit local action',async()=>{
  let fail=true,attempts=0,saved=emptyNoticeHistory();
  const f=fixture({update:async operation=>{attempts++;if(fail)throw new Error('private diagnostic');
    const item=operation.kind==='publish'?{id:await noticeKey(operation.input),input:operation.input,createdAtUnixMs:1,read:false}:undefined;
    saved=projectNotices(saved,[{operation,...(item?{item}:{})}]);return structuredClone(saved);
  }});
  await f.history.publish(input);await settled(f.history);assert.equal(f.history.getSnapshot().availability,'unavailable');assert.equal(f.history.getSnapshot().items.length,1);
  await flush();await flush();assert.equal(attempts,1);assert.equal(JSON.stringify(f.history.getSnapshot()).includes('private diagnostic'),false);
  const id=f.history.getSnapshot().items[0].id;f.history.read(id);f.history.dismiss(id);await settled(f.history);assert.equal(attempts,1);
  fail=false;f.history.retry();await settled(f.history);assert.equal(f.history.getSnapshot().availability,'ready');assert.equal(f.history.getSnapshot().items.length,0);assert.deepEqual(saved.dismissed,[id]);assert.equal(attempts,4);
});

test('an unavailable initial history keeps incoming outcomes and cannot claim a complete unread count',async()=>{
  let fail=true;const f=fixture({load:async()=>{if(fail)throw new Error('unreadable');return emptyNoticeHistory();}});
  f.history.start();await flush();await f.history.publish(input);await flush();assert.equal(f.history.getSnapshot().items.length,1);assert.equal(f.history.getSnapshot().availability,'unavailable');
  const ready={startupComplete:true,workspaceReady:true,answers:'ready',refreshing:false,unavailable:false};
  assert.equal(notificationAvailability({...ready,notices:'loading'}),'loading');assert.equal(notificationAvailability({...ready,notices:'unavailable'}),'unavailable');
  fail=false;f.history.retry();await settled(f.history);assert.equal(f.history.getSnapshot().availability,'ready');
});

test('repeated setup and a burst of duplicate notices start one local load and one publication',async()=>{
  const gate=deferred();const f=fixture({load:()=>gate.promise});f.history.start();f.history.start();
  await Promise.all(Array.from({length:40},()=>f.history.publish(input)));assert.equal(f.history.getSnapshot().items.length,1);gate.resolve(emptyNoticeHistory());await settled(f.history);
  assert.deepEqual(f.calls.map(c=>c.kind),['load','publish']);
});

test('read and dismiss projection never mutates the saved data or another notice',async()=>{
  const normalized=normalizeNotice(input),id=await noticeKey(normalized),item={id,input:normalized,createdAtUnixMs:1,read:false};const base={schemaVersion:1,items:[item],dismissed:[]};
  assert.equal(projectNotices(base,[{operation:{kind:'read',id}}]).items[0].read,true);assert.equal(base.items[0].read,false);
  assert.equal(projectNotices(base,[{operation:{kind:'dismiss',id}}]).items.length,0);assert.equal(base.items.length,1);assert.deepEqual(base.dismissed,[]);
});

test('new background notices notify subscribers without executing navigation or reopening a panel',async()=>{
  const f=fixture();let changes=0;const stop=f.history.subscribe(()=>changes++);await f.history.publish(input);await settled(f.history);assert.ok(changes>0);
  const count=changes;stop();await f.history.publish({...input,message:'After unsubscribe'});await settled(f.history);assert.equal(changes,count);
  assert.ok(f.calls.every(c=>['load','publish'].includes(c.kind)));
});

test('a fresh explicit attempt can report the same failure after its earlier outcome was dismissed',async()=>{
  const f=fixture();await f.history.publish({...input,occurrenceId:'attempt-one'});await settled(f.history);
  f.history.dismiss(f.history.getSnapshot().items[0].id);await settled(f.history);
  await f.history.publish({...input,occurrenceId:'attempt-one'});await settled(f.history);assert.equal(f.history.getSnapshot().items.length,0);
  await f.history.publish({...input,occurrenceId:'attempt-two'});await settled(f.history);assert.equal(f.history.getSnapshot().items.length,1);assert.equal(f.history.getSnapshot().items[0].read,false);
});

test('resolution during identity preparation cannot resurrect an already resolved outcome',async()=>{
  const f=fixture();const preparing=f.history.publish({...input,occurrenceId:'episode-one'});
  f.history.resolveOccurrence(input.projectId,input.snapshotId,'episode-one');await preparing;await flush();
  assert.deepEqual(f.history.getSnapshot().items,[]);assert.deepEqual(f.calls,[]);
});

test('resolution during a held publication queues the exact acknowledgment and preserves other scopes',async()=>{
  const gate=deferred(),started=deferred();let saved=emptyNoticeHistory();let held=true;
  const f=fixture({update:async operation=>{
    if(operation.kind==='publish'&&held){held=false;started.resolve();await gate.promise;}
    const item=operation.kind==='publish'?{id:await noticeKey(operation.input),input:operation.input,createdAtUnixMs:1,read:false}:undefined;
    saved=projectNotices(saved,[{operation,...(item?{item}:{})}]);return structuredClone(saved);
  }});
  await f.history.publish({...input,occurrenceId:'episode-one'});await started.promise;
  await f.history.publish({...input,projectId:'another-project',occurrenceId:'episode-one'});
  f.history.resolveOccurrence(input.projectId,input.snapshotId,'episode-one');gate.resolve();await settled(f.history);
  assert.equal(f.history.getSnapshot().items.length,1);assert.equal(f.history.getSnapshot().items[0].input.projectId,'another-project');assert.equal(saved.dismissed.length,1);
});

test('production uses typed persisted targets, scoped outcomes and explicit local history recovery',async()=>{
  const [app,hook,native,lib]=await Promise.all(['../src/App.tsx','../src/lib/useNoticeHistory.ts','../src-tauri/src/app_notices.rs','../src-tauri/src/lib.rs'].map(path=>readFile(new URL(path,import.meta.url),'utf8')));
  assert.match(app,/useNoticeHistory\(\)/);assert.doesNotMatch(app,/useState<Notice>|id: 'workspace-notice'|setNotice\(null\)|title: 'Repository selected'/);
  assert.match(app,/notices:noticeHistoryState.availability/);assert.match(app,/noticeHistory\.read\(item.id\);openNoticeTarget\(item\)/);assert.match(app,/run:\(\)=>noticeHistory.retry\(\)/);
  assert.match(app,/getWorkspace\(projectId,snapshotId\)\.explorations\.some/);assert.match(app,/projectId:currentCapture.projectId,snapshotId:currentCapture.snapshotId/);
  assert.match(app,/pickLocalRepository\('notifications'\)/);assert.match(app,/focusTarget:'workspace-composer'/);
  assert.match(hook,/invoke<NoticeHistoryData>\('list_app_notices'\)/);assert.match(hook,/invoke<NoticeHistoryData>\('update_app_notices',\{operation\}\)/);
  assert.match(lib,/app_notices::list_app_notices/);assert.match(lib,/app_notices::update_app_notices/);
  assert.doesNotMatch(native,/start_repository_guide|start_workspace|review_runtime|reqwest|executable|api_key/);
});
