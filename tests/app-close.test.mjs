import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {CloseCoordinator,CloseRequestController} from '../src/lib/closeCoordinator.ts';

const id='11111111-1111-4111-8111-111111111111';
const second='22222222-2222-4222-8222-222222222222';
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function fixture(){
  const coordinator=new CloseCoordinator(),completed=[],busy=[];let failures=0;
  const controller=new CloseRequestController(coordinator,{complete:async(key,saved)=>{completed.push({key,saved});},busy:value=>busy.push(value),failed:()=>{failures++;}});
  return {coordinator,controller,completed,busy,get failures(){return failures;}};
}

test('captures every reader before freezing any store, and waits for all writes before acknowledging',async()=>{
  const f=fixture(),gate=deferred(),order=[];
  for(const name of ['conversation','notices','audio'])f.coordinator.register({capture:()=>order.push(`capture:${name}`),freeze:()=>{order.push(`freeze:${name}`);return()=>order.push(`resume:${name}`);},flush:()=>{order.push(`flush:${name}`);return name==='conversation'?gate.promise:Promise.resolve();}});
  const closing=f.controller.request(id);await tick();assert.equal(f.completed.length,0);
  assert.deepEqual(order.slice(0,6),['capture:conversation','capture:notices','capture:audio','freeze:conversation','freeze:notices','freeze:audio']);
  gate.resolve();await closing;assert.deepEqual(f.completed,[{key:id,saved:true}]);assert.equal(order.some(s=>s.startsWith('resume')),false,'stores stay frozen until native closes');
  f.controller.cancel(id,false);assert.deepEqual(order.slice(-3),['resume:audio','resume:notices','resume:conversation']);
});

test('a failed save releases every reader, refuses close and never retries on its own',async()=>{
  const f=fixture();let writes=0,resumes=0;
  f.coordinator.register({freeze:()=>()=>{resumes++;},flush:async()=>{writes++;throw new Error('private filesystem detail');}});
  await f.controller.request(id);await tick();
  assert.deepEqual(f.completed,[{key:id,saved:false}]);assert.deepEqual(f.busy,[true,false]);assert.equal(f.failures,1);assert.equal(resumes,1);assert.equal(writes,1);
  await tick();assert.equal(writes,1);
});

test('later flush stages include save acknowledgments produced by earlier writers',async()=>{
  const coordinator=new CloseCoordinator(),abort=new AbortController(),held=deferred(),order=[];
  coordinator.register({flushOrder:1,flush:async()=>{order.push('notices');}});
  coordinator.register({flush:async()=>{order.push('conversation');await held.promise;order.push('resolved');}});
  const saving=coordinator.prepare(abort.signal);await tick();assert.deepEqual(order,['conversation']);
  held.resolve();const release=await saving;assert.deepEqual(order,['conversation','resolved','notices']);release();
});

test('native timeout releases a hung writer and its late success cannot close the app',async()=>{
  const f=fixture(),held=deferred();let resumes=0;
  f.coordinator.register({freeze:()=>()=>{resumes++;},flush:()=>held.promise});
  const closing=f.controller.request(id);await tick();f.controller.cancel('not-a-request');assert.equal(resumes,0);
  f.controller.cancel(id);await closing;assert.equal(resumes,1);assert.equal(f.failures,1);assert.deepEqual(f.completed,[]);
  held.resolve();await tick();assert.deepEqual(f.completed,[]);
});

test('repeated native events share one barrier and forged IDs cannot initiate an acknowledgment',async()=>{
  const f=fixture(),held=deferred();let writes=0;
  f.coordinator.register({flush:()=>{writes++;return held.promise;}});
  await f.controller.request('../forged');const first=f.controller.request(id),again=f.controller.request(id);
  assert.equal(first,again);assert.equal(writes,1);held.resolve();await first;assert.equal(f.completed.length,1);
  f.controller.cancel(id,false);
});

test('a superseded request cannot acknowledge or release the next request',async()=>{
  const f=fixture(),one=deferred(),two=deferred();let writes=0,frozen=false;
  f.coordinator.register({freeze:()=>{assert.equal(frozen,false);frozen=true;return()=>{frozen=false;};},flush:()=>++writes===1?one.promise:two.promise});
  const first=f.controller.request(id);const next=f.controller.request(second);await first;
  assert.equal(frozen,true);one.resolve();await tick();assert.equal(frozen,true);assert.deepEqual(f.completed,[]);
  two.resolve();await next;assert.deepEqual(f.completed,[{key:second,saved:true}]);f.controller.cancel(second,false);
});

