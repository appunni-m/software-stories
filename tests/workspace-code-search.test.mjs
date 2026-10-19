import test from 'node:test';
import assert from 'node:assert/strict';
import { searchCapturedSource, searchExploration, shouldOfferExploreWithAI, workspaceSearchResults } from '../src/workspace/codeSearch.ts';

function harness(request={}) {
  const state={hits:['old'],limited:true,nextOffset:32,busy:false,error:'old'},calls=[],timers=new Map();let next=0,reply;
  const promise=new Promise((resolve,reject)=>{reply={resolve,reject};});
  const runtime={invoke:(command,args)=>{calls.push({command,args});return command==='search_workspace_source'?promise:Promise.resolve();},createId:()=>`request:${next}`,defer:run=>{timers.set(++next,run);return next;},clear:id=>timers.delete(id)};
  const updates=Object.fromEntries(Object.keys(state).map(key=>[key,value=>{state[key]=value;}]));
  updates.hits=(hits,append)=>{state.hits=append?[...state.hits,...hits]:hits;};
  const stop=searchCapturedSource({projectId:'p',snapshotId:'s',query:'needle',mode:'code',native:true,...request},updates,runtime);
  return {state,calls,reply,stop,dispatch:()=>{for(const [id,run] of timers){timers.delete(id);run();}}};
}
const settle=()=>new Promise(resolve=>setImmediate(resolve));
test('debounced cancellation before dispatch performs no native work and clears previous results',()=>{
  const h=harness();assert.deepEqual(h.state,{hits:[],limited:false,nextOffset:null,busy:true,error:null});h.stop();h.dispatch();assert.deepEqual(h.calls,[]);
});
test('regex sends exact case and escapes with explicit mode; search never invokes AI',async()=>{
  const query=String.raw`\S+::(Ready|Busy)`;const h=harness({query,mode:'regex'});h.dispatch();
  assert.deepEqual(h.calls,[{command:'search_workspace_source',args:{projectId:'p',snapshotId:'s',query,mode:'regex',requestId:'request:0',offset:0}}]);
  h.reply.resolve({matches:[{path:'a.rs',line:2,text:'value::Ready'}],limited:true,nextOffset:32});await settle();
  assert.equal(h.state.busy,false);assert.equal(h.state.limited,true);assert.equal(h.state.nextOffset,32);assert.equal(h.state.hits[0].line,2);assert.equal(h.state.error,null);
});
test('a requested next page preserves and appends prior results with its next offset',async()=>{
  const h=harness({offset:32});assert.deepEqual(h.state.hits,['old']);assert.equal(h.state.nextOffset,32);h.dispatch();
  assert.deepEqual(h.calls,[{command:'search_workspace_source',args:{projectId:'p',snapshotId:'s',query:'needle',mode:'code',requestId:'request:0',offset:32}}]);
  h.reply.resolve({matches:[{path:'next.rs',line:40,text:'needle'}],limited:true,nextOffset:64});await settle();
  assert.deepEqual(h.state.hits,['old',{path:'next.rs',line:40,text:'needle'}]);assert.equal(h.state.nextOffset,64);
});
test('cancellation after dispatch owns a native stop and rejects every late result',async()=>{
  for(const reject of [false,true]){
    const h=harness();h.dispatch();h.stop();const stopped=structuredClone(h.state);
    if(reject)h.reply.reject(new Error('late native error'));else h.reply.resolve({matches:[{path:'old',line:1,text:'old'}],limited:true});await settle();
    assert.deepEqual(h.state,stopped);assert.deepEqual(h.calls.at(-1),{command:'cancel_repository_context',args:{requestId:'request:0'}});
  }
});
test('mode changes cannot let an older literal response replace regex results',async()=>{
  const old=harness();old.dispatch();old.stop();const current=harness({mode:'regex',query:'.'});current.dispatch();current.reply.resolve({matches:[{path:'new',line:1,text:'regex'}],limited:false});await settle();
  old.reply.resolve({matches:[{path:'old',line:1,text:'literal'}],limited:true});await settle();assert.equal(current.state.hits[0].path,'new');assert.deepEqual(old.state.hits,[]);
});
test('native regex findings remain actionable and ordinary search retains its fallback message',async()=>{
  for(const mode of ['regex','code']){
    const h=harness({mode});h.dispatch();h.reply.reject('Unsupported regular expression.');await settle();
    assert.equal(h.state.busy,false);assert.deepEqual(h.state.hits,[]);assert.equal(h.state.error,mode==='regex'?'Unsupported regular expression.':'Captured source search is unavailable. File and symbol results are still shown.');
  }
});
test('minimum lengths follow mode and Unicode code points; nonnative and excessive inputs do no work',()=>{
  for(const request of [{native:false},{query:''},{query:'a'},{query:'x'.repeat(241)},{query:'🧭'.repeat(241),mode:'regex'}]){const h=harness(request);h.dispatch();assert.deepEqual(h.calls,[]);assert.equal(h.state.busy,false);}
  for(const query of ['.','🧭'.repeat(240)]){const h=harness({query,mode:'regex'});h.dispatch();assert.equal(h.calls.length,1);h.stop();}
});
test('regex view contains only native captured matches, with no fuzzy or JavaScript interpretation',()=>{
  const captured=[{path:'actual.rs',line:7,text:'value::Ready'}];
  const hits=workspaceSearchResults({query:'[',mode:'regex',files:[{path:'['}],symbols:[{path:'fake',name:'[',anchor:{startLine:1,endLine:1}}],source:{path:'fake',contents:'['},captured});
  assert.deepEqual(hits,[{path:'actual.rs',text:'value::Ready',from:7,to:7}]);
});
test('ordinary results keep native ranking before current-file lines, deduplicate and cap local matches',()=>{
  const args={query:'NEEDLE',mode:'code',files:[{path:'needle.rs'}],symbols:[],source:{path:'current.rs',contents:'needle\n'.repeat(500)},captured:[{path:'native.rs',line:4,text:'needle'}, {path:'native.rs',line:4,text:'needle'}]};
  const hits=workspaceSearchResults(args);assert.equal(hits[0].path,'needle.rs');assert.equal(hits[1].path,'native.rs');assert.equal(hits.length,34);assert.equal(hits.at(-1).from,32);
});

