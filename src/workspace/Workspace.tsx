import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Badge, Button, Dialog, Flex, IconButton, Tabs, TextField, Theme, Tooltip, useThemeContext } from '@radix-ui/themes';
import { Group, Panel, Separator } from 'react-resizable-panels';
import { Background, BaseEdge, Controls, EdgeLabelRenderer, getSmoothStepPath, MarkerType, ReactFlow, type Edge, type EdgeProps, type Node } from '@xyflow/react';
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, BookOpen, ChevronRight, Code2, FileCode2, Files, GitBranch, GitCommitHorizontal, History, LoaderCircle, MessageSquare, Network, Paperclip, Pencil, Play, Search, ShieldCheck, Square, X } from 'lucide-react';
import { CodePane } from '../design/CodePane';
import { FileFlow } from '../FileFlow';
import { InlineCodeText } from '../InlineCodeText';
import { VisualStory } from '../VisualStory';
import { ReaderProgressContext } from '../lib/ReaderProgressContext';
import { sourceAnchorForVisualNode, sourceValuesByVisualNode } from '../lib/visualStory';
import { cleanFlowViewport, FLOW_DIAGRAM_LAYOUT_VERSION, sameSection, type DiagramProgress, type FlowViewport, type StoryProgress } from '../lib/readerProgress';
import { viewportClipsNodeBounds, type FlowViewportInstance } from '../lib/flowViewport';
import { buildInventory } from '../lib/inventory';
import { isNativeRuntime, listChapterRevisions, readSnapshotSource, type ChapterRevision, type ChapterSourceAnchor, type SnapshotManifest, type SnapshotSource, type SnapshotIndex, type HotspotDestination } from '../lib/commands';
import { projectProvenRelationshipGraph } from '../lib/atlas';
import { inspectInteractiveVisual } from '../lib/visuals';
import type { AtlasHotspotSelection, SourceLineRange, Workspace as WorkspaceId } from '../lib/navigation';
import { ancestors, canContinueRepositoryGuide, contextualGuideAction, discardUnstartedGuide, focusAnswerSection, historyEntry, isRepositoryGuideIntent, questionProgressText, repositoryGuideCoverageLabel, repositoryGuideOmissionQuestion, repositoryGuideOmissionSummary, repositoryGuideTimeoutRecovery, chooseFlowRevision, rankSourcePaths, presentationOwner, explorationSummaryCounts, rememberCodePane, rememberFileFlow, rememberSourceContext, rememberPresentation, rememberScroll, rememberSection, sectionPosition, workspaceAnswerLifecycleBadge, type AnswerSummary, type Context, type Exploration, type ReadingPosition, type Turn, type View } from './model';
import { advanceRepositoryGuide, appendQuestion, appendRepositoryGuide, authorizeWorkspace, ensureWorkspaceIndex, prepareWorkspaceConversation, revokeWorkspace, restoreWorkspaceAnswers, runQuestion, runRepositoryGuide, stopQuestion, updateWorkspace, useConversationAvailability, useWorkspace, watchWorkspaceAnswers } from './store';
import { useConnectionState } from '../lib/connectionStore';
import type { WorkspaceAnswerRestoration } from '../lib/notificationSummary';
import { errorMessageOr, explanationErrorMessage, explanationFailureMessage } from '../lib/errorMessage';
import { createChapterHistory, type ChapterHistory } from '../lib/chapterHistory';
import { featuredGuideTurnId, groupUnavailablePublishedSections, guideSlideIndex, questionSummaryText, lastAvailableAnswerItemIndex, openWorkspaceExploration, openWorkspaceQuestion, savedAnswerAvailability, shouldSequencePublishedAnswer, visibleConversationTurns } from './outcomes';
import { listWorkspaceCommits } from '../lib/commands';
import { largeFileExplanationContexts } from '../lib/largeFileExplanation';
import { answerReadingPosition, answerRevisionOptions, rememberAnswerVersion, resolveAnswerRevision } from './answerVersions';
import type { GitCommit } from '../lib/gitHistory';
import { searchCapturedSource, searchExploration, searchLimitMessage, searchResultsMessage, shouldOfferExploreWithAI, workspaceSearchResults, type CodeSearchMode } from './codeSearch';
import { linearFlowSteps, unlinkedFlowNodes } from './flowPresentation';
import { layoutFlowEdgeLabels, layoutFlowNodes, type FlowPoint } from './flowLayout';
import { commitHistoryEmptyMessage, nextFocusableCommitIndex } from './gitCommitNavigation';
import { GitHistoryEmptyContext } from './gitHistoryContext';
import { readOnlyDiagramAriaLabelConfig } from '../lib/readOnlyDiagramAccessibility';
import { readGenerationBackend } from '../lib/generationPreferences';
import { WorkspaceExplorer } from './WorkspaceExplorer';
import './workspace.css';

interface Props {
  manifest: SnapshotManifest; projectName:string; workspace:WorkspaceId; routePath?:string; routeRange?:SourceLineRange; routeSelection?:AtlasHotspotSelection;
  comparisonRefs?: { baseRef?:string; headRef?:string };
  pendingLargeFileExplanation?: { id:string; projectId:string; snapshotId:string; targetPath:string; paths:string[]; question:string } | null;
  onPendingLargeFileExplanationConsumed?:(id:string)=>void;
  children:ReactNode; onWorkspace:(workspace:WorkspaceId)=>void; onSettings:()=>void;
  onReview:(revision:ChapterRevision)=>void; onRead:(revision:ChapterRevision)=>void;
  onCompare:(base:string,head:string)=>void; onRememberFile:(path:string)=>void;
  onAnswerRestoration:(state:WorkspaceAnswerRestoration)=>void;
  conversationAvailability?:'ready'|'unavailable';
}
const label = (c:Context) => `${c.path.split('/').at(-1)}${c.from ? `:${c.from}${c.to !== c.from ? `–${c.to}` : ''}` : ''}`;

/** The production workspace consumes native snapshots and revision references, never example data. */
export function WorkspaceFrame({manifest,...props}:Omit<Props,'manifest'>&{manifest:SnapshotManifest|null}) { return manifest ? <SavedWorkspace key={`${manifest.summary.projectId}:${manifest.summary.snapshotId}`} {...props} manifest={manifest}/> : <>{props.children}</>; }

function SavedWorkspace(props:Props){
  const {projectId,snapshotId}=props.manifest.summary;
  const availability=useConversationAvailability(projectId,snapshotId);
  useEffect(()=>{
    let active=true;
    if(isNativeRuntime()){
      props.onAnswerRestoration({projectId,snapshotId,availability:'loading'});
      void prepareWorkspaceConversation(projectId,snapshotId).catch(()=>{if(active)props.onAnswerRestoration({projectId,snapshotId,availability:'unavailable'});});
    }
    return()=>{active=false;};
  },[projectId,snapshotId,props.onAnswerRestoration]);
  return availability==='loading'?<div className="dw-source-empty" role="status"><LoaderCircle size={18}/><p>Opening your workspace…</p></div>
    :<DeveloperWorkspace {...props} conversationAvailability={availability}/>;
}

