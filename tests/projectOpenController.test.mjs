import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ProjectOpenController } from '../src/lib/projectOpenController.ts';

const ready = { status: 'opened', project: { projectId: 'repo' }, message: null };
const abort = (error) => error?.name === 'AbortError';
function setup(failCancel = false) {
  const calls=[], cancelled=[];let id=0;
  const controller=new ProjectOpenController({
    open:(input)=>new Promise((resolve,reject)=>calls.push({input,resolve,reject})),
    cancel:async(id)=>{cancelled.push(id);if(failCancel)throw new Error('cancel transport failed');},
  },()=>`request-${++id}`);
  return {controller,calls,cancelled};
}
test('project opening publishes busy and one owned identity for either destination',async()=>{
  const {controller,calls,cancelled}=setup();const states=[];
  const unsubscribe=controller.subscribe(()=>states.push(controller.isBusy()));
  const result=controller.open({type:'recent',projectId:'repo'});
  assert.deepEqual(calls[0].input,{type:'recent',projectId:'repo',requestId:'request-1'});
  calls[0].resolve(ready);assert.equal(await result,ready);
  assert.deepEqual(states,[true,false]);assert.deepEqual(cancelled,[]);unsubscribe();
});
test('Stop releases UI immediately and refuses a late successful opening',async()=>{
  const {controller,calls,cancelled}=setup();const result=controller.open({type:'picker'});
  const rejected=assert.rejects(result,abort);controller.cancel();
  assert.equal(controller.isBusy(),false);assert.deepEqual(cancelled,['request-1']);
  calls[0].resolve(ready);await rejected;
});
test('a newer recent or picker intent supersedes the old destination without losing ownership',async()=>{
  const {controller,calls,cancelled}=setup();const first=controller.open({type:'picker'});
  const rejected=assert.rejects(first,abort);const second=controller.open({type:'recent',projectId:'other'});
  assert.deepEqual(cancelled,['request-1']);calls[0].resolve(ready);await rejected;
  assert.equal(controller.isBusy(),true);calls[1].resolve(ready);assert.equal(await second,ready);
});
test('an already aborted caller does not dispatch or stop the current operation',async()=>{
  const {controller,calls,cancelled}=setup();const first=controller.open({type:'picker'});
  const signal=new AbortController();signal.abort();
  await assert.rejects(controller.open({type:'recent',projectId:'other'},signal.signal),abort);
  assert.equal(calls.length,1);assert.deepEqual(cancelled,[]);assert.equal(controller.isBusy(),true);
  calls[0].resolve(ready);await first;
});
test('an external abort suppresses both a late error and a failing cancellation reply',async()=>{
  const {controller,calls,cancelled}=setup(true);const signal=new AbortController();
  const result=controller.open({type:'picker'},signal.signal);const rejected=assert.rejects(result,abort);
  signal.abort();assert.equal(controller.isBusy(),false);assert.deepEqual(cancelled,['request-1']);
  calls[0].reject(new Error('late native failure'));await rejected;
});
test('a cancelled native folder dialog is an ordinary result and leaves no busy owner',async()=>{
  const {controller,calls,cancelled}=setup();const result=controller.open({type:'picker'});
  calls[0].resolve({status:'cancelled',project:null});assert.equal((await result).status,'cancelled');
  assert.equal(controller.isBusy(),false);assert.deepEqual(cancelled,[]);
});
test('a synchronous Stop during busy notification prevents native dispatch',async()=>{
  const {controller,calls,cancelled}=setup();controller.subscribe(()=>{if(controller.isBusy())controller.cancel();});
  await assert.rejects(controller.open({type:'picker'}),abort);
  assert.equal(calls.length,0);assert.deepEqual(cancelled,[]);assert.equal(controller.isBusy(),false);
});
test('a native error releases ownership and completion removes external abort listeners',async()=>{
  const {controller,calls,cancelled}=setup();const signal=new AbortController();
  const result=controller.open({type:'recent',projectId:'repo'},signal.signal);
  const rejected=assert.rejects(result,/real failure/);calls[0].reject(new Error('real failure'));await rejected;
  signal.abort();assert.deepEqual(cancelled,[]);assert.equal(controller.isBusy(),false);
});