test('rejected native acknowledgment keeps the app open and restores interactivity',async()=>{
  const coordinator=new CloseCoordinator(),completed=[],busy=[];let resumes=0,failures=0;
  coordinator.register({freeze:()=>()=>{resumes++;},flush:async()=>{}});
  const controller=new CloseRequestController(coordinator,{busy:value=>busy.push(value),failed:()=>{failures++;},complete:async(key,saved)=>{completed.push(saved);if(saved)throw new Error('expired');}});
  await controller.request(id);assert.deepEqual(completed,[true,false]);assert.equal(resumes,1);assert.equal(failures,1);assert.deepEqual(busy,[true,false]);
});

test('capture and partial-freeze errors never grant close and release earlier participants',async()=>{
  for(const phase of ['capture','freeze']){
    const f=fixture();let resumes=0,flushes=0;
    f.coordinator.register({freeze:()=>()=>{resumes++;},flush:async()=>{flushes++;}});
    f.coordinator.register({[phase]:()=>{throw new Error('reader failed');},flush:async()=>{flushes++;}});
    await f.controller.request(id);assert.deepEqual(f.completed,[{key:id,saved:false}]);assert.equal(flushes,0);assert.equal(resumes,phase==='freeze'?1:0);
  }
});

test('release remains idempotent and continues if one subscriber throws',async()=>{
  const coordinator=new CloseCoordinator(),abort=new AbortController();let resumed=0;
  coordinator.register({freeze:()=>()=>{resumed++;},flush:async()=>{}});
  coordinator.register({freeze:()=>()=>{throw new Error('subscriber');},flush:async()=>{}});
  const release=await coordinator.prepare(abort.signal);release();release();abort.abort();assert.equal(resumed,1);
});

test('unregistered readers and already cancelled requests do not participate',async()=>{
  const coordinator=new CloseCoordinator(),abort=new AbortController();let calls=0;
  coordinator.register({capture:()=>{calls++;},flush:async()=>{calls++;}})();
  const release=await coordinator.prepare(abort.signal);release();assert.equal(calls,0);
  abort.abort();await assert.rejects(coordinator.prepare(abort.signal),/cancelled/);assert.equal(calls,0);
});

test('production registers the native handshake before mounting and covers Quit, window close and loaded audio',async()=>{
  const files=['src/main.tsx','src/lib/appClose.ts','src/lib/useNoticeHistory.ts','src/NarrationPlayer.tsx','src-tauri/src/app_close.rs','src-tauri/src/lib.rs'];
  const [main,bridge,notices,audio,native,lib]=await Promise.all(files.map(p=>readFile(new URL(`../${p}`,import.meta.url),'utf8')));
  assert.ok(main.indexOf('await installAppCloseGuard()')<main.indexOf('root.render(<AppErrorBoundary>'));
  assert.match(bridge,/freeze:beginWorkspaceClose,flush:flushAllWorkspaceConversations/);assert.match(bridge,/root\.inert = true/);assert.match(bridge,/root\.inert = previousInert/);
  assert.match(bridge,/status\.setAttribute\('role','status'\)/);assert.match(bridge,/Saving your place…/);
  assert.match(notices,/freeze:\(\)=>history\.beginClose\(\),flush:\(\)=>history\.flush\(\)/);
  assert.match(audio,/player\.readyState>=1&&restoreTimeRef\.current===null/);assert.match(audio,/persistPlaybackPosition\(player\.currentTime\)/);
  assert.match(native,/RunEvent::ExitRequested/);assert.match(native,/WindowEvent::CloseRequested/);assert.match(native,/api\.prevent_exit\(\)/);assert.match(native,/api\.prevent_close\(\)/);assert.match(native,/Duration::from_secs\(20\)/);
  assert.match(lib,/app_close::handle_event\(app, &event\)/);
});

test('macOS menu Quit and Cmd+Q start the save handshake without the predefined terminate action',async()=>{
  const [native,lib]=await Promise.all(['src-tauri/src/app_close.rs','src-tauri/src/lib.rs'].map(p=>readFile(new URL(`../${p}`,import.meta.url),'utf8')));
  assert.match(native,/application\.remove_at\(position\)/);
  assert.match(native,/MenuItem::with_id\(app, QUIT_MENU, title, true, Some\("CmdOrCtrl\+Q"\)\)/);
  assert.match(native,/event\.id\(\)\.as_ref\(\) == QUIT_MENU/);
  assert.match(native,/if request\(app, Intent::Quit\) == Decision::Allow \{\s*app\.exit\(0\);/);
  assert.match(native,/app\.exit\(0\)/);
  assert.match(lib,/\.menu\(app_close::menu\)/);
  assert.match(lib,/\.on_menu_event\(app_close::handle_menu_event\)/);
});
