import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { addTurn, ancestors, chapterTarget, chooseFlowRevision, cleanAnswerNotes, cleanPublishedSections, decodeWorkspace, focusAnswerSection, historyEntry, questionProgressText, mergePublishedSections, createWorkspace, learningQuestion, parseWorkspace, rankSourcePaths, rememberScroll, rememberSection, sectionPosition } from '../src/workspace/model.ts';
const turn = (id,question='Explain the subtotal implementation') => ({id,question,context:[{path:'src/cart.ts',from:1,to:6}],status:'queued'});

test('history never calls an empty or unavailable outcome a saved explanation',()=>{
  const available = new Set(['revision:existing']);
  for(const status of ['queued','working','ready','partial','failed','stopped','interrupted']) {
    const entry=historyEntry({...turn('one'),status,revisionId:'revision:missing'},available);
    assert.equal(entry.action,'Open question');
    assert.doesNotMatch(entry.text,/Open the saved explanation|Saved sections are available/);
    if(status==='stopped'||status==='interrupted')assert.match(entry.text,/before an explanation was saved/);
  }
  assert.match(historyEntry({...turn('one'),status:'working'},available).text,/Working/);
});
test('history preserves partial results only when an exact saved revision is available',()=>{
  const available=new Set(['revision:existing']);
  const partial={...turn('one'),status:'stopped',publishedSections:[{revisionId:'revision:existing'}]};
  assert.deepEqual(historyEntry(partial,available),{text:'Stopped. Saved sections are available.',action:'Open explanation'});
  assert.equal(historyEntry(partial,new Set()).action,'Open question');
  assert.deepEqual(historyEntry({...turn('one'),status:'ready',revisionId:'revision:existing'},available),{text:'Saved explanation.',action:'Open explanation'});
  for(const status of ['queued','working','partial','failed','interrupted']) {
    assert.match(historyEntry({...partial,status},available).text,/Saved sections are available/);
  }
});
test('queued work is distinct from the active question in the reader and history',()=>{
  assert.equal(questionProgressText(turn('queued')), 'Waiting for the current question to finish…');
  assert.equal(questionProgressText({ ...turn('active'), status: 'working' }), 'Building your explanation…');
  assert.equal(questionProgressText({
    ...turn('guide'), status: 'working', guideId: 'guide',
  }), 'Your guide is coming together…');
  assert.equal(questionProgressText({
    ...turn('guide'), status: 'working', guideId: 'guide',
    guide: {status:'building'},
  }), 'Your first slide is taking shape…');
  assert.equal(questionProgressText({
    ...turn('guide'), status: 'working', guideId: 'guide',
    guide: {status:'building'}, publishedSections:[{revisionId:'one'}],
  }), '1 slide ready · more on the way');
  assert.equal(questionProgressText({
    ...turn('guide'), status: 'working', guideId: 'guide',
    guide: {status:'building'}, publishedSections:[{revisionId:'one'},{revisionId:'two'}],
  }), '2 slides ready · more on the way');
  assert.equal(questionProgressText({ ...turn('saved'), status: 'ready' }), null);
  const savedSections = new Set(['revision:existing']);
  const queued = historyEntry({
    ...turn('queued'), status: 'queued',
    publishedSections: [{ revisionId: 'revision:existing' }],
  }, savedSections);
  assert.deepEqual(queued, {
    text: 'Waiting for the current question to finish. Saved sections are available.',
    action: 'Open explanation',
  });
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.equal((source.match(/questionProgressText\(turn\)/g) ?? []).length, 2);
});
test('repository search and explorer filters have independent inputs and branches count questions',()=>{
  const source=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  assert.match(source,/\[fileFilter,setFileFilter\] = useState\(''\)/);
  assert.match(source,/\[repositorySearch,setRepositorySearch\] = useState\(''\)/);
  assert.match(source,/value=\{activity==='search'\?repositorySearch:fileFilter\}/);
  assert.match(source,/<WorkspaceExplorer paths=\{sourcePaths\} query=\{fileFilter\}/);
  assert.doesNotMatch(source,/<small>\{e\.turns\.length\} answer/);
  assert.match(source,/<small>\{e\.turns\.length\} question/);
});
test('reading graphs use real source buttons without canvas editing or deletion shortcuts',()=>{
  const source=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  assert.match(source,/<button type="button" className="dw-flow-source nodrag nopan" aria-label=\{`Open source for \$\{n.label\}`\} onClick=\{\(\)=>openNodeSource\(n,anchor\)\}/);
  for(const path of ['../src/workspace/Workspace.tsx','../src/VisualStory.tsx']) {
    const component=readFileSync(new URL(path,import.meta.url),'utf8');
    assert.match(component,/nodesFocusable=\{false\} edgesFocusable=\{false\} elementsSelectable=\{false\} deleteKeyCode=\{null\}/);
  }
});
test('follow-ups stack at the current level; only explicit exploration creates a branch',()=>{
  let s=createWorkspace('p','snap');s=addTurn(s,turn('one'),'guide');s=addTurn(s,turn('two'),'flow');
  assert.equal(s.explorations.length,1);assert.equal(s.currentId,'root');assert.equal(s.explorations[0].turns.length,2);
  s=addTurn(s,turn('three'),'guide',{id:'child',originTurnId:'two'});
  s=addTurn(s,turn('four'),'flow',{id:'grandchild',originTurnId:'three'});
  assert.deepEqual(ancestors(s).map(e=>e.id),['root','child','grandchild']);
  assert.equal(s.explorations[1].originTurnId,'two');assert.equal(s.explorations[2].originTurnId,'three');
  assert.equal(s.explorations[0].turns.length,2);assert.equal(s.explorations[0].view,'flow');
  assert.equal(s.file,'src/cart.ts');
});
test('invalid branch owners and duplicate branch IDs cannot corrupt the tree',()=>{
  let s=createWorkspace('p','snap');s=addTurn(s,turn('one'),'guide');
  assert.equal(addTurn(s,turn('two'),'guide',{id:'child',originTurnId:'missing'}),s);
  assert.equal(addTurn(s,turn('two'),'guide',{id:'root',originTurnId:'one'}),s);
});
test('reopening restores history and last view, and never resubmits interrupted work',()=>{
  let s=createWorkspace('p','snap');s=addTurn(s,turn('one'),'flow');s.explorations[0].turns[0].status='working';
  const restored=parseWorkspace(JSON.stringify(s),'p','snap');
  assert.equal(restored.explorations[0].view,'flow');assert.equal(restored.explorations[0].turns[0].status,'interrupted');
  assert.equal(restored.explorations[0].turns.length,1);
  assert.equal(parseWorkspace(JSON.stringify(s),'other','snap').explorations[0].turns.length,0);
});
test('partial-answer scope notes survive reopening with bounded plain text',()=>{
  let s=createWorkspace('p','snap');s=addTurn(s,{...turn('one'),status:'ready',notes:[' This explanation covers inspected paths. ','This explanation covers inspected paths.',null,'bad\ntext','x'.repeat(1001)]},'guide');
  const restored=parseWorkspace(JSON.stringify(s),'p','snap');
  assert.deepEqual(restored.explorations[0].turns[0].notes,['This explanation covers inspected paths.']);
  assert.deepEqual(cleanAnswerNotes('invalid'),[]);
  assert.equal(cleanAnswerNotes(Array.from({length:20},(_,i)=>`Note ${i}`)).length,8);
  delete s.explorations[0].turns[0].notes;
  assert.deepEqual(parseWorkspace(JSON.stringify(s),'p','snap').explorations[0].turns[0].notes,[]);
});
test('damaged, cyclic or path-traversing saved UI state falls back safely',()=>{
  const s=createWorkspace('p','snap');s.explorations.push({id:'a',parentId:'a',originTurnId:null,title:'bad',turns:[],view:'guide'});
  assert.equal(parseWorkspace(JSON.stringify(s),'p','snap').explorations.length,1);
  s.explorations.pop();s.explorations[0].turns=[{...turn('one'),context:[{path:'../../private'}]}];
  assert.equal(parseWorkspace(JSON.stringify(s),'p','snap').explorations[0].turns.length,0);
  assert.equal(parseWorkspace('{bad','p','snap').currentId,'root');
});
test('implementation entry points rank before assistant settings and documentation',()=>{
  const paths=['.claude/agents/instructions.md','README.md','tests/cart.test.ts','src/lib.rs','Cargo.toml'];
  assert.equal(rankSourcePaths(paths)[0],'src/lib.rs');assert.equal(paths[0],'.claude/agents/instructions.md');
});