export function DeveloperWorkspace(props:Props) {
  const {manifest,projectName,workspace,routePath,routeRange,routeSelection,children,onWorkspace,onSettings,onReview,onRead,onCompare,onRememberFile,onAnswerRestoration}=props;
  const {appearance:themeAppearance}=useThemeContext();
  const appearance=themeAppearance==='light'?'light':'dark';
  const {projectId,snapshotId} = manifest.summary;
  const conversationUnavailable=props.conversationAvailability==='unavailable';
  const state = useWorkspace(projectId,snapshotId);
  const connection=useConnectionState();
  const [generationBackend,setGenerationBackend]=useState(()=>readGenerationBackend());
  useEffect(()=>{
    const refresh=()=>{
      const selected=readGenerationBackend();
      setGenerationBackend(selected);
      setConnectionNotice(null);
    };
    window.addEventListener('code-atlas:connection-changed',refresh);
    return()=>window.removeEventListener('code-atlas:connection-changed',refresh);
  },[]);
  const current = state.explorations.find((e)=>e.id===state.currentId)!;
  const path = ancestors(state);
  const rows = useMemo(()=>buildInventory(manifest).filter((r)=>r.kind==='file' && r.disposition==='captured' && !['binary','invalidEncoding'].includes(r.contentKind) && !r.categories.includes('secret')), [manifest]);
  const sourcePaths = useMemo(()=>rankSourcePaths(rows.map((r)=>r.path)),[rows]);
  const [activity,setActivity] = useState<'files'|'search'|'changes'|'history'>('files');
  useEffect(()=>{
    if(workspace!=='changes'&&activity==='changes')setActivity('files');
  },[workspace,activity]);
  const [fileFilter,setFileFilter] = useState('');
  const [repositorySearch,setRepositorySearch] = useState('');
  const [repositoryRegex,setRepositoryRegex] = useState(false);
  const [sourceHits,setSourceHits] = useState<{path:string;line:number;text:string}[]>([]);
  const [searchLimited,setSearchLimited] = useState(false);
  const [searchNextOffset,setSearchNextOffset] = useState<number|null>(null);
  const [searchOffset,setSearchOffset] = useState(0);
  const [searchBusy,setSearchBusy] = useState(false);
  const [searchError,setSearchError] = useState<string|null>(null);
  const [openFolders,setOpenFolders] = useState<Set<string>>(()=>new Set(['src','app','lib','Sources']));
  const [query,setQuery] = useState('');
  const [attachments,setAttachments] = useState<Context[]>([]);
  const [narrowPane,setNarrowPane] = useState<'explanation'|'source'|'explorer'>('explanation');
  const codeOpen = current.codeOpen ?? true;
  const setCodeOpen = (open:boolean) => {
    setNarrowPane(open?'source':'explanation');
    updateWorkspace(projectId,snapshotId,(s)=>rememberCodePane(s,current.id,open));
  };
  const [source,setSource] = useState<SnapshotSource|null>(null);
  const [sourceError,setSourceError] = useState<string|null>(null);
  const [sourceBusy,setSourceBusy] = useState(false);
  const [reveal,setReveal] = useState<Context|null>(null);
  const [chapters,setChapters] = useState<ChapterRevision[]>([]);
  const [chaptersLoading,setChaptersLoading] = useState(true);
  const availableRevisionIds = useMemo(()=>new Set(chapters.map((chapter)=>chapter.revisionId)),[chapters]);
  const visibleTurns = visibleConversationTurns(state, current, availableRevisionIds, chaptersLoading);
  const featuredTurnId = featuredGuideTurnId(visibleTurns, current.focusedTurnId, availableRevisionIds);
  const [chapterError,setChapterError] = useState<string|null>(null);
  const [index,setIndex] = useState<SnapshotIndex|null>(null);
  const [indexBusy,setIndexBusy] = useState(true);
  const [indexError,setIndexError] = useState<string|null>(null);
  const [commits,setCommits] = useState<GitCommit[]>([]);
  const [focusedCommitId,setFocusedCommitId] = useState<string|null>(null);
  const commitButtons = useRef(new Map<string,HTMLButtonElement>());
  const defaultComparedHead = useRef<string|null>(null);
  const [gitError,setGitError] = useState<string|null>(null);
  const [gitBusy,setGitBusy] = useState(false);
  const [gitLoaded,setGitLoaded] = useState(false);
  const [gitStopped,setGitStopped] = useState(false);
  const [gitReload,setGitReload] = useState(0);
  const gitRequest = useRef<AbortController|null>(null);
  const [consentTurn,setConsentTurn] = useState<string|null>(null);
  const [guideConsent,setGuideConsent] = useState<{id:string;resume:boolean}|null>(null);
  const [consentBusy,setConsentBusy] = useState(false);
  const [consentError,setConsentError] = useState<string|null>(null);
  const [connectionNotice,setConnectionNotice] = useState<string|null>(null);
  const [sharingOpen,setSharingOpen] = useState(false);
  const [searchFocused,setSearchFocused] = useState(false);
  const handledLargeFileExplanation = useRef<string|null>(null);
  const requestedConnectionForLargeFileExplanation = useRef<string|null>(null);
  const composer = useRef<HTMLTextAreaElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const explorer = useRef<HTMLDivElement>(null);
  const answerScroll = useRef<HTMLDivElement>(null);
  const chapterRefresh = useRef<ChapterHistory|null>(null);
  const restoringScroll = useRef(true);
  const tool = ['changes','walkthrough','chapter-studio'].includes(workspace);
  const firstFocusableCommitId = commits.find((commit)=>commit.parents.length>0)?.oid ?? null;
  const commitHistoryEmpty = commitHistoryEmptyMessage(commits,{loaded:gitLoaded,loading:gitBusy,stopped:gitStopped,error:gitError});
  const rovingCommitId = commits.some((commit)=>commit.oid===focusedCommitId&&commit.parents.length>0)
    ? focusedCommitId : firstFocusableCommitId;
  const showChanges = activity==='changes'||workspace==='changes';
  const returnToGuide = (pane:'explorer'|'explanation') => {
    // Changes replaces the reading workspace. Restore its matching activity
    // selection as well, so the activity rail and sidebar agree with the Guide.
    if (workspace==='changes' || activity==='changes') setActivity('files');
    setNarrowPane(pane);
    onWorkspace('guide-plan');
  };
  const update = (fn:(s:typeof state)=>typeof state) => updateWorkspace(projectId,snapshotId,fn);
  const focusTurn = (turnId:string) => update((s)=>({...s,explorations:s.explorations.map(e=>e.id===current.id?{...e,focusedTurnId:turnId}:e)}));
  const focusAnswer = (turnId:string,revisionId:string,sectionId:string) => {
    update((s)=>focusAnswerSection(s,current.id,turnId,{revisionId,sectionId}));
    requestAnimationFrame(()=>{
      const target=Array.from(answerScroll.current?.querySelectorAll<HTMLElement>('[data-question-id]')??[])
        .find((node)=>node.dataset.questionId===turnId);
      target?.scrollIntoView({block:'start'});
    });
  };
  const go = (id:string) => { update((s)=>openWorkspaceExploration(s,id)); setNarrowPane('explanation');onWorkspace('guide-plan'); };
  const setView = (view:View) => {setNarrowPane('explanation');update((s)=>({...s,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,view}:e)}));};
  const requireCodexConnection=()=>{
    if(generationBackend!=='codex'){
      const selected=generationBackend==='api'?'Direct API':generationBackend==='handoff'?'manual agent handoff':'Claude Code';
      setConnectionNotice(`Workspace answers currently use Codex. Your ${selected} connection is saved; switch once in Connections to continue. Your question stays here.`);
      return false;
    }
    if(!connection.status?.authenticated){
      setConnectionNotice('Connect Codex in Connections to continue. Your question stays here.');
      onSettings();
      return false;
    }
    setConnectionNotice(null);
    return true;
  };
  const saveSection = (turnId:string,view:'guide'|'flow',revisionId:string,sectionId:string) => update((s)=>rememberSection(s,current.id,turnId,view,{revisionId,sectionId}));
  const savePresentation = (turnId:string,kind:'stories'|'diagrams',progress:StoryProgress|DiagramProgress) => update((s)=>rememberPresentation(s,current.id,turnId,kind,progress));
  const selectVersion = (turnId:string,originalRevisionId:string,revisionId:string) => update(s=>rememberAnswerVersion(s,current.id,turnId,originalRevisionId,revisionId,chapters));
  const refreshChapters = () => chapterRefresh.current?.refresh();
  useEffect(()=>{
    if(!isNativeRuntime()){setChaptersLoading(false);return;}
    setChaptersLoading(true);
    const queue=createChapterHistory({
      projectId,
      load:()=>listChapterRevisions(projectId,snapshotId),
      onValue:(result)=>{setChapters(result);setChapterError(null);},
      onLoaded:()=>setChaptersLoading(false),
      onError:(failure)=>{setChapterError(errorMessageOr(failure,'Saved explanations could not be loaded.'));setChaptersLoading(false);},
      onIdle:()=>setChaptersLoading(false),
    });
    chapterRefresh.current=queue;
    void queue.refresh();
    const changed=(event:Event)=>{
      const detail=(event as CustomEvent<{projectId?:string;revision?:ChapterRevision}>).detail;
      if(detail?.projectId&&detail.projectId!==projectId)return;
      const revision=detail?.revision;
      if(revision?.projectId===projectId&&revision.snapshotId===snapshotId&&revision.revisionId){
        setChapterError(null);
        setChaptersLoading(false);
        queue.record(revision);
      }else void queue.refresh();
    };
    window.addEventListener('code-atlas:chapters-changed',changed);
    return()=>{queue.dispose();if(chapterRefresh.current===queue)chapterRefresh.current=null;window.removeEventListener('code-atlas:chapters-changed',changed);};
  },[projectId,snapshotId]);
  useEffect(()=>{
    let closed=false;let unlisten:(()=>void)|undefined;
    if(isNativeRuntime()) {
      onAnswerRestoration({projectId,snapshotId,availability:'loading'});
      void watchWorkspaceAnswers(projectId,snapshotId,(availability)=>{
        if(!closed)onAnswerRestoration({projectId,snapshotId,availability});
      }).then((stop)=>{
        if(closed){stop();return;}
        unlisten=stop;
        // The scoped observer owns initial and background availability. A
        // late initial promise must not overwrite a newer refresh status.
        void restoreWorkspaceAnswers(projectId,snapshotId).catch(()=>{});
      }).catch(()=>{if(!closed){
        onAnswerRestoration({projectId,snapshotId,availability:'unavailable'});
      }});
    }
    return()=>{closed=true;unlisten?.();};
  },[projectId,snapshotId,onAnswerRestoration]);
  useEffect(()=>{let active=true; if(isNativeRuntime()) void ensureWorkspaceIndex(projectId,snapshotId).then((value)=>{if(active){setIndex(value);setIndexError(null);}}).catch((failure)=>{if(active)setIndexError(errorMessageOr(failure,'Code relationships could not be loaded.'));}).finally(()=>{if(active)setIndexBusy(false);}); return()=>{active=false;};},[projectId,snapshotId]);
  useEffect(()=>{if(!state.file && sourcePaths.length) update((s)=>({...s,file:sourcePaths[0],explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,file:sourcePaths[0]}:e)}));},[state.file,sourcePaths]);
  useEffect(()=>{if(routePath && sourcePaths.includes(routePath)){const context={path:routePath,from:routeRange?.startLine,to:routeRange?.endLine,sha256:routeRange?.sourceSha256};update((s)=>({...s,file:routePath,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,file:routePath,reveal:context,codeOpen:true}:e)}));setReveal(context);}},[routePath,routeRange]);
  useEffect(()=>{
    setSource(null);setSourceError(null);if(!state.file||!isNativeRuntime()) return;
    let active=true;setSourceBusy(true);
    void readSnapshotSource(projectId,snapshotId,state.file).then((value)=>{if(active){setSource(value);setSourceBusy(false);}}).catch((failure)=>{if(active){setSourceError(errorMessageOr(failure,'This captured file could not be opened.'));setSourceBusy(false);}});
    return()=>{active=false;};
  },[projectId,snapshotId,state.file]);
  useEffect(()=>{setReveal(current.reveal??null);},[current.id,current.reveal]);
  useLayoutEffect(()=>{
    // Ignore layout-generated scroll events until the saved answer has loaded.
    // Capture the owner/view here so a late event cannot move another exploration.
    restoringScroll.current=true;
    if(chaptersLoading||tool||!answerScroll.current)return;
    let release=0;
    const restore=requestAnimationFrame(()=>{
      // `focusedTurnId` is durable and identifies the answer the reader last
      // opened. `revealTurnId` is a one-shot History navigation request that
      // may also move keyboard focus; after restart, restore reading without
      // taking focus away from the user's current control.
      const restoreTurnId=current.revealTurnId??current.focusedTurnId;
      const target = current.view === 'guide' && restoreTurnId
        ? Array.from(answerScroll.current?.querySelectorAll<HTMLElement>('[data-question-id]') ?? []).find((node)=>node.dataset.questionId===restoreTurnId) : null;
      if (target) {
        target.scrollIntoView({block:'start'});
        if(current.revealTurnId)target.focus({preventScroll:true});
        const top=answerScroll.current?.scrollTop??0;
        update((s)=>({...s,explorations:s.explorations.map((e)=>{
          if(e.id!==current.id)return e;
          const clearReveal=!!current.revealTurnId&&e.revealTurnId===current.revealTurnId;
          if(!clearReveal&&e.scrollPositions?.guide===top)return e;
          return {...e,...(clearReveal?{revealTurnId:undefined}:{}),scrollPositions:{...e.scrollPositions,guide:top}};
        })}));
      } else answerScroll.current?.scrollTo({top:current.scrollPositions?.[current.view]??0});
      release=requestAnimationFrame(()=>{restoringScroll.current=false;});
    });
    return()=>{cancelAnimationFrame(restore);cancelAnimationFrame(release);};
  },[current.id,current.view,current.revealTurnId,current.focusedTurnId,chaptersLoading,tool]);
  useEffect(()=>{
    if(!isNativeRuntime()||!showChanges)return;
    let active=true;const controller=new AbortController();gitRequest.current=controller;
    setGitBusy(true);setGitLoaded(false);setGitStopped(false);setGitError(null);
    void listWorkspaceCommits(projectId,controller.signal).then((value)=>{if(active)setCommits(value);}).catch((failure)=>{
      if(active&&!controller.signal.aborted)setGitError(errorMessageOr(failure,'Git history is unavailable.'));
    }).finally(()=>{if(active){setGitBusy(false);setGitLoaded(true);}if(gitRequest.current===controller)gitRequest.current=null;});
    return()=>{active=false;controller.abort();if(gitRequest.current===controller)gitRequest.current=null;};
  },[projectId,showChanges,gitReload]);
  useEffect(()=>{
    if(workspace!=='changes'||gitBusy||gitRequest.current||props.comparisonRefs?.baseRef?.trim()||props.comparisonRefs?.headRef?.trim())return;
    const head=commits.find((commit)=>commit.parents[0]);
    const comparisonKey=`${projectId}:${head?.oid??''}`;
    if(!head||defaultComparedHead.current===comparisonKey)return;
    defaultComparedHead.current=comparisonKey;
    onCompare(head.parents[0],head.oid);
  },[projectId,workspace,gitBusy,commits,props.comparisonRefs?.baseRef,props.comparisonRefs?.headRef,onCompare]);
  function openFile(context:Context) { update((s)=>rememberSourceContext(s,s.currentId,context));setReveal(context);setCodeOpen(true);setNarrowPane('source');onRememberFile(context.path);setSearchFocused(false); }
  function focusComposer(){setNarrowPane('explanation');onWorkspace('guide-plan');requestAnimationFrame(()=>composer.current?.focus());}
  function showView(view:View){setView(view);setNarrowPane('explanation');onWorkspace('guide-plan');}
  function attach(context:Context) {setAttachments((all)=>all.some((c)=>c.path===context.path&&c.from===context.from&&c.to===context.to)?all:[...all,context]);focusComposer();}
  function startRepositoryGuide(question:string,parentTurn?:string) {
    if(conversationUnavailable)return;
    if(!requireCodexConnection())return;
    const id=appendRepositoryGuide(projectId,snapshotId,question,parentTurn);
    setQuery('');setAttachments([]);setSearchFocused(false);setNarrowPane('explanation');onWorkspace('guide-plan');
    if(state.grantId)void runRepositoryGuide(projectId,snapshotId,id,state.grantId);
    else {setGuideConsent({id,resume:false});setConsentError(null);}
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  }
  function ask(question:string,view:View=current.view==='flow'?'flow':'guide',context=attachments,parentTurn?:string){
    if(conversationUnavailable)return;
    const clean=question.replace(/[\r\n\t]+/g,' ').trim();if(!clean){setView(view);focusComposer();return;}
    if(!requireCodexConnection())return;
    if(isRepositoryGuideIntent(clean)){startRepositoryGuide(clean,parentTurn);return;}
    const id=appendQuestion(projectId,snapshotId,clean,context,view,parentTurn);setQuery('');setAttachments([]);setSearchFocused(false);setNarrowPane('explanation');onWorkspace('guide-plan');
    if(state.grantId) void runQuestion(projectId,snapshotId,id,state.grantId);else{setConsentError(null);setConsentTurn(id);}
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  }
  function explain(context:Context){const selected={...context,sha256:source?.path===context.path?source.sha256:context.sha256};openFile(selected);ask(`Explain what ${label(selected)} does and how it works.`,current.view==='flow'?'flow':'guide',[selected]);}
  useEffect(()=>{
    const pending=props.pendingLargeFileExplanation;
    if(conversationUnavailable||!pending||props.workspace!=='guide-plan'||pending.projectId!==projectId||pending.snapshotId!==snapshotId
      ||handledLargeFileExplanation.current===pending.id)return;
    if(generationBackend!=='codex'){
      setConnectionNotice('This file explanation needs Codex. Switch the saved connection once in Connections to continue.');
      return;
    }
    if(!connection.status?.authenticated){
      setConnectionNotice('Connect Codex in Connections to continue this file explanation.');
      if(requestedConnectionForLargeFileExplanation.current!==pending.id){
        requestedConnectionForLargeFileExplanation.current=pending.id;
        props.onSettings();
      }
      return;
    }
    setConnectionNotice(null);
    const contexts=largeFileExplanationContexts(pending,props.manifest.files);
    if(!contexts.some((context)=>context.path===pending.targetPath)){
      handledLargeFileExplanation.current=pending.id;
      setConsentError('This file is not available as readable text in the selected snapshot.');
      props.onPendingLargeFileExplanationConsumed?.(pending.id);
      return;
    }
    handledLargeFileExplanation.current=pending.id;
    requestedConnectionForLargeFileExplanation.current=null;
    const turnId=appendQuestion(projectId,snapshotId,pending.question,contexts,'guide');
    setQuery('');setAttachments([]);setSearchFocused(false);setNarrowPane('explanation');
    if(state.grantId)void runQuestion(projectId,snapshotId,turnId,state.grantId);
    else{setConsentError(null);setConsentTurn(turnId);}
    props.onPendingLargeFileExplanationConsumed?.(pending.id);
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  },[props.pendingLargeFileExplanation,props.workspace,props.manifest.files,props.onSettings,
    props.onPendingLargeFileExplanationConsumed,projectId,snapshotId,generationBackend,connection.status?.authenticated,state.grantId,conversationUnavailable]);
  function retryTurn(turn:Turn) {
    if(conversationUnavailable)return;
    if(!requireCodexConnection())return;
    // The timeout recovery UI promises a new incremental guide. Check it
    // before the saved guideId branch, which would otherwise resume the same
    // legacy operation and risk repeating its old deadline failure.
    if(repositoryGuideTimeoutRecovery(turn)){startRepositoryGuide(turn.question);return;}
    if(turn.guideId){
      if(canContinueRepositoryGuide(turn.guide)&&state.grantId)void runRepositoryGuide(projectId,snapshotId,turn.id,state.grantId,true);
      else if(canContinueRepositoryGuide(turn.guide))setGuideConsent({id:turn.id,resume:true});
      else setGuideConsent({id:appendRepositoryGuide(projectId,snapshotId,turn.question),resume:false});
      setConsentError(null);return;
    }
    if(state.grantId)void runQuestion(projectId,snapshotId,turn.id,state.grantId);else setConsentTurn(turn.id);
  }
  function branch(turn:Turn,question:string,context:Context[]){ask(question,current.view==='flow'?'flow':'guide',context,turn.id);}
  function exploreGuideOmissions(turn:Turn){
    const question=repositoryGuideOmissionQuestion(turn.guide?.omissions??[]);
    if(question)startRepositoryGuide(question,turn.id);
  }
  useEffect(()=>{
    const find=()=>{setActivity('search');setNarrowPane('explorer');onWorkspace('guide-plan');requestAnimationFrame(()=>search.current?.focus());};
    const listener=(event:KeyboardEvent)=>{
      if(document.querySelector('[role="dialog"][aria-modal="true"]'))return;
      const mod=event.metaKey||event.ctrlKey; const k=event.key.toLowerCase();
      if(mod&&(k==='p'||k==='k'||(event.shiftKey&&k==='f'))){event.preventDefault();event.stopImmediatePropagation();find();}
      if(mod&&event.shiftKey&&k==='e'){event.preventDefault();event.stopImmediatePropagation();setActivity('files');setNarrowPane('explorer');onWorkspace('guide-plan');requestAnimationFrame(()=>explorer.current?.focus());}
      if(mod&&k==='l'){event.preventDefault();event.stopImmediatePropagation();focusComposer();}
      if(mod&&['1','2','3'].includes(k)){event.preventDefault();event.stopImmediatePropagation();setView((['guide','flow','history'] as const)[Number(k)-1]);onWorkspace('guide-plan');}
      if(event.altKey&&k==='arrowleft'&&current.parentId){event.preventDefault();go(current.parentId);}
    };
    window.addEventListener('keydown',listener,true);window.addEventListener('code-atlas:search',find);return()=>{window.removeEventListener('keydown',listener,true);window.removeEventListener('code-atlas:search',find);};
  },[current.id,current.parentId]);
  const term=(activity==='search'?repositorySearch:query).trim();
  const searchMode:CodeSearchMode=activity==='search'&&repositoryRegex?'regex':'code';
  useEffect(()=>searchCapturedSource({projectId,snapshotId,query:term,mode:searchMode,native:isNativeRuntime(),offset:searchOffset},
    {hits:(hits,append)=>setSourceHits((previous)=>{
      if(!append)return hits;
      const seen=new Set(previous.map((hit)=>JSON.stringify([hit.path,hit.line,hit.text])));
      return [...previous,...hits.filter((hit)=>{const key=JSON.stringify([hit.path,hit.line,hit.text]);if(seen.has(key))return false;seen.add(key);return true;})];
    }),limited:setSearchLimited,nextOffset:setSearchNextOffset,error:setSearchError,busy:setSearchBusy},{invoke}),[term,searchMode,projectId,snapshotId,searchOffset]);
  const results=useMemo(()=>workspaceSearchResults({query:term,mode:searchMode,files:rows,symbols:index?.symbols??[],source,captured:sourceHits}),[term,searchMode,rows,index,source,sourceHits]);
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
  const flowOriginals=latestTurn?.publishedSections?.length?latestTurn.publishedSections.map(s=>chapters.find(r=>r.revisionId===s.revisionId)).filter((r):r is ChapterRevision=>!!r):chapters.filter(r=>r.revisionId===latestTurn?.revisionId);
  const flowRevisions=latestTurn?flowOriginals.map(r=>resolveAnswerRevision(r,latestTurn,chapters)):[];
  const latestRevision=chooseFlowRevision(flowRevisions,latestTurn?.reading?.flow?.revisionId,
    (revision)=>revision.sections.some((section)=>inspectInteractiveVisual(revision,section).status==='available'));
  const activityButtons=[{id:'files',label:'Explorer',icon:Files},{id:'search',label:'Search',icon:Search},{id:'changes',label:'Changes',icon:GitBranch},{id:'history',label:'Explorations',icon:History}] as const;
  const explorationCounts=explorationSummaryCounts(state.explorations);
  const renderExplorations=(parent:string|null,depth=0):ReactNode=>state.explorations.filter((e)=>e.parentId===parent).map((e)=>{
    const nested=renderExplorations(e.id,depth+1);
    const title=e.id==='root'?projectName:e.title;
    const displayTitle=e.id==='root'?projectName:questionSummaryText(title);
    const counts=explorationCounts.get(e.id)!;
    const questionCount=`${counts.questions} ${counts.questions===1?'question':'questions'}`;
    const childCount=`${counts.children} ${counts.children===1?'child exploration':'child explorations'}`;
    return <li key={e.id}><button title={`${displayTitle} · ${questionCount} · ${childCount}`} aria-label={`${displayTitle}, ${questionCount}, ${childCount}`} aria-current={e.id===current.id?'page':undefined} className={e.id===current.id?'active':''} style={{paddingLeft:10+depth*12}} onClick={()=>go(e.id)}><MessageSquare size={13}/><span><InlineCodeText text={displayTitle}/></span><small title={`${questionCount} in this exploration`}>{counts.questions} Q</small>{counts.children>0&&<small className="dw-child-count" title={childCount}><GitBranch size={11} aria-hidden="true"/>{counts.children}</small>}</button><ul>{nested}</ul></li>;
  });
  const fromAnchor=(a:ChapterSourceAnchor):Context=>({path:a.path,sha256:a.sourceSha256,from:a.startLine,to:a.endLine});
  const sourcePane=source
    ? <CodePane path={source.path} contents={source.contents} theme={appearance} reveal={reveal?.sha256 && reveal.sha256!==source.sha256 ? null : reveal} canExplain={!conversationUnavailable} onExplain={explain} onAttach={(c)=>attach({...c,sha256:source.sha256})} onClose={()=>{setCodeOpen(false);setNarrowPane('explanation');}}/>
    : <div className="dw-source-empty">{sourceBusy?<><LoaderCircle size={18}/><p>Opening source…</p></>:sourceError?<p role="alert">{sourceError}</p>:<><Code2 size={24}/><p>Open a file to read its code.</p></>}</div>;
  return <ReaderProgressContext.Provider value={{
    story:(identity)=>{const owner=presentationOwner(current,identity.revisionId);return owner?{progress:owner.presentation?.stories.find(p=>sameSection(p,identity)),onProgressChange:(progress)=>savePresentation(owner.id,'stories',progress)}:undefined;},
    section:(revisionId)=>{const owner=presentationOwner(current,revisionId);return owner?{sectionId:answerReadingPosition(owner,'guide',revisionId)?.sectionId,onSectionChange:(sectionId)=>{if(chapters.some(r=>r.revisionId===revisionId&&r.sections.some(s=>s.sectionId===sectionId)))saveSection(owner.id,'guide',revisionId,sectionId);}}:undefined;},
  }}><Theme accentColor="blue" grayColor="slate" radius="medium" className={`developer-workspace dw-narrow-${narrowPane}`}>
    <nav className="dw-activity" aria-label="Repository tools">{activityButtons.map(({id,label:caption,icon:Icon})=><Tooltip content={caption} key={id}><button className={(workspace==='changes'?id==='changes':activity===id)?'active':''} aria-label={caption} aria-pressed={workspace==='changes'?id==='changes':activity===id} onClick={()=>{setActivity(id);setNarrowPane('explorer');if(id==='changes')onWorkspace('changes');else onWorkspace('guide-plan');if(id==='search')requestAnimationFrame(()=>search.current?.focus());}}><Icon size={21}/></button></Tooltip>)}<div className="dw-activity-bottom"><Tooltip content="Repository sharing"><button aria-label="Repository sharing" onClick={()=>setSharingOpen(true)}><ShieldCheck size={19}/></button></Tooltip></div></nav>
    <Group orientation="horizontal" className="dw-panels">
      <Panel id="repository-sidebar" defaultSize="220px" minSize="160px" maxSize="400px">
        <aside className="dw-sidebar" aria-label="Repository sidebar">
          <header><strong>{showChanges?'Source control':activity==='search'?'Search':activity==='history'?'Explorations':'Explorer'}</strong><Button className="dw-sidebar-back" size="1" variant="ghost" color="gray" aria-label="Back to explanation" onClick={()=>returnToGuide('explanation')}><ArrowLeft size={13}/>Back</Button></header>
          {showChanges?<><div className="dw-sidebar-section">Commit history <span>{commits.length ? `${commits.length} recent`:''}</span><Button size="1" variant="ghost" disabled={gitBusy} onClick={()=>setGitReload((value)=>value+1)}>Refresh</Button></div><div className="dw-commits" role="group" aria-label="Recent commits. Use Up and Down arrows to move between commits.">{gitBusy&&<p role="status">Loading commits… <Button size="1" variant="ghost" onClick={()=>{gitRequest.current?.abort();setGitBusy(false);setGitStopped(true);}}>Stop</Button></p>}{gitStopped&&<p role="status">Loading stopped.{commits.length>0?' Your previous history is still available.':''}</p>}{gitError&&<p role="alert">{gitError}</p>}{commitHistoryEmpty&&<p role="status">{commitHistoryEmpty}</p>}{commits.map((c)=><button key={c.oid} ref={(button)=>{if(button)commitButtons.current.set(c.oid,button);else commitButtons.current.delete(c.oid);}} className="dw-commit" disabled={!c.parents.length} tabIndex={c.oid===rovingCommitId?0:-1} title={c.parents.length?`${c.subject}\n${c.oid}`:`${c.subject} — initial commit; parent comparison unavailable`} onFocus={()=>setFocusedCommitId(c.oid)} onKeyDown={(event)=>{if(event.key!=='ArrowDown'&&event.key!=='ArrowUp')return;event.preventDefault();const direction=event.key==='ArrowDown'?1:-1;const currentIndex=commits.findIndex((commit)=>commit.oid===c.oid);const nextIndex=nextFocusableCommitIndex(commits,currentIndex,direction);const next=commits[nextIndex];if(next&&nextIndex!==currentIndex)commitButtons.current.get(next.oid)?.focus();}} onClick={()=>{if(!c.parents.length)return;onCompare(c.parents[0],c.oid);setNarrowPane('explanation');onWorkspace('changes');}}><span className={`dw-commit-line ${c.parents.length>1?'merge':''}`}><GitCommitHorizontal size={16}/></span><span><strong>{c.subject||'Untitled commit'}</strong><small>{c.author} · {c.oid.slice(0,7)}{c.parents.length>1?' · merge':!c.parents.length?' · initial commit':''}</small></span></button>)}</div></>:activity==='history'?<ul className="dw-explorations">{renderExplorations(null)}</ul>:<>
            <div className="dw-sidebar-search"><TextField.Root ref={search} size="1" value={activity==='search'?repositorySearch:fileFilter} placeholder={activity==='search'?(repositoryRegex?'Code pattern…':'Files, symbols, code…'):'Find a file…'} maxLength={activity==='search'?240:4096} aria-label={activity==='search'?'Search repository':'Filter files'} onChange={(e)=>{if(activity==='search'){setRepositorySearch(e.target.value);setSearchOffset(0);}else setFileFilter(e.target.value);}} onKeyDown={(event)=>{if(activity==='search'&&event.altKey&&event.code==='KeyR'){event.preventDefault();setRepositoryRegex(value=>!value);setSearchOffset(0);}}}><TextField.Slot><Search size={13}/></TextField.Slot>{activity==='search'&&<TextField.Slot side="right"><Tooltip content="Use regular expression (⌥R)"><IconButton type="button" size="1" variant="ghost" color={repositoryRegex?'blue':'gray'} aria-label="Use regular expression" aria-pressed={repositoryRegex} onClick={()=>{setRepositoryRegex(value=>!value);setSearchOffset(0);}}><span aria-hidden="true">.*</span></IconButton></Tooltip></TextField.Slot>}</TextField.Root></div>
            {activity==='search'?<div className="dw-search-results"><small>{repositoryRegex?'Captured code · regular expression':'Files, symbols and captured code'}</small>{searchBusy&&<p role="status">Searching…</p>}{repositorySearch.trim()&&!searchBusy&&!searchError&&results.length>0&&<p className="dw-search-count" role="status">{results.length>=80||(searchLimited&&searchNextOffset===null)?searchLimitMessage(results.length):searchResultsMessage(results.length,searchLimited&&searchNextOffset!==null)}</p>}{searchError&&<p role="status">{searchError}</p>}{repositorySearch&&!searchBusy&&!searchError&&results.length===0&&<p>No matching code in this search.</p>}{results.map((r,i)=><button key={i} onClick={()=>openFile(r)}><FileCode2 size={14}/><span><strong>{r.text}</strong><small>{r.path}{r.from?`:${r.from}`:''}</small></span></button>)}{searchLimited&&searchNextOffset!==null&&results.length<80&&<Button size="1" variant="soft" disabled={searchBusy} onClick={()=>setSearchOffset(searchNextOffset)}>Load more matches</Button>}{!conversationUnavailable&&shouldOfferExploreWithAI(repositorySearch,searchMode,searchBusy,searchError,results.length)&&<Button size="1" variant="soft" onClick={()=>{const intent=searchExploration(repositorySearch,searchMode,sourceHits);ask(intent.question,undefined,searchMode==='regex'?intent.context:attachments);}}><MessageSquare size={13}/>Explore with AI</Button>}</div>:<><div className="dw-sidebar-section">{projectName}</div><WorkspaceExplorer paths={sourcePaths} query={fileFilter} selectedPath={state.file} expanded={openFolders} onExpandedChange={setOpenFolders} onOpen={(path)=>openFile({path})} treeRef={explorer}/></>}
          </>}
        </aside>
      </Panel><Separator className="dw-separator" aria-label="Resize repository sidebar"/>
      <Panel id="reading-workspace" minSize="320px"><section className="dw-main" aria-label="Code exploration">
        <nav className="dw-path" aria-label="Exploration path"><IconButton size="1" variant="ghost" color="gray" aria-label="Back one exploration" disabled={!current.parentId} onClick={()=>current.parentId&&go(current.parentId)}><ArrowLeft size={14}/></IconButton>{path.map((e,i)=><span key={e.id}>{i>0&&<ChevronRight size={12}/>}<button aria-current={e.id===current.id?'page':undefined} onClick={()=>go(e.id)}>{e.id==='root'?projectName:e.title}</button></span>)}{path.length>1&&<Badge size="1" color="gray">Level {path.length}</Badge>}{tool&&<span className="dw-tool-label">{workspace==='changes'?'Changes':workspace==='chapter-studio'?'Edit explanation':'Player'}</span>}</nav>
        {conversationUnavailable&&<div className="dw-conversation-warning" role="status"><MessageSquare size={16}/><p><strong>Saved conversation unavailable.</strong> Your saved work has not been changed. You can browse and search this snapshot; new questions cannot be saved.</p></div>}
        <div className="dw-composer" onDragOver={(e)=>{if(!conversationUnavailable&&e.dataTransfer.types.includes('application/x-code-atlas-context'))e.preventDefault();}} onDrop={(e)=>{e.preventDefault();if(conversationUnavailable)return;try{const c=JSON.parse(e.dataTransfer.getData('application/x-code-atlas-context')) as Context;if(rows.some((r)=>r.path===c.path))attach(c);}catch{/* Ignore unrelated drops. */}}}>
          <Flex gap="2" mb="2"><Button size="1" variant="soft" onClick={()=>{if(conversationUnavailable){showView('guide');return;}const action=contextualGuideAction(query,attachments,!!current.parentId);if(action.kind==='repository-guide')startRepositoryGuide(action.intent);else ask(action.intent,'guide');}}><BookOpen size={14}/>Guide me</Button><Button size="1" variant="soft" color="gray" onClick={()=>{if(conversationUnavailable||!query.trim())showView('flow');else ask(query,'flow');}}><Network size={14}/>Show the flow</Button><IconButton className="dw-narrow-toggle" size="1" variant="ghost" color="gray" aria-label={narrowPane==='source'?'Show explanation':'Show source'} onClick={()=>{if(narrowPane==='source')setNarrowPane('explanation');else setCodeOpen(true);}}><Code2 size={15}/></IconButton></Flex>
          <div className="dw-input">{attachments.length>0&&<div className="dw-context">{attachments.map((c,i)=><Badge key={i} color="gray"><FileCode2 size={12}/>{label(c)}<button aria-label={`Remove ${label(c)}`} onClick={()=>setAttachments((all)=>all.filter((_,n)=>n!==i))}><X size={12}/></button></Badge>)}</div>}
            <textarea ref={composer} data-focus-key="workspace-composer" rows={1} value={query} maxLength={1000} disabled={conversationUnavailable} aria-label="Ask about this codebase" placeholder={conversationUnavailable?'Saved explanations are read-only for now':'Ask about this codebase'} onChange={(e)=>{e.target.setCustomValidity('');setQuery(e.target.value);}} onFocus={()=>setSearchFocused(true)} onKeyDown={(e)=>{if(e.key==='Escape')setSearchFocused(false);if(e.key==='Enter'&&!e.shiftKey&&!e.nativeEvent.isComposing){e.preventDefault();ask(query);}}}/>
            <Tooltip content="Attach open file"><IconButton size="1" variant="ghost" color="gray" disabled={!source||conversationUnavailable} aria-label="Attach open file" onClick={()=>source&&attach({path:source.path,sha256:source.sha256})}><Paperclip size={15}/></IconButton></Tooltip><IconButton size="1" aria-label="Send question" disabled={!query.trim()||conversationUnavailable} onClick={()=>ask(query)}><ArrowUp size={15}/></IconButton>
          </div>
          {connectionNotice&&<div className="dw-conversation-warning" role="status"><MessageSquare size={16}/><p>{connectionNotice}</p><Button size="1" variant="soft" onClick={onSettings}>Connections</Button></div>}
          {!conversationUnavailable&&searchFocused&&query.trim().length>1&&<div className="dw-suggestions"><small>{searchBusy?'Searching…':searchError?'Search unavailable':results.length?'In your code':'No matching code in this search'}</small>{results.slice(0,4).map((r,i)=><button key={i} onMouseDown={(e)=>e.preventDefault()} onClick={()=>openFile(r)}><FileCode2 size={13}/><span>{r.path}{r.from?`:${r.from}`:''}<small>{r.text}</small></span><ArrowRight size={13}/></button>)}{shouldOfferExploreWithAI(query,searchMode,searchBusy,searchError,results.length)&&<button onMouseDown={(e)=>e.preventDefault()} onClick={()=>ask(query)}><MessageSquare size={13}/>Explore with AI<ArrowRight size={13}/></button>}</div>}
        </div>
        {tool?<Group orientation="horizontal" className="dw-reading dw-tool-reading">
          {codeOpen&&<><Panel id="source" defaultSize="43%" minSize="220px">{sourcePane}</Panel><Separator className="dw-separator" aria-label="Resize source and explanation"/></>}
          <Panel id="explanations" minSize="280px"><div className="dw-tool-content"><Button className="dw-return" size="1" variant="ghost" onClick={()=>returnToGuide('explorer')}><ArrowLeft size={13}/>Back to exploration</Button><GitHistoryEmptyContext.Provider value={commitHistoryEmpty}>{children}</GitHistoryEmptyContext.Provider></div></Panel>
        </Group>:<Group orientation="horizontal" className="dw-reading">
          {codeOpen&&<><Panel id="source" defaultSize="43%" minSize="220px">{sourcePane}</Panel><Separator className="dw-separator" aria-label="Resize source and explanation"/></>}
          <Panel id="explanations" minSize="280px"><Tabs.Root className="dw-explanation" value={current.view} onValueChange={(value)=>setView(value as View)}><div className="dw-viewbar"><Tabs.List aria-label="Exploration views"><Tabs.Trigger value="guide"><BookOpen size={13}/>Guide</Tabs.Trigger><Tabs.Trigger value="flow"><Network size={13}/>Flow</Tabs.Trigger><Tabs.Trigger value="history"><MessageSquare size={13}/>Chat history</Tabs.Trigger></Tabs.List>{!codeOpen&&<IconButton size="1" variant="ghost" color="gray" aria-label="Show code pane" onClick={()=>setCodeOpen(true)}><Code2 size={15}/></IconButton>}</div>
            <div className="dw-answer-scroll" ref={answerScroll} onScroll={(event)=>{if(restoringScroll.current)return;const top=event.currentTarget.scrollTop;update((s)=>rememberScroll(s,current.id,current.view,top));}}>
              {chapterError&&<p className="dw-error" role="alert">{chapterError}<button onClick={()=>{void refreshChapters();}}>Retry</button></p>}
              <Tabs.Content value="guide">
                {visibleTurns.length===0?<section className="dw-welcome"><BookOpen size={25}/><h1>Get to know {projectName}</h1><p>{conversationUnavailable?'This session is read-only. Browse a file or search the captured source.':'Ask a question, or use Guide me for a project walkthrough. Open a file to follow its source.'}</p>{!conversationUnavailable&&source&&<button className="dw-text-action" onClick={()=>explain({path:source.path,sha256:source.sha256})}>Explain {source.path.split('/').at(-1)}</button>}</section>:visibleTurns.map((turn)=><Answer key={turn.id} chapters={chapters} turn={turn} compact={turn.id!==featuredTurnId} loading={chaptersLoading} revision={chapters.find((r)=>r.revisionId===turn.revisionId)} branches={state.explorations.filter((e)=>e.parentId===current.id&&e.originTurnId===turn.id)} onGo={go} onOpen={(a)=>openFile(fromAnchor(a))} onDestination={openDestination} onBranch={(question,context)=>branch(turn,question,context)} onExploreOmissions={exploreGuideOmissions} onStop={()=>stopQuestion(projectId,snapshotId,turn.id)} onRetry={()=>retryTurn(turn)} onReview={onReview} onFocus={(revisionId,sectionId)=>focusAnswer(turn.id,revisionId,sectionId)} onRead={(revision)=>{focusTurn(turn.id);onRead(revision);}} onSettings={onSettings} onSection={(revisionId,sectionId)=>saveSection(turn.id,'guide',revisionId,sectionId)} onStoryProgress={(progress)=>savePresentation(turn.id,'stories',progress)} onVersion={(originalId,revisionId)=>selectVersion(turn.id,originalId,revisionId)} onGuideSlideConsumed={turn.guideId?((revisionId)=>void advanceRepositoryGuide(projectId,snapshotId,turn.id,revisionId)):undefined}/>) }
              </Tabs.Content>
              <Tabs.Content value="flow">{flowRevisions.length>1&&<div className="dw-flow-choice"><label htmlFor="answer-flow-section">Explanation</label><select id="answer-flow-section" value={latestRevision?.revisionId??''} onChange={(event)=>{const revision=flowRevisions.find((r)=>r.revisionId===event.target.value);if(latestTurn&&revision?.sections[0])saveSection(latestTurn.id,'flow',revision.revisionId,revision.sections[0].sectionId);}}>{flowRevisions.map((revision)=><option key={revision.revisionId} value={revision.revisionId}>{revision.sections[0]?.title??revision.title}</option>)}</select></div>}{indexError&&!latestRevision&&<p role="status" className="dw-error">{indexError}</p>}{latestRevision?<EvidenceFlow revision={latestRevision} saved={latestTurn?answerReadingPosition(latestTurn,'flow',latestRevision.revisionId):undefined} progress={latestTurn?.presentation?.diagrams} onProgress={(progress)=>{if(latestTurn)savePresentation(latestTurn.id,'diagrams',progress);}} onSection={(sectionId)=>{if(latestTurn)saveSection(latestTurn.id,'flow',latestRevision.revisionId,sectionId);}} theme={appearance} onOpen={(a)=>openFile(fromAnchor(a))}/>:source?<FileFlow index={index} edges={graph?.edges??[]} loading={indexBusy} path={source.path} focus={reveal?.path===source.path&&(!reveal.sha256||reveal.sha256===source.sha256)?reveal:null} showAll={current.fileFlow?.path===source.path&&current.fileFlow.showAll} onShowAllChange={(showAll)=>update((s)=>rememberFileFlow(s,current.id,source.path,showAll))} onOpenFile={(p,_snapshot,range)=>openFile({path:p,from:range?.startLine,to:range?.endLine,sha256:range?.sourceSha256})}/>:<p>Open a file or ask a question to explore its flow.</p>}</Tabs.Content>
              <Tabs.Content value="history"><div className="dw-history">{path.map((e)=><section key={e.id}><button className="dw-history-level" onClick={()=>go(e.id)}><MessageSquare size={13}/>{e.id==='root'?projectName:<InlineCodeText text={e.title}/>}<small>Level {ancestors(state,e.id).length}</small></button>{e.turns.map((turn)=>{const entry=historyEntry(turn,new Set(chapters.map((r)=>r.revisionId)));return <article key={turn.id}><details className="dw-history-question"><summary><strong><InlineCodeText text={questionSummaryText(turn.question)}/></strong></summary><p><InlineCodeText text={turn.question}/></p></details><p><InlineCodeText text={entry.text}/></p><button className="dw-text-action" onClick={()=>{update((s)=>openWorkspaceQuestion(s,e.id,turn.id));setNarrowPane('explanation');onWorkspace('guide-plan');}}>{entry.action}<ArrowRight size={13}/></button>{turn.runIds?.length ? <AnswerDiagnostics project={projectId} snapshot={snapshotId} runId={turn.runIds.at(-1)!}/> : null}</article>;})}</section>)}{!path.some((e)=>e.turns.length>0)&&chapters.length===0&&<p>Your questions and explanations will appear here.</p>}{chapters.length>0&&<details className="dw-library"><summary>Saved explanations <span>{chapters.length}</span></summary>{chapters.slice().sort((a,b)=>b.createdAtUnixMs-a.createdAtUnixMs).map((r)=><button key={r.revisionId} onClick={()=>{
                  const id=crypto.randomUUID();update((s)=>({...s,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,turns:e.turns.some((t)=>t.revisionId===r.revisionId)?e.turns:[...e.turns,{id,question:r.title,context:[],status:'ready',revisionId:r.revisionId}]}:e)}));
                }}><BookOpen size={13}/><span>{r.title}</span><small>{r.lifecycle}</small></button>)}</details>}</div></Tabs.Content>
            </div>
          </Tabs.Root></Panel>
        </Group>}
      </section></Panel>
    </Group>
    <Dialog.Root open={!!consentTurn} onOpenChange={(open)=>{if(!open&&!consentBusy){if(consentTurn)stopQuestion(projectId,snapshotId,consentTurn);setConsentTurn(null);}}}><Dialog.Content maxWidth="520px"><Dialog.Title>Explore {projectName} with Codex</Dialog.Title><Dialog.Description size="2">Allow questions, indexed names, documentation hints and relevant captured code from this snapshot to be sent to OpenAI through your connected Codex account. Account quota applies.</Dialog.Description><p className="dw-permission-note">Codex searches and reads relevant portions of captured code. Each question can share up to 250 KB across 12 files; large files can be read in portions. Secret-like paths and excluded files remain blocked. This permission is limited to this snapshot and account. To revoke it, open Repository sharing and choose Revoke permission and stop questions.</p>{consentError&&<p role="alert" className="dw-error">{consentError}</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray" disabled={consentBusy}>Cancel</Button></Dialog.Close><Button disabled={consentBusy} onClick={()=>{if(!consentTurn)return;const id=consentTurn;setConsentBusy(true);setConsentError(null);void authorizeWorkspace(projectId,snapshotId).then((grant)=>{setConsentTurn(null);void runQuestion(projectId,snapshotId,id,grant);}).catch((failure)=>setConsentError(errorMessageOr(failure,'Connect Codex in the header, then try again.'))).finally(()=>setConsentBusy(false));}}>{consentBusy?'Connecting…':'Allow and explain'}</Button></Flex>{consentError&&<Button variant="ghost" onClick={()=>{if(consentTurn)stopQuestion(projectId,snapshotId,consentTurn);setConsentTurn(null);requestAnimationFrame(onSettings);}}>Open connection settings</Button>}</Dialog.Content></Dialog.Root>
    <Dialog.Root open={!!guideConsent} onOpenChange={(open)=>{if(!open&&!consentBusy){if(guideConsent&&!guideConsent.resume)update((state)=>discardUnstartedGuide(state,guideConsent.id));setGuideConsent(null);}}}><Dialog.Content maxWidth="520px"><Dialog.Title>{guideConsent?.resume?'Continue your guide':'Build a guide to '+projectName}</Dialog.Title><Dialog.Description size="2">A clear, illustrated guide will appear here, with each slide linked to the code behind it.</Dialog.Description><p className="dw-permission-note">The outline comes first. Read or play each slide as soon as it is ready while the next takes shape. You can stop at any time.</p><details className="dw-error-details"><summary>What will be shared</summary><p className="dw-permission-note">{guideConsent?.resume?'Only unfinished parts are generated; your saved outline and slides remain available.':'Code Atlas builds an outline and illustrated slides from this captured repository.'} Relevant captured code is sent to your connected Codex account, up to 250 KB across 12 files per slide. Account quota applies. Repository code is not run. Saved slides remain available if a later slide has a problem. Restarting the app will not continue work in the background.</p></details>{consentError&&<p role="alert" className="dw-error">{errorMessageOr(consentError,'This guide could not be started.')}</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray" disabled={consentBusy}>Cancel</Button></Dialog.Close><Button disabled={consentBusy} onClick={()=>{if(!guideConsent)return;const pending=guideConsent;setConsentBusy(true);setConsentError(null);void (state.grantId?Promise.resolve(state.grantId):authorizeWorkspace(projectId,snapshotId)).then((grant)=>{setGuideConsent(null);void runRepositoryGuide(projectId,snapshotId,pending.id,grant,pending.resume);}).catch((error)=>setConsentError(errorMessageOr(error,'Connect Codex in the header, then try again.'))).finally(()=>setConsentBusy(false));}}>{consentBusy?'Connecting…':guideConsent?.resume?'Continue guide':'Build guide'}</Button></Flex></Dialog.Content></Dialog.Root>
    <Dialog.Root open={sharingOpen} onOpenChange={setSharingOpen}><Dialog.Content maxWidth="460px"><Dialog.Title>Repository sharing</Dialog.Title><Dialog.Description size="2">{state.grantId?'Codex can explore captured code in this snapshot when you ask a question. Your live working copy is not included.':'Codex has no workspace sharing permission for this snapshot.'}</Dialog.Description>{state.grantId&&<Button mt="4" variant="soft" color="red" onClick={()=>{void revokeWorkspace(projectId,snapshotId).then(()=>setSharingOpen(false)).catch((failure)=>setConsentError(errorMessageOr(failure,'Permission could not be revoked.')));}}>Revoke permission and stop questions</Button>}{consentError&&<p role="alert">{consentError}</p>}<Flex justify="end" mt="4"><Dialog.Close><Button variant="soft" color="gray">Done</Button></Dialog.Close></Flex></Dialog.Content></Dialog.Root>
  </Theme></ReaderProgressContext.Provider>;
}

interface AnswerProps {chapters?:ChapterRevision[];hideQuestion?:boolean;turn:Turn; loading?:boolean; revision?:ChapterRevision; branches:Exploration[]; compact?:boolean;onGo:(id:string)=>void;onOpen:(a:ChapterSourceAnchor)=>void;onBranch:(question:string,context:Context[])=>void;onExploreOmissions:(turn:Turn)=>void;onStop:()=>void;onRetry:()=>void;onReview:(r:ChapterRevision)=>void;onFocus:(revisionId:string,sectionId:string)=>void;onRead:(r:ChapterRevision)=>void;onSettings:()=>void;onDestination:(d:HotspotDestination)=>void;onSection:(revisionId:string,sectionId:string)=>void;onStoryProgress:(progress:StoryProgress)=>void;onVersion:(originalRevisionId:string,revisionId:string)=>void;onGuideSlideConsumed?:(revisionId:string)=>void;branchLabel?:string;coverageSummary?:string}
function QuestionDisclosure({question}:{question:string}){
  return <details className="dw-user-question"><summary><MessageSquare size={14}/><span><InlineCodeText text={questionSummaryText(question)}/></span><ChevronRight size={13}/></summary><p><InlineCodeText text={question}/></p></details>;
}
function Answer(props:AnswerProps){
  const {turn,chapters,branches}=props;
  if(!turn.runIds?.length&&!turn.guideId)return <AnswerSection {...props}/>;
  const sections=turn.publishedSections??[];
  const sectionItems=groupUnavailablePublishedSections(sections,new Set((chapters??[]).map((chapter)=>chapter.revisionId)),Boolean(props.loading));
  const progressiveAnswerSequence=shouldSequencePublishedAnswer(turn);
  const sectionSequence=Boolean(turn.guideId||progressiveAnswerSequence);
  const guideSlides=sectionSequence&&!props.loading?sectionItems.flatMap((item)=>item.kind==='section'?[item.section]:[]):[];
  const selectedGuideSlide=sectionSequence?guideSlideIndex(guideSlides,turn.reading?.guide?.revisionId):-1;
  const guideSlide=selectedGuideSlide>=0?guideSlides[selectedGuideSlide]:undefined;
  const guideSlideItemIndex=guideSlide?sectionItems.findIndex((item)=>item.kind==='section'&&item.section.revisionId===guideSlide.revisionId):-1;
  const guideSlideRevision=guideSlide?chapters?.find((candidate)=>candidate.revisionId===guideSlide.revisionId):undefined;
  const selectGuideSlide=(index:number)=>{
    const next=guideSlides[index];
    const revision=next&&chapters?.find((chapter)=>chapter.revisionId===next.revisionId);
    const firstSection=revision?.sections[0];
    if(revision&&firstSection)props.onFocus(revision.revisionId,firstSection.sectionId);
  };
  const lastAvailableItemIndex=props.loading?-1:lastAvailableAnswerItemIndex(sectionItems);
  const capacityError=!!turn.error&&/retained locally|saved answers|(?:answer|saved explanations).*storage limit|answer.*directory limit/i.test(turn.error);
  const failureMessage=explanationFailureMessage(turn.failureCode);
  const progressText=questionProgressText(turn);
  const timeoutRecovery=repositoryGuideTimeoutRecovery(turn);
  const omissionQuestion=turn.guideId&&turn.guide&&!canContinueRepositoryGuide(turn.guide)
    &&(turn.status==='ready'||turn.status==='partial')&&selectedGuideSlide===guideSlides.length-1
    ?repositoryGuideOmissionQuestion(turn.guide.omissions):null;
  const omissionSummary=omissionQuestion?repositoryGuideOmissionSummary(turn.guide?.omissions??[]):null;
  const guideCoverageLabel=repositoryGuideCoverageLabel(turn.guide);
  return <section className="dw-progressive-answer" data-question-id={turn.id} tabIndex={-1} aria-label={`Question: ${questionSummaryText(turn.question)}`}>
    <QuestionDisclosure question={turn.question}/>
    <div className="dw-answer-progress" role="status">{progressText?<><LoaderCircle size={14}/><span>{progressText}</span><Button size="1" variant="ghost" color="gray" onClick={props.onStop}><Square size={12}/>Stop</Button></>:timeoutRecovery?<><span>{timeoutRecovery}</span><Button size="1" variant="soft" onClick={props.onRetry}>Start a new guide</Button></>:turn.status==='ready'?null:turn.status==='waiting'?<span>{sections.length} {turn.guideId?'slide':'section'}{sections.length===1?'':'s'} ready. The next {turn.guideId?'slide':'section'} builds while you listen.</span>:<><span>{turn.status==='stopped'?'Stopped.':turn.status==='interrupted'?'Interrupted.':turn.status==='partial'?'Some details could not be completed.':failureMessage??'This question could not be completed.'} {sections.length>0?'Your saved sections are available below.':''}</span><Button size="1" variant="ghost" onClick={props.onRetry}>{turn.guideId?(canContinueRepositoryGuide(turn.guide)?'Continue guide':'Start a fresh guide'):'Try again'}</Button></>}</div>
    {turn.guide&&turn.guide.units.length>0&&<details className="dw-guide-outline"><summary>{turn.guide.title}{guideCoverageLabel&&<span className="dw-guide-status" role="status">{guideCoverageLabel}</span>}<span className="dw-guide-outline-count">{guideSlides.length} slide{guideSlides.length===1?'':'s'} ready · {turn.guide.units.filter((u)=>u.status==='complete').length}/{turn.guide.units.length} topics explored</span></summary><ol>{turn.guide.units.map((unit)=>{const ready=sections.some((section)=>section.key===unit.key);return <li key={unit.key}><span>{unit.question}</span><small>{ready?'Ready':unit.status==='working'?'Building':unit.status}</small></li>;})}</ol><p>{turn.guide.capturedFiles} captured files are available to investigate. {turn.guide.startingFiles} starting files helped form this outline; this is not an explained-file count.</p>{turn.guide.omissions.map((note)=><p key={note}>{note}</p>)}</details>}
    {turn.notes?.map((note)=><p className="dw-answer-note" key={note}>{note}</p>)}
    {capacityError&&<AnswerStorage/>}
    {turn.error&&!timeoutRecovery&&<details className="dw-error-details"><summary>Details</summary><p>{explanationErrorMessage(turn.error)}</p></details>}
    {sectionSequence&&props.loading&&sections.length>0&&<p className="dw-guide-loading" role="status">{turn.guideId?'Restoring saved slides…':'Restoring saved answer sections…'}</p>}
    {sectionSequence&&guideSlide&&<nav className="dw-guide-slide-nav" aria-label={turn.guideId?'Guide slides':'Answer sections'}><Button size="1" variant="ghost" color="gray" disabled={selectedGuideSlide<=0} onClick={()=>selectGuideSlide(selectedGuideSlide-1)}><ArrowLeft size={13}/>Previous</Button><span>{turn.guideId?'Slide':'Section'} {selectedGuideSlide+1} of {guideSlides.length}{progressText?' · more on the way':''}</span><Button size="1" variant="soft" disabled={selectedGuideSlide>=guideSlides.length-1&&(!turn.guideId||!canContinueRepositoryGuide(turn.guide))} onClick={()=>{if(selectedGuideSlide<guideSlides.length-1){if(turn.guideId)props.onGuideSlideConsumed?.(guideSlide.revisionId);selectGuideSlide(selectedGuideSlide+1);}else if(turn.guideId&&canContinueRepositoryGuide(turn.guide))props.onGuideSlideConsumed?.(guideSlide.revisionId);}}>{selectedGuideSlide>=guideSlides.length-1&&progressText?`Building next ${turn.guideId?'slide':'section'}…`:selectedGuideSlide>=guideSlides.length-1&&turn.guideId&&turn.status==='waiting'?'Build next slide':`Next ${turn.guideId?'slide':'section'}`}<ArrowRight size={13}/></Button></nav>}
    {sectionSequence&&guideSlide&&<AnswerSection {...props} key={`${guideSlide.runId}:${guideSlide.generationId}`} hideQuestion loading={props.loading} revision={guideSlideRevision}
      turn={{...turn,status:'ready',revisionId:guideSlide.revisionId,notes:[],error:undefined}}
      branches={guideSlideItemIndex===lastAvailableItemIndex?branches:[]}
      branchLabel={omissionQuestion?'Explore uncovered topics':undefined}
      coverageSummary={omissionSummary??undefined}
      onBranch={omissionQuestion?()=>props.onExploreOmissions(turn):props.onBranch}/>}
    {!turn.guideId&&progressiveAnswerSequence&&sectionItems.some((item)=>item.kind==='unavailable')&&<details className="dw-unavailable-group"><summary>{sectionItems.filter((item)=>item.kind==='unavailable').reduce((count,item)=>count+(item.kind==='unavailable'?item.sections.length:0),0)} saved answer part(s) unavailable</summary><p>These parts could not be reopened. Re-explain this question to build them again.</p><Button size="1" variant="soft" onClick={props.onRetry}>Explain this question again</Button></details>}
    {!turn.guideId&&!progressiveAnswerSequence&&sectionItems.map((item,index)=>item.kind==='section'?<AnswerSection {...props} key={`${item.section.runId}:${item.section.generationId}`} hideQuestion
      loading={props.loading} revision={chapters?.find((r)=>r.revisionId===item.section.revisionId)}
      turn={{...turn,status:'ready',revisionId:item.section.revisionId,notes:[],error:undefined}}
      branches={index===lastAvailableItemIndex?branches:[]}/>:<details className="dw-unavailable-group" key={`unavailable:${item.sections[0].runId}:${item.sections[0].generationId}`}>
      <summary>{item.sections.length} saved answer part{item.sections.length===1?'':'s'} unavailable</summary>
      <p>These parts could not be reopened. Re-explain this question to build them again.</p>
      <Button size="1" variant="soft" onClick={props.onRetry}>Explain this question again</Button>
    </details>)}
    {turn.guideId&&sectionItems.some((item)=>item.kind==='unavailable')&&<details className="dw-unavailable-group"><summary>{sectionItems.filter((item)=>item.kind==='unavailable').reduce((count,item)=>count+(item.kind==='unavailable'?item.sections.length:0),0)} saved guide slide(s) unavailable</summary><p>These parts could not be reopened. Re-explain the affected part to build it again.</p><Button size="1" variant="soft" onClick={props.onRetry}>Continue guide</Button></details>}
    {lastAvailableItemIndex<0&&(branches.length>0||sectionItems.some((item)=>item.kind==='unavailable'))&&<footer className="dw-branches">
      <Button size="1" variant="ghost" onClick={()=>props.onBranch(`Explore ${turn.question} in more detail.`,turn.context)}><Network size={13}/>Explore further</Button>
      {branches.length>0&&<span>{branches.length} exploration{branches.length===1?'':'s'}</span>}
      {branches.map((exploration)=><button className="dw-branch-link" key={exploration.id} onClick={()=>props.onGo(exploration.id)}><span className="dw-branch-connector">↳</span><span>{exploration.title}</span><small>{exploration.turns.length} question{exploration.turns.length===1?'':'s'}</small><ChevronRight size={13}/></button>)}
    </footer>}
    {sections.length===0&&!progressText&&<p className="dw-empty-answer">No section was published for this attempt. Earlier saved explanations are still in the library.</p>}
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
  const [open,setOpen]=useState(false);const [answers,setAnswers]=useState<AnswerSummary[]>([]);const [unavailableCount,setUnavailableCount]=useState(0);const [error,setError]=useState<string|null>(null);const [busy,setBusy]=useState<string|null>(null);
  useEffect(()=>{if(open)void invoke<{records:AnswerSummary[];unavailableCount:number}>('list_workspace_answer_records').then(result=>{setAnswers(result.records);setUnavailableCount(result.unavailableCount);}).catch((failure)=>setError(errorMessageOr(failure,'Saved answers could not be loaded.')));},[open]);
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
  return <Dialog.Root open={open} onOpenChange={setOpen}><Dialog.Trigger><Button size="1" variant="soft">Manage saved answers</Button></Dialog.Trigger><Dialog.Content maxWidth="560px"><Dialog.Title>Free local answer storage</Dialog.Title><Dialog.Description size="2">Keep your explanations and source links in the library while clearing an older investigation record. This removes its detailed generation receipt. It does not send code or ask AI again.</Dialog.Description>{unavailableCount>0&&<p role="status">{unavailableCount} older or unverifiable record{unavailableCount===1?' is':'s are'} still saved on this Mac and were left untouched.</p>}<div className="dw-saved-answers">{answers.map((answer)=><section key={answer.runId}><p>{answer.question}</p><small>{answer.sections.length} saved section{answer.sections.length===1?'':'s'} · {answer.outcome}</small><Button size="1" variant="soft" disabled={!!busy||answer.outcome==='working'} onClick={()=>void release(answer)}>{busy===answer.runId?'Saving sections…':'Keep explanations and free space'}</Button></section>)}{answers.length===0&&<p>{unavailableCount>0?'No verified answer records are available to clear.':'No saved answers need cleanup.'}</p>}</div>{error&&<p role="alert">{error}</p>}<Flex justify="end" mt="4"><Dialog.Close><Button variant="soft" color="gray">Done</Button></Dialog.Close></Flex></Dialog.Content></Dialog.Root>;
}
function AnswerSection({turn,revision:original,chapters=[],loading,branches,compact,hideQuestion,onGo,onOpen,onBranch,onStop,onRetry,onReview,onRead,onSettings,onDestination,onSection,onStoryProgress,onVersion,onFocus,onGuideSlideConsumed,branchLabel,coverageSummary}:AnswerProps){
  const versions=original?answerRevisionOptions(original,chapters):[];
  const revision=original?resolveAnswerRevision(original,turn,chapters):undefined;
  const availability=savedAnswerAvailability(Boolean(loading),Boolean(revision));
  const unavailable=!revision&&availability==='unavailable';
  const position=revision?sectionPosition(revision.revisionId,revision.sections,answerReadingPosition(turn,'guide',revision.revisionId)):0;
  const section=revision?.sections[position];
  const selectSection=(index:number)=>{
    const next=revision?.sections[index];if(!revision||!next)return;
    onSection(revision.revisionId,next.sectionId);
  };
  const readableError=turn.error?explanationErrorMessage(turn.error):undefined;
  const knownEvidenceError=!!turn.error&&readableError!==turn.error;
  const failureMessage=explanationFailureMessage(turn.failureCode);
  const progressText=questionProgressText(turn);
  const contexts=section?.claims.flatMap((c)=>c.anchors).slice(0,3).map((a)=>({path:a.path,sha256:a.sourceSha256,from:a.startLine,to:a.endLine}))??turn.context;
  return <article className="dw-turn" data-question-id={hideQuestion?undefined:turn.id} tabIndex={hideQuestion?undefined:-1} aria-label={unavailable?'Question with unavailable saved explanation':`Question: ${questionSummaryText(turn.question)}`}>
    {!hideQuestion&&<QuestionDisclosure question={turn.question}/>}
    <div className={`dw-answer-card${unavailable?' dw-answer-card-unavailable':''}`}>
    {progressText?<div className="dw-working" role="status"><LoaderCircle size={16}/><span>{progressText}</span><Button size="1" variant="ghost" color="gray" onClick={onStop}><Square size={12}/>Stop</Button></div>:turn.status==='failed'||turn.status==='interrupted'||turn.status==='stopped'?<><p className="dw-error">{turn.status==='stopped'?'Stopped. Saved explanations are preserved.':turn.status==='interrupted'?'This question was interrupted. Saved explanations are preserved.':failureMessage??(knownEvidenceError?readableError:'This explanation could not be completed. Your saved work is preserved.')}</p>{turn.error&&!knownEvidenceError&&!failureMessage&&<details className="dw-error-details"><summary>Details</summary><p>{explanationErrorMessage(turn.error)}</p></details>}<Button size="1" variant="soft" onClick={onRetry}>Try this question again</Button></>:!revision?availability==='loading'?<p role="status">Loading saved explanation…</p>:<><p className="dw-unavailable-status" role="status">The answer for this question could not be reopened. Your question is saved; explain it again to create a new answer.</p><Button size="1" variant="soft" onClick={onRetry}>Explain again</Button></>:section?<>
      <div className="dw-answer-heading"><span>{compact?'Explanation':`${position+1} / ${revision.sections.length}`}</span>{workspaceAnswerLifecycleBadge(revision.lifecycle)&&<Badge size="1" color="gray">{workspaceAnswerLifecycleBadge(revision.lifecycle)}</Badge>}<div><Tooltip content="Edit this explanation"><IconButton size="1" variant="ghost" color="gray" aria-label={`Edit ${revision.title}`} onClick={()=>onReview(revision)}><Pencil size={13}/></IconButton></Tooltip><Tooltip content="Open interactive player"><IconButton size="1" variant="ghost" color="gray" aria-label={`Play ${revision.title}`} onClick={()=>onRead(revision)}><Play size={13}/></IconButton></Tooltip></div></div>
      {versions.length>1&&original&&<label className="dw-answer-version">Version<select aria-label={`Answer version for ${original.title}`} value={revision.revisionId} onChange={event=>onVersion(original.revisionId,event.target.value)}>{versions.map((r,index)=><option key={r.revisionId} value={r.revisionId}>{index===0?'Original':'Accepted'} · r{r.revisionNumber}{index===versions.length-1?' · latest accepted':''}</option>)}</select>{revision.revisionId!==versions.at(-1)?.revisionId&&<span>Updated version available</span>}</label>}
      {turn.notes?.map((note)=><p key={note} className="dw-answer-note" role="status">{note}</p>)}
      <h3>{compact?<button type="button" className="dw-answer-expand" aria-label={`Open ${section.title} in Guide`} onClick={()=>onFocus(revision.revisionId,section.sectionId)}><InlineCodeText text={section.title}/><ChevronRight size={16}/></button>:<InlineCodeText text={section.title}/>}</h3>
      {!compact ? <>
        <VisualStory key={`${revision.revisionId}:${section.sectionRevisionId}`} revision={revision} section={section} progress={turn.presentation?.stories.find((p)=>sameSection(p,{revisionId:revision.revisionId,sectionId:section.sectionId,sectionRevisionId:section.sectionRevisionId}))} onProgressChange={onStoryProgress} onSettings={onSettings} onOpenAnchor={onOpen} onSectionPlaybackStarted={turn.guideId?()=>onGuideSlideConsumed?.(revision.revisionId):undefined} onSectionPlaybackComplete={turn.guideId?()=>onGuideSlideConsumed?.(revision.revisionId):undefined} onFollowHotspot={(hotspot)=>{
          const destination=hotspot.destination;
          if(destination.kind==='section')selectSection(revision.sections.findIndex((s)=>s.sectionId===destination.sectionId));
          else if(destination.kind==='codeLens'){const anchor=revision.sections.flatMap((s)=>s.claims.flatMap((c)=>c.anchors)).find((a)=>a.anchorId===destination.anchorId);if(anchor)onOpen(anchor);}
          else onDestination(destination);
        }}/>
        <div className="dw-section-controls"><IconButton size="1" variant="ghost" color="gray" aria-label="Previous section" disabled={position===0} onClick={()=>selectSection(position-1)}><ArrowLeft size={14}/></IconButton><span>{position+1} / {revision.sections.length}</span><Button size="1" variant="soft" disabled={position>=revision.sections.length-1} onClick={()=>selectSection(position+1)}>Next section<ArrowRight size={13}/></Button></div>
      </> : <details className="dw-flow-narration"><summary>Read narration</summary><p className="dw-prose"><InlineCodeText text={section.spokenText || section.prose}/></p><div className="dw-citations">{section.claims.flatMap((c)=>c.anchors).slice(0,5).map((a)=><button key={a.anchorId} onClick={()=>onOpen(a)}><Code2 size={12}/>{a.path.split('/').at(-1)}:{a.startLine}–{a.endLine}</button>)}</div></details>}
      {coverageSummary&&<div className="dw-guide-coverage" role="status"><strong>Partial guide</strong><span>Still uncovered: {coverageSummary}</span></div>}
      <footer className="dw-branches"><Button size="1" variant="ghost" onClick={()=>onBranch(`Explain ${section.title} in more detail.`,contexts)}><Network size={13}/>{branchLabel??'Explore further'}</Button>{branches.length>0&&<span>{branches.length} exploration{branches.length===1?'':'s'}</span>}{branches.map((e)=><button className="dw-branch-link" key={e.id} onClick={()=>onGo(e.id)}><span className="dw-branch-connector">↳</span><span><InlineCodeText text={e.title}/></span><small>{e.turns.length} question{e.turns.length===1?'':'s'}</small><ChevronRight size={13}/></button>)}</footer>
    </>:<p>This saved explanation has no sections.</p>}
  </div></article>;
}
function EvidenceFlow({revision,saved,progress,onProgress,onSection,theme,onOpen}:{revision:ChapterRevision;saved?:ReadingPosition;progress?:DiagramProgress[];onProgress:(progress:DiagramProgress)=>void;onSection:(sectionId:string)=>void;theme:'dark'|'light';onOpen:(a:ChapterSourceAnchor)=>void}){
  const diagrams=revision.sections.map((s)=>inspectInteractiveVisual(revision,s));
  const availableDiagramCount=diagrams.filter((diagram)=>diagram.status==='available').length;
  const remembered=sectionPosition(revision.revisionId,revision.sections,saved);
  const selected=diagrams[remembered]?.status==='available'?remembered:Math.max(0,diagrams.findIndex((d)=>d.status==='available'));
  const result=diagrams[selected];
  const available=diagrams.some((d)=>d.status==='available');
  if(!available)return <div className="dw-flow-empty"><Network size={23}/><h2>No evidenced diagram in this explanation</h2><p>Use the source links in Guide to inspect the implementation. A graph is shown only when it has verified evidence.</p></div>;
  if(result?.status!=='available')return null;
  const identity={revisionId:revision.revisionId,sectionId:revision.sections[selected].sectionId,sectionRevisionId:revision.sections[selected].sectionRevisionId};
  const viewport=progress?.find(p=>sameSection(p,identity))?.viewport;
  const {visual,anchorsById}=result;
  const sourceValues=sourceValuesByVisualNode(visual);
  const openNodeSource=(node:typeof visual.nodes[number],anchor:ChapterSourceAnchor)=>onOpen(sourceAnchorForVisualNode(anchor,node,node.sourceValue??sourceValues.get(node.nodeId)));
  const linearSteps=linearFlowSteps(visual);
  const unlinkedNodes=unlinkedFlowNodes(visual);
  const positions=layoutFlowNodes(visual.nodes,visual.edges,visual.layoutHint.direction);
  const labelPositions=layoutFlowEdgeLabels(visual.nodes,visual.edges,positions);
  const nodes=visual.nodes.map((n)=>{
    const anchor=(n.sourceAnchorId&&n.evidenceAnchorIds.includes(n.sourceAnchorId)?anchorsById.get(n.sourceAnchorId):undefined)??(n.evidenceAnchorIds[0]?anchorsById.get(n.evidenceAnchorIds[0]):undefined);
    const label=<><strong>{n.label}</strong>{n.detail&&<small>{n.detail}</small>}</>;
    return {id:n.nodeId,position:positions.get(n.nodeId)??{x:0,y:0},
      data:{label:anchor?<button type="button" className="dw-flow-source nodrag nopan" aria-label={`Open source for ${n.label}`} onClick={()=>openNodeSource(n,anchor)}>{label}</button>:label},style:{width:250}};
  });
  const edges=visual.edges.map((e)=>({id:e.edgeId,source:e.fromNodeId,target:e.toNodeId,type:'evidenceRelationship' as const,label:e.label??undefined,
    data:{labelPosition:labelPositions.get(e.edgeId)},style:{stroke:theme==='dark'?'#8c96a8':'#6a7486',strokeWidth:1.5},
    markerEnd:{type:MarkerType.ArrowClosed}}));
  return <section className="dw-flow-panel"><div className="dw-flow-heading"><h2>{visual.title}</h2>{availableDiagramCount>1&&<select aria-label="Diagram section" value={selected} onChange={(e)=>{const section=revision.sections[Number(e.target.value)];if(section)onSection(section.sectionId);}}>{diagrams.map((d,i)=>d.status==='available'?<option key={i} value={i}>{revision.sections[i].title}</option>:null)}</select>}</div>
    {linearSteps ? <>
      <p>Follow the steps in order. Select a step to open its source.</p>
      <ol className="dw-linear-flow" aria-label={`Steps in ${visual.title}`}>
        {linearSteps.map(({node, next}, index) => {
          const sourceAnchorId=node.sourceAnchorId&&node.evidenceAnchorIds.includes(node.sourceAnchorId)?node.sourceAnchorId:node.evidenceAnchorIds[0];
          const evidenceAnchor=anchorsById.get(sourceAnchorId);
          const anchor=evidenceAnchor?sourceAnchorForVisualNode(evidenceAnchor,node,node.sourceValue??sourceValues.get(node.nodeId)):undefined;
          if (!anchor) return null;
          const sourceLabel=`${anchor.path}:${anchor.startLine}${anchor.endLine===anchor.startLine?'':`–${anchor.endLine}`}`;
          return <li className="dw-linear-flow-item" key={node.nodeId}>
            <button type="button" className="dw-linear-flow-step" aria-label={`Step ${index+1}: ${node.label}. Open source ${sourceLabel}`} onClick={()=>onOpen(anchor)}>
              <span className="dw-linear-flow-index">Step {index+1}</span>
              <strong>{node.label}</strong>
              {node.detail&&<small>{node.detail}</small>}
              <span className="dw-linear-flow-source"><Code2 size={13}/>{sourceLabel}<ArrowRight size={13}/></span>
            </button>
            {next&&<div className="dw-linear-flow-edge"><ArrowDown size={15}/><span>{next.label||'Then'}</span></div>}
          </li>;
        })}
      </ol>
    </> : unlinkedNodes ? <>
      <p>The evidence for this explanation does not define a path between these points. Select one to inspect its code.</p>
      <ul className="dw-unlinked-flow" aria-label={`Source points in ${visual.title}`}>
        {unlinkedNodes.map((node) => {
          const sourceAnchorId=node.sourceAnchorId&&node.evidenceAnchorIds.includes(node.sourceAnchorId)?node.sourceAnchorId:node.evidenceAnchorIds[0];
          const evidenceAnchor=anchorsById.get(sourceAnchorId);
          const anchor=evidenceAnchor?sourceAnchorForVisualNode(evidenceAnchor,node,node.sourceValue??sourceValues.get(node.nodeId)):undefined;
          if(!anchor)return null;
          const sourceLabel=`${anchor.path}:${anchor.startLine}${anchor.endLine===anchor.startLine?'':`–${anchor.endLine}`}`;
          return <li key={node.nodeId}><button type="button" className="dw-unlinked-flow-point" aria-label={`Open source for ${node.label} at ${sourceLabel}`} onClick={()=>onOpen(anchor)}>
            <strong>{node.label}</strong>
            {node.detail&&<small>{node.detail}</small>}
            <span className="dw-linear-flow-source"><Code2 size={13}/>{sourceLabel}<ArrowRight size={13}/></span>
          </button></li>;
        })}
      </ul>
    </> : <>
      <p>Select a step to open its source.</p>
      <EvidenceFlowCanvas key={`${identity.revisionId}:${identity.sectionRevisionId}:${FLOW_DIAGRAM_LAYOUT_VERSION}`} revisionId={identity.revisionId} sectionId={identity.sectionId} sectionRevisionId={identity.sectionRevisionId} nodes={nodes} edges={edges} viewport={viewport} theme={theme} onViewportChange={(viewport)=>onProgress({...identity,viewport,layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION})}/>
    </>}
  </section>;
}

function EvidenceFlowCanvas({revisionId,sectionId,sectionRevisionId,nodes,edges,viewport,theme,onViewportChange}:{
  revisionId:string;sectionId:string;sectionRevisionId:string;nodes:Node[];edges:Edge[];viewport?:FlowViewport;theme:'dark'|'light';onViewportChange:(viewport:FlowViewport)=>void;
}){
  const stage=useRef<HTMLDivElement>(null);
  const instance=useRef<FlowViewportInstance|null>(null);
  const scheduleCheck=useRef<()=>void>(()=>{});
  const generationRef=useRef(0);
  const fitInProgress=useRef(false);
  const onViewportChangeRef=useRef(onViewportChange);
  onViewportChangeRef.current=onViewportChange;
  const nodeKey=JSON.stringify(nodes.map((node)=>node.id));
  useEffect(()=>{
    const container=stage.current;
    if(!container)return;
    const generation=++generationRef.current;
    let frame:number|undefined;
    const schedule=()=>{
      if(frame!==undefined)return;
      frame=requestAnimationFrame(()=>{
        frame=undefined;
        const flow=instance.current;
        const current=stage.current;
        if(!flow?.viewportInitialized||!current||fitInProgress.current)return;
        const bounds=flow.getNodesBounds(nodes.map((node)=>node.id));
        const size=current.getBoundingClientRect();
        if(!bounds.width||!bounds.height||!viewportClipsNodeBounds(bounds,flow.getViewport(),size.width,size.height,28))return;
        fitInProgress.current=true;
        void flow.fitView({nodes:nodes.map((node)=>({id:node.id})),padding:.08,minZoom:.1,maxZoom:1.3}).then((fitted)=>{
          if(!fitted||generation!==generationRef.current||flow!==instance.current)return;
          const clean=cleanFlowViewport(flow.getViewport());
          if(clean)onViewportChangeRef.current(clean);
        }).catch(()=>{
          // Keep the diagram available when React Flow cannot apply a saved-view repair.
        }).finally(()=>{if(generation===generationRef.current)fitInProgress.current=false;});
      });
    };
    scheduleCheck.current=schedule;
    const observer=typeof ResizeObserver==='undefined'?undefined:new ResizeObserver(schedule);
    observer?.observe(container);
    window.addEventListener('resize',schedule);
    schedule();
    return()=>{
      observer?.disconnect();window.removeEventListener('resize',schedule);
      if(frame!==undefined)cancelAnimationFrame(frame);
      if(scheduleCheck.current===schedule)scheduleCheck.current=()=>{};
      generationRef.current+=1;fitInProgress.current=false;
    };
  },[revisionId,sectionId,sectionRevisionId,nodeKey]);
  return <div ref={stage} className="dw-flow-canvas"><ReactFlow nodes={nodes} edges={edges} edgeTypes={evidenceFlowEdgeTypes} defaultViewport={viewport} fitView={!viewport} minZoom={.1} maxZoom={4}
    aria-label={`Read-only code relationship diagram: ${nodes.length} nodes`} ariaLabelConfig={readOnlyDiagramAriaLabelConfig} disableKeyboardA11y
    onInit={(flow)=>{instance.current=flow;scheduleCheck.current();}}
    onNodesChange={(changes)=>{if(changes.some((change)=>change.type==='dimensions'))scheduleCheck.current();}}
    onMoveEnd={(_event,next)=>{const clean=cleanFlowViewport(next);if(clean)onViewportChange(clean);}}
    nodesDraggable={false} nodesConnectable={false} nodesFocusable={false} edgesFocusable={false} elementsSelectable={false} deleteKeyCode={null} colorMode={theme}>
    <Background gap={20}/><Controls showInteractive={false}/>
  </ReactFlow></div>;
}

type EvidenceRelationshipEdge = Edge<{labelPosition?:FlowPoint},'evidenceRelationship'>;
function EvidenceRelationship({id,sourceX,sourceY,targetX,targetY,sourcePosition,targetPosition,markerEnd,style,label,data}:EdgeProps<EvidenceRelationshipEdge>){
  const [path]=getSmoothStepPath({sourceX,sourceY,targetX,targetY,sourcePosition,targetPosition});
  const point=data?.labelPosition;
  return <>
    <BaseEdge id={id} path={path} markerEnd={markerEnd} style={style}/>
    {typeof label==='string'&&point&&<EdgeLabelRenderer><div className="dw-flow-edge-label" style={{transform:`translate(-50%, -50%) translate(${point.x}px, ${point.y}px)`}}>{label}</div></EdgeLabelRenderer>}
  </>;
}
const evidenceFlowEdgeTypes={evidenceRelationship:EvidenceRelationship};
