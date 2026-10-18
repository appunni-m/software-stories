import test from 'node:test';
import assert from 'node:assert/strict';
import {workspaceStoreHarness} from './helpers/workspace-store-harness.mjs';

test('repeated failed conversation writes report one scoped episode and preserve the in-memory workspace',()=>{
  const h=workspaceStoreHarness(()=>assert.fail('saving UI state cannot call native generation'));const notices=[];
  h.onWindowEvent('code-atlas:workspace-notice',event=>notices.push(event.detail));
  h.storage.set=()=>{throw new Error('controlled storage failure');};
  for(let n=0;n<40;n++)h.store.updateWorkspace('/work/Font App','snapshot',state=>({...state,file:`src/file-${n}.rs`}));
  assert.equal(notices.length,1);assert.equal(notices[0].projectId,'/work/Font App');assert.equal(notices[0].snapshotId,'snapshot');
  assert.match(notices[0].occurrenceId,/^[a-f0-9-]{36}$/);assert.doesNotMatch(notices[0].message,/controlled|quota|Error/);
  assert.equal(h.store.getWorkspace('/work/Font App','snapshot').file,'src/file-39.rs');assert.deepEqual(h.calls,[]);
});

test('a successful write resolves only its episode and a later failure is a new outcome',()=>{
  const h=workspaceStoreHarness(()=>assert.fail('no native work'));const notices=[],resolved=[];const original=h.storage.set;
  h.onWindowEvent('code-atlas:workspace-notice',event=>notices.push(event.detail));h.onWindowEvent('code-atlas:workspace-notice-resolved',event=>resolved.push({...event.detail}));
  h.storage.set=()=>{throw new Error('unavailable');};
  h.store.updateWorkspace('one','snapshot',s=>({...s,file:'one.rs'}));h.store.updateWorkspace('two','snapshot',s=>({...s,file:'two.rs'}));
  h.storage.set=original;h.store.updateWorkspace('one','snapshot',s=>({...s,file:'saved.rs'}));
  assert.deepEqual(resolved,[{projectId:'one',snapshotId:'snapshot',occurrenceId:notices[0].occurrenceId}]);
  h.storage.set=()=>{throw new Error('unavailable');};h.store.updateWorkspace('one','snapshot',s=>({...s,file:'new.rs'}));
  assert.equal(notices.length,3);assert.notEqual(notices[0].occurrenceId,notices[2].occurrenceId);assert.equal(resolved.length,1);
  h.store.updateWorkspace('two','snapshot',s=>({...s,file:'still-unsaved.rs'}));assert.equal(notices.length,3);assert.deepEqual(h.calls,[]);
});