test('learning questions retain recent intent and branch topic within the native limit',()=>{
  let s=createWorkspace('p','snap');
  s=addTurn(s,turn('one','How are subtotal prices validated?'),'guide');
  s=addTurn(s,turn('two','Why does it do that?'),'guide');
  assert.match(learningQuestion(s,'root','two',300),/How are subtotal prices validated/);
  assert.match(learningQuestion(s,'root','two',300),/Current question: Why does it do that/);
  s=addTurn(s,turn('three','Explain the RangeError propagation'),'flow',{originTurnId:'two',id:'branch'});
  s=addTurn(s,turn('four','What receives that exception?'),'flow');
  const q=learningQuestion(s,'branch','four',300);
  assert.match(q,/RangeError propagation/);
  assert.ok(q.endsWith('What receives that exception?'));
  assert.doesNotMatch(q, /[\u0000-\u001f\u007f]/);
  assert.ok(q.length >= 8 && q.length <= 300);
  assert.equal(q, q.trim());
  assert.ok(learningQuestion(s,'branch','four',60).length<=60);
  assert.equal(learningQuestion(s,'missing','four',300),'');
});

// The installed Codex adapter admits one model turn at a time.
test('question queue serializes work and a stopped waiter does not start inference', async()=>{
  const { createQuestionQueue } = await import('../src/workspace/queue.ts');
  const enter=createQuestionQueue();
  const first=await enter();let secondStarted=false;let thirdStarted=false;let cancelled=false;
  const second=enter().then(release=>{try {if(!cancelled)secondStarted=true;} finally {release();}});
  const third=enter().then(release=>{thirdStarted=true;release();});
  await Promise.resolve();assert.equal(secondStarted,false);assert.equal(thirdStarted,false);
  cancelled=true;first();await Promise.all([second,third]);
  assert.equal(secondStarted,false);assert.equal(thirdStarted,true);
});

