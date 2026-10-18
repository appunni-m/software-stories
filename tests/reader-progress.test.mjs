import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {cleanFlowViewport,cleanReaderProgress,cleanStoryProgress,sameSection,withReaderProgress} from '../src/lib/readerProgress.ts';
import {readSectionStoryProgress,readStoryProgress} from '../src/lib/visualStory.ts';
import {ancestors,createWorkspace,addTurn,cleanAnswerNotes,decodeWorkspace,presentationOwner,rememberCodePane,rememberFileFlow,rememberPresentation,rememberScroll,rememberSection,sectionPosition} from '../src/workspace/model.ts';

const identity={revisionId:'revision:one',sectionId:'section:one',sectionRevisionId:'section-revision:one'};
const story={...identity,sceneId:'claim:second',positions:{'claim:first':12.4,'claim:second':3.2},fullMap:true,viewport:{x:-34,y:91,zoom:.6}};
const diagram={...identity,viewport:{x:250,y:-91,zoom:1.7}};
const ready=(id,revisionId=identity.revisionId)=>({id,question:'Follow the bytes',context:[],status:'ready',revisionId});

test('presentation records contain only bounded metadata and exact immutable section identity',()=>{
  assert.deepEqual(cleanStoryProgress({...story,html:'private generated content',autoPlay:true}),story);
  const raw={stories:[story,story,{...story,sectionRevisionId:'section-revision:changed',sceneId:'claim:first'}],diagrams:[diagram]};
  const result=cleanReaderProgress(raw);
  assert.equal(result.stories.length,2);assert.equal(result.diagrams.length,1);
  assert.ok(!sameSection(result.stories[0],result.stories[1]));
  assert.equal('autoPlay' in result.stories[0],false);
  assert.equal('html' in result.stories[0],false);
  result.stories[0].positions['claim:first']=500;
  assert.equal(story.positions['claim:first'],12.4,'cleaning must not mutate input');
});

test('native zero and scene selection restore without autoplay or unscoped fallback',()=>{
  const scenes=[{id:'claim:first'},{id:'claim:second'}];
  const saved=readSectionStoryProgress({...story,positions:{'claim:first':0,'claim:second':0}},identity,scenes);
  assert.deepEqual(saved,{index:1,positions:[['claim:first',0],['claim:second',0]],fullMap:true,viewport:story.viewport});
  assert.equal('autoPlay' in saved,false);
  const changed=readSectionStoryProgress(story,{...identity,sectionRevisionId:'section-revision:changed'},scenes);
  assert.deepEqual(changed,{index:0,positions:[['claim:first',0],['claim:second',0]],fullMap:false,viewport:undefined});
  const removed=readSectionStoryProgress(story,identity,[scenes[0]]);assert.equal(removed.index,0);assert.equal(removed.viewport,undefined);
  const unscoped=JSON.stringify({sceneId:'claim:second',positions:{'claim:second':12}});
  assert.equal(readStoryProgress(unscoped,identity,scenes),undefined);
  const whole=[{id:'claim:second'}];
  assert.equal(readSectionStoryProgress(undefined,identity,whole,19.5).positions[0][1],19.5);
  assert.equal(readSectionStoryProgress({...story,positions:{'claim:second':0}},identity,whole,19.5).positions[0][1],0,'explicit native zero wins over a route fallback');
});

test('valid bounded native progress does not pass through a browser string parser',()=>{
  const scenes=Array.from({length:128},(_,i)=>({id:`claim:${i}:${'x'.repeat(200)}`}));
  const metadata={...story,sceneId:scenes[127].id,positions:Object.fromEntries(scenes.map((s,i)=>[s.id,i]))};
  assert.ok(JSON.stringify(metadata).length>16000);
  const result=readSectionStoryProgress(metadata,identity,scenes);
  assert.equal(result.index,127);assert.equal(result.positions[127][1],127);assert.deepEqual(result.viewport,story.viewport);
});

test('corrupt, nonfinite, oversized and foreign-looking presentation metadata is bounded',()=>{
  assert.deepEqual(cleanAnswerNotes(['🧭'.repeat(501),'hidden\u0085control','Safe note.']),['Safe note.']);
  for(const value of [null,[],true,'bad'])assert.deepEqual(cleanReaderProgress(value),{stories:[],diagrams:[]});
  for(const viewport of [{x:Infinity,y:0,zoom:1},{x:0,y:NaN,zoom:1},{x:10_000_001,y:0,zoom:1},{x:0,y:0,zoom:.09},{x:0,y:0,zoom:4.01},[],null])assert.equal(cleanFlowViewport(viewport),undefined);
  for(const value of [{...story,sceneId:'bad\nkey'},{...story,positions:{x:-1}},{...story,positions:{x:3601}},{...story,positions:{x:'12'}},{...story,positions:Array(129).fill(0)},{...story,fullMap:'true'},{...story,viewport:{x:0,y:0,zoom:8}}])assert.equal(cleanStoryProgress(value),undefined);
  const tooMany={...story,positions:Object.fromEntries(Array.from({length:129},(_,i)=>[`claim:${i}`,0]))};assert.equal(cleanStoryProgress(tooMany),undefined);
  assert.deepEqual(cleanReaderProgress({stories:Array(257).fill(story)}).stories,[]);
});

