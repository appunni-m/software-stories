import test from 'node:test';
import assert from 'node:assert/strict';
import {FLOW_DIAGRAM_LAYOUT_VERSION} from '../src/lib/readerProgress.ts';
import {answerReadingPosition, answerRevisionOptions, cleanAnswerVersions, rememberAnswerVersion, resolveAnswerRevision} from '../src/workspace/answerVersions.ts';
import {addTurn, createWorkspace, decodeWorkspace, mergePublishedSections, presentationOwner, rememberPresentation, rememberSection} from '../src/workspace/model.ts';

const revision=(n,parent=null,more={})=>({schemaVersion:5,projectId:'project',snapshotId:'snapshot',chapterId:'chapter:cart',target:{targetId:'target:cart',scope:{kind:'file',path:'src/cart.ts'}},revisionId:`revision:${n}`,revisionNumber:n,basedOnRevisionId:parent,lifecycle:'draft',validationStatus:'valid',title:'Cart',sections:[{sectionId:'section:one',sectionRevisionId:`section-revision:${n}`}],...more});
const original=revision(4,'revision:3');
const draft=revision(5,original.revisionId);
const rejected=revision(6,draft.revisionId,{lifecycle:'rejected'});
const accepted=revision(7,rejected.revisionId,{lifecycle:'accepted'});
const history=[original,draft,rejected,accepted];
const questionId='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const ready={id:questionId,question:'What happens?',context:[],status:'ready',revisionId:original.revisionId,generationId:'generation:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'};
const workspace=()=>addTurn(createWorkspace('project','snapshot'),structuredClone(ready),'guide');
const turn=s=>s.explorations[0].turns[0];
const select=(s,id,versions=history)=>rememberAnswerVersion(s,'root',questionId,original.revisionId,id,versions);

test('only valid accepted descendants of the exact original answer are offered',()=>{
  const unrelated=revision(100,null,{lifecycle:'accepted'});
  const older=revision(3,null,{lifecycle:'accepted'});
  const broken=revision(12,'revision:missing',{lifecycle:'accepted'});
  const foreigns=[{projectId:'other'},{snapshotId:'other'},{chapterId:'other'},{target:{targetId:'other',scope:{kind:'file',path:'src/cart.ts'}}},{target:{targetId:'target:cart',scope:{kind:'file',path:'src/other.ts'}}}].map((more,i)=>revision(20+i,original.revisionId,{lifecycle:'accepted',...more}));
  assert.deepEqual(answerRevisionOptions(original,[...history,unrelated,older,broken,...foreigns,revision(8,accepted.revisionId,{lifecycle:'accepted',validationStatus:'invalid'})]).map(r=>r.revisionId),[original.revisionId,accepted.revisionId]);
  assert.deepEqual(answerRevisionOptions(original,history.filter(r=>r!==rejected)),[original],'missing ancestry does not prove a relationship');
  const cycle=[revision(8,'revision:9',{lifecycle:'accepted'}),revision(9,'revision:8',{lifecycle:'accepted'})];
  assert.deepEqual(answerRevisionOptions(original,cycle),[original]);
  assert.deepEqual(answerRevisionOptions(original,[revision(3,original.revisionId,{lifecycle:'accepted'})]),[original],'parent numbers must increase');
});

test('selections preserve immutable receipts, original reading, and revised reading independently',()=>{
  let s=workspace();s=rememberSection(s,'root',questionId,'guide',{revisionId:original.revisionId,sectionId:'section:original-last'});
  const before=JSON.stringify(s);s=select(s,accepted.revisionId);
  assert.equal(resolveAnswerRevision(original,turn(s),history),accepted);
  assert.equal(turn(s).revisionId,original.revisionId);assert.equal(turn(s).generationId,'generation:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa');
  assert.equal(answerReadingPosition(turn(s),'guide',accepted.revisionId).sectionId,'section:one');
  s=rememberSection(s,'root',questionId,'guide',{revisionId:accepted.revisionId,sectionId:'section:edited-last'});
  s=select(s,original.revisionId);
  assert.equal(answerReadingPosition(turn(s),'guide',original.revisionId).sectionId,'section:original-last');
  s=select(s,accepted.revisionId);
  assert.equal(answerReadingPosition(turn(s),'guide',accepted.revisionId).sectionId,'section:edited-last');
  assert.equal(JSON.parse(before).explorations[0].turns[0].answerVersions,undefined);
  const restored=decodeWorkspace(JSON.stringify(s),'project','snapshot');
  assert.equal(resolveAnswerRevision(original,turn(restored),history),accepted);
  assert.equal(answerReadingPosition(turn(restored),'guide',accepted.revisionId).sectionId,'section:edited-last');
});

test('a newly accepted edit never moves the current reader and stale or forged choices fall back safely',()=>{
  let s=workspace();assert.equal(resolveAnswerRevision(original,turn(s),history),original);
  s=select(s,accepted.revisionId);
  const later=revision(8,accepted.revisionId,{lifecycle:'accepted'});
  assert.equal(resolveAnswerRevision(original,turn(s),[...history,later]),accepted);
  assert.equal(resolveAnswerRevision(original,turn(s),[original]),original);
  assert.equal(select(s,draft.revisionId),s);assert.equal(select(s,'missing'),s);
  assert.equal(rememberAnswerVersion(s,'root',questionId,'unrelated',accepted.revisionId,history),s);
  const forged={...turn(s),answerVersions:[{originalRevisionId:original.revisionId,revisionId:'revision:foreign',selected:true}]};
  assert.equal(resolveAnswerRevision(original,forged,[...history,revision(100,null,{revisionId:'revision:foreign',lifecycle:'accepted'})]),original);
});