test('AI-selected single code files use the file contract without manual attachment',()=>{
  assert.equal(chapterTarget([],['src/cart.ts']),'src/cart.ts');
  assert.equal(chapterTarget([],['README.md']),null);
  assert.equal(chapterTarget([],['src/cart.ts','src/tax.ts']),null);
  assert.equal(chapterTarget([{path:'README.md'},{path:'src/tax.ts'}],['README.md','src/tax.ts']),'src/tax.ts');
  assert.equal(chapterTarget([{path:'missing.ts'}],['src/cart.ts']),'src/cart.ts');
});

test('reading positions belong to the exact exploration, turn, revision and view',()=>{
  const ready=(id,revisionId)=>({...turn(id),status:'ready',revisionId});
  let s=addTurn(createWorkspace('p','snap'),ready('parent','rev-parent'),'guide');
  s=rememberSection(s,'root','parent','guide',{revisionId:'rev-parent',sectionId:'section-two'});
  s=rememberSection(s,'root','parent','flow',{revisionId:'rev-parent',sectionId:'diagram-three'});
  s=rememberScroll(s,'root','guide',420);s=rememberScroll(s,'root','flow',84);s=rememberScroll(s,'root','history',125);
  s=addTurn(s,ready('child-turn','rev-child'),'flow',{id:'child',originTurnId:'parent'});
  s=rememberSection(s,'child','child-turn','guide',{revisionId:'rev-child',sectionId:'section-child'});
  s=rememberScroll(s,'child','flow',230);
  s=addTurn(s,{...ready('grandchild-turn','rev-grandchild'),status:'stopped'},'history',{id:'grandchild',originTurnId:'child-turn'});
  s={...s,currentId:'root'};
  s=addTurn(s,ready('sibling-turn','rev-sibling'),'guide',{id:'sibling',originTurnId:'parent'});
  s=rememberSection(s,'sibling','sibling-turn','guide',{revisionId:'rev-sibling',sectionId:'section-sibling'});
  s=rememberScroll(s,'sibling','guide',710);
  const restored=parseWorkspace(JSON.stringify(s),'p','snap');
  const root=restored.explorations.find(e=>e.id==='root');
  const child=restored.explorations.find(e=>e.id==='child');
  const grandchild=restored.explorations.find(e=>e.id==='grandchild');
  const sibling=restored.explorations.find(e=>e.id==='sibling');
  assert.equal(restored.currentId,'sibling');
  assert.deepEqual(root.turns[0].reading,{guide:{revisionId:'rev-parent',sectionId:'section-two'},flow:{revisionId:'rev-parent',sectionId:'diagram-three'}});
  assert.deepEqual(root.scrollPositions,{guide:420,flow:84,history:125});
  assert.deepEqual(child.scrollPositions,{flow:230});assert.equal(child.view,'flow');
  assert.equal(child.turns[0].reading.guide.sectionId,'section-child');
  assert.equal(sibling.turns[0].reading.guide.sectionId,'section-sibling');assert.equal(sibling.scrollPositions.guide,710);
  assert.equal(grandchild.turns[0].status,'stopped');assert.equal(grandchild.view,'history');
  assert.deepEqual(root.turns[0].context,turn('parent').context);
  assert.equal(root.turns[0].revisionId,'rev-parent');assert.equal(root.turns.length,1);
  assert.deepEqual(ancestors(restored,'grandchild').map(e=>e.id),['root','child','grandchild']);
});

