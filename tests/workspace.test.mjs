import test from 'node:test';
import assert from 'node:assert/strict';
import { addTurn, ancestors, chapterTarget, createWorkspace, learningQuestion, parseWorkspace, rankSourcePaths } from '../src/workspace/model.ts';
const turn = (id,question='Explain the subtotal implementation') => ({id,question,context:[{path:'src/cart.ts',from:1,to:6}],status:'queued'});
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