test('revised story and player progress stays owned by its question across version switches and restart',()=>{
  let s=select(workspace(),accepted.revisionId);
  const progress={revisionId:accepted.revisionId,sectionId:'section:one',sectionRevisionId:'section-revision:7',sceneId:'claim:one',positions:{'claim:one':2},fullMap:false,focusLayoutVersion:7};
  s=rememberPresentation(s,'root',questionId,'stories',progress);
  s=select(s,original.revisionId);
  const restored=decodeWorkspace(JSON.stringify(s),'project','snapshot');
  assert.deepEqual(turn(restored).presentation.stories,[progress]);
  assert.equal(presentationOwner(restored.explorations[0],accepted.revisionId).id,questionId);
  assert.equal(presentationOwner(restored.explorations[0],'foreign'),undefined);
});

test('progressive sections keep original publication identities and independent version choices',()=>{
  const run='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  const receipt={runId:run,key:'part-one',order:0,generationId:`generation:${run}`,revisionId:original.revisionId,supersedes:null,contextElapsedMs:0,runElapsedMs:null};
  let s=workspace();turn(s).runIds=[run];turn(s).publishedSections=[receipt];
  s=select(s,accepted.revisionId);const updated=mergePublishedSections(turn(s),[{...receipt,key:'part-two',order:1,generationId:'generation:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',revisionId:'revision:another'}]);
  assert.deepEqual(updated.publishedSections[0],receipt);assert.deepEqual(updated.answerVersions,turn(s).answerVersions);
  assert.equal(resolveAnswerRevision(original,updated,history),accepted);
});

test('corrupt metadata is bounded and cannot claim another original question',()=>{
  const good={originalRevisionId:original.revisionId,revisionId:accepted.revisionId,selected:true,reading:{guide:'section:one'}};
  assert.deepEqual(cleanAnswerVersions([good,good,{...good,revisionId:'revision:8'}, {...good,originalRevisionId:'foreign'}],ready),[good]);
  for(const v of [null,{},Array(257).fill(good),[{...good,selected:'yes'}],[{...good,revisionId:'bad\nkey'}]])assert.deepEqual(cleanAnswerVersions(v,ready),[]);
  assert.deepEqual(cleanAnswerVersions([{...good,html:'not retained',reading:{guide:'bad\nkey',flow:'section:flow',secret:'ignored'}}],ready),[{...good,reading:{flow:'section:flow'}}]);
});

test('Flow follows an explicit version switch in its current part and keeps another part selected',()=>{
  let s=workspace();s=rememberSection(s,'root',questionId,'flow',{revisionId:original.revisionId,sectionId:'section:old-flow'});
  s=select(s,accepted.revisionId);assert.equal(turn(s).reading.flow.revisionId,accepted.revisionId);
  s=rememberSection(s,'root',questionId,'flow',{revisionId:accepted.revisionId,sectionId:'section:new-flow'});
  s=select(s,original.revisionId);assert.equal(turn(s).reading.flow.sectionId,'section:old-flow');
  s=select(s,accepted.revisionId);assert.equal(turn(s).reading.flow.sectionId,'section:new-flow');
  turn(s).reading.flow={revisionId:'revision:other-part',sectionId:'section:other'};
  s=select(s,original.revisionId);assert.equal(turn(s).reading.flow.revisionId,'revision:other-part');
});

test('bounded version retention removes orphan progress so the native journal can still save',()=>{
  const revisions=Array.from({length:257},(_,i)=>revision(i+5,original.revisionId,{lifecycle:'accepted'}));
  const [oldest]=revisions,current=revisions[255],next=revisions[256];
  let s=workspace();const t=turn(s);
  t.answerVersions=revisions.slice(0,256).map(r=>({originalRevisionId:original.revisionId,revisionId:r.revisionId,selected:r===current,reading:{guide:'section:one'}}));
  const progress=r=>({revisionId:r.revisionId,sectionId:'section:one',sectionRevisionId:`section-revision:${r.revisionNumber}`,sceneId:'claim:one',positions:{},fullMap:false,focusLayoutVersion:7});
  t.presentation={stories:[progress(original),progress(oldest),progress(current)],diagrams:[{...progress(oldest),viewport:{x:0,y:0,zoom:1},layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION}]};
  const before=JSON.stringify(s);s=select(s,next.revisionId,[original,...revisions]);
  assert.equal(turn(s).answerVersions.length,256);assert.equal(resolveAnswerRevision(original,turn(s),[original,...revisions]),next);
  assert.deepEqual(turn(s).presentation.stories.map(p=>p.revisionId),[original.revisionId,current.revisionId]);
  assert.deepEqual(turn(s).presentation.diagrams,[]);
  assert.ok(!turn(s).answerVersions.some(v=>v.revisionId===oldest.revisionId));
  assert.equal(JSON.parse(before).explorations[0].turns[0].presentation.stories.length,3,'input and chapter history are untouched');
});
