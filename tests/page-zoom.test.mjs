import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {APP_ZOOM_KEY,compactZoomLayout,scaledZoomScrollOffset,validZoom,savedZoom,zoomShortcut} from '../src/lib/pageZoom.ts';

test('remembered page zoom accepts only bounded exact 20 percent steps',()=>{
  assert.equal(APP_ZOOM_KEY,'code-atlas:page-zoom:v1');
  for(let percent=20;percent<=1000;percent+=20){assert.equal(validZoom(percent),true);assert.equal(savedZoom(String(percent)),percent);}
  for(const value of [null,undefined,NaN,Infinity,-20,0,19,21,100.1,1001,'200',{},[]])assert.equal(validZoom(value),false);
  for(const value of [null,'',' ','0200','2e2','200.0','200x','-200','0','21','1001','10000000000000000000'])assert.equal(savedZoom(value),100);
});

test('zoom shortcuts include shifted plus and actual size without intercepting IME, Alt or handled keys',()=>{
  const event={key:'=',metaKey:true,ctrlKey:false,altKey:false,isComposing:false,defaultPrevented:false};
  for(const [key,action] of [['=','in'],['+','in'],['-','out'],['0','reset']]){
    assert.equal(zoomShortcut({...event,key}),action);assert.equal(zoomShortcut({...event,key,metaKey:false,ctrlKey:true}),action);
  }
  for(const overrides of [{metaKey:false},{altKey:true},{isComposing:true},{defaultPrevented:true},{key:'1'},{key:'p'},{key:'ArrowUp'}])assert.equal(zoomShortcut({...event,...overrides}),null);
});

test('native high zoom opts into the compact workspace layout',()=>{
  for(const percent of [20,100,120,140,160])assert.equal(compactZoomLayout(percent),false);
  for(const percent of [180,200,240,1000])assert.equal(compactZoomLayout(percent),true);
  for(const value of [0,101,NaN,Infinity])assert.equal(compactZoomLayout(value),false);
});

test('native zoom preserves the visible reading position across scale changes',()=>{
  assert.equal(scaledZoomScrollOffset(240,100,200),120);
  assert.equal(scaledZoomScrollOffset(120,200,100),240);
  assert.equal(scaledZoomScrollOffset(240,200,200),240);
  for(const values of [[0,100,200],[-10,100,200],[NaN,100,200],[Infinity,100,200],[240,0,200],[240,100,101]]){
    assert.equal(scaledZoomScrollOffset(...values),0);
  }
});

test('the packaged app has one zoom owner, installs it before reading and preserves guarded Quit',async()=>{
  const paths=['src/main.tsx','src/lib/appZoom.ts','src-tauri/src/app_zoom.rs','src-tauri/src/app_close.rs','src-tauri/src/lib.rs','src-tauri/tauri.conf.json','src/KeyboardShortcuts.tsx'];
  const [main,bridge,native,close,lib,raw,help]=await Promise.all(paths.map(p=>readFile(new URL(`../${p}`,import.meta.url),'utf8')));
  assert.ok(main.indexOf('await installAppZoom()')<main.indexOf('root.render(<AppErrorBoundary>'));
  assert.equal(JSON.parse(raw).app.windows[0].zoomHotkeysEnabled,false);
  assert.match(bridge,/code-atlas:zoom-changed/);assert.match(bridge,/dataset\.appZoomCompact=String\(compactZoomLayout\(percent\)\)/);assert.match(bridge,/scaledZoomScrollOffset\(element\.scrollTop,previousPercent,percent\)/);assert.match(bridge,/\.dw-answer-scroll,\.dw-tool-content/);assert.match(bridge,/localStorage\.setItem\(APP_ZOOM_KEY/);
  assert.doesNotMatch(bridge,/\.then\(remember\)/,'an older RPC reply cannot overwrite the ordered native event');
  assert.match(native,/window\s*\.set_zoom\(f64::from\(percent\) \/ 100\.\)/);
  assert.match(native,/item\.as_submenu\(\)\.and_then\(\|view\| view\.get\(LEVEL\)\)/,'the label is inside View, not an immediate app menu item');
  assert.match(native,/app\.run_on_main_thread/);assert.match(native,/Zoom: \{percent\}%/);
  assert.match(native,/CmdOrCtrl\+Equal/);assert.match(native,/CmdOrCtrl\+Minus/);assert.match(native,/CmdOrCtrl\+0/);
  assert.match(close,/crate::app_zoom::extend_menu\(app, &menu\)/);assert.match(close,/app\.exit\(0\)/);
  assert.match(lib,/app_zoom::AppZoom::default/);assert.match(lib,/app_zoom::restore_app_zoom/);assert.match(lib,/app_zoom::change_app_zoom/);
  assert.match(help,/Actual size \(100%\)/);
});