test('opening a compact answer stores its exact section across Guide and Flow switches',()=>{
  let state=addTurn(createWorkspace('p','snap'),{...turn('answer'),status:'ready',revisionId:'revision:font'},'guide');
  state=focusAnswerSection(state,'root','answer',{revisionId:'revision:font',sectionId:'section:probes'});
  state=rememberSection(state,'root','answer','flow',{revisionId:'revision:font',sectionId:'section:dispatch'});
  state={...state,explorations:state.explorations.map((exploration)=>exploration.id==='root'?{...exploration,view:'flow'}:exploration)};
  state={...state,explorations:state.explorations.map((exploration)=>exploration.id==='root'?{...exploration,view:'guide'}:exploration)};
  const reopened=parseWorkspace(JSON.stringify(state),'p','snap');
  assert.equal(reopened.explorations[0].focusedTurnId,'answer');
  assert.deepEqual(reopened.explorations[0].turns[0].reading,{
    guide:{revisionId:'revision:font',sectionId:'section:probes'},
    flow:{revisionId:'revision:font',sectionId:'section:dispatch'},
  });
  const source=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  assert.match(source,/onClick=\{\(\)=>onFocus\(revision\.revisionId,section\.sectionId\)\}/,
    'opening a compact answer must save the section represented by that card');
  assert.match(source,/onFocus=\{\(revisionId,sectionId\)=>focusAnswer\(turn\.id,revisionId,sectionId\)\}/,
    'the focused turn and its Guide reading position must be updated together');
});

test('selecting another Guide slide reveals its answer card in the reader',()=>{
  const source=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  assert.match(source,/if\(revision&&firstSection\)props\.onFocus\(revision\.revisionId,firstSection\.sectionId\)/,
    'Guide slide navigation must focus the selected section through the workspace');
  assert.match(source,/target\?\.scrollIntoView\(\{block:'start'\}\)/,
    'the selected Guide answer must return to view when its section changes');
});

test('stable section identity survives ordering and safely falls back for a changed answer',()=>{
  const sections=[{sectionId:'two'},{sectionId:'one'},{sectionId:'three'}];
  assert.equal(sectionPosition('rev',sections,{revisionId:'rev',sectionId:'one'}),1);
  assert.equal(sectionPosition('rev-new',sections,{revisionId:'rev',sectionId:'one'}),0);
  assert.equal(sectionPosition('rev',sections,{revisionId:'rev',sectionId:'deleted'}),0);
  assert.equal(sectionPosition('rev',[],{revisionId:'rev',sectionId:'one'}),0);
  assert.equal(sectionPosition('rev',sections),0);
});

