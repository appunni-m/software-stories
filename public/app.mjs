import { renderScene, escape } from '/shared/render.mjs';
import { durationOf } from '/shared/story.mjs';
import { journeyEntries, chapterTrail, Exploration } from '/shared/journey.mjs';
import { visualForms, timelineAt } from '/shared/timeline.mjs';
import { isCurrentNarrative } from '/shared/narrative.mjs';
import { ResourceCache } from '/shared/cache.mjs';

const $ = selector => document.querySelector(selector);
const audio = new Audio();
audio.preservesPitch = true;
const sourceCache=new ResourceCache(64),mediaCache=new ResourceCache(8,URL.revokeObjectURL);
let audioId;
let story, selected=0, progress=0, playing=false, lastFrame=0, lastSave=0, busy=false, navigation;
let files=[], filesSnapshot, inspectedPath;
const messages=[];
const chapter=()=>story.chapters[selected];
const clock=seconds=>`${Math.floor(seconds/60)}:${String(Math.floor(seconds%60)).padStart(2,'0')}`;
const locked=()=>busy||navigation.locked;
async function api(path,data) {
  if(!data&&(path.startsWith('/api/source?')||path.startsWith('/api/files?')))return sourceCache.get(path,()=>request(path));
  return request(path,data);
}
async function request(path,data) {
  const response=await fetch(path,data?{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data)}:undefined);
  const result=await response.json();
  if(!response.ok)throw new Error(result.error||`Request failed (${response.status})`);
  return result;
}
const resultOf=(request,token)=>request.result?Promise.resolve(request.result):waitJob(request.jobId,token);
function preparedAudio(item) {
  return mediaCache.get(item.audio.id,async()=>{
    const response=await fetch(item.audio.url);
    if(!response.ok)throw new Error('Narration could not load.');
    return URL.createObjectURL(await response.blob());
  });
}
async function bindAudio(token) {
  const item=chapter();
  const url=await preparedAudio(item);
  if(!navigation.accepts(token)||chapter().id!==item.id)return false;
  if(audioId!==item.audio.id) {
    audioId=item.audio.id;
    const ready=new Promise((accept,reject)=>{
      const finish=()=>{audio.removeEventListener('loadedmetadata',success);audio.removeEventListener('error',failure);};
      const success=()=>{finish();accept();},failure=()=>{finish();reject(new Error('Narration could not load.'));};
      audio.addEventListener('loadedmetadata',success,{once:true});audio.addEventListener('error',failure,{once:true});
    });
    audio.src=url;
    await ready;
  }
  return navigation.accepts(token)&&chapter().id===item.id;
}
function activity(message,error=false) {
  $('#activity').hidden=!message; $('#activity').textContent=message;
  $('#activity').classList.toggle('error',error);
}
function savePlace() {
  if(!navigation||navigation.phase==='loading')return;
  navigation.current.progress=progress;
  try { sessionStorage.setItem(`journey-${story.id}`,JSON.stringify({current:navigation.current,stack:navigation.stack,pending:navigation.pending})); } catch {}
  history.replaceState(null,'',`?story=${story.id}&chapter=${chapter().id}`);
}
function render() {
  const item=chapter(),disabled=locked();
  $('#repo-name').textContent=story.repository.name;
  $('#journey-title').textContent=story.title;
  $('#chapter-count').textContent=String(story.chapters.length).padStart(2,'0');
  const entries=journeyEntries(story.chapters);
  $('#chapters').innerHTML=entries.map(({chapter:item,index:i,depth})=>`<button class="chapter ${i===selected?'active':''} ${depth?'branch':''}" style="--depth:${depth}" data-chapter="${i}" ${disabled?'disabled':''} ${i===selected?'aria-current="step"':''}><span class="chapter-number">${depth?'↳':String(story.chapters.filter(c=>!c.parentId).indexOf(item)+1).padStart(2,'0')}</span><span class="chapter-text">${item.originLabel||item.sourcePath?`<span class="chapter-origin">${escape(item.originLabel||item.sourcePath)}</span>`:''}${escape(item.title)}<span class="chapter-meta">${visualForms[item.scene.type].label} · ${clock(durationOf(item))}</span></span></button>`).join('');
  const visits=[...navigation.stack,navigation.current];
  $('#trail').innerHTML=visits.map((frame,i)=>{
    const title=i===visits.length-1&&navigation.phase==='loading'?`Opening ${navigation.label}…`:story.chapters.find(c=>c.id===frame.id)?.sourcePath||story.chapters.find(c=>c.id===frame.id)?.title||'Explanation';
    return `${i?'<span aria-hidden="true">›</span>':''}${i===visits.length-1?`<span aria-current="page">${escape(title)}</span>`:`<button data-return="${i}">${escape(title)}</button>`}`;
  }).join('');
  $('#back').disabled=!navigation.stack.length;
  $('#back').textContent=navigation.stack.length?'← Back to previous explanation':'← Back';
  $('#chapter-title').textContent=item.title;
  $('#chapter-summary').textContent=item.summary;
  const roots=story.chapters.filter(c=>!c.parentId);
  $('#chapter-position').textContent=item.parentId?'Detail':`Chapter ${roots.indexOf(item)+1} of ${roots.length}`;
  $('#scene-kind').textContent=visualForms[item.scene.type].label;
  $('#scene-purpose').textContent=item.scene.rationale||visualForms[item.scene.type].purpose;
  $('#timing-note').textContent=timelineAt(item,progress).aligned?'Visuals follow spoken cues':'Visual timing is estimated';
  $('#snapshot-badge').textContent=`Snapshot ${story.snapshotId.slice(0,8)} · ${story.repository.fileCount} files`;
  $('#transcript').textContent=item.narration;
  $('#evidence-count').textContent=`Inspect ${item.claims.length} supporting claims`;
  $('#follows').innerHTML=item.follows.map((follow,index)=>`<button class="follow" data-follow="${index}" ${disabled?'disabled':''}><span>${escape(follow.label)}</span><span>→</span></button>`).join('');
  $('#evidence').innerHTML=item.claims.map((claim,i)=>`<article class="claim"><p class="claim-text">${escape(claim.text)}</p><span class="claim-basis ${claim.basis==='inference'?'inference':''}">${escape(claim.basis)}</span>${claim.evidence.map((citation,j)=>`<button class="citation" data-claim="${i}" data-citation="${j}">${escape(citation.path)}:${citation.startLine}${citation.endLine!==citation.startLine?'–'+citation.endLine:''} ↗</button>`).join('')}</article>`).join('');
  $('#narrate').hidden=Boolean(item.audio);
  for(const id of ['narrate','ask','export','open-repo','play-source','simplify'])$('#'+id).disabled=disabled;
  $('#play').disabled=busy;
  $('#seek').disabled=busy;
  $('#scene').setAttribute('aria-busy',String(busy));
  $('#lock-status').hidden=!disabled;
  $('#lock-status').textContent=busy?(navigation.pending?'Preparing your explanation. Other exploration is locked; Back returns to your place.':'Working locally. Playback and exploration will return when this finishes.'):navigation.pending?(playing?'Playing this detail. Finish it before opening another, or use Back.':'This detail is paused. Resume to finish, or use Back.'):'Narration is playing. Pause it to explore a detail.';
  $('.interaction-hint').textContent=disabled?'Exploration locked':'Select a part to hear its details →';
  if(filesSnapshot!==story.snapshotId)loadFiles();else renderFiles();
  renderFrame();
}
function renderFrame() {
  const compact=$('#scene').clientWidth<700;
  $('#scene').classList.toggle('compact',compact);
  $('#scene').innerHTML=renderScene(chapter(),progress,{compact,locked:locked()});
  $('#seek').value=String(Math.round(progress*1000));
  $('#time').textContent=clock(progress*durationOf(chapter()));
  $('#duration').textContent=clock(durationOf(chapter()));
  $('#play').textContent=playing?'Ⅱ':'▶';
  $('#play').setAttribute('aria-label',playing?'Pause narration':'Play narration');
  const step=timelineAt(chapter(),progress).index;
  $('#spoken-step').textContent=`Beat ${step+1} of ${chapter().scene.steps.length}`;
  $('#spoken-step').title=chapter().scene.steps[step].detail;
}
function pause() { playing=false;audio.pause();navigation.pause();savePlace();render(); }
function display(id,position=0) {
  playing=false;audio.pause();
  selected=Math.max(0,story.chapters.findIndex(item=>item.id===id));
  progress=Math.max(0,Math.min(1,position));
  navigation.current={id:chapter().id,progress};
  // Prepare completed clips once. Returning to them uses local blob URLs,
  // without clearing the player or making another media request.
  if(chapter().audio)preparedAudio(chapter()).catch(()=>{});
  render();savePlace();
}
function goBack(target=navigation.stack.length-1) {
  if(!navigation.stack.length)return;
  playing=false;audio.pause();
  let frame; do {frame=navigation.back();}while(navigation.stack.length>target);
  busy=false;display(frame.id,frame.progress);
  activity('Returned to your saved place.');
}
$('#back').addEventListener('click',()=>goBack());
$('#trail').addEventListener('click',event=>{const target=event.target.closest('[data-return]');if(target)goBack(Number(target.dataset.return));});
function showJourney(showFiles=false) {
  $('#files-panel').hidden=!showFiles;$('#chapters').hidden=showFiles;
  $('#show-journey').setAttribute('aria-pressed',String(!showFiles));$('#show-files').setAttribute('aria-pressed',String(showFiles));
}
async function loadFiles() {
  const snapshot=story.snapshotId;filesSnapshot=snapshot;
  try {const result=await api(`/api/files?snapshot=${snapshot}`);if(story.snapshotId!==snapshot)return;files=result;renderFiles();}
  catch(error){filesSnapshot=null;activity(error.message,true);}
}
function renderFiles() {
  const query=$('#file-filter').value.toLowerCase();
  $('#file-list').innerHTML=files.filter(file=>file.path.toLowerCase().includes(query)).map(file=>`<div class="file-row"><button class="file-play" data-file="${escape(file.path)}" aria-label="Play walkthrough of ${escape(file.path)}" ${locked()?'disabled':''}>▶ ${escape(file.path)}<small>${escape(file.kind)} · ${file.lines} lines</small></button><button class="file-source" data-source="${escape(file.path)}" aria-label="View source of ${escape(file.path)}">↗</button></div>`).join('')||'<p class="files-hint">No matching files.</p>';
}
function playFile(path) {
  if(locked())return;
  ask(`Explain the specific file ${path}. Walk me through its purpose, important functions, inputs, outputs, and how it connects to the rest of the project.`,{filePath:path,originLabel:`File · ${path}`});
}
$('#show-journey').addEventListener('click',()=>showJourney());
$('#show-files').addEventListener('click',()=>showJourney(true));
$('#file-filter').addEventListener('input',renderFiles);
$('#file-list').addEventListener('click',event=>{const play=event.target.closest('[data-file]'),source=event.target.closest('[data-source]');if(play)playFile(play.dataset.file);else if(source)inspectSource(source.dataset.source);});
async function waitJob(jobId,token) {
  while(navigation.accepts(token)) {
    const job=await api(`/api/jobs/${jobId}`);
    if(!navigation.accepts(token))return null;
    activity(job.message);
    if(job.status==='error')throw new Error(job.message);
    if(job.status==='done')return job.result;
    await new Promise(resolve=>setTimeout(resolve,1000));
  }
  return null;
}
function addMessage(kind,text) {
  messages.push({kind,text});
  $('#messages').innerHTML=messages.map(item=>`<div class="${item.kind==='user'?'user-message':'answer-message'}">${escape(item.text)}</div>`).join('');
  $('#messages').scrollTop=$('#messages').scrollHeight;
}
async function ensureAudio(token) {
  if(chapter().audio)return true;
  busy=true;render();
  const target=chapter().id;
  const request=await api('/api/narrate',{storyId:story.id,chapterId:target,backend:$('#backend').value});
  const result=await resultOf(request,token);
  if(!result||!navigation.accepts(token)||chapter().id!==target)return false;
  story=result.story;busy=false;
  render();
  return true;
}
async function ensureRecap(token) {
  if(isCurrentNarrative(chapter()))return true;
  const target=chapter().id;
  busy=true;navigation.pending=navigation.pending||navigation.stack.length>0;navigation.phase='loading';activity('Preparing the software story behind this explanation…');render();
  const request=await api('/api/recap',{storyId:story.id,chapterId:target,backend:$('#backend').value});
  const result=await resultOf(request,token);
  if(!result||!navigation.accepts(token)||chapter().id!==target)return false;
  story=result.story;navigation.arrive(token,target);busy=false;display(target);
  return true;
}
async function startAudio(token=navigation.epoch) {
  if(!await ensureRecap(token))return;
  if(!await ensureAudio(token)||!navigation.accepts(token))return;
  if(!await bindAudio(token))return;
  if(progress>=.995)progress=0;
  audio.currentTime=progress*durationOf(chapter());
  audio.playbackRate=Number($('#speed').value);
  await audio.play();
  if(!navigation.accepts(token))return;
  playing=true;navigation.start();activity('');render();savePlace();
}
async function ask(query,options={}) {
  if(locked())return;
  navigation.current.progress=progress;
  const parentId=chapter().id,token=navigation.begin(options.originLabel||'Your question');
  if(token===null)return;
  playing=false;audio.pause();busy=true;showJourney();addMessage('user',query);
  activity(`Opening ${options.originLabel||'your question'}: finding the relevant source evidence…`);render();
  try {
    const request=await api('/api/explain',{snapshotId:story.snapshotId,storyId:story.id,chapterId:parentId,query,backend:$('#backend').value,mode:'follow-up',filePath:options.filePath,originLabel:options.originLabel,nodeId:options.nodeId});
    const result=await resultOf(request,token);
    if(!result||!navigation.arrive(token,result.chapterId))return;
    story=result.story;busy=false;display(result.chapterId);
    addMessage('assistant',`${result.reused?'Reopened':'Added'} “${chapter().title}”. ${chapter().summary}`);
    await startAudio(token);
  }catch(error){if(navigation.accepts(token)){busy=false;const frame=navigation.fail(token);if(frame)display(frame.id,frame.progress);activity(error.message,true);addMessage('assistant','This explanation could not be completed. You are back at your starting point.');}}
}
async function openChapter(index) {
  if(locked()||index===selected)return;
  navigation.current.progress=progress;
  const token=navigation.begin(story.chapters[index].title);
  if(token===null)return;
  navigation.arrive(token,story.chapters[index].id);display(story.chapters[index].id);
  try {await startAudio(token);}catch(error){if(navigation.accepts(token)){busy=false;const frame=navigation.fail(token);display(frame.id,frame.progress);activity(error.message,true);}}
}
$('#chapters').addEventListener('click',event=>{const target=event.target.closest('[data-chapter]');if(target)openChapter(Number(target.dataset.chapter));});
$('#follows').addEventListener('click',event=>{const target=event.target.closest('[data-follow]');if(target&&!locked())ask(chapter().follows[Number(target.dataset.follow)].prompt);});
function explore(event) {
  if(locked())return;
  const target=event.target.closest('[data-node]');if(!target)return;
  const node=chapter().scene.nodes.find(node=>node.id===target.dataset.node);
  ask(`Help me understand the “${node.label}” part in “${chapter().title}”. Explain its responsibility, inputs, outputs, and the most useful next detail using this project’s code.`,{originLabel:node.label,nodeId:node.id});
}
$('#scene').addEventListener('click',explore);
$('#scene').addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();event.stopPropagation();explore(event);}});
$('#question-form').addEventListener('submit',event=>{event.preventDefault();if(locked())return;const query=$('#question').value.trim();if(query){$('#question').value='';ask(query);}});
$('#question').addEventListener('keydown',event=>{if(event.key==='Enter'&&!event.shiftKey){event.preventDefault();$('#question-form').requestSubmit();}});
$('#simplify').addEventListener('click',()=>ask(`I did not understand “${chapter().title}”. Explain the same central idea more simply using one concrete example supported by this source. Avoid introducing extra concepts.`,{originLabel:'A simpler explanation'}));
$('#play').addEventListener('click',async()=>{
  if(busy)return;if(playing)return pause();
  try {await startAudio();}catch(error){busy=false;navigation.pause();activity(`Audio could not play: ${error.message}. You can retry or use Back.`,true);render();}
});
$('#seek').addEventListener('input',()=>{if(busy)return;progress=Number($('#seek').value)/1000;if(chapter().audio)audio.currentTime=progress*durationOf(chapter());savePlace();renderFrame();});
$('#speed').addEventListener('change',()=>{audio.playbackRate=Number($('#speed').value);});
$('#volume').addEventListener('input',()=>{audio.volume=Number($('#volume').value)/100;$('#volume-value').textContent=`${$('#volume').value}%`;try{localStorage.setItem('software-stories-volume',String(audio.volume));}catch{}});
audio.addEventListener('ended',()=>{playing=false;progress=1;navigation.complete();savePlace();activity('Explanation finished. Choose a part to go deeper, or use Back.');render();});
audio.addEventListener('error',()=>{if(!audio.getAttribute('src')||!audio.error)return;playing=false;busy=false;navigation.pause();activity('Narration could not load. Retry playback or use Back.',true);render();});
function tick(now) {
  if(playing&&now-lastFrame>=65){lastFrame=now;progress=Math.min(1,audio.currentTime/durationOf(chapter()));navigation.current.progress=progress;renderFrame();if(now-lastSave>1000){lastSave=now;savePlace();}}
  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
window.addEventListener('resize',()=>{if(story)renderFrame();});
$('#transcript-toggle').addEventListener('click',()=>{$('#transcript').hidden=!$('#transcript').hidden;$('#transcript-toggle').setAttribute('aria-expanded',String(!$('#transcript').hidden));});
$('#narrate').addEventListener('click',async()=>{if(locked())return;busy=true;render();try{const token=navigation.epoch;if(await ensureRecap(token))await ensureAudio(token);}catch(error){busy=false;navigation.pause();activity(error.message,true);render();}});
$('#export').addEventListener('click',async()=>{
  if(locked())return;busy=true;render();const token=navigation.epoch;
  try{const request=await api('/api/export',{storyId:story.id,backend:$('#backend').value});const result=await waitJob(request.jobId,token);if(!result)return;story=result.story;$('#download').hidden=false;$('#download').innerHTML=`<a href="${escape(result.url)}" download>Download your narrated MP4 ↓</a>`;busy=false;render();activity('Your video is ready.');}catch(error){if(navigation.accepts(token)){busy=false;render();activity(error.message,true);}}
});
async function inspectSource(path,claim,citation) {
  try{
    const file=await api(`/api/source?snapshot=${story.snapshotId}&path=${encodeURIComponent(path)}`);inspectedPath=file.path;
    $('#source-title').textContent=file.path;$('#source-basis').textContent=`${(claim?.basis||file.kind).toUpperCase()} SOURCE`;
    $('#source-meta').textContent=`${file.revision} · Snapshot ${story.snapshotId.slice(0,8)} · SHA256 ${file.sha256.slice(0,12)}${citation?` · lines ${citation.startLine}–${citation.endLine}`:''}`;
    $('#source-code').innerHTML=file.text.split('\n').map((line,index)=>`<span class="source-line ${citation&&index+1>=citation.startLine&&index+1<=citation.endLine?'highlight':''}"><span class="line-no">${index+1}</span>${escape(line)}</span>`).join('');
    $('#source-dialog').showModal();$('.source-line.highlight')?.scrollIntoView({block:'center'});
  }catch(error){activity(error.message,true);}
}
$('#play-source').addEventListener('click',()=>{if(locked())return;$('#source-dialog').close();playFile(inspectedPath);});
$('#evidence').addEventListener('click',event=>{const target=event.target.closest('[data-citation]');if(!target)return;const claim=chapter().claims[Number(target.dataset.claim)],citation=claim.evidence[Number(target.dataset.citation)];inspectSource(citation.path,claim,citation);});
$('#open-repo').addEventListener('click',()=>{if(locked())return;$('#repo-path').value=story.repository.root;$('#repo-dialog').showModal();});
$('#mode').addEventListener('change',()=>{const isPR=$('#mode').value==='pr';$('#base-ref').hidden=!isPR;$('#base-label').hidden=!isPR;$('#base-ref').required=isPR;});
$('#repo-form').addEventListener('submit',async event=>{
  event.preventDefault();if(locked())return;$('#repo-error').textContent='';busy=true;render();const token=navigation.epoch;
  try{
    const repository=await api('/api/repositories',{path:$('#repo-path').value,baseRef:$('#mode').value==='pr'?$('#base-ref').value:undefined});
    const request=await api('/api/explain',{snapshotId:repository.id,backend:$('#backend').value,mode:$('#mode').value,query:$('#topic').value.trim()||($('#mode').value==='pr'?'Explain what changed, the behavior before and after, and any uncertainty about why.':'Explain the major responsibilities, entry points, and how data moves through this repository.')});
    $('#repo-dialog').close();const result=await waitJob(request.jobId,token);if(!result)return;
    story=result.story;navigation=new Exploration(story.chapters[0].id);busy=false;display(navigation.current.id);messages.length=0;addMessage('assistant','Your journey is ready. Follow the responsibilities, then choose a part to understand.');
  }catch(error){busy=false;render();if($('#repo-dialog').open)$('#repo-error').textContent=error.message;else activity(error.message,true);}
});
$('#about').addEventListener('click',()=>$('#about-dialog').showModal());
document.querySelectorAll('[data-close]').forEach(button=>button.addEventListener('click',()=>button.closest('dialog').close()));
document.addEventListener('keydown',event=>{if(event.key===' '&&!['INPUT','TEXTAREA','SELECT','BUTTON'].includes(event.target.tagName)&&!document.querySelector('dialog[open]')){event.preventDefault();$('#play').click();}});
try{
  const result=await api('/api/bootstrap'),params=new URLSearchParams(location.search),saved=params.get('story');
  story=saved?await api(`/api/stories/${encodeURIComponent(saved)}`):result.story;navigation=new Exploration(story.chapters[0].id);
  try{
    const visit=JSON.parse(sessionStorage.getItem(`journey-${story.id}`)),valid=frame=>frame&&story.chapters.some(c=>c.id===frame.id)&&Number.isFinite(frame.progress)&&frame.progress>=0&&frame.progress<=1;
    if(visit&&valid(visit.current)&&Array.isArray(visit.stack)&&visit.stack.every(valid)){navigation.current=visit.current;navigation.stack=visit.stack;navigation.pending=Boolean(visit.pending);navigation.phase='paused';}
    else if(params.get('chapter')&&story.chapters.some(c=>c.id===params.get('chapter'))){const path=chapterTrail(story.chapters,params.get('chapter'));navigation.stack=path.slice(0,-1).map(c=>({id:c.id,progress:0}));navigation.current={id:params.get('chapter'),progress:0};}
  }catch{}
  $('#backend').value=result.settings.agent;
  $('#backend').querySelector('option[value="codex"]').textContent=`Codex CLI · Luna ${result.settings.agentReasoningEffort}`;
  $('#voice-label').textContent=result.settings.ttsBackend==='vulkan'?'Serena · GPU (Vulkan)':'Serena · CPU';
  audio.volume=result.settings.playbackVolume??.6;
  try{const volume=localStorage.getItem('software-stories-volume');if(volume!==null&&Number.isFinite(Number(volume)))audio.volume=Math.max(0,Math.min(1,Number(volume)));}catch{}
  $('#volume').value=String(Math.round(audio.volume*100));$('#volume-value').textContent=`${$('#volume').value}%`;
  display(navigation.current.id,navigation.current.progress);
}catch(error){activity(error.message,true);}
