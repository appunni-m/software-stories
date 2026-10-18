import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInThisContext} from 'node:vm';
import ts from 'typescript';
import {baseRevision} from './helpers/revision-history-fixture.mjs';
const source=ts.createSourceFile('ChapterCitationEditor.tsx',readFileSync(new URL('../src/ChapterCitationEditor.tsx',import.meta.url),'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const component=source.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='ChapterCitationEditor');
const apply=component.body.statements.filter(ts.isVariableStatement).flatMap(s=>s.declarationList.declarations).find(d=>d.name.getText(source)==='apply').initializer;
function harness(overrides={}){
  const anchor=baseRevision().sections[0].claims[0].anchors[0],state={busy:false,error:null,applied:[],closed:[],calls:[]};
  const b={busy:false,disabled:false,path:anchor.path,from:'2',to:'4',projectId:'p',baseRevisionId:'revision:base',snapshotId:anchor.snapshotId,anchor,request:{current:0},
    setBusy:v=>state.busy=v,setError:v=>state.error=v,repairChapterCitation:async(...args)=>{state.calls.push(args);return anchor;},
    onApply:v=>state.applied.push(v),changeOpen:v=>{state.closed.push(v);b.request.current++;},errorMessageOr:(e,f)=>e?.message??f,...overrides};
  const compiled=ts.transpileModule(`exports.apply=${apply.getText(source)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
  const mod={exports:{}};runInThisContext(`(function(exports,${Object.keys(b).join(',')}){${compiled}\n})`)(mod.exports,...Object.values(b));
  return {b,state,run:()=>mod.exports.apply({preventDefault(){}})};
}
test('the real dialog submits only captured line identity and applies a matching reply',async()=>{
  const h=harness();await h.run();assert.deepEqual(h.state.calls[0],['p','revision:base',h.b.anchor.anchorId,h.b.anchor.path,2,4]);
  assert.deepEqual(h.state.applied,[h.b.anchor]);assert.deepEqual(h.state.closed,[false]);
});
test('invalid ranges and disabled actions cannot read or change source',async()=>{
  for(const values of [{from:'0'},{from:'5',to:'4'},{from:'1.5'},{to:'4294967296'},{path:' '},{disabled:true},{busy:true}]){
    const h=harness(values);await h.run();assert.deepEqual(h.state.calls,[]);assert.deepEqual(h.state.applied,[]);
  }
});
test('late replies after cancellation or leaving the chapter do not apply or report success',async()=>{
  let resolve;const pending=new Promise(r=>resolve=r);const h=harness({repairChapterCitation:()=>pending});const running=h.run();h.b.request.current++;resolve(h.b.anchor);await running;
  assert.deepEqual(h.state.applied,[]);assert.deepEqual(h.state.closed,[]);assert.equal(h.state.error,null);
});
test('native failure and foreign identity preserve the draft and leave an actionable error',async()=>{
  for(const response of [()=>Promise.reject(new Error('Controlled native read failure')),async()=>({...baseRevision().sections[0].claims[0].anchors[0],anchorId:'anchor:foreign'}),async()=>({...baseRevision().sections[0].claims[0].anchors[0],snapshotId:'foreign'})]){
    const h=harness({repairChapterCitation:response});await h.run();assert.equal(h.state.applied.length,0);assert.equal(h.state.closed.length,0);assert.ok(h.state.error);assert.equal(h.state.busy,false);
  }
});

test('each real citation input clears its obsolete error without reading or applying source',()=>{
  const inputs=[];
  function visit(node){
    if(ts.isJsxSelfClosingElement(node)&&node.tagName.getText(source)==='TextField.Root')inputs.push(node);
    ts.forEachChild(node,visit);
  }
  visit(component);
  assert.equal(inputs.length,3);
  for(const [index,input] of inputs.entries()){
    const handler=input.attributes.properties.find(a=>ts.isJsxAttribute(a)&&a.name.getText(source)==='onChange').initializer.expression;
    const calls=[],compiled=ts.transpileModule(`exports.change=${handler.getText(source)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
    const exports={};runInThisContext(`(function(exports,setPath,setFrom,setTo,setError){${compiled}\n})`)(exports,v=>calls.push(['path',v]),v=>calls.push(['from',v]),v=>calls.push(['to',v]),v=>calls.push(['error',v]));
    exports.change({target:{value:'changed'}});
    assert.deepEqual(calls,[[['path','from','to'][index],'changed'],['error',null]]);
  }
});

test('repair transport names a trusted saved base and lines, without renderer source bytes or hash',async()=>{
  const code=ts.createSourceFile('commands.ts',readFileSync(new URL('../src/lib/commands.ts',import.meta.url),'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
  const fn=code.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='repairChapterCitation');
  const compiled=ts.transpileModule(fn.getText(code),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
  const calls=[],mod={exports:{}};runInThisContext(`(function(exports,invoke){${compiled}\n})`)(mod.exports,async(command,args)=>{calls.push({command,args});return 'reply';});
  assert.equal(await mod.exports.repairChapterCitation('p','base','anchor:one','src/a.rs',2,4),'reply');
  assert.deepEqual(calls,[{command:'repair_chapter_citation',args:{projectId:'p',baseRevisionId:'base',anchorId:'anchor:one',relativePath:'src/a.rs',startLine:2,endLine:4}}]);
});
