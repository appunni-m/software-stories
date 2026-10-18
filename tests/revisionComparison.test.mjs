import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {runInThisContext} from 'node:vm';
import ts from 'typescript';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import * as history from '../src/lib/revisionHistory.ts';
import {baseRevision,sceneRevision} from './helpers/revision-history-fixture.mjs';

const file=new URL('../src/ChapterRevisionComparison.tsx',import.meta.url);
const compiled=ts.transpileModule(readFileSync(file,'utf8'),{fileName:file.pathname,compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX}}).outputText;
const nativeRequire=createRequire(import.meta.url),component={exports:{}};
runInThisContext(`(function(require,module,exports){${compiled}\n})`,{filename:file.pathname})(name=>{
  if(name==='./lib/revisionHistory')return history;
  assert.equal(name,'react/jsx-runtime','comparison must remain a pure read-only view');return nativeRequire(name);
},component,component.exports);
const render=(before,after)=>renderToStaticMarkup(createElement(component.exports.ChapterRevisionComparison,{before,after}));

test('the real comparison component renders named scenes and elements with source values hidden',()=>{
  const before=sceneRevision(),after=structuredClone(before),scene=after.sections[0].visual.presentation.scenes[0];
  scene.narration='The request follows this path before the handler receives it.';scene.elements[1].value='SOURCE_VALUE_NEW';
  const html=render(before,after);
  assert.match(html,/Changed scene · narration/);assert.match(html,/Changed visual element · value/);
  assert.match(html,/Request entry point \/ A request arrives \/ Element 2/);
  assert.doesNotMatch(html,/SOURCE_VALUE_(OLD|NEW)|section:request|claim:entry|node:request/);
  assert.match(html,/contents hidden/);
});

test('long authored narration has a collapsed full change, including differences beyond the summary',()=>{
  const before=sceneRevision(),after=structuredClone(before);
  const common='The request enters this route and follows its validated path. '.repeat(3);
  before.sections[0].visual.presentation.scenes[0].narration=common+'The old ending is retained.';
  after.sections[0].visual.presentation.scenes[0].narration=common+'The revised ending explains the outcome.';
  const html=render(before,after);
  assert.equal((html.match(/<details class="chapter-revision-full-text">/g)??[]).length,2);
  assert.doesNotMatch(html,/<details[^>]*\bopen(?:=|>)/);
  assert.match(html,/The old ending is retained/);assert.match(html,/The revised ending explains the outcome/);
});

test('identical chapter input never claims to have compared external cached audio assets',()=>{
  const revision=baseRevision(),html=render(revision,structuredClone(revision));
  assert.match(html,/No chapter text, source references or visual definitions changed/);
  assert.doesNotMatch(html,/assets changed|cached audio/);
});

test('chapter title changes appear once and hostile authored strings remain escaped text',()=>{
  const before=baseRevision(),after=structuredClone(before);after.title='<script>window.pwned=true</script>';
  const html=render(before,after);
  assert.equal((html.match(/Changed chapter · title/g)??[]).length,1);
  assert.doesNotMatch(html,/<script>/);assert.match(html,/&lt;script&gt;/);
});