test('each section retains its position and metadata retention never changes artifacts',()=>{
  const list=Array.from({length:256},(_,i)=>({...story,sectionId:`section:${i}`}));
  const added={...story,sectionId:'section:next'};
  const result=withReaderProgress(list,added);
  assert.equal(result.length,256);assert.equal(result[0].sectionId,'section:1');assert.deepEqual(result.at(-1),added);
  assert.equal(list[0].sectionId,'section:0');
  const replaced=withReaderProgress(result,{...added,sceneId:'claim:first'});assert.equal(replaced.length,256);assert.equal(replaced.at(-1).sceneId,'claim:first');
});

test('parent, sibling and nested readers retain independent views without changing selected sections',()=>{
  let state=addTurn(createWorkspace('project','snapshot'),ready('root-turn'),'guide');
  state=rememberCodePane(state,'root',false);state=rememberFileFlow(state,'root','src/font.rs',true);
  state=rememberPresentation(state,'root','root-turn','stories',story);state=rememberPresentation(state,'root','root-turn','diagrams',diagram);
  const before=JSON.stringify(state.explorations[0]);
  state=addTurn(state,ready('child-turn'),'flow',{id:'child',originTurnId:'root-turn'});
  state=rememberCodePane(state,'child',true);state=rememberPresentation(state,'child','child-turn','stories',{...story,sceneId:'claim:first',fullMap:false});
  state={...state,currentId:'root'};state=addTurn(state,ready('sibling-turn'),'history',{id:'sibling',originTurnId:'root-turn'});
  state=rememberCodePane(state,'sibling',true);
  assert.equal(JSON.stringify(state.explorations[0]),before);
  const restored=decodeWorkspace(JSON.stringify(state),'project','snapshot');
  assert.equal(restored.explorations[0].codeOpen,false);assert.deepEqual(restored.explorations[0].fileFlow,{path:'src/font.rs',showAll:true});
  assert.deepEqual(restored.explorations[0].turns[0].presentation,{stories:[story],diagrams:[diagram]});
  assert.equal(restored.explorations[1].turns[0].presentation.stories[0].sceneId,'claim:first');
  assert.equal(restored.explorations[0].view,'guide');assert.equal(restored.explorations[1].view,'flow');assert.equal(restored.explorations[2].view,'history');
});

test('five nested exploration levels and a sibling restore independent answer positions and views',()=>{
  let state=createWorkspace('project','snapshot');
  state=addTurn(state,ready('turn-level-0','revision:level-0'),'guide');
  for(let depth=0;depth<=5;depth++){
    const explorationId=depth===0?'root':`child-${depth}`;
    const turnId=`turn-level-${depth}`;
    const view=depth%2===0?'guide':'flow';
    const sectionId=`section:level-${depth}:${view}`;
    state=rememberSection(state,explorationId,turnId,view,{revisionId:`revision:level-${depth}`,sectionId});
    state=rememberScroll(state,explorationId,view,depth*137.5);
    if(depth<5){
      const nextDepth=depth+1;
      state=addTurn(state,ready(`turn-level-${nextDepth}`,`revision:level-${nextDepth}`),nextDepth%2===0?'guide':'flow',{id:`child-${nextDepth}`,originTurnId:turnId});
    }
  }

  state={...state,currentId:'child-2'};
  state=addTurn(state,ready('turn-sibling','revision:sibling'),'flow',{id:'child-2-sibling',originTurnId:'turn-level-2'});
  state=rememberSection(state,'child-2-sibling','turn-sibling','flow',{revisionId:'revision:sibling',sectionId:'section:sibling:flow'});
  state=rememberScroll(state,'child-2-sibling','flow',913.25);

  const restored=decodeWorkspace(JSON.stringify(state),'project','snapshot');
  assert.deepEqual(ancestors(restored,'child-5').map(item=>item.id),['root','child-1','child-2','child-3','child-4','child-5']);
  const deepest=restored.explorations.find(item=>item.id==='child-5');
  assert.equal(deepest.view,'flow');
  assert.deepEqual(deepest.turns[0].reading.flow,{revisionId:'revision:level-5',sectionId:'section:level-5:flow'});
  assert.equal(sectionPosition('revision:level-5',[{sectionId:'first'},{sectionId:'section:level-5:flow'}],deepest.turns[0].reading.flow),1);
  assert.equal(deepest.scrollPositions.flow,687.5);
  const parent=restored.explorations.find(item=>item.id==='child-2');
  const child=restored.explorations.find(item=>item.id==='child-3');
  const sibling=restored.explorations.find(item=>item.id==='child-2-sibling');
  assert.equal(parent.turns[0].reading.guide.sectionId,'section:level-2:guide');
  assert.equal(child.turns[0].reading.flow.sectionId,'section:level-3:flow');
  assert.equal(sibling.view,'flow');
  assert.deepEqual(sibling.turns[0].reading.flow,{revisionId:'revision:sibling',sectionId:'section:sibling:flow'});
  assert.equal(sibling.scrollPositions.flow,913.25);
});