test('an exact symbol declaration leads partial symbol and fixture-file matches',()=>{
  const hits=workspaceSearchResults({query:'memory_face',mode:'code',files:[
    {path:'tests/fixtures/ft_new_memory_face.json'},
    {path:'src/font.rs'},
  ],symbols:[
    {path:'src/api.rs',name:'new_memory_face',anchor:{startLine:165,endLine:180}},
    {path:'src/font.rs',name:'memory_face',anchor:{startLine:4053,endLine:4115}},
    {path:'src/other.rs',name:'memory_face_wrapper',anchor:{startLine:10,endLine:20}},
  ],source:null,captured:[]});
  assert.deepEqual(hits.slice(0,4).map(hit=>[hit.path,hit.text]),[
    ['src/font.rs','memory_face'],
    ['src/other.rs','memory_face_wrapper'],
    ['src/api.rs','new_memory_face'],
    ['tests/fixtures/ft_new_memory_face.json','tests/fixtures/ft_new_memory_face.json'],
  ]);
});

test('production search binds the helper, explicit pressed mode and Mac physical shortcut',async()=>{
  const {readFileSync}=await import('node:fs');const source=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  const native=readFileSync(new URL('../src-tauri/src/repository_context_commands.rs',import.meta.url),'utf8');
  const handlerStart=native.indexOf('pub(crate) async fn search_workspace_source(');
  const handlerEnd=native.indexOf('\n#[cfg(test)]',handlerStart);
  const handler=native.slice(handlerStart,handlerEnd);
  assert.match(handler,/\boffset: usize,/);
  assert.doesNotMatch(handler,/offset:\s*Option/);
  assert.match(handler,/limit:\s*32,\s*offset,/);
  assert.match(source,/searchMode:CodeSearchMode=activity==='search'&&repositoryRegex\?'regex':'code'/);
  assert.match(source,/useEffect\(\(\)=>searchCapturedSource\(\{projectId,snapshotId,query:term,mode:searchMode,native:isNativeRuntime\(\),offset:searchOffset\}/);
  assert.match(source,/nextOffset:setSearchNextOffset/);
  assert.match(source,/Load more matches/);
  assert.match(source,/searchNextOffset!==null&&results\.length<80/);
  assert.match(source,/aria-label="Use regular expression" aria-pressed=\{repositoryRegex\}/);
  assert.match(source,/event\.altKey&&event\.code==='KeyR'/);
  assert.match(source,/repositorySearch&&!searchBusy&&!searchError&&results\.length===0/);
  assert.match(source,/shouldOfferExploreWithAI\(repositorySearch,searchMode,searchBusy,searchError,results\.length\)/);
  assert.match(source,/shouldOfferExploreWithAI\(query,searchMode,searchBusy,searchError,results\.length\)/);
});

test('Explore with AI appears only when a settled search has no matches or search error',()=>{
  assert.equal(shouldOfferExploreWithAI('memory_face','code',true,null,0),false);
  assert.equal(shouldOfferExploreWithAI('memory_face','code',false,'Search failed',0),false);
  assert.equal(shouldOfferExploreWithAI('memory_face','code',false,null,1),false);
  assert.equal(shouldOfferExploreWithAI('memory_face','code',false,null,0),true);
  assert.equal(shouldOfferExploreWithAI('a','code',false,null,0),false);
  assert.equal(shouldOfferExploreWithAI('.','regex',false,null,0),true);
  assert.equal(shouldOfferExploreWithAI('x'.repeat(241),'code',false,null,0),false);
  assert.equal(shouldOfferExploreWithAI('   ','code',false,null,0),false);
});

test('explicit AI exploration uses regex matches as bounded starting ranges without running generation',()=>{
  const hits=Array.from({length:10},(_,i)=>({path:`file-${Math.floor(i/2)}.rs`,line:i+1,text:'source'}));
  const intent=searchExploration(String.raw`value::\S+`,'regex',hits);
  assert.match(intent.question,/regular expression/);assert.equal(intent.context.length,4);assert.equal(new Set(intent.context.map(c=>c.path)).size,4);assert.ok(intent.context.every(c=>c.from===c.to));
  assert.deepEqual(searchExploration('glyph','code',hits),{question:'Explain glyph',context:[]});
});
