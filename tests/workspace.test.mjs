import test from 'node:test';
import assert from 'node:assert/strict';
import { addTurn, ancestors, createWorkspace, parseWorkspace, rankSourcePaths } from '../src/workspace/model.ts';
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