test('late reader events cannot change another revision or exploration',()=>{
  let s=addTurn(createWorkspace('p','snap'),{...turn('one'),status:'ready',revisionId:'rev'},'guide');
  s=rememberSection(s,'root','one','guide',{revisionId:'rev',sectionId:'two'});
  assert.equal(rememberSection(s,'root','one','guide',{revisionId:'old-rev',sectionId:'one'}),s);
  assert.equal(rememberSection(s,'missing','one','guide',{revisionId:'rev',sectionId:'one'}),s);
  assert.equal(rememberSection(s,'root','missing','guide',{revisionId:'rev',sectionId:'one'}),s);
  assert.equal(rememberSection(s,'root','one','history',{revisionId:'rev',sectionId:'one'}),s);
  assert.equal(rememberSection(s,'root','one','guide',{revisionId:'rev',sectionId:'two'}),s);
  s=addTurn(s,turn('child-turn'),'guide',{id:'child',originTurnId:'one'});
  s=rememberScroll(s,'root','guide',303);
  assert.equal(s.currentId,'child');assert.equal(s.explorations[0].scrollPositions.guide,303);
  assert.equal(s.explorations[1].scrollPositions,undefined);
});

test('invalid and oversized reader positions are discarded without losing saved work',()=>{
  let s=addTurn(createWorkspace('p','snap'),{...turn('one'),status:'ready',revisionId:'rev'},'guide');
  s.explorations[0].turns[0].reading={guide:{revisionId:'rev',sectionId:'x'.repeat(257)},flow:{revisionId:'rev',sectionId:'ok',unexpected:'discard'}};
  s.explorations[0].scrollPositions={guide:-1,flow:Infinity,history:10_000_001,unknown:1};
  let restored=parseWorkspace(JSON.stringify(s),'p','snap');
  assert.equal(restored.explorations[0].turns[0].question,turn('one').question);
  assert.equal(restored.explorations[0].turns[0].revisionId,'rev');
  assert.deepEqual(restored.explorations[0].turns[0].reading,{flow:{revisionId:'rev',sectionId:'ok'}});
  assert.deepEqual(restored.explorations[0].scrollPositions,{});
  for(const bad of [null,[],42,{revisionId:'rev',sectionId:'bad\u0000'},{revisionId:'',sectionId:'ok'}]){
    s.explorations[0].turns[0].reading={guide:bad};
    restored=parseWorkspace(JSON.stringify(s),'p','snap');
    assert.deepEqual(restored.explorations[0].turns[0].reading,{});
    assert.equal(rememberSection(s,'root','one','guide',bad),s);
  }
  for(const bad of [-1,NaN,Infinity,10_000_001,'100'])assert.equal(rememberScroll(s,'root','guide',bad),s);
});

test('current scroll positions persist and the removed scrollTop field is rejected',()=>{
  const s=createWorkspace('p','snap');s.explorations[0].view='history';s.explorations[0].scrollTop=123;
  assert.equal(decodeWorkspace(JSON.stringify(s),'p','snap'),null);
  let restored=parseWorkspace(JSON.stringify(s),'p','snap');
  assert.equal(restored.explorations[0].scrollPositions,undefined);
  s.explorations[0].scrollPositions={history:456};
  delete s.explorations[0].scrollTop;
  restored=parseWorkspace(JSON.stringify(s),'p','snap');assert.equal(restored.explorations[0].scrollPositions.history,456);
});