test('a late presentation callback belongs to its original turn and cannot select or move another branch',()=>{
  let state=addTurn(createWorkspace('project','snapshot'),ready('one'),'guide');
  state=addTurn(state,ready('two'),'flow',{id:'child',originTurnId:'one'});
  const child=JSON.stringify(state.explorations[1]);
  state=rememberPresentation(state,'root','one','stories',story);
  assert.equal(state.currentId,'child');assert.equal(JSON.stringify(state.explorations[1]),child);
  for(const [exploration,turn,progress] of [['child','one',story],['root','missing',story],['root','one',{...story,revisionId:'revision:foreign'}]])assert.equal(rememberPresentation(state,exploration,turn,'stories',progress),state);
  assert.equal(rememberPresentation(state,'root','one','stories',story),state,'an exact acknowledgment does not create another write');
});

test('a dedicated player uses the focused owner and cannot write into an unrelated question',()=>{
  const exploration={id:'root',turns:[ready('one'),ready('two'),ready('different','revision:other')],focusedTurnId:'one'};
  assert.equal(presentationOwner(exploration,identity.revisionId).id,'one');
  exploration.focusedTurnId='different';assert.equal(presentationOwner(exploration,identity.revisionId).id,'two');
  assert.equal(presentationOwner(exploration,'revision:missing'),undefined);
});

test('invalid optional presentation metadata does not erase valid conversation history',()=>{
  const state=addTurn(createWorkspace('project','snapshot'),ready('one'),'guide');
  state.explorations[0].codeOpen='false';state.explorations[0].fileFlow={path:'../outside',showAll:true};
  state.explorations[0].turns[0].presentation={stories:[story,{...story,revisionId:'revision:foreign'}],diagrams:[{...diagram,viewport:{x:0,y:0,zoom:Infinity}}]};
  const restored=decodeWorkspace(JSON.stringify(state),'project','snapshot');
  assert.equal(restored.explorations[0].codeOpen,undefined);assert.equal(restored.explorations[0].fileFlow,undefined);
  assert.equal(restored.explorations[0].turns[0].question,'Follow the bytes');
  assert.deepEqual(restored.explorations[0].turns[0].presentation,{stories:[story],diagrams:[]});
});

test('real reading components bind scoped progress without persisting an autoplay request',()=>{
  const workspace=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
  const visual=readFileSync(new URL('../src/VisualStory.tsx',import.meta.url),'utf8');
  assert.match(workspace,/codeOpen = current\.codeOpen \?\? true/);
  assert.match(workspace,/rememberCodePane\(s,current\.id,open\)/);
  assert.match(workspace,/<ReaderProgressContext\.Provider/);
  assert.match(workspace,/progress=\{turn\.presentation\?\.stories\.find/);
  assert.match(workspace,/onProgressChange=\{onStoryProgress\}/);
  assert.match(workspace,/progress=\{latestTurn\?\.presentation\?\.diagrams\}/);
  assert.match(workspace,/defaultViewport=\{viewport\} fitView=\{!viewport\}/);
  assert.match(workspace,/rememberFileFlow\(s,current\.id,source\.path,showAll\)/);
  assert.match(visual,/useReaderProgress\(identity\)/);
  assert.match(visual,/!nativeProgress && !isNativeRuntime\(\)/);
  assert.match(visual,/else if\(!isNativeRuntime\(\)\)try \{ localStorage\.setItem/);
  assert.doesNotMatch(visual,/if\(!nativeProgress&&onProgressChange\)/,'browser progress is never imported into an owned conversation');
  const reader=readFileSync(new URL('../src/Walkthrough.tsx',import.meta.url),'utf8');
  assert.match(reader,/useReaderSection\(selectedRevision\?\.revisionId\)/);
  assert.match(reader,/ownedSection\?\.sectionId \?\? rememberedProgress\?\.sectionId/);
  assert.match(reader,/ownedSection\?\.onSectionChange\(selectedSectionId\)/);
  const narration=readFileSync(new URL('../src/NarrationPlayer.tsx',import.meta.url),'utf8');
  assert.match(narration,/if\(mountedRef\.current\)persistPlaybackPosition\(event\.currentTarget\.currentTime\)/);
  assert.match(visual,/\[autoPlay, setAutoPlay\] = useState\(false\)/);
  assert.match(visual,/onMoveEnd=/);
  assert.match(visual,/sceneId:activeScene\.current/,'a late paused clip cannot select the previously active scene');
});
