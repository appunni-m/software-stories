import test from 'node:test';
import assert from 'node:assert/strict';
import { ResourceCache } from '../src/shared/cache.mjs';
import { ResidentWorker } from '../src/server/voice-worker.mjs';

test('returning to immutable resources shares in-flight reads and avoids another load', async () => {
  const cache=new ResourceCache(2); let loads=0,release;
  const load=()=>{loads++;return new Promise(accept=>{release=accept;});};
  const first=cache.get('snapshot/file',load),second=cache.get('snapshot/file',load);
  assert.equal(first,second);await Promise.resolve();release({text:'saved source'});
  assert.deepEqual(await first,{text:'saved source'});
  await cache.get('snapshot/file',load);assert.equal(loads,1);
  await cache.get('different-snapshot/file',()=>++loads);assert.equal(loads,2);
});
test('failed cached requests can retry and evicted resources are disposed',async()=>{
  const disposed=[],cache=new ResourceCache(1,value=>disposed.push(value));
  await assert.rejects(cache.get('retry',()=>{throw new Error('offline');}),/offline/);
  assert.equal(await cache.get('retry',()=> 'recovered'),'recovered');
  await cache.get('new',()=> 'new resource');await Promise.resolve();
  assert.deepEqual(disposed,['recovered']);
});
test('resident worker handles multiple requests in one process and rejects pending work on exit',async()=>{
  const program=`console.log(JSON.stringify({type:'ready',pid:process.pid,modelLoads:1}));
    require('readline').createInterface({input:process.stdin}).on('line',line=>{
      const r=JSON.parse(line);if(r.stop)process.exit(7);
      console.log(JSON.stringify({type:'result',id:r.id,text:r.text,pid:process.pid}));
    });`;
  const worker=new ResidentWorker(process.execPath,['-e',program],{readyTimeout:5000,requestTimeout:5000});
  try{
    const first=await worker.request({text:'one'}),second=await worker.request({text:'two'});
    assert.equal(first.pid,second.pid);assert.equal(second.text,'two');
    await assert.rejects(worker.request({stop:true}),/stopped/);assert.equal(worker.alive,false);
  }finally{worker.stop();}
});
