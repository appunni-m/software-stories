import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {resolveAppAppearance,readAppAppearance,watchAppAppearance} from '../src/lib/useAppAppearance.ts';

test('explicit appearance wins and system follows the current OS preference',()=>{
  for(const preferred of [false,true]){
    assert.equal(resolveAppAppearance('light',preferred),'light');assert.equal(resolveAppAppearance('dark',preferred),'dark');
    for(const mode of ['system',undefined,'unknown'])assert.equal(resolveAppAppearance(mode,preferred),preferred?'light':'dark');
  }
});

test('one shared subscription follows header and system changes and releases both listeners',()=>{
  const old={document:globalThis.document,MutationObserver:globalThis.MutationObserver,matchMedia:globalThis.matchMedia};
  const element={dataset:{theme:'system'}};let mutation,change,disconnected=false,removed=false,notifications=0;
  const media={matches:false,addEventListener:(name,fn)=>{assert.equal(name,'change');change=fn;},removeEventListener:(name,fn)=>{assert.equal(name,'change');assert.equal(fn,change);removed=true;}};
  try{
    globalThis.document={documentElement:element};globalThis.matchMedia=()=>media;
    globalThis.MutationObserver=class{constructor(callback){mutation=callback;}observe(target,options){assert.equal(target,element);assert.deepEqual(options.attributeFilter,['data-theme']);}disconnect(){disconnected=true;}};
    const stop=watchAppAppearance(()=>notifications++);assert.equal(readAppAppearance(),'dark');
    media.matches=true;change();assert.equal(readAppAppearance(),'light');element.dataset.theme='dark';mutation();assert.equal(readAppAppearance(),'dark');
    assert.equal(notifications,2);stop();assert.equal(disconnected,true);assert.equal(removed,true);
  }finally{for(const [name,value] of Object.entries(old)){if(value===undefined)delete globalThis[name];else globalThis[name]=value;}}
});

test('the app owns the standard theme context above keyboard help and the developer workspace',async()=>{
  const source=await readFile(new URL('../src/App.tsx',import.meta.url),'utf8');const tree=ts.createSourceFile('App.tsx',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
  let rootTheme;
  function walk(node){if(ts.isJsxElement(node)&&node.openingElement.tagName.getText(tree)==='Theme'&&node.openingElement.attributes.properties.some(p=>p.name?.getText(tree)==='asChild'))rootTheme=node;ts.forEachChild(node,walk);}walk(tree);
  assert.ok(rootTheme,'a provider must own the portalled dialog, not just the editor');
  const content=rootTheme.getText(tree);assert.match(content,/<KeyboardShortcuts/);assert.match(content,/<WorkspaceFrame/);assert.match(content,/appearance=\{appearance\}/);
  const workspace=await readFile(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');assert.doesNotMatch(workspace,/setAppearance|new MutationObserver/);assert.match(workspace,/useThemeContext\(\)/);
  const library=await readFile(new URL('../node_modules/@radix-ui/themes/styles.css',import.meta.url),'utf8');
  assert.match(library,/\.radix-themes:where\(\[data-is-root-theme='true'\]\)\s*\{\s*position: relative;/);
});
