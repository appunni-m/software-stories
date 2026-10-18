import test from 'node:test';
import assert from 'node:assert/strict';
import {createChapterHistory} from '../src/lib/chapterHistory.ts';
import {baseRevision} from './helpers/revision-history-fixture.mjs';

const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
const revision=(id,number=1)=>({...baseRevision(),projectId:'p',revisionId:id,revisionNumber:number});
function setup(load){
  const values=[],errors=[];let loaded=0,idle=0;
  const history=createChapterHistory({projectId:'p',load,onValue:value=>values.push(value),onLoaded:()=>loaded++,onError:error=>errors.push(error),onIdle:()=>idle++});
  return {history,values,errors,loaded:()=>loaded,idle:()=>idle};
}

test('a native write is visible before a failed history read and an explicit retry preserves all history',async()=>{
  const first=revision('first'),saved=revision('saved',2);let calls=0;
  const {history,values,errors,loaded}=setup(async()=>{calls++;if(calls===2)throw new Error('read unavailable');return calls===1?[first]:[first,saved];});
  await history.refresh();history.record(saved);
  assert.deepEqual(values.at(-1),[first,saved]);await history.refresh();
  assert.equal(calls,2);assert.equal(errors.length,1);assert.equal(loaded(),1);assert.deepEqual(values.at(-1),[first,saved]);
  await Promise.resolve();assert.equal(calls,2,'read errors cannot schedule a retry');
  await history.refresh();assert.equal(loaded(),2);assert.deepEqual(values.at(-1),[first,saved]);
});

test('a pre-write in-flight history read cannot hide the native receipt',async()=>{
  const reads=[deferred(),deferred()],started=[deferred(),deferred()];let calls=0;
  const {history,values,errors}=setup(()=>{const i=calls++;started[i].resolve();return reads[i].promise;});
  const work=history.refresh();await started[0].promise;
  const saved=revision('saved');history.record(saved);assert.deepEqual(values.at(-1),[saved]);
  reads[0].resolve([]);await started[1].promise;assert.deepEqual(values,[[saved]]);
  reads[1].resolve([saved]);await work;assert.deepEqual(values.at(-1),[saved]);assert.deepEqual(errors,[]);assert.equal(calls,2);
});

test('multiple receipts remain distinct when a history read predates one of them',async()=>{
  let rows=[];const {history,values}=setup(async()=>rows);
  const first=revision('first'),second=revision('second',2);
  history.record(first);await history.refresh();assert.deepEqual(values.at(-1),[first]);
  rows=[first];history.record(second);await history.refresh();assert.deepEqual(values.at(-1),[first,second]);
  rows=[first,second];await history.refresh();assert.deepEqual(values.at(-1),rows);
  assert.notEqual(values.at(-1),rows,'the native response array remains owned by its caller');
});

test('disposing an editor drops queued reads and late UI callbacks and ignores later receipts',async()=>{
  const read=deferred(),started=deferred();let calls=0;
  const {history,values,errors,idle}=setup(()=>{calls++;started.resolve();return read.promise;});
  const work=history.refresh();await started.promise;history.refresh();history.dispose();history.record(revision('late'));
  read.reject(new Error('old editor'));await work;await history.refresh();
  assert.equal(calls,1);assert.deepEqual(values,[]);assert.deepEqual(errors,[]);assert.equal(idle(),0);
});

test('a receipt for a different repository cannot enter current history or trigger a read',async()=>{
  let calls=0;const {history,values}=setup(async()=>{calls++;return [];});
  history.record({...revision('foreign'),projectId:'other'});await Promise.resolve();
  assert.deepEqual(values,[]);assert.equal(calls,0);
});

test('the serial history reader collapses invalidations without mutating immutable revisions',async()=>{
  const first=revision('first'),saved=revision('saved',2);const original=JSON.stringify([first,saved]);
  Object.freeze(first);Object.freeze(saved);
  const read=deferred(),started=deferred();let calls=0;
  const {history,values}=setup(async()=>{calls++;if(calls===1){started.resolve();return read.promise;}return [first,saved];});
  const work=history.refresh();await started.promise;history.record(saved);
  for(let i=0;i<40;i++)assert.equal(history.refresh(),work);
  read.resolve([first]);await work;assert.equal(calls,2);assert.deepEqual(values.at(-1),[first,saved]);assert.equal(JSON.stringify([first,saved]),original);
});
