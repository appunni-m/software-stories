import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Badge, Button, Dialog, Flex, IconButton, Tabs, TextField, Theme, Tooltip } from '@radix-ui/themes';
import { Group, Panel, Separator } from 'react-resizable-panels';
import { Background, Controls, MarkerType, ReactFlow } from '@xyflow/react';
import { ArrowLeft, ArrowRight, ArrowUp, BookOpen, ChevronDown, ChevronRight, Code2, FileCode2, Files, Folder, GitBranch, GitCommitHorizontal, History, LoaderCircle, MessageSquare, Network, Paperclip, Pencil, Play, Search, ShieldCheck, Square, X } from 'lucide-react';
import { CodePane } from '../design/CodePane';
import { FileFlow } from '../FileFlow';
import { VisualStory } from '../VisualStory';
import { buildInventory } from '../lib/inventory';
import { isNativeRuntime, listChapterRevisions, readSnapshotSource, type ChapterRevision, type ChapterSourceAnchor, type SnapshotManifest, type SnapshotSource, type SnapshotIndex, type HotspotDestination } from '../lib/commands';
import { projectProvenRelationshipGraph } from '../lib/atlas';
import { inspectInteractiveVisual } from '../lib/visuals';
import type { AtlasHotspotSelection, SourceLineRange, Workspace as WorkspaceId } from '../lib/navigation';
import { ancestors, contextualGuideIntent, discardUnstartedGuide, chooseFlowRevision, rankSourcePaths, rememberScroll, rememberSection, sectionPosition, type AnswerSummary, type Context, type Exploration, type ReadingPosition, type Turn, type View } from './model';
import { appendQuestion, appendRepositoryGuide, authorizeWorkspace, ensureWorkspaceIndex, revokeWorkspace, restoreWorkspaceAnswers, runQuestion, runRepositoryGuide, stopQuestion, updateWorkspace, useWorkspace, watchWorkspaceAnswers } from './store';
import { useConnectionState } from '../lib/connectionStore';
import { errorMessageOr, explanationErrorMessage } from '../lib/errorMessage';
import './workspace.css';

interface Commit { oid:string; parents:string[]; subject:string; author:string; timestamp:string }
interface Props {
  manifest: SnapshotManifest; projectName:string; workspace:WorkspaceId; routePath?:string; routeRange?:SourceLineRange; routeSelection?:AtlasHotspotSelection;
  children:ReactNode; onWorkspace:(workspace:WorkspaceId)=>void; onSettings:()=>void;
  onReview:(revision:ChapterRevision)=>void; onRead:(revision:ChapterRevision)=>void;
  onCompare:(base:string,head:string)=>void; onRememberFile:(path:string)=>void;
}
const label = (c:Context) => `${c.path.split('/').at(-1)}${c.from ? `:${c.from}${c.to !== c.from ? `–${c.to}` : ''}` : ''}`;

/** The production workspace consumes native snapshots and revision references, never example data. */
export function WorkspaceFrame({manifest,...props}:Omit<Props,'manifest'>&{manifest:SnapshotManifest|null}) { return manifest ? <DeveloperWorkspace key={`${manifest.summary.projectId}:${manifest.summary.snapshotId}`} {...props} manifest={manifest}/> : <>{props.children}</>; }