const runOne='11111111-1111-4111-8111-111111111111';
const runTwo='22222222-2222-4222-8222-222222222222';
const sectionRef=(order=0,runId=runOne)=>({runId,key:`section-${order}`,order,generationId:`generation:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa${order}`,revisionId:`revision-${order}`,supersedes:null,contextElapsedMs:40,runElapsedMs:80});
test('progressive cards append once and preserve reading, drafts, and status after restart',()=>{
  let t={...turn('one'),status:'working',runIds:[runOne]};
  t=mergePublishedSections(t,[sectionRef()]);
  t.reading={guide:{revisionId:'revision-0',sectionId:'saved-section'}};
  t=mergePublishedSections(t,[sectionRef(),sectionRef(1)]);
  assert.equal(t.publishedSections.length,2);assert.equal(t.revisionId,'revision-0');
  assert.deepEqual(t.reading.guide,{revisionId:'revision-0',sectionId:'saved-section'});
  assert.equal(mergePublishedSections(t,[sectionRef(2,runTwo)]).publishedSections.length,2);
  const s=addTurn(createWorkspace('p','snap'),t,'guide');
  const restored=parseWorkspace(JSON.stringify(s),'p','snap').explorations[0].turns[0];
  assert.equal(restored.status,'interrupted');assert.equal(restored.publishedSections.length,2);
  assert.deepEqual(restored.reading,t.reading);assert.deepEqual(restored.runIds,[runOne]);
});
test('partial and stopped answers retain every published section and independent Flow selection',()=>{
  for(const status of ['partial','stopped','failed']){
    const t=mergePublishedSections({...turn('one'),status,runIds:[runOne]},[sectionRef(),sectionRef(1)]);
    let s=addTurn(createWorkspace('p','snap'),t,'flow');
    s=rememberSection(s,'root','one','flow',{revisionId:'revision-1',sectionId:'second'});
    const restored=parseWorkspace(JSON.stringify(s),'p','snap').explorations[0].turns[0];
    assert.equal(restored.status,status);assert.equal(restored.publishedSections.length,2);
    assert.deepEqual(restored.reading.flow,{revisionId:'revision-1',sectionId:'second'});
  }
});
test('progressive section identities and timing are bounded and unknown runs cannot acquire a card',()=>{
  assert.deepEqual(cleanPublishedSections([{...sectionRef(),order:6}]),[]);
  assert.deepEqual(cleanPublishedSections([{...sectionRef(),runElapsedMs:Infinity}]),[]);
  assert.deepEqual(cleanPublishedSections([{...sectionRef(),generationId:'forged'}]),[]);
  assert.deepEqual(cleanPublishedSections([{...sectionRef(),generationId:runOne}]),[]);
  assert.equal(cleanPublishedSections([{...sectionRef(1),supersedes:sectionRef().generationId}]).length,1);
  assert.deepEqual(cleanPublishedSections([{...sectionRef(1),supersedes:runOne}]),[]);
  assert.equal(cleanPublishedSections([sectionRef(),sectionRef()]).length,1);
  const state=addTurn(createWorkspace('p','snap'),{...turn('one'),runIds:[runOne],publishedSections:[sectionRef(0,runTwo)]},'guide');
  assert.equal(parseWorkspace(JSON.stringify(state),'p','snap').explorations[0].turns[0].publishedSections.length,0);
});

test('real native section notices survive Guide state and local restart without changing their IDs',()=>{
  const {result}=JSON.parse(readFileSync(new URL('../docs/acceptance/artifacts/2026-10-17-progressive-synthetic-complete.json',import.meta.url),'utf8'));
  const t=mergePublishedSections({...turn('native'),status:'stopped',runIds:[result.runId]},result.sections);
  assert.equal(t.publishedSections.length,2);
  assert.deepEqual(t.publishedSections.map(s=>s.generationId),result.sections.map(s=>s.generationId));
  const saved=addTurn(createWorkspace(result.projectId,result.snapshotId),t,'guide');
  const restored=parseWorkspace(JSON.stringify(saved),result.projectId,result.snapshotId).explorations[0].turns[0];
  assert.equal(restored.status,'stopped');
  assert.deepEqual(restored.publishedSections,t.publishedSections);
});

test('Flow starts with the answer diagram and keeps an explicit selection as sections arrive',()=>{
  const plain={revisionId:'old-draft',diagram:false};
  const main={revisionId:'dispatch',diagram:true};
  const later={revisionId:'validation',diagram:true};
  const hasDiagram=(revision)=>revision.diagram;
  assert.equal(chooseFlowRevision([plain,main],undefined,hasDiagram),main);
  assert.equal(chooseFlowRevision([plain,main,later],main.revisionId,hasDiagram),main);
  assert.equal(chooseFlowRevision([plain,main,later],plain.revisionId,hasDiagram),plain);
  assert.equal(chooseFlowRevision([plain,main],'other-question',hasDiagram),main);
  assert.equal(chooseFlowRevision([plain],undefined,hasDiagram),plain);
  assert.equal(chooseFlowRevision([],undefined,hasDiagram),undefined);
});
