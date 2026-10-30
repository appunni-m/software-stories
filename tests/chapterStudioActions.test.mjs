import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInThisContext} from 'node:vm';
import ts from 'typescript';
import {baseRevision} from './helpers/revision-history-fixture.mjs';
import {removeClaimAndReferences} from '../src/lib/chapterEditing.ts';
import {chapterIssueFeedback} from '../src/lib/chapterValidationPresentation.ts';

// Execute the production callbacks with deterministic native/event boundaries.
// This does not emulate rendering, focus or native storage validation.
const source=ts.createSourceFile('App.tsx',readFileSync(new URL('../src/App.tsx',import.meta.url),'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const studio=source.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='ChapterStudioScreen');
assert.ok(studio?.body);
function callback(name,bindings){
  const declaration=studio.body.statements.filter(ts.isVariableStatement).flatMap(s=>s.declarationList.declarations).find(d=>d.name.getText(source)===name);
  const effect=studio.body.statements.find(s=>ts.isExpressionStatement(s)&&ts.isCallExpression(s.expression)&&s.expression.expression.getText(source)==='useEffect'&&s.getText(source).includes("'code-atlas:chapters-changed'"));
  const node=name==='eventEffect'?effect?.expression.arguments[0]:declaration?.initializer;
  assert.ok(node,`production callback ${name} exists`);
  const compiled=ts.transpileModule(`exports.callback=${node.getText(source)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
  const module={exports:{}};
  runInThisContext(`(function(exports,${Object.keys(bindings).join(',')}){${compiled}\n})`)(module.exports,...Object.values(bindings));
  return module.exports.callback;
}
function setup(){
  const before=baseRevision();before.projectId='p';before.snapshotId='s';
  const saved=structuredClone(before);saved.revisionId='revision:saved';saved.revisionNumber=2;
  const candidate=structuredClone(before);candidate.title='My unsaved edit';
  const state={candidate,selected:before.revisionId,editing:true,rebasing:false,feedback:null,refreshes:0,recorded:[],navigation:[],publications:[],busy:false,loading:false,loadError:null};
  const change=key=>value=>{state[key]=value;};
  const result={revision:saved,validation:{status:'valid',issues:[]}};
  const bindings={projectId:'p',snapshotId:'s',window:new EventTarget(),CustomEvent,candidate,selectedRevision:before,isLatestCurrent:true,busy:false,editing:false,
    pendingDraft:null,preservedDraftReason:null,historyAvailability:'ready',
    selectedTarget:{latestCurrent:saved},studioMounted:{current:true},rejectReason:'Needs revision',
    chapterHistory:{current:{refresh:async()=>{state.refreshes++;},record:r=>{state.recorded.push(r);state.refreshes++;}}},
    setRefreshKey:()=>state.refreshes++,setSelectedTargetKey:change('target'),setSelectedRevisionId:change('selected'),
    setLoading:change('loading'),setLoadError:change('loadError'),
    setEditing:change('editing'),setCandidate:change('candidate'),setRebasing:change('rebasing'),setValidation:change('validation'),setFeedback:change('feedback'),
    setRejecting:change('rejecting'),setRejectReason:change('reason'),setBusy:change('busy'),
    onSelectRevision:r=>state.navigation.push(r),
    saveChapterEdits:async()=>result,restoreChapterRevision:async()=>result,acceptChapterDraft:async()=>result,rejectChapterDraft:async()=>saved,
    reload:async()=>{throw new Error('history read failed after persistence');},
    errorMessageOr:(error,fallback)=>error?.message??fallback,
    chapterIssueFeedback,
  };
  bindings.recordSavedRevision=callback('recordSavedRevision',bindings);
  bindings.window.addEventListener('code-atlas:chapters-changed',event=>state.publications.push(event.detail));
  return {before,saved,state,bindings};
}

test('reviewing a finding focuses the current claim without navigation or candidate changes',()=>{
  const calls=[],element={focus:options=>calls.push(['focus',options]),scrollIntoView:options=>calls.push(['scroll',options])};
  const review=callback('reviewIssueLocation',{document:{getElementById:id=>{calls.push(['find',id]);return element;}}});
  review('section:one','claim:two');
  assert.deepEqual(calls,[['find','studio-claim-claim:two'],['focus',{preventScroll:true}],['scroll',{block:'nearest',behavior:'auto'}]]);
  calls.length=0;review('section:one',null);assert.equal(calls[0][1],'studio-section-section:one');
  assert.doesNotThrow(()=>callback('reviewIssueLocation',{document:{getElementById:()=>null}})('missing',null));
});

test('repairing a finding retains an existing candidate, starts a new edit only when needed, and blocks stale or busy actions',()=>{
  for(const editing of [false,true]){
    const calls=[],anchor=baseRevision().sections[0].claims[0].anchors[0];
    const bindings={busy:false,isLatestCurrent:true,editing,pendingDraft:null,preservedDraftReason:null,beginEditing:()=>calls.push('edit'),setRepairingAnchorId:id=>calls.push(id)};
    callback('repairIssueCitation',bindings)(anchor);
    assert.deepEqual(calls,editing?[anchor.anchorId]:['edit',anchor.anchorId]);
    for(const blocked of [{busy:true},{isLatestCurrent:false}]){
      calls.length=0;callback('repairIssueCitation',{...bindings,...blocked})(anchor);assert.deepEqual(calls,[]);
    }
  }
});
function changed(target,detail){const event=new Event('code-atlas:chapters-changed');Object.defineProperty(event,'detail',{value:detail});target.dispatchEvent(event);}

test('the real Remove claim callback updates linked scenes, cues and section dependencies together',()=>{
  const s=baseRevision().sections[0];
  s.claims.push({...structuredClone(s.claims[0]),claimId:'claim:retained',anchors:[{...s.claims[0].anchors[0],anchorId:'anchor:retained'}]});
  let result;
  callback('removeClaim',{setSection:(_id,update)=>{result=update(s);},removeClaimAndReferences,setFeedback:()=>{}})(s.sectionId,s.claims[0].claimId);
  const remaining=new Set(result.claims.flatMap(c=>c.anchors.map(a=>a.anchorId)));
  assert.ok(result.hotspots.every(h=>h.destination.kind!=='codeLens'||remaining.has(h.destination.anchorId)));
  assert.ok(result.visual===null||result.visual.nodes.every(n=>n.evidenceAnchorIds.every(a=>remaining.has(a))));
});

test('background chapter publication records its native receipt and refreshes without replacing an unsaved edit or selection',()=>{
  const {state,bindings,saved}=setup(),candidate=state.candidate;state.loadError='old read failure';
  const cleanup=callback('eventEffect',bindings)();
  changed(bindings.window,{projectId:'p',revision:saved});
  assert.equal(state.refreshes,1);assert.deepEqual(state.recorded,[saved]);assert.equal(state.selected,'revision:one');assert.equal(state.candidate,candidate);assert.equal(state.editing,true);assert.deepEqual(state.navigation,[]);
  assert.equal(state.loading,true);assert.equal(state.loadError,null);
  cleanup();changed(bindings.window,{projectId:'p',revision:saved});assert.equal(state.refreshes,1);
});

test('another repository publication does not refresh or navigate the current editor',()=>{
  const {state,bindings,saved}=setup();callback('eventEffect',bindings)();
  changed(bindings.window,{projectId:'other',revision:{...saved,projectId:'other'}});
  assert.equal(state.refreshes,0);assert.deepEqual(state.navigation,[]);
});

test('a scoped invalidation without a revision receipt still refreshes history',()=>{
  const {state,bindings}=setup(),cleanup=callback('eventEffect',bindings)();
  changed(bindings.window,{projectId:'p'});
  assert.equal(state.refreshes,1);assert.deepEqual(state.recorded,[]);assert.equal(state.loading,true);
  cleanup();
});

for(const name of ['saveEdits','restoreSelectedRevision','accept','reject']){
  test(`${name}: a successful native mutation is acknowledged even when a later history read fails`,async()=>{
    const {state,bindings,saved}=setup();if(name==='restoreSelectedRevision')bindings.isLatestCurrent=false;
    await callback(name,bindings)({preventDefault(){}});
    assert.deepEqual(state.recorded,[saved]);assert.deepEqual(state.navigation,[saved]);assert.equal(state.feedback?.tone,'success');assert.equal(state.busy,false);assert.deepEqual(state.publications,[{projectId:'p'}]);
    if(name==='saveEdits'){assert.equal(state.candidate,null);assert.equal(state.editing,false);}
  });
  test(`${name}: leaving the editor before the native reply suppresses every late UI write`,async()=>{
    const {state,bindings}=setup();if(name==='restoreSelectedRevision')bindings.isLatestCurrent=false;
    let resolve;const operation=new Promise(yes=>resolve=yes);
    const native={saveEdits:'saveChapterEdits',restoreSelectedRevision:'restoreChapterRevision',accept:'acceptChapterDraft',reject:'rejectChapterDraft'}[name];
    bindings[native]=()=>operation;
    const work=callback(name,bindings)({preventDefault(){}});bindings.studioMounted.current=false;
    resolve(native==='rejectChapterDraft'?baseRevision():{revision:baseRevision(),validation:{status:'valid',issues:[]}});await work;
    assert.deepEqual(state.recorded,[]);assert.deepEqual(state.navigation,[]);assert.equal(state.feedback,null);
  });
}

test('saving an outdated base reports the conflict and preserves the edit instead of silently doing nothing',async()=>{
  const {state,bindings}=setup();bindings.isLatestCurrent=false;const candidate=state.candidate;
  bindings.saveChapterEdits=()=>assert.fail('a stale base must not overwrite a newer revision');
  await callback('saveEdits',bindings)();
  assert.equal(state.candidate,candidate);assert.equal(state.editing,true);assert.equal(state.feedback?.tone,'info');assert.match(state.feedback.message,/newer|latest/i);
});

test('native save failure retains the unsaved candidate and never records a successful revision',async()=>{
  const {state,bindings}=setup(),candidate=state.candidate;bindings.saveChapterEdits=async()=>{throw new Error('native save rejected');};
  await callback('saveEdits',bindings)();assert.equal(state.candidate,candidate);assert.equal(state.editing,true);assert.deepEqual(state.recorded,[]);assert.deepEqual(state.navigation,[]);assert.equal(state.feedback.tone,'error');
});

test('the real Validate callback reports one issue with singular feedback and retains its full report',async()=>{
  const {state,bindings}=setup();bindings.activeDocument=bindings.candidate;
  const report={status:'invalid',issues:[{code:'invalidClaim',message:'A claim needs text.'}]};
  bindings.validateSavedChapterRevision=async()=>report;
  await callback('validateCurrent',bindings)();
  assert.equal(state.validation,report);assert.equal(state.feedback.message,'1 issue needs review before acceptance.');assert.equal(state.feedback.tone,'error');
});

for(const editing of [false,true])test(`Validate uses ${editing?'the trusted saved base for an unsaved edit':'the native saved record without a renderer round-trip'}`,async()=>{
  const {state,bindings,before}=setup();const requests=[];bindings.editing=editing;bindings.activeDocument=bindings.candidate;
  bindings.validateChapterRevision=async(...args)=>{assert.ok(editing);requests.push(args);return {status:'valid',issues:[]};};
  bindings.validateSavedChapterRevision=async(...args)=>{assert.ok(!editing);requests.push(args);return {status:'valid',issues:[]};};
  await callback('validateCurrent',bindings)();
  assert.deepEqual(requests,editing?[['p',bindings.candidate,before.revisionId]]:[['p',bindings.candidate.revisionId]]);assert.equal(state.feedback.tone,'success');
});

test('saved validation transport carries only project and revision identity',async()=>{
  const file=ts.createSourceFile('commands.ts',readFileSync(new URL('../src/lib/commands.ts',import.meta.url),'utf8'),ts.ScriptTarget.Latest,true);
  const node=file.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='validateSavedChapterRevision');assert.ok(node);
  const compiled=ts.transpileModule(node.getText(file),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
  const requests=[],exports={};runInThisContext(`(function(exports,invoke){${compiled}\n})`)(exports,async(command,args)=>{requests.push({command,args});return {status:'invalid',issues:[]};});
  await exports.validateSavedChapterRevision('p','revision:invalid');
  assert.deepEqual(requests,[{command:'validate_saved_chapter_revision',args:{projectId:'p',revisionId:'revision:invalid'}}]);
});


test('the actual edit validation and save transports carry base identity without changing provenance',async()=>{
  const file=ts.createSourceFile('commands.ts',readFileSync(new URL('../src/lib/commands.ts',import.meta.url),'utf8'),ts.ScriptTarget.Latest,true);
  const requests=[];const invoke=async(command,args)=>{requests.push({command,args});return {};};
  function transport(name){
    const node=file.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text===name);assert.ok(node);
    const compiled=ts.transpileModule(node.getText(file),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
    const exports={};runInThisContext(`(function(exports,invoke){${compiled}\n})`)(exports,invoke);return exports[name];
  }
  const candidate={...baseRevision(),generationOrigin:{testSentinel:'must be preserved for the native boundary'}};
  const validate=transport('validateChapterRevision'),save=transport('saveChapterEdits');
  await validate('p',candidate,'trusted-base');await validate('p',candidate);await save('p','trusted-base',candidate);
  assert.deepEqual(requests.map(r=>r.command),['validate_chapter_revision','validate_chapter_revision','save_chapter_edits']);
  assert.equal(requests[0].args.baseRevisionId,'trusted-base');assert.equal(Object.hasOwn(requests[1].args,'baseRevisionId'),false);assert.equal(requests[2].args.baseRevisionId,'trusted-base');
  for(const request of requests)assert.deepEqual(JSON.parse(request.args.revisionJson),candidate);
});
