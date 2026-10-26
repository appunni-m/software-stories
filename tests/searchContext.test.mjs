import assert from 'node:assert/strict';
import {describe,it} from 'node:test';
import {readRepositorySearchContext,rememberRepositorySearchContext} from '../src/workspace/searchContext.ts';

describe('repository search context',()=>{
  const withStorage=(storage,run)=>{
    const previous=Object.getOwnPropertyDescriptor(globalThis,'localStorage');
    Object.defineProperty(globalThis,'localStorage',{configurable:true,value:storage});
    try{run();}finally{
      if(previous)Object.defineProperty(globalThis,'localStorage',previous);
      else delete globalThis.localStorage;
    }
  };

  it('restores a search only for the same project and immutable snapshot',()=>{
    const saved=new Map();
    withStorage({getItem:key=>saved.get(key)??null,setItem:(key,value)=>saved.set(key,value)},()=>{
      rememberRepositorySearchContext('project-a','snapshot-a',{query:'Font::memory_face',mode:'regex',activity:'search'});
      const restored=readRepositorySearchContext('project-a','snapshot-a');
      assert.deepEqual(restored&&{projectId:restored.projectId,snapshotId:restored.snapshotId,query:restored.query,mode:restored.mode,activity:restored.activity},{
        projectId:'project-a',snapshotId:'snapshot-a',query:'Font::memory_face',mode:'regex',activity:'search',
      });
      assert.ok(Number.isSafeInteger(restored?.updatedAt));
      assert.equal(readRepositorySearchContext('project-b','snapshot-a'),undefined);
      assert.equal(readRepositorySearchContext('project-a','snapshot-b'),undefined);
      const record=JSON.parse([...saved.values()][0]);
      assert.deepEqual(Object.keys(record),['version','entries']);
      assert.deepEqual(Object.keys(record.entries[0]).sort(),['activity','mode','projectId','query','snapshotId','updatedAt']);
    });
  });

  it('keeps only the newest 32 bounded entries',()=>{
    const saved=new Map();
    withStorage({getItem:key=>saved.get(key)??null,setItem:(key,value)=>saved.set(key,value)},()=>{
      for(let index=0;index<33;index++)rememberRepositorySearchContext(`project-${index}`,`snapshot-${index}`,{
        query:`query-${index}`,mode:'code',activity:'files',
      });
      assert.equal(readRepositorySearchContext('project-0','snapshot-0'),undefined);
      assert.equal(readRepositorySearchContext('project-32','snapshot-32')?.query,'query-32');
      assert.ok([...saved.values()][0].length<=32_768);
    });
  });

  it('ignores corrupt, oversized, or unavailable local preference data without throwing',()=>{
    const saved=new Map([['code-atlas:repository-search:v1','{']]);
    withStorage({getItem:key=>saved.get(key)??null,setItem:(key,value)=>saved.set(key,value)},()=>{
      assert.equal(readRepositorySearchContext('project-a','snapshot-a'),undefined);
      saved.set('code-atlas:repository-search:v1',' '.repeat(32_769));
      assert.equal(readRepositorySearchContext('project-a','snapshot-a'),undefined);
      assert.doesNotThrow(()=>rememberRepositorySearchContext('project-a','snapshot-a',{query:'x'.repeat(241),mode:'code',activity:'search'}));
      assert.equal(saved.get('code-atlas:repository-search:v1'),' '.repeat(32_769));
    });
    withStorage({getItem(){throw Error('blocked');},setItem(){throw Error('blocked');}},()=>{
      assert.equal(readRepositorySearchContext('project-a','snapshot-a'),undefined);
      assert.doesNotThrow(()=>rememberRepositorySearchContext('project-a','snapshot-a',{query:'foo',mode:'code',activity:'search'}));
    });
  });
});