export function DeveloperWorkspace({manifest,projectName,workspace,routePath,routeRange,routeSelection,children,onWorkspace,onSettings,onReview,onRead,onCompare,onRememberFile}:Props) {
  const {projectId,snapshotId} = manifest.summary;
  const state = useWorkspace(projectId,snapshotId);
  const connection=useConnectionState();
  const current = state.explorations.find((e)=>e.id===state.currentId)!;
  const path = ancestors(state);
  const rows = useMemo(()=>buildInventory(manifest).filter((r)=>r.kind==='file' && r.disposition==='captured' && !['binary','invalidEncoding'].includes(r.contentKind) && !r.categories.includes('secret')), [manifest]);
  const sourcePaths = useMemo(()=>rankSourcePaths(rows.map((r)=>r.path)),[rows]);
  const [activity,setActivity] = useState<'files'|'search'|'changes'|'history'>('files');
  const [filter,setFilter] = useState('');
  const [sourceHits,setSourceHits] = useState<{path:string;line:number;text:string}[]>([]);
  const [searchLimited,setSearchLimited] = useState(false);
  const [searchBusy,setSearchBusy] = useState(false);
  const [searchError,setSearchError] = useState<string|null>(null);
  const [openFolders,setOpenFolders] = useState<Set<string>>(()=>new Set(['src','app','lib','Sources']));
  const [query,setQuery] = useState('');
  const [attachments,setAttachments] = useState<Context[]>([]);
  const [narrowPane,setNarrowPane] = useState<'explanation'|'source'|'explorer'>('explanation');
  const [codeOpen,setCodeOpen] = useState(true);
  const [source,setSource] = useState<SnapshotSource|null>(null);
  const [sourceError,setSourceError] = useState<string|null>(null);
  const [sourceBusy,setSourceBusy] = useState(false);
  const [reveal,setReveal] = useState<Context|null>(null);
  const [chapters,setChapters] = useState<ChapterRevision[]>([]);
  const [chaptersLoading,setChaptersLoading] = useState(true);
  const [chapterError,setChapterError] = useState<string|null>(null);
  const [index,setIndex] = useState<SnapshotIndex|null>(null);
  const [indexBusy,setIndexBusy] = useState(true);
  const [indexError,setIndexError] = useState<string|null>(null);
  const [commits,setCommits] = useState<Commit[]>([]);
  const [gitError,setGitError] = useState<string|null>(null);
  const [gitBusy,setGitBusy] = useState(false);
  const [consentTurn,setConsentTurn] = useState<string|null>(null);
  const [guideConsent,setGuideConsent] = useState<{id:string;resume:boolean}|null>(null);
  const [consentBusy,setConsentBusy] = useState(false);
  const [consentError,setConsentError] = useState<string|null>(null);
  const [sharingOpen,setSharingOpen] = useState(false);
  const [searchFocused,setSearchFocused] = useState(false);
  const [appearance,setAppearance] = useState<'dark'|'light'>(()=>document.documentElement.dataset.theme==='light' || (!['light','dark'].includes(document.documentElement.dataset.theme??'') && matchMedia('(prefers-color-scheme: light)').matches) ? 'light':'dark');
  const composer = useRef<HTMLTextAreaElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const answerScroll = useRef<HTMLDivElement>(null);
  const restoringScroll = useRef(true);
  const tool = ['changes','walkthrough','chapter-studio'].includes(workspace);
  const update = (fn:(s:typeof state)=>typeof state) => updateWorkspace(projectId,snapshotId,fn);
  const go = (id:string) => { update((s)=>({...s,currentId:id,file:s.explorations.find((e)=>e.id===id)?.file??s.file})); setNarrowPane('explanation');onWorkspace('guide-plan'); };
  const setView = (view:View) => update((s)=>({...s,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,view}:e)}));
  const saveSection = (turnId:string,view:'guide'|'flow',revisionId:string,sectionId:string) => update((s)=>rememberSection(s,current.id,turnId,view,{revisionId,sectionId}));
  const refreshChapters = async () => {
    if (!isNativeRuntime()) return;
    try { const result=await listChapterRevisions(projectId,snapshotId); setChapters(result); setChapterError(null); }
    catch (failure) { setChapterError(errorMessageOr(failure,'Saved explanations could not be loaded.')); }
    finally { setChaptersLoading(false); }
  };
  useEffect(()=>{
    const observer = new MutationObserver(()=>setAppearance(document.documentElement.dataset.theme==='light' || (!['light','dark'].includes(document.documentElement.dataset.theme??'') && matchMedia('(prefers-color-scheme: light)').matches) ? 'light':'dark'));
    observer.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
    const media=matchMedia('(prefers-color-scheme: light)'); const changed=()=>{if (!['light','dark'].includes(document.documentElement.dataset.theme??'')) setAppearance(media.matches?'light':'dark');};media.addEventListener('change',changed);
    return ()=>{observer.disconnect();media.removeEventListener('change',changed);};
  },[]);
  useEffect(()=>{ void refreshChapters(); const changed=(event:Event)=>{const project=(event as CustomEvent<{projectId?:string}>).detail?.projectId;if(!project||project===projectId) void refreshChapters();};
    window.addEventListener('code-atlas:chapters-changed',changed);return()=>window.removeEventListener('code-atlas:chapters-changed',changed);
  },[projectId,snapshotId]);
  useEffect(()=>{
    let closed=false;let unlisten:(()=>void)|undefined;
    if(isNativeRuntime())void watchWorkspaceAnswers(projectId,snapshotId).then((stop)=>{if(closed){stop();return;}unlisten=stop;return restoreWorkspaceAnswers(projectId,snapshotId);}).catch((failure)=>{if(!closed)setChapterError(errorMessageOr(failure,'Saved answers could not be restored.'));});
    return()=>{closed=true;unlisten?.();};
  },[projectId,snapshotId]);
  useEffect(()=>{let active=true; if(isNativeRuntime()) void ensureWorkspaceIndex(projectId,snapshotId).then((value)=>{if(active){setIndex(value);setIndexError(null);}}).catch((failure)=>{if(active)setIndexError(errorMessageOr(failure,'Code relationships could not be loaded.'));}).finally(()=>{if(active)setIndexBusy(false);}); return()=>{active=false;};},[projectId,snapshotId,chapters.length]);
  useEffect(()=>{if(!state.file && sourcePaths.length) update((s)=>({...s,file:routePath && sourcePaths.includes(routePath)?routePath:sourcePaths[0]}));},[state.file,sourcePaths]);
  useEffect(()=>{if(routePath && sourcePaths.includes(routePath)){update((s)=>({...s,file:routePath}));setCodeOpen(true);setReveal({path:routePath,from:routeRange?.startLine,to:routeRange?.endLine,sha256:routeRange?.sourceSha256});}},[routePath,routeRange]);
  useEffect(()=>{
    setSource(null);setSourceError(null);if(!state.file||!isNativeRuntime()) return;
    let active=true;setSourceBusy(true);
    void readSnapshotSource(projectId,snapshotId,state.file).then((value)=>{if(active){setSource(value);setSourceBusy(false);}}).catch((failure)=>{if(active){setSourceError(errorMessageOr(failure,'This captured file could not be opened.'));setSourceBusy(false);}});
    return()=>{active=false;};
  },[projectId,snapshotId,state.file]);
  useEffect(()=>{setReveal(current.reveal??null);},[current.id]);
  useLayoutEffect(()=>{
    // Ignore layout-generated scroll events until the saved answer has loaded.
    // Capture the owner/view here so a late event cannot move another exploration.
    restoringScroll.current=true;
    if(chaptersLoading||tool||!answerScroll.current)return;
    let release=0;
    const restore=requestAnimationFrame(()=>{
      answerScroll.current?.scrollTo({top:current.scrollPositions?.[current.view]??0});
      release=requestAnimationFrame(()=>{restoringScroll.current=false;});
    });
    return()=>{cancelAnimationFrame(restore);cancelAnimationFrame(release);};
  },[current.id,current.view,chaptersLoading,tool]);
  useEffect(()=>{if(!isNativeRuntime())return;let active=true;setGitBusy(true);setGitError(null);
    void invoke<Commit[]>('list_workspace_commits',{projectId}).then((value)=>{if(active)setCommits(value);}).catch((failure)=>{if(active)setGitError(errorMessageOr(failure,'Git history is unavailable.'));}).finally(()=>{if(active)setGitBusy(false);});
    return()=>{active=false;};
  },[projectId]);
  function openFile(context:Context) { update((s)=>({...s,file:context.path,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,file:context.path,reveal:context}:e)}));setReveal(context);setNarrowPane('source');setCodeOpen(true);onRememberFile(context.path);setSearchFocused(false); }
  function attach(context:Context) {setAttachments((all)=>all.some((c)=>c.path===context.path&&c.from===context.from&&c.to===context.to)?all:[...all,context]);composer.current?.focus();}
  function ask(question:string,view:View=current.view==='flow'?'flow':'guide',context=attachments,parentTurn?:string){
    const clean=question.replace(/[\r\n\t]+/g,' ').trim();if(!clean){setView(view);onWorkspace('guide-plan');composer.current?.focus();return;}
    if(!connection.status?.authenticated){onSettings();return;}
    const id=appendQuestion(projectId,snapshotId,clean,context,view,parentTurn);setQuery('');setAttachments([]);setSearchFocused(false);setNarrowPane('explanation');onWorkspace('guide-plan');
    if(state.grantId) void runQuestion(projectId,snapshotId,id,state.grantId);else{setConsentError(null);setConsentTurn(id);}
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  }
  function explain(context:Context){ask(`Explain what ${label(context)} does and how it works.`,current.view==='flow'?'flow':'guide',[{...context,sha256:source?.path===context.path?source.sha256:context.sha256}]);}
  function buildGuide() {
    if(!connection.status?.authenticated){onSettings();return;}
    const id=appendRepositoryGuide(projectId,snapshotId,'How does this project work, from its entry points through its main behavior and failure paths?');
    setGuideConsent({id,resume:false});setConsentError(null);setNarrowPane('explanation');onWorkspace('guide-plan');
  }
  function retryTurn(turn:Turn) {
    if(turn.guideId){setGuideConsent({id:turn.id,resume:true});setConsentError(null);return;}
    if(state.grantId)void runQuestion(projectId,snapshotId,turn.id,state.grantId);else setConsentTurn(turn.id);
  }
  function branch(turn:Turn,question:string,context:Context[]){ask(question,current.view==='flow'?'flow':'guide',context,turn.id);}
  useEffect(()=>{
    const listener=(event:KeyboardEvent)=>{
      if(document.querySelector('[role="dialog"][aria-modal="true"]'))return;
      const mod=event.metaKey||event.ctrlKey; const k=event.key.toLowerCase();
      if(mod&&(k==='p'||k==='k'||(event.shiftKey&&k==='f'))){event.preventDefault();event.stopImmediatePropagation();setActivity('search');onWorkspace('guide-plan');requestAnimationFrame(()=>search.current?.focus());}
      if(mod&&k==='l'){event.preventDefault();event.stopImmediatePropagation();composer.current?.focus();}
      if(mod&&['1','2','3'].includes(k)){event.preventDefault();event.stopImmediatePropagation();setView((['guide','flow','history'] as const)[Number(k)-1]);onWorkspace('guide-plan');}
      if(event.altKey&&k==='arrowleft'&&current.parentId){event.preventDefault();go(current.parentId);}
    };
    const find=()=>{setActivity('search');onWorkspace('guide-plan');requestAnimationFrame(()=>search.current?.focus());};
    window.addEventListener('keydown',listener,true);window.addEventListener('code-atlas:search',find);return()=>{window.removeEventListener('keydown',listener,true);window.removeEventListener('code-atlas:search',find);};
  },[current.id,current.parentId]);
  const term=(activity==='search'?filter:query).trim().toLowerCase();
  useEffect(()=>{
    let active=true;setSourceHits([]);setSearchLimited(false);setSearchError(null);setSearchBusy(false);
    if(term.length<2||term.length>120||!isNativeRuntime())return;
    const requestId=crypto.randomUUID();let dispatched=false;
    setSearchBusy(true);const timer=setTimeout(()=>{dispatched=true;void invoke<{matches:{path:string;line:number;text:string}[];limited:boolean}>('search_workspace_source',{projectId,snapshotId,query:term,requestId}).then((result)=>{if(active){setSourceHits(result.matches);setSearchLimited(result.limited);}}).catch(()=>{if(active)setSearchError('Captured source search is unavailable. File and symbol results are still shown.');}).finally(()=>{if(active)setSearchBusy(false);});},250);
    return()=>{active=false;clearTimeout(timer);if(dispatched)void invoke('cancel_repository_context',{requestId}).catch(()=>{});};
  },[term,projectId,snapshotId]);
  const results=useMemo(()=>{
    if(!term)return[];
    const files=rows.filter((r)=>r.path.toLowerCase().includes(term)).slice(0,8).map((r)=>({path:r.path,text:r.path,from:undefined as number|undefined,to:undefined as number|undefined}));
    const symbols=(index?.symbols??[]).filter((s)=>s.name.toLowerCase().includes(term)).slice(0,8).map((s)=>({path:s.path,text:s.name,from:s.anchor.startLine,to:s.anchor.endLine}));
    const lines=source?.contents.split('\n').flatMap((text,i)=>text.toLowerCase().includes(term)?[{path:source.path,text:text.trim(),from:i+1,to:i+1}]:[])??[];
    const captured=sourceHits.map((r)=>({path:r.path,text:r.text,from:r.line,to:r.line}));
    return [...files,...symbols,...lines,...captured].filter((r,i,all)=>all.findIndex((x)=>x.path===r.path&&x.from===r.from&&x.text===r.text)===i).slice(0,80);
  },[term,rows,index,source,sourceHits]);
  const graph=useMemo(()=>index?projectProvenRelationshipGraph(index,manifest,new Set(rows.map((r)=>r.path))):null,[index,manifest,rows]);
  function openDestination(destination:HotspotDestination) {
    if(destination.kind==='atlasFile' && rows.some((r)=>r.path===destination.path)) openFile({path:destination.path});
    else if(destination.kind==='atlasSymbol') {
      const symbol=index?.symbols.find((s)=>s.symbolId===destination.symbolId&&s.path===destination.path);
      if(symbol)openFile({path:symbol.path,sha256:symbol.anchor.sourceSha256,from:symbol.anchor.startLine,to:symbol.anchor.endLine});
    } else if(destination.kind==='relationship') {
      const edge=graph?.edges.find((e)=>e.relationship.relationshipId===destination.relationshipId);
      if(edge)openFile({path:edge.evidence.path,sha256:edge.evidence.sha256,from:edge.evidence.startLine,to:edge.evidence.endLine});
    }
  }
  useEffect(()=>{if(routeSelection)openDestination(routeSelection.kind==='file'?{kind:'atlasFile',path:routeSelection.path}:routeSelection.kind==='symbol'?{kind:'atlasSymbol',path:routeSelection.path,symbolId:routeSelection.symbolId}:routeSelection);},[routeSelection,index,graph]);
  const latestTurn=current.turns.slice().reverse().find((t)=>chapters.some((r)=>r.revisionId===t.revisionId));
  const flowRevisions=latestTurn?.publishedSections?.map((s)=>chapters.find((r)=>r.revisionId===s.revisionId)).filter((r):r is ChapterRevision=>!!r)??[];
  const latestRevision=chooseFlowRevision(flowRevisions.length?flowRevisions:chapters.filter((r)=>r.revisionId===latestTurn?.revisionId),latestTurn?.reading?.flow?.revisionId,
    (revision)=>revision.sections.some((section)=>inspectInteractiveVisual(revision,section).status==='available'));
  const activityButtons=[{id:'files',label:'Explorer',icon:Files},{id:'search',label:'Search',icon:Search},{id:'changes',label:'Changes',icon:GitBranch},{id:'history',label:'Explorations',icon:History}] as const;
  const treeFiles=rows.map((r)=>r.path).filter((p)=>!filter||p.toLowerCase().includes(filter.toLowerCase())).sort((a,b)=>a.localeCompare(b));
  const folderEntries=(prefix:string):{path:string;folder:boolean}[]=>{
    const result=new Map<string,boolean>();for(const p of treeFiles){if(!p.startsWith(prefix))continue;const rest=p.slice(prefix.length);const first=rest.split('/')[0];result.set(prefix+first,rest.includes('/'));}return [...result].map(([path,folder])=>({path,folder})).sort((a,b)=>Number(b.folder)-Number(a.folder)||a.path.localeCompare(b.path));
  };
  const renderFolder=(prefix:string,depth=0):ReactNode=>folderEntries(prefix).map((entry)=>entry.folder?<div key={entry.path}><button className="dw-file dw-folder" style={{paddingLeft:10+depth*12}} aria-expanded={!!filter||openFolders.has(entry.path)} onClick={()=>setOpenFolders((all)=>{const next=new Set(all);if(next.has(entry.path))next.delete(entry.path);else next.add(entry.path);return next;})}>{filter||openFolders.has(entry.path)?<ChevronDown size={12}/>:<ChevronRight size={12}/>}<Folder size={14}/><span>{entry.path.split('/').at(-1)}</span></button>{(filter||openFolders.has(entry.path))&&renderFolder(entry.path+'/',depth+1)}</div>:<button key={entry.path} className={`dw-file ${state.file===entry.path&&codeOpen?'active':''}`} style={{paddingLeft:22+depth*12}} title={entry.path} draggable onDragStart={(e)=>{e.dataTransfer.setData('application/x-code-atlas-context',JSON.stringify({path:entry.path}));e.dataTransfer.effectAllowed='copy';}} onClick={()=>openFile({path:entry.path})}><FileCode2 size={14}/><span>{entry.path.split('/').at(-1)}</span></button>);
  const renderExplorations=(parent:string|null,depth=0):ReactNode=>state.explorations.filter((e)=>e.parentId===parent).map((e)=>{
    const nested=renderExplorations(e.id,depth+1);
    return <li key={e.id}><button aria-current={e.id===current.id?'page':undefined} className={e.id===current.id?'active':''} style={{paddingLeft:10+depth*12}} onClick={()=>go(e.id)}><MessageSquare size={13}/><span>{e.id==='root'?projectName:e.title}</span><small>{e.turns.length}</small></button><ul>{nested}</ul></li>;
  });
  const fromAnchor=(a:ChapterSourceAnchor):Context=>({path:a.path,sha256:a.sourceSha256,from:a.startLine,to:a.endLine});
  return <Theme appearance={appearance} accentColor="blue" grayColor="slate" radius="medium" className={`developer-workspace dw-narrow-${narrowPane}`}>
    <nav className="dw-activity" aria-label="Repository tools">{activityButtons.map(({id,label:caption,icon:Icon})=><Tooltip content={caption} key={id}><button className={(workspace==='changes'?id==='changes':activity===id)?'active':''} aria-label={caption} aria-pressed={workspace==='changes'?id==='changes':activity===id} onClick={()=>{setActivity(id);setNarrowPane('explorer');if(id==='changes')onWorkspace('changes');else onWorkspace('guide-plan');if(id==='search')requestAnimationFrame(()=>search.current?.focus());}}><Icon size={21}/></button></Tooltip>)}<div className="dw-activity-bottom"><Tooltip content="Repository sharing"><button aria-label="Repository sharing" onClick={()=>setSharingOpen(true)}><ShieldCheck size={19}/></button></Tooltip></div></nav>
    <Group orientation="horizontal" className="dw-panels">
      <Panel id="repository-sidebar" defaultSize="220px" minSize="160px" maxSize="400px">
        <aside className="dw-sidebar" aria-label="Repository sidebar">
          <header><strong>{activity==='changes'?'Source control':activity==='search'?'Search':activity==='history'?'Explorations':'Explorer'}</strong></header>
          {activity==='changes'?<><div className="dw-sidebar-section">Commit history <span>{commits.length ? `${commits.length} recent`:''}</span></div><div className="dw-commits">{gitBusy&&<p role="status">Loading commits…</p>}{gitError&&<p role="alert">{gitError}</p>}{commits.map((c)=><button key={c.oid} className="dw-commit" disabled={!c.parents.length} title={c.parents.length?`${c.subject}\n${c.oid}`:`${c.subject} — initial commit; parent comparison unavailable`} onClick={()=>{if(!c.parents.length)return;onCompare(c.parents[0],c.oid);setNarrowPane('explanation');onWorkspace('changes');}}><span className={`dw-commit-line ${c.parents.length>1?'merge':''}`}><GitCommitHorizontal size={16}/></span><span><strong>{c.subject||'Untitled commit'}</strong><small>{c.author} · {c.oid.slice(0,7)}{c.parents.length>1?' · merge':!c.parents.length?' · initial commit':''}</small></span></button>)}</div></>:activity==='history'?<ul className="dw-explorations">{renderExplorations(null)}</ul>:<>
            <div className="dw-sidebar-search"><TextField.Root ref={search} size="1" value={filter} placeholder={activity==='search'?'Files, symbols, code…':'Find a file…'} aria-label={activity==='search'?'Search repository':'Filter files'} onChange={(e)=>setFilter(e.target.value)}><TextField.Slot><Search size={13}/></TextField.Slot></TextField.Root></div>
            {activity==='search'?<div className="dw-search-results"><small>Files, symbols and captured code</small>{searchBusy&&<p role="status">Searching…</p>}{searchLimited&&<p>Search limit reached. Refine your search to find more.</p>}{searchError&&<p role="status">{searchError}</p>}{filter&&results.length===0&&<p>No matching code in this search.</p>}{results.map((r,i)=><button key={i} onClick={()=>openFile(r)}><FileCode2 size={14}/><span><strong>{r.text}</strong><small>{r.path}{r.from?`:${r.from}`:''}</small></span></button>)}{filter&&<Button size="1" variant="soft" onClick={()=>ask(filter.length<8?`Explain ${filter}`:filter)}><MessageSquare size={13}/>Explore with AI</Button>}</div>:<><div className="dw-sidebar-section">{projectName}</div><nav className="dw-file-list" aria-label="Captured files">{renderFolder('')}</nav><div className="dw-sidebar-section dw-explorations-heading"><History size={13}/>Explorations</div><ul className="dw-explorations">{renderExplorations(null)}</ul></>}
          </>}
        </aside>
      </Panel><Separator className="dw-separator" aria-label="Resize repository sidebar"/>
      <Panel id="reading-workspace" minSize="320px"><section className="dw-main" aria-label="Code exploration">
        <nav className="dw-path" aria-label="Exploration path"><IconButton size="1" variant="ghost" color="gray" aria-label="Back one exploration" disabled={!current.parentId} onClick={()=>current.parentId&&go(current.parentId)}><ArrowLeft size={14}/></IconButton>{path.map((e,i)=><span key={e.id}>{i>0&&<ChevronRight size={12}/>}<button aria-current={e.id===current.id?'page':undefined} onClick={()=>go(e.id)}>{e.id==='root'?projectName:e.title}</button></span>)}{path.length>1&&<Badge size="1" color="gray">Level {path.length}</Badge>}{tool&&<span className="dw-tool-label">{workspace==='changes'?'Changes':workspace==='chapter-studio'?'Edit explanation':'Player'}</span>}</nav>
        <div className="dw-composer" onDragOver={(e)=>{if(e.dataTransfer.types.includes('application/x-code-atlas-context'))e.preventDefault();}} onDrop={(e)=>{e.preventDefault();try{const c=JSON.parse(e.dataTransfer.getData('application/x-code-atlas-context')) as Context;if(rows.some((r)=>r.path===c.path))attach(c);}catch{/* Ignore unrelated drops. */}}}>
          <Flex gap="2" mb="2"><Button size="1" variant="soft" onClick={()=>{const intent=contextualGuideIntent(query,attachments,!!current.parentId);if(intent)ask(intent,'guide');else buildGuide();}}><BookOpen size={14}/>Guide me</Button><Button size="1" variant="soft" color="gray" onClick={()=>ask(query,'flow')}><Network size={14}/>Show the flow</Button><IconButton className="dw-narrow-toggle" size="1" variant="ghost" color="gray" aria-label={narrowPane==='source'?'Show explanation':'Show source'} onClick={()=>setNarrowPane(narrowPane==='source'?'explanation':'source')}><Code2 size={15}/></IconButton></Flex>
          <div className="dw-input">{attachments.length>0&&<div className="dw-context">{attachments.map((c,i)=><Badge key={i} color="gray"><FileCode2 size={12}/>{label(c)}<button aria-label={`Remove ${label(c)}`} onClick={()=>setAttachments((all)=>all.filter((_,n)=>n!==i))}><X size={12}/></button></Badge>)}</div>}
            <textarea ref={composer} rows={1} value={query} maxLength={1000} aria-label="Ask about this codebase" placeholder={current.parentId?'Ask a follow-up in this exploration…':'What would you like to understand?'} onChange={(e)=>{e.target.setCustomValidity('');setQuery(e.target.value);}} onFocus={()=>setSearchFocused(true)} onKeyDown={(e)=>{if(e.key==='Escape')setSearchFocused(false);if(e.key==='Enter'&&!e.shiftKey&&!e.nativeEvent.isComposing){e.preventDefault();ask(query);}}}/>
            <Tooltip content="Attach open file"><IconButton size="1" variant="ghost" color="gray" disabled={!source} aria-label="Attach open file" onClick={()=>source&&attach({path:source.path,sha256:source.sha256})}><Paperclip size={15}/></IconButton></Tooltip><IconButton size="1" aria-label="Send question" disabled={!query.trim()} onClick={()=>ask(query)}><ArrowUp size={15}/></IconButton>
          </div>
          {searchFocused&&query.length>1&&<div className="dw-suggestions"><small>{results.length?'In your code':'No matching code in this search'}</small>{results.slice(0,4).map((r,i)=><button key={i} onMouseDown={(e)=>e.preventDefault()} onClick={()=>openFile(r)}><FileCode2 size={13}/><span>{r.path}{r.from?`:${r.from}`:''}<small>{r.text}</small></span><ArrowRight size={13}/></button>)}<button onMouseDown={(e)=>e.preventDefault()} onClick={()=>ask(query)}><MessageSquare size={13}/>Explore with AI<ArrowRight size={13}/></button></div>}
        </div>
        {tool?<div className="dw-tool-content"><Button className="dw-return" size="1" variant="ghost" onClick={()=>onWorkspace('guide-plan')}><ArrowLeft size={13}/>Back to exploration</Button>{children}</div>:<Group orientation="horizontal" className="dw-reading">
          {codeOpen&&<><Panel id="source" defaultSize="43%" minSize="220px">{source?<CodePane path={source.path} contents={source.contents} theme={appearance} reveal={reveal?.sha256 && reveal.sha256!==source.sha256 ? null : reveal} onExplain={explain} onAttach={(c)=>attach({...c,sha256:source.sha256})} onClose={()=>{setCodeOpen(false);setNarrowPane('explanation');}}/>:<div className="dw-source-empty">{sourceBusy?<><LoaderCircle size={18}/><p>Opening source…</p></>:sourceError?<p role="alert">{sourceError}</p>:<><Code2 size={24}/><p>Open a file to read its code.</p></>}</div>}</Panel><Separator className="dw-separator" aria-label="Resize source and explanation"/></>}
          <Panel id="explanations" minSize="280px"><Tabs.Root className="dw-explanation" value={current.view} onValueChange={(value)=>setView(value as View)}><div className="dw-viewbar"><Tabs.List aria-label="Exploration views"><Tabs.Trigger value="guide"><BookOpen size={13}/>Guide</Tabs.Trigger><Tabs.Trigger value="flow"><Network size={13}/>Flow</Tabs.Trigger><Tabs.Trigger value="history"><MessageSquare size={13}/>Chat history</Tabs.Trigger></Tabs.List>{!codeOpen&&<IconButton size="1" variant="ghost" color="gray" aria-label="Show code pane" onClick={()=>setCodeOpen(true)}><Code2 size={15}/></IconButton>}</div>
            <div className="dw-answer-scroll" ref={answerScroll} onScroll={(event)=>{if(restoringScroll.current)return;const top=event.currentTarget.scrollTop;update((s)=>rememberScroll(s,current.id,current.view,top));}}>
              {chapterError&&<p className="dw-error" role="alert">{chapterError}<button onClick={()=>{void refreshChapters();}}>Retry</button></p>}
              <Tabs.Content value="guide">
                {current.turns.length===0?<section className="dw-welcome"><BookOpen size={25}/><h1>Get to know {projectName}</h1><p>Ask a question or start with the open file. Your explanations stay here as you explore.</p><Button size="2" variant="soft" onClick={buildGuide}>Build repository guide<ArrowRight size={14}/></Button>{source&&<button className="dw-text-action" onClick={()=>explain({path:source.path,sha256:source.sha256})}>Explain {source.path.split('/').at(-1)}</button>}</section>:current.turns.map((turn)=><Answer key={turn.id} chapters={chapters} turn={turn} loading={chaptersLoading} revision={chapters.find((r)=>r.revisionId===turn.revisionId)} branches={state.explorations.filter((e)=>e.parentId===current.id&&e.originTurnId===turn.id)} onGo={go} onOpen={(a)=>openFile(fromAnchor(a))} onDestination={openDestination} onBranch={(question,context)=>branch(turn,question,context)} onStop={()=>stopQuestion(projectId,snapshotId,turn.id)} onRetry={()=>retryTurn(turn)} onReview={onReview} onRead={onRead} onSettings={onSettings} onSection={(revisionId,sectionId)=>saveSection(turn.id,'guide',revisionId,sectionId)}/>) }
                {chapters.length>0&&<details className="dw-library"><summary>Saved explanations <span>{chapters.length}</span></summary>{chapters.slice().sort((a,b)=>b.createdAtUnixMs-a.createdAtUnixMs).map((r)=><button key={r.revisionId} onClick={()=>{
                  const id=crypto.randomUUID();update((s)=>({...s,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,turns:e.turns.some((t)=>t.revisionId===r.revisionId)?e.turns:[...e.turns,{id,question:r.title,context:[],status:'ready',revisionId:r.revisionId}]}:e)}));
                }}><BookOpen size={13}/><span>{r.title}</span><small>{r.lifecycle}</small></button>)}</details>}
              </Tabs.Content>
              <Tabs.Content value="flow">{flowRevisions.length>1&&<div className="dw-flow-choice"><label htmlFor="answer-flow-section">Explanation</label><select id="answer-flow-section" value={latestRevision?.revisionId??''} onChange={(event)=>{const revision=flowRevisions.find((r)=>r.revisionId===event.target.value);if(latestTurn&&revision?.sections[0])saveSection(latestTurn.id,'flow',revision.revisionId,revision.sections[0].sectionId);}}>{flowRevisions.map((revision)=><option key={revision.revisionId} value={revision.revisionId}>{revision.sections[0]?.title??revision.title}</option>)}</select></div>}{indexError&&!latestRevision&&<p role="status" className="dw-error">{indexError}</p>}{latestRevision?<EvidenceFlow revision={latestRevision} saved={latestTurn?.reading?.flow} onSection={(sectionId)=>{if(latestTurn)saveSection(latestTurn.id,'flow',latestRevision.revisionId,sectionId);}} theme={appearance} onOpen={(a)=>openFile(fromAnchor(a))}/>:source?<FileFlow index={index} edges={graph?.edges??[]} loading={indexBusy} path={source.path} onOpenFile={(p,_snapshot,range)=>openFile({path:p,from:range?.startLine,to:range?.endLine,sha256:range?.sourceSha256})}/>:<p>Open a file or ask a question to explore its flow.</p>}{current.turns.length>0&&<div className="dw-flow-conversation">{current.turns.map((turn)=><Answer key={turn.id} chapters={chapters} compact turn={turn} loading={chaptersLoading} revision={chapters.find((r)=>r.revisionId===turn.revisionId)} branches={state.explorations.filter((e)=>e.parentId===current.id&&e.originTurnId===turn.id)} onGo={go} onOpen={(a)=>openFile(fromAnchor(a))} onDestination={openDestination} onBranch={(question,context)=>branch(turn,question,context)} onStop={()=>stopQuestion(projectId,snapshotId,turn.id)} onRetry={()=>retryTurn(turn)} onReview={onReview} onRead={onRead} onSettings={onSettings} onSection={(revisionId,sectionId)=>saveSection(turn.id,'guide',revisionId,sectionId)}/>)}</div>}</Tabs.Content>
              <Tabs.Content value="history"><div className="dw-history">{path.map((e)=><section key={e.id}><button className="dw-history-level" onClick={()=>go(e.id)}><MessageSquare size={13}/>{e.id==='root'?projectName:e.title}<small>Level {ancestors(state,e.id).length}</small></button>{e.turns.map((turn)=><article key={turn.id}><strong>{turn.question}</strong><p>{chapters.find((r)=>r.revisionId===turn.revisionId)?.sections[0]?.prose??(turn.status==='working'?'Working on this question…':turn.error?explanationErrorMessage(turn.error):'Open the saved explanation.')}</p><button className="dw-text-action" onClick={()=>{go(e.id);update((s)=>({...s,explorations:s.explorations.map((x)=>x.id===e.id?{...x,view:'guide'}:x)}));}}>Open explanation<ArrowRight size={13}/></button>{turn.runIds?.length ? <AnswerDiagnostics project={projectId} snapshot={snapshotId} runId={turn.runIds.at(-1)!}/> : null}</article>)}</section>)}{!current.turns.length&&<p>Your questions and explanations will appear here.</p>}</div></Tabs.Content>
            </div>
          </Tabs.Root></Panel>
        </Group>}
      </section></Panel>
    </Group>
    <Dialog.Root open={!!consentTurn} onOpenChange={(open)=>{if(!open&&!consentBusy){if(consentTurn)stopQuestion(projectId,snapshotId,consentTurn);setConsentTurn(null);}}}><Dialog.Content maxWidth="520px"><Dialog.Title>Explore {projectName} with Codex</Dialog.Title><Dialog.Description size="2">Allow questions, indexed names, documentation hints and relevant captured code from this snapshot to be sent to OpenAI through your connected Codex account. Account quota applies.</Dialog.Description><p className="dw-permission-note">Codex searches and reads relevant portions of captured code. Each question can share up to 250 KB across 12 files; large files can be read in portions. Secret-like paths and excluded files remain blocked. This permission is limited to this snapshot and account; revoke it from the shield in the left bar.</p>{consentError&&<p role="alert" className="dw-error">{consentError}</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray" disabled={consentBusy}>Cancel</Button></Dialog.Close><Button disabled={consentBusy} onClick={()=>{if(!consentTurn)return;const id=consentTurn;setConsentBusy(true);setConsentError(null);void authorizeWorkspace(projectId,snapshotId).then((grant)=>{setConsentTurn(null);void runQuestion(projectId,snapshotId,id,grant);}).catch((failure)=>setConsentError(errorMessageOr(failure,'Connect Codex in the header, then try again.'))).finally(()=>setConsentBusy(false));}}>{consentBusy?'Connecting…':'Allow and explain'}</Button></Flex>{consentError&&<Button variant="ghost" onClick={()=>{if(consentTurn)stopQuestion(projectId,snapshotId,consentTurn);setConsentTurn(null);requestAnimationFrame(onSettings);}}>Open connection settings</Button>}</Dialog.Content></Dialog.Root>
    <Dialog.Root open={!!guideConsent} onOpenChange={(open)=>{if(!open&&!consentBusy){if(guideConsent&&!guideConsent.resume)update((state)=>discardUnstartedGuide(state,guideConsent.id));setGuideConsent(null);}}}><Dialog.Content maxWidth="520px"><Dialog.Title>{guideConsent?.resume?'Continue your guide':'Build a guide to '+projectName}</Dialog.Title><Dialog.Description size="2">Codex chooses the code and builds an illustrated guide in parts. Saved parts stay available while it works.</Dialog.Description><p className="dw-permission-note">Uses your connected Codex account for up to {guideConsent?.resume?'six further':'seven'} investigations. Account quota applies.</p><details className="dw-error-details"><summary>What will be shared</summary><p className="dw-permission-note">{guideConsent?.resume?'Continue unfinished parts without rebuilding completed ones. Up to six further investigations; each part allows at most one explicit continuation.':'One outline investigation and up to six explanation investigations will use your connected Codex account.'} Account quota applies. Each investigation can share up to 250 KB of captured code across 12 files and run for up to ten minutes. Large files are read in portions. This does not run repository code. You can stop at any time; restarting the app never resumes generation automatically.</p></details>{consentError&&<p role="alert" className="dw-error">{consentError}</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray" disabled={consentBusy}>Cancel</Button></Dialog.Close><Button disabled={consentBusy} onClick={()=>{if(!guideConsent)return;const pending=guideConsent;setConsentBusy(true);setConsentError(null);void (state.grantId?Promise.resolve(state.grantId):authorizeWorkspace(projectId,snapshotId)).then((grant)=>{setGuideConsent(null);void runRepositoryGuide(projectId,snapshotId,pending.id,grant,pending.resume);}).catch((error)=>setConsentError(errorMessageOr(error,'Connect Codex in the header, then try again.'))).finally(()=>setConsentBusy(false));}}>{consentBusy?'Connecting…':guideConsent?.resume?'Continue guide':'Build guide'}</Button></Flex></Dialog.Content></Dialog.Root>
    <Dialog.Root open={sharingOpen} onOpenChange={setSharingOpen}><Dialog.Content maxWidth="460px"><Dialog.Title>Repository sharing</Dialog.Title><Dialog.Description size="2">{state.grantId?'Codex can explore captured code in this snapshot when you ask a question. Your live working copy is not included.':'Codex has no workspace sharing permission for this snapshot.'}</Dialog.Description>{state.grantId&&<Button mt="4" variant="soft" color="red" onClick={()=>{void revokeWorkspace(projectId,snapshotId).then(()=>setSharingOpen(false)).catch((failure)=>setConsentError(errorMessageOr(failure,'Permission could not be revoked.')));}}>Revoke permission and stop questions</Button>}{consentError&&<p role="alert">{consentError}</p>}<Flex justify="end" mt="4"><Dialog.Close><Button variant="soft" color="gray">Done</Button></Dialog.Close></Flex></Dialog.Content></Dialog.Root>
  </Theme>;
}

interface AnswerProps {chapters?:ChapterRevision[];hideQuestion?:boolean;turn:Turn; loading?:boolean; revision?:ChapterRevision; branches:Exploration[]; compact?:boolean;onGo:(id:string)=>void;onOpen:(a:ChapterSourceAnchor)=>void;onBranch:(question:string,context:Context[])=>void;onStop:()=>void;onRetry:()=>void;onReview:(r:ChapterRevision)=>void;onRead:(r:ChapterRevision)=>void;onSettings:()=>void;onDestination:(d:HotspotDestination)=>void;onSection:(revisionId:string,sectionId:string)=>void}
function Answer(props:AnswerProps){
  const {turn,chapters,branches}=props;
  if(!turn.runIds?.length&&!turn.guideId)return <AnswerSection {...props}/>;
  const sections=turn.publishedSections??[];
  const capacityError=!!turn.error&&/retained locally|saved answers|answer.*storage limit|answer.*directory limit/i.test(turn.error);
  const working=turn.status==='working'||turn.status==='queued';
  return <section className="dw-progressive-answer" aria-label={`Question: ${turn.question}`}>
    <div className="dw-user-question"><MessageSquare size={14}/><h2>{turn.question}</h2></div>
    <div className="dw-answer-progress" role="status">{working?<><LoaderCircle size={14}/><span>{turn.guideId?(sections.length?'Building the next part of your guide…':'Building your repository guide…'):sections.length?'Adding more detail…':'Building your explanation…'}</span><Button size="1" variant="ghost" color="gray" onClick={props.onStop}><Square size={12}/>Stop</Button></>:turn.status==='ready'?null:<><span>{turn.status==='stopped'?'Stopped.':turn.status==='interrupted'?'Interrupted.':turn.status==='partial'?'Some details could not be completed.':'This question could not be completed.'} {sections.length>0?'Your saved sections are available below.':''}</span>{(!turn.guideId||turn.guide?.units.some((u)=>u.status!=='complete'&&u.runIds.length<2))&&<Button size="1" variant="ghost" onClick={props.onRetry}>{turn.guideId?'Continue guide':'Try again'}</Button>}</>}</div>
    {turn.guide&&<details className="dw-guide-outline"><summary>{turn.guide.title}<span>{turn.guide.units.filter((u)=>u.status==='complete').length}/{turn.guide.units.length} parts</span></summary><ol>{turn.guide.units.map((unit)=><li key={unit.key}><span>{unit.question}</span><small>{unit.status}</small></li>)}</ol><p>{turn.guide.capturedFiles} captured files are available to investigate. {turn.guide.startingFiles} starting files helped form this outline; this is not an explained-file count.</p>{turn.guide.omissions.map((note)=><p key={note}>{note}</p>)}</details>}
    {turn.notes?.map((note)=><p className="dw-answer-note" key={note}>{note}</p>)}
    {capacityError&&<AnswerStorage/>}
    {turn.error&&<details className="dw-error-details"><summary>Details</summary><p>{explanationErrorMessage(turn.error)}</p></details>}
    {sections.map((section,index)=><AnswerSection {...props} key={`${section.runId}:${section.generationId}`} hideQuestion
      loading={props.loading||!chapters?.some((r)=>r.revisionId===section.revisionId)} revision={chapters?.find((r)=>r.revisionId===section.revisionId)}
      turn={{...turn,status:'ready',revisionId:section.revisionId,notes:[],error:undefined}}
      branches={index===sections.length-1?branches:[]}/>)}
    {sections.length===0&&!working&&<p className="dw-empty-answer">No section was published for this attempt. Earlier saved explanations are still in the library.</p>}
  </section>;
}
function AnswerDiagnostics({project,snapshot,runId}:{project:string;snapshot:string;runId:string}) {
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState<string|null>(null);
  const [saved,setSaved]=useState(false);
  useEffect(()=>{setSaved(false);setError(null);},[runId]);
  const download=async()=>{
    setBusy(true);setError(null);setSaved(false);
    try{
      setSaved(await invoke<boolean>('save_workspace_answer_timing',{projectId:project,snapshotId:snapshot,runId}));
    }catch(failure){setError(errorMessageOr(failure,'Timing diagnostics could not be loaded. Your explanation is preserved.'));}
    finally{setBusy(false);}
  };
  return <details className="dw-error-details"><summary>Diagnostics</summary><p>Save timings and counters to investigate a slow or incomplete answer. Source code and conversation text are excluded.</p><Button size="1" variant="soft" disabled={busy} onClick={()=>void download()}>{busy?'Preparing…':'Save timing report'}</Button>{saved&&<p role="status">Timing report saved.</p>}{error&&<p role="alert">{error}</p>}</details>;
}
function AnswerStorage(){
  const [open,setOpen]=useState(false);const [answers,setAnswers]=useState<AnswerSummary[]>([]);const [error,setError]=useState<string|null>(null);const [busy,setBusy]=useState<string|null>(null);
  useEffect(()=>{if(open)void invoke<AnswerSummary[]>('list_workspace_answer_records').then(setAnswers).catch((failure)=>setError(errorMessageOr(failure,'Saved answers could not be loaded.')));},[open]);
  const release=async(answer:AnswerSummary)=>{
    setBusy(answer.runId);setError(null);
    try{
      const scope={runId:answer.runId,projectId:answer.projectId,snapshotId:answer.snapshotId};
      for(const section of answer.sections)await invoke('save_workspace_answer_section',{...scope,generationId:section.generationId});
      await invoke('remove_workspace_answer_record',scope);
      setAnswers((saved)=>saved.filter((a)=>a.runId!==answer.runId));
      window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed',{detail:{projectId:answer.projectId}}));
    }catch(failure){setError(errorMessageOr(failure,'This saved answer could not be cleared. Its sections are preserved.'));}finally{setBusy(null);}
  };
  return <Dialog.Root open={open} onOpenChange={setOpen}><Dialog.Trigger><Button size="1" variant="soft">Manage saved answers</Button></Dialog.Trigger><Dialog.Content maxWidth="560px"><Dialog.Title>Free local answer storage</Dialog.Title><Dialog.Description size="2">Keep your explanations and source links in the library while clearing an older investigation record. This removes its detailed generation receipt. It does not send code or ask AI again.</Dialog.Description><div className="dw-saved-answers">{answers.map((answer)=><section key={answer.runId}><p>{answer.question}</p><small>{answer.sections.length} saved section{answer.sections.length===1?'':'s'} · {answer.outcome}</small><Button size="1" variant="soft" disabled={!!busy||answer.outcome==='working'} onClick={()=>void release(answer)}>{busy===answer.runId?'Saving sections…':'Keep explanations and free space'}</Button></section>)}</div>{error&&<p role="alert">{error}</p>}<Flex justify="end" mt="4"><Dialog.Close><Button variant="soft" color="gray">Done</Button></Dialog.Close></Flex></Dialog.Content></Dialog.Root>;
}
function AnswerSection({turn,revision,loading,branches,compact,hideQuestion,onGo,onOpen,onBranch,onStop,onRetry,onReview,onRead,onSettings,onDestination,onSection}:AnswerProps){
  const position=revision?sectionPosition(revision.revisionId,revision.sections,turn.reading?.guide):0;
  const section=revision?.sections[position];
  const selectSection=(index:number)=>{
    const next=revision?.sections[index];if(!revision||!next)return;
    onSection(revision.revisionId,next.sectionId);
  };
  const readableError=turn.error?explanationErrorMessage(turn.error):undefined;
  const knownEvidenceError=!!turn.error&&readableError!==turn.error;
  const contexts=section?.claims.flatMap((c)=>c.anchors).slice(0,3).map((a)=>({path:a.path,sha256:a.sourceSha256,from:a.startLine,to:a.endLine}))??turn.context;
  return <article className="dw-turn" aria-label={`Question: ${turn.question}`}>{!hideQuestion&&<div className="dw-user-question"><MessageSquare size={14}/><h2>{turn.question}</h2></div>}<div className="dw-answer-card">
    {turn.status==='working'||turn.status==='queued'?<div className="dw-working" role="status"><LoaderCircle size={16}/><span>Building your explanation…</span><Button size="1" variant="ghost" color="gray" onClick={onStop}><Square size={12}/>Stop</Button></div>:turn.status==='failed'||turn.status==='interrupted'||turn.status==='stopped'?<><p className="dw-error">{turn.status==='stopped'?'Stopped. Saved explanations are preserved.':turn.status==='interrupted'?'This question was interrupted. Saved explanations are preserved.':knownEvidenceError?readableError:'This explanation could not be completed. Your saved work is preserved.'}</p>{turn.error&&!knownEvidenceError&&<details className="dw-error-details"><summary>Details</summary><p>{explanationErrorMessage(turn.error)}</p></details>}<Button size="1" variant="soft" onClick={onRetry}>Try this question again</Button></>:!revision?loading?<p role="status">Loading saved explanation…</p>:<><p>This saved explanation is unavailable. Your question is preserved.</p><Button size="1" variant="soft" onClick={onRetry}>Explain this question again</Button></>:section?<>
      <div className="dw-answer-heading"><span>{compact?'Explanation':`${position+1} / ${revision.sections.length}`}</span>{revision.lifecycle!=='accepted'&&<Badge size="1" color="gray">{revision.lifecycle==='rejected'?'Rejected':'Draft'}</Badge>}<div><Tooltip content="Edit this explanation"><IconButton size="1" variant="ghost" color="gray" aria-label={`Edit ${revision.title}`} onClick={()=>onReview(revision)}><Pencil size={13}/></IconButton></Tooltip><Tooltip content="Open interactive player"><IconButton size="1" variant="ghost" color="gray" aria-label={`Play ${revision.title}`} onClick={()=>onRead(revision)}><Play size={13}/></IconButton></Tooltip></div></div>
      {turn.notes?.map((note)=><p key={note} className="dw-answer-note" role="status">{note}</p>)}
      <h3>{section.title}</h3>
      {!compact ? <>
        <VisualStory key={section.sectionRevisionId} revision={revision} section={section} onSettings={onSettings} onOpenAnchor={onOpen} onFollowHotspot={(hotspot)=>{
          const destination=hotspot.destination;
          if(destination.kind==='section')selectSection(revision.sections.findIndex((s)=>s.sectionId===destination.sectionId));
          else if(destination.kind==='codeLens'){const anchor=revision.sections.flatMap((s)=>s.claims.flatMap((c)=>c.anchors)).find((a)=>a.anchorId===destination.anchorId);if(anchor)onOpen(anchor);}
          else onDestination(destination);
        }}/>
        <div className="dw-section-controls"><IconButton size="1" variant="ghost" color="gray" aria-label="Previous section" disabled={position===0} onClick={()=>selectSection(position-1)}><ArrowLeft size={14}/></IconButton><span>{position+1} / {revision.sections.length}</span><Button size="1" variant="soft" disabled={position>=revision.sections.length-1} onClick={()=>selectSection(position+1)}>Next section<ArrowRight size={13}/></Button></div>
      </> : <details className="dw-flow-narration"><summary>Read narration</summary><p className="dw-prose">{section.spokenText || section.prose}</p><div className="dw-citations">{section.claims.flatMap((c)=>c.anchors).slice(0,5).map((a)=><button key={a.anchorId} onClick={()=>onOpen(a)}><Code2 size={12}/>{a.path.split('/').at(-1)}:{a.startLine}–{a.endLine}</button>)}</div></details>}
      <footer className="dw-branches"><Button size="1" variant="ghost" onClick={()=>onBranch(`Explain ${section.title} in more detail.`,contexts)}><Network size={13}/>Explore further</Button>{branches.length>0&&<span>{branches.length} exploration{branches.length===1?'':'s'}</span>}{branches.map((e)=><button className="dw-branch-link" key={e.id} onClick={()=>onGo(e.id)}><span className="dw-branch-connector">↳</span><span>{e.title}</span><small>{e.turns.length} answer{e.turns.length===1?'':'s'}</small><ChevronRight size={13}/></button>)}</footer>
    </>:<p>This saved explanation has no sections.</p>}
  </div></article>;
}
function EvidenceFlow({revision,saved,onSection,theme,onOpen}:{revision:ChapterRevision;saved?:ReadingPosition;onSection:(sectionId:string)=>void;theme:'dark'|'light';onOpen:(a:ChapterSourceAnchor)=>void}){
  const diagrams=revision.sections.map((s)=>inspectInteractiveVisual(revision,s));
  const remembered=sectionPosition(revision.revisionId,revision.sections,saved);
  const selected=diagrams[remembered]?.status==='available'?remembered:Math.max(0,diagrams.findIndex((d)=>d.status==='available'));
  const result=diagrams[selected];
  const available=diagrams.some((d)=>d.status==='available');
  if(!available)return <div className="dw-flow-empty"><Network size={23}/><h2>No evidenced diagram in this explanation</h2><p>Use the source links in Guide to inspect the implementation. A graph is shown only when it has verified evidence.</p></div>;
  if(result?.status!=='available')return null;
  const {visual,anchorsById}=result;
  const horizontal=visual.layoutHint.direction==='leftToRight';
  const nodes=visual.nodes.map((n,i)=>({id:n.nodeId,position:{x:horizontal?i*290:0,y:horizontal?0:i*130},data:{label:<><strong>{n.label}</strong>{n.detail&&<small>{n.detail}</small>}</>},style:{width:250}}));
  const edges=visual.edges.map((e)=>({id:e.edgeId,source:e.fromNodeId,target:e.toNodeId,label:e.label??undefined,markerEnd:{type:MarkerType.ArrowClosed}}));
  return <section className="dw-flow-panel"><div className="dw-flow-heading"><h2>{visual.title}</h2><select aria-label="Diagram section" value={selected} onChange={(e)=>{const section=revision.sections[Number(e.target.value)];if(section)onSection(section.sectionId);}}>{diagrams.map((d,i)=>d.status==='available'?<option key={i} value={i}>{revision.sections[i].title}</option>:null)}</select></div><p>Select a step to open its source.</p><div className="dw-flow-canvas"><ReactFlow key={`${revision.revisionId}:${selected}`} nodes={nodes} edges={edges} fitView nodesDraggable={false} nodesConnectable={false} colorMode={theme} onNodeClick={(_,node)=>{const n=visual.nodes.find((n)=>n.nodeId===node.id);const a=n?.evidenceAnchorIds[0]?anchorsById.get(n.evidenceAnchorIds[0]):undefined;if(a)onOpen(a);}}><Background gap={20}/><Controls showInteractive={false}/></ReactFlow></div></section>;
}
