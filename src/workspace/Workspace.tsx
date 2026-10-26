import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Badge, Button, Dialog, Flex, IconButton, Tabs, TextField, Theme, Tooltip, useThemeContext } from '@radix-ui/themes';
import { Group, Panel, Separator } from 'react-resizable-panels';
import { Background, BaseEdge, Controls, EdgeLabelRenderer, getSmoothStepPath, MarkerType, ReactFlow, type Edge, type EdgeProps, type Node } from '@xyflow/react';
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, BookOpen, ChevronRight, Code2, FileCode2, Files, GitBranch, GitCommitHorizontal, History, LoaderCircle, MessageSquare, Network, Paperclip, Pencil, Search, ShieldCheck, Square, X } from 'lucide-react';
import { CodePane } from '../design/CodePane';
import { CodeDetail } from '../design/CodeDetail';
import { FileFlow } from '../FileFlow';
import { InlineCodeText } from '../InlineCodeText';
import { VisualStory } from '../VisualStory';
import { OfflineExport } from '../OfflineExport';
import { ReaderProgressContext } from '../lib/ReaderProgressContext';
import { bindSelectionToDisplayedSource } from '../lib/codeSelection';
import { capturedSourceLineIndex, capturedSourceLineWindow } from '../lib/codeLens';
import { sourceAnchorForVisualNode, sourceValuesByVisualNode } from '../lib/visualStory';
import { cleanFlowViewport, FLOW_DIAGRAM_LAYOUT_VERSION, sameSection, type DiagramProgress, type FlowViewport, type StoryProgress } from '../lib/readerProgress';
import { viewportClipsNodeBounds, type FlowViewportInstance } from '../lib/flowViewport';
import { buildInventory } from '../lib/inventory';
import { isNativeRuntime, listChapterRevisions, readSnapshotSource, type ChapterRevision, type ChapterSourceAnchor, type SnapshotManifest, type SnapshotSource, type SnapshotIndex, type HotspotDestination } from '../lib/commands';
import { projectProvenRelationshipGraph } from '../lib/atlas';
import { inspectInteractiveVisual } from '../lib/visuals';
import type { AtlasHotspotSelection, SourceLineRange, Workspace as WorkspaceId } from '../lib/navigation';
import { ancestors, canContinueRepositoryGuide, contextualGuideAction, discardUnstartedGuide, focusAnswerSection, historyEntry, isRepositoryGuideIntent, questionProgressText, repositoryGuideCoverageLabel, repositoryGuideOmissionSummary, repositoryGuideUncoveredTopics, chooseFlowRevision, rankSourcePaths, presentationOwner, explorationSummaryCounts, rememberCodePane, rememberFileFlow, rememberSourceContext, rememberPresentation, rememberScroll, rememberSection, sectionPosition, workspaceAnswerLifecycleBadge, type AnswerSummary, type Context, type Exploration, type ReadingPosition, type Turn, type View } from './model';
import { advanceRepositoryGuide, appendQuestion, appendRepositoryGuide, authorizeWorkspace, ensureWorkspaceIndex, prepareWorkspaceConversation, resolveWorkspaceConversationConflict, revokeWorkspace, restoreWorkspaceAnswers, runQuestion, runRepositoryGuide, stopQuestion, updateWorkspace, updateWorkspaceDraft, useConversationAvailability, useWorkspace, useWorkspaceConversationConflict, watchWorkspaceAnswers } from './store';
import { useConnectionState } from '../lib/connectionStore';
import { codexSelectionIsSupported } from '../lib/codexConnection';
import type { WorkspaceAnswerRestoration } from '../lib/notificationSummary';
import { errorMessageOr, explanationErrorMessage, explanationFailureMessage } from '../lib/errorMessage';
import { createChapterHistory, type ChapterHistory } from '../lib/chapterHistory';
import { featuredGuideTurnId, groupUnavailablePublishedSections, guideSlideIndex, questionSummaryText, lastAvailableAnswerItemIndex, openWorkspaceExploration, openWorkspaceQuestion, openWorkspaceRevision, savedAnswerAvailability, shouldSequencePublishedAnswer, visibleConversationTurns } from './outcomes';
import { listWorkspaceCommits } from '../lib/commands';
import { largeFileExplanationContexts } from '../lib/largeFileExplanation';
import { answerReadingPosition, answerRevisionOptions, rememberAnswerVersion, resolveAnswerRevision } from './answerVersions';
import type { GitCommit } from '../lib/gitHistory';
import { searchCapturedSource, searchExploration, searchLimitMessage, searchResultsMessage, shouldOfferExploreWithAI, workspaceSearchResults, type CodeSearchHit, type CodeSearchMode } from './codeSearch';
import { linearFlowSteps, unlinkedFlowNodes } from './flowPresentation';
import { layoutFlowEdgeLabels, layoutFlowNodes, type FlowPoint } from './flowLayout';
import { commitHistoryEmptyMessage, nextFocusableCommitIndex } from './gitCommitNavigation';
import { GitHistoryEmptyContext } from './gitHistoryContext';
import { readOnlyDiagramAriaLabelConfig } from '../lib/readOnlyDiagramAccessibility';
import { plainInlineCodeText } from '../lib/inlineCode';
import { readGenerationBackend } from '../lib/generationPreferences';
import { WorkspaceExplorer } from './WorkspaceExplorer';
import { readRepositorySearchContext, rememberRepositorySearchContext } from './searchContext';
import './workspace.css';

interface Props {
  manifest: SnapshotManifest; projectName:string; workspace:WorkspaceId; routePath?:string; routeRange?:SourceLineRange; routeSelection?:AtlasHotspotSelection;
  routeSearchQuery?:string; routeSearchMode?:CodeSearchMode; routeSidebarActivity?:'files'|'search'|'changes'|'history';
  onRememberSearch?:(query:string,mode:CodeSearchMode,activity:'files'|'search'|'changes'|'history')=>void;
  comparisonRefs?: { baseRef?:string; headRef?:string };
  pendingLargeFileExplanation?: { id:string; projectId:string; snapshotId:string; targetPath:string; paths:string[]; question:string } | null;
  onPendingLargeFileExplanationConsumed?:(id:string)=>void;
  children:ReactNode; onWorkspace:(workspace:WorkspaceId)=>void; onSettings:()=>void;
  onReview:(revision:ChapterRevision)=>void;
  onCompare:(base:string,head:string)=>void; onRememberFile:(path:string)=>void;
  onAnswerRestoration:(state:WorkspaceAnswerRestoration)=>void;
  conversationAvailability?:'ready'|'unavailable';
}
const label = (c:Context) => `${c.path.split('/').at(-1)}${c.from ? `:${c.from}${c.to !== c.from ? `–${c.to}` : ''}` : ''}`;
const MAX_CONTEXT_REFERENCES = 12;
const contextsFromAnchors = (anchors:readonly ChapterSourceAnchor[]):Context[]=>anchors.slice(0,MAX_CONTEXT_REFERENCES).map((anchor)=>({path:anchor.path,sha256:anchor.sourceSha256,from:anchor.startLine,to:anchor.endLine}));
interface CodeRequestReview { context:Context; question:string; snapshotId:string; sourceSha256:string; sourceExcerpt:string; excerptTruncated:boolean }

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
  const {manifest,projectName,workspace,routePath,routeRange,routeSelection,children,onWorkspace,onSettings,onReview,onCompare,onRememberFile,onAnswerRestoration}=props;
  const {appearance:themeAppearance}=useThemeContext();
  const appearance=themeAppearance==='light'?'light':'dark';
  const {projectId,snapshotId} = manifest.summary;
  const conversationUnavailable=props.conversationAvailability==='unavailable';
  const state = useWorkspace(projectId,snapshotId);
  const conversationConflict=useWorkspaceConversationConflict(projectId,snapshotId);
  const generationPaused=conversationUnavailable||!!conversationConflict;
  const [conflictResolutionBusy,setConflictResolutionBusy]=useState(false);
  const [conflictResolutionError,setConflictResolutionError]=useState<string|null>(null);
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
  const query=current.draft?.question??'';
  const attachments=current.draft?.context??[];
  function setQuery(question:string){updateWorkspaceDraft(projectId,snapshotId,current.id,{question,context:current.draft?.context??[]});}
  function setAttachments(next:Context[]|((existing:Context[])=>Context[])){
    const context=typeof next==='function'?next(current.draft?.context??[]):next;
    if(context.length<MAX_CONTEXT_REFERENCES)setDraftNotice(null);
    updateWorkspaceDraft(projectId,snapshotId,current.id,{question:current.draft?.question??'',context});
  }
  const path = ancestors(state);
  const rows = useMemo(()=>buildInventory(manifest).filter((r)=>r.kind==='file' && r.disposition==='captured' && !['binary','invalidEncoding'].includes(r.contentKind) && !r.categories.includes('secret')), [manifest]);
  const sourcePaths = useMemo(()=>rankSourcePaths(rows.map((r)=>r.path)),[rows]);
  const initialSearchContext=useMemo(()=>{
    if(props.routeSearchQuery!==undefined||props.routeSearchMode!==undefined||props.routeSidebarActivity!==undefined){
      const activity=props.routeSidebarActivity??(props.routeSearchQuery!==undefined?'search':'files');
      return {activity:activity==='changes'?'files' as const:activity,query:props.routeSearchQuery??'',mode:props.routeSearchMode==='regex'?'regex' as const:'code' as const};
    }
    const saved=readRepositorySearchContext(projectId,snapshotId);
    return saved?{activity:saved.activity,query:saved.query,mode:saved.mode}:{activity:'files' as const,query:'',mode:'code' as const};
  },[projectId,snapshotId,props.routeSearchQuery,props.routeSearchMode,props.routeSidebarActivity]);
  const [activity,setActivityState] = useState<'files'|'search'|'changes'|'history'>(initialSearchContext.activity);
  const [fileFilter,setFileFilter] = useState('');
  const [repositorySearch,setRepositorySearchState] = useState(initialSearchContext.query);
  const [repositoryRegex,setRepositoryRegexState] = useState(initialSearchContext.mode==='regex');
  const searchRouteRef=useRef({activity,query:repositorySearch,regex:repositoryRegex,remember:props.onRememberSearch});
  searchRouteRef.current={activity,query:repositorySearch,regex:repositoryRegex,remember:props.onRememberSearch};
  function rememberSearchState(next:{activity?:'files'|'search'|'changes'|'history';query?:string;regex?:boolean}) {
    const current=searchRouteRef.current;
    const nextActivity=next.activity??current.activity;
    const nextQuery=next.query??current.query;
    const nextRegex=next.regex??current.regex;
    searchRouteRef.current={...current,activity:nextActivity,query:nextQuery,regex:nextRegex};
    if(next.activity!==undefined)setActivityState(next.activity);
    if(next.query!==undefined)setRepositorySearchState(next.query);
    if(next.regex!==undefined)setRepositoryRegexState(next.regex);
    rememberRepositorySearchContext(projectId,snapshotId,{query:nextQuery,mode:nextRegex?'regex':'code',activity:nextActivity==='changes'?'files':nextActivity});
    current.remember?.(nextQuery,nextRegex?'regex':'code',nextActivity);
  }
  const setActivity=(next:'files'|'search'|'changes'|'history')=>rememberSearchState({activity:next});
  const setRepositorySearch=(next:string)=>rememberSearchState({query:next});
  const toggleRepositoryRegex=()=>rememberSearchState({regex:!repositoryRegex});
  useEffect(()=>{
    setActivityState(initialSearchContext.activity);
    setRepositorySearchState(initialSearchContext.query);
    setRepositoryRegexState(initialSearchContext.mode==='regex');
  },[initialSearchContext]);
  useEffect(()=>{
    if(workspace!=='changes'&&activity==='changes'){
      setActivityState('files');
      props.onRememberSearch?.(repositorySearch,repositoryRegex?'regex':'code','files');
    }
  },[workspace,activity,repositorySearch,repositoryRegex,props.onRememberSearch]);
  const [sourceHits,setSourceHits] = useState<{key:string;hits:{path:string;line:number;text:string}[]}>({key:'',hits:[]});
  const [searchLimited,setSearchLimited] = useState(false);
  const [searchNextOffset,setSearchNextOffset] = useState<number|null>(null);
  const [searchOffset,setSearchOffset] = useState(0);
  const [searchBusy,setSearchBusy] = useState(false);
  const [searchError,setSearchError] = useState<string|null>(null);
  const [openFolders,setOpenFolders] = useState<Set<string>>(()=>new Set());
  const [narrowPane,setNarrowPane] = useState<'explanation'|'source'|'explorer'>('explanation');
  const codeOpen = current.codeOpen ?? true;
  const setCodeOpen = (open:boolean) => {
    setNarrowPane(open?'source':'explanation');
    updateWorkspace(projectId,snapshotId,(s)=>rememberCodePane(s,current.id,open));
  };
  const [sourceEntry,setSourceEntry] = useState<{projectId:string;snapshotId:string;path:string;document:SnapshotSource}|null>(null);
  const source=sourceEntry?.projectId===projectId&&sourceEntry.snapshotId===snapshotId&&sourceEntry.path===state.file
    ? sourceEntry.document : null;
  const sourceLineIndex=useMemo(()=>source?capturedSourceLineIndex(source.contents):null,[source?.contents]);
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
  const [guideConsent,setGuideConsent] = useState<{id:string;resume:boolean;unitKey?:string}|null>(null);
  const [consentBusy,setConsentBusy] = useState(false);
  const [consentError,setConsentError] = useState<string|null>(null);
  const [codeRequestReview,setCodeRequestReview] = useState<CodeRequestReview|null>(null);
  const [codeRequestReviewError,setCodeRequestReviewError] = useState<string|null>(null);
  const [connectionNotice,setConnectionNotice] = useState<string|null>(null);
  const [draftNotice,setDraftNotice] = useState<string|null>(null);
  const [sharingOpen,setSharingOpen] = useState(false);
  const [searchFocused,setSearchFocused] = useState(false);
  const handledLargeFileExplanation = useRef<string|null>(null);
  const requestedConnectionForLargeFileExplanation = useRef<string|null>(null);
  const autoOpenedInitialFile = useRef(false);
  const composer = useRef<HTMLTextAreaElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const explorer = useRef<HTMLDivElement>(null);
  const answerScroll = useRef<HTMLDivElement>(null);
  const chapterRefresh = useRef<ChapterHistory|null>(null);
  const restoringScroll = useRef(true);
  const tool = ['changes','chapter-studio'].includes(workspace);
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
      const selected=generationBackend===null?'saved AI connection':generationBackend==='api'?'Direct API':generationBackend==='handoff'?'manual agent handoff':'Claude Agent';
      setConnectionNotice(generationBackend===null
        ? `The ${selected} could not be read. Choose a connection in Connections to continue. Your question stays here.`
        : `Workspace answers currently use Codex. Your ${selected} connection is saved; switch once in Connections to continue. Your question stays here.`);
      onSettings();
      return false;
    }
    if(connection.checking){
      setConnectionNotice('Codex is reconnecting. Your question stays here.');
      return false;
    }
    if(!connection.status?.authenticated||!connection.status.runtimeRunning){
      setConnectionNotice('Connect Codex in Connections to continue. Your question stays here.');
      onSettings();
      return false;
    }
    if(!codexSelectionIsSupported(connection.models,connection.selection)){
      setConnectionNotice('Choose a supported Codex model and reasoning level in Connections. Your question stays here.');
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
  useEffect(()=>{if(routePath && sourcePaths.includes(routePath)){const context={path:routePath,from:routeRange?.startLine,to:routeRange?.endLine,sha256:routeRange?.sourceSha256};update((s)=>({...s,file:routePath,explorations:s.explorations.map((e)=>e.id===s.currentId?{...e,file:routePath,reveal:context,codeOpen:true}:e)}));setReveal(context);setNarrowPane('source');setSearchFocused(false);}},[routePath,routeRange]);
  useEffect(()=>{
    const path=state.file;
    setSourceEntry(null);setSourceError(null);
    if(!path||!isNativeRuntime()){setSourceBusy(false);return;}
    let active=true;setSourceBusy(true);
    void readSnapshotSource(projectId,snapshotId,path).then((value)=>{if(!active)return;
      if(value.snapshotId!==snapshotId||value.path!==path){setSourceError('The returned source did not match the selected snapshot and file.');setSourceBusy(false);return;}
      setSourceEntry({projectId,snapshotId,path,document:value});setSourceBusy(false);
    }).catch((failure)=>{if(active){setSourceError(errorMessageOr(failure,'This captured file could not be opened.'));setSourceBusy(false);}});
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
      const restoreTurnId=current.revealTurnId??current.focusedTurnId??featuredTurnId;
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
  },[current.id,current.view,current.revealTurnId,current.focusedTurnId,featuredTurnId,chaptersLoading,tool]);
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
  function openQuickMatch(value:string) {
    const query=value.trim().toLowerCase();
    if(!query)return false;
    const unique=(predicate:(path:string)=>boolean)=>{
      const matches=sourcePaths.filter(predicate);
      return matches.length===1?matches[0]:undefined;
    };
    const match=unique((path)=>path.toLowerCase()===query)
      ??unique((path)=>path.split('/').at(-1)?.toLowerCase()===query)
      ??unique((path)=>path.toLowerCase().startsWith(query))
      ??unique((path)=>path.toLowerCase().includes(query));
    if(!match)return false;
    openFile({path:match});
    return true;
  }
  function focusComposer(){setNarrowPane('explanation');onWorkspace('guide-plan');requestAnimationFrame(()=>composer.current?.focus());}
  function showView(view:View){setView(view);setNarrowPane('explanation');onWorkspace('guide-plan');}
  function attach(context:Context) {
    if(attachments.some((c)=>c.path===context.path&&c.from===context.from&&c.to===context.to)){focusComposer();return;}
    if(attachments.length>=MAX_CONTEXT_REFERENCES){setDraftNotice(`A question can include up to ${MAX_CONTEXT_REFERENCES} source references. Remove one to add another.`);return;}
    setDraftNotice(null);
    setAttachments((all)=>[...all,context]);focusComposer();
  }
  function startRepositoryGuide(question:string,parentTurn?:string) {
    if(generationPaused)return;
    if(!requireCodexConnection())return;
    const id=appendRepositoryGuide(projectId,snapshotId,question,parentTurn);
    setSearchFocused(false);setNarrowPane('explanation');onWorkspace('guide-plan');
    if(state.grantId)void runRepositoryGuide(projectId,snapshotId,id,state.grantId);
    else {setGuideConsent({id,resume:false});setConsentError(null);}
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  }
  function ask(question:string,view:View=current.view==='flow'?'flow':'guide',context=attachments,parentTurn?:string){
    if(generationPaused)return;
    const clean=question.replace(/[\r\n\t]+/g,' ').trim();if(!clean){setView(view);focusComposer();return;}
    if(context.length>MAX_CONTEXT_REFERENCES){setDraftNotice(`A question can include up to ${MAX_CONTEXT_REFERENCES} source references. Remove some before sending.`);return;}
    if(!requireCodexConnection())return;
    if(isRepositoryGuideIntent(clean)){startRepositoryGuide(clean,parentTurn);return;}
    const id=appendQuestion(projectId,snapshotId,clean,context,view,parentTurn);setSearchFocused(false);setNarrowPane('explanation');onWorkspace('guide-plan');
    if(state.grantId) void runQuestion(projectId,snapshotId,id,state.grantId);else{setConsentError(null);setConsentTurn(id);}
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  }
  function isCurrentCodeRequestReview(review:CodeRequestReview){
    if(review.snapshotId!==snapshotId||!source||source.snapshotId!==snapshotId||source.path!==review.context.path
      ||source.sha256!==review.sourceSha256||!rows.some((row)=>row.path===review.context.path))return false;
    const from=review.context.from,to=review.context.to;
    if(from===undefined&&to===undefined)return review.context.sha256===review.sourceSha256;
    if(typeof from!=='number'||typeof to!=='number')return false;
    const lineCount=sourceLineIndex?.lineCount??0;
    return Number.isInteger(from)&&Number.isInteger(to)&&from>0&&to>=from&&to<=lineCount
      &&review.context.sha256===review.sourceSha256;
  }
  function explain(context:Context){
    if(generationPaused)return;
    const selected=bindSelectionToDisplayedSource(context,source);
    const pendingDraft=current.draft;
    const question=`Explain what ${label(selected)} does and how it works.`;
    const reject=(message:string)=>{
      setCodeRequestReview({context:selected,question,snapshotId,
        sourceSha256:source?.sha256??selected.sha256??'',sourceExcerpt:'',excerptTruncated:false});
      setCodeRequestReviewError(message);
    };
    if(!source||source.path!==selected.path||source.snapshotId!==snapshotId||!source.sha256
      ||!rows.some((row)=>row.path===selected.path)){
      reject('This selection is no longer available in the captured snapshot. Close this review, reopen the file, and select its code again.');
      return;
    }
    const lineCount=sourceLineIndex?.lineCount??0;
    const hasRange=selected.from!==undefined||selected.to!==undefined;
    const rangeValid=!hasRange||(typeof selected.from==='number'&&typeof selected.to==='number'
      &&Number.isInteger(selected.from)&&Number.isInteger(selected.to)&&selected.from>=1
      &&selected.to>=selected.from&&selected.to<=lineCount);
    if(!rangeValid||selected.sha256!==source.sha256){
      reject('The selected source range could not be verified. Close this review, reopen the file, and select its code again.');
      return;
    }
    const excerptStart=hasRange?selected.from!:1;
    const excerptEnd=hasRange?selected.to!:lineCount;
    const previewEnd=Math.min(excerptEnd,excerptStart+39);
    const preview=capturedSourceLineWindow(source.contents,excerptStart,previewEnd,{maxLineCharacters:4_000,lineIndex:sourceLineIndex??undefined});
    const previewParts:string[]=[];
    let previewCodePoints=0;
    let previewTextTruncated=false;
    for(const [index,line]of preview.lines.entries()){
      const text=index===0?line:`\n${line}`;
      const characters=Array.from(text);
      const remaining=4_000-previewCodePoints;
      if(characters.length>remaining){
        if(remaining>0)previewParts.push(characters.slice(0,remaining).join(''));
        previewTextTruncated=true;
        break;
      }
      previewParts.push(text);
      previewCodePoints+=characters.length;
    }
    const sourceExcerpt=previewParts.join('');
    const excerptTruncated=excerptEnd>previewEnd||preview.omittedBefore.some(Boolean)||preview.omittedAfter.some(Boolean)||previewTextTruncated;
    if(!requireCodexConnection()){
      setCodeRequestReview({context:selected,question,snapshotId,sourceSha256:source.sha256,
        sourceExcerpt,excerptTruncated});
      setCodeRequestReviewError(null);
      return;
    }
    setConnectionNotice(null);setCodeRequestReview(null);setCodeRequestReviewError(null);
    ask(question,current.view==='flow'?'flow':'guide',[selected]);
    if(pendingDraft)updateWorkspaceDraft(projectId,snapshotId,current.id,pendingDraft);
  }
  function acceptCodeRequestReview(){
    const review=codeRequestReview;
    if(!review||generationPaused)return;
    if(!isCurrentCodeRequestReview(review)){
      setCodeRequestReviewError('This source selection changed. Close this review, reopen the captured file, and select the code again.');
      return;
    }
    // Keep the reviewed question available if the saved account/backend needs
    // attention. The connection check can open Settings and return here.
    if(!requireCodexConnection())return;
    const pendingDraft=current.draft;
    setCodeRequestReview(null);setCodeRequestReviewError(null);
    openFile(review.context);
    ask(review.question,current.view==='flow'?'flow':'guide',[review.context]);
    if(pendingDraft)updateWorkspaceDraft(projectId,snapshotId,current.id,pendingDraft);
  }
  useEffect(()=>{
    const pending=props.pendingLargeFileExplanation;
    if(generationPaused||!pending||props.workspace!=='guide-plan'||pending.projectId!==projectId||pending.snapshotId!==snapshotId
      ||handledLargeFileExplanation.current===pending.id)return;
    if(generationBackend!=='codex'){
      setConnectionNotice(generationBackend===null
        ? 'The saved AI connection could not be read. Choose a connection in Connections to explain this file.'
        : 'This file explanation needs Codex. Switch the saved connection once in Connections to continue.');
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
    setSearchFocused(false);setNarrowPane('explanation');
    if(state.grantId)void runQuestion(projectId,snapshotId,turnId,state.grantId);
    else{setConsentError(null);setConsentTurn(turnId);}
    props.onPendingLargeFileExplanationConsumed?.(pending.id);
    requestAnimationFrame(()=>answerScroll.current?.scrollTo({top:answerScroll.current.scrollHeight}));
  },[props.pendingLargeFileExplanation,props.workspace,props.manifest.files,props.onSettings,
    props.onPendingLargeFileExplanationConsumed,projectId,snapshotId,generationBackend,connection.status?.authenticated,state.grantId,generationPaused]);
  function retryTurn(turn:Turn) {
    if(generationPaused)return;
    if(!requireCodexConnection())return;
    if(turn.guideId){
      if(canContinueRepositoryGuide(turn.guide)&&state.grantId)void runRepositoryGuide(projectId,snapshotId,turn.id,state.grantId,true);
      else if(canContinueRepositoryGuide(turn.guide))setGuideConsent({id:turn.id,resume:true});
      else setGuideConsent({id:appendRepositoryGuide(projectId,snapshotId,turn.question),resume:false});
      setConsentError(null);return;
    }
    if(isRepositoryGuideIntent(turn.question)){startRepositoryGuide(turn.question);return;}
    if(state.grantId)void runQuestion(projectId,snapshotId,turn.id,state.grantId);else setConsentTurn(turn.id);
  }
  function retryGuideUnit(turnId:string,unitKey:string) {
    if(generationPaused||!requireCodexConnection())return;
    if(state.grantId)void runRepositoryGuide(projectId,snapshotId,turnId,state.grantId,true,unitKey);
    else {setGuideConsent({id:turnId,resume:true,unitKey});setConsentError(null);}
  }
  function branch(turn:Turn,question:string,context:Context[]){ask(question,current.view==='flow'?'flow':'guide',context,turn.id);}
  function exploreGuideOmissions(turn:Turn){
    if(generationPaused||!turn.guideId||!turn.guide?.canExtend||!requireCodexConnection())return;
    if(state.grantId)void runRepositoryGuide(projectId,snapshotId,turn.id,state.grantId,true);
    else {setGuideConsent({id:turn.id,resume:true});setConsentError(null);}
  }
  useEffect(()=>{
    const findInRepository=()=>{setActivity('search');setSearchFocused(false);setSearchOffset(0);setNarrowPane('explorer');onWorkspace('guide-plan');requestAnimationFrame(()=>{search.current?.focus();search.current?.select();});};
    const quickOpen=()=>{setActivity('files');setFileFilter('');setSearchFocused(false);setSearchOffset(0);setNarrowPane('explorer');onWorkspace('guide-plan');requestAnimationFrame(()=>{search.current?.focus();search.current?.select();});};
    const listener=(event:KeyboardEvent)=>{
      if(document.querySelector('[role="dialog"][aria-modal="true"]'))return;
      const mod=event.metaKey||event.ctrlKey; const k=event.key.toLowerCase();
      if(mod&&event.shiftKey&&k==='f'){event.preventDefault();event.stopImmediatePropagation();findInRepository();}
      else if(mod&&(k==='p'||k==='k')){event.preventDefault();event.stopImmediatePropagation();quickOpen();}
      if(mod&&event.shiftKey&&k==='e'){event.preventDefault();event.stopImmediatePropagation();setActivity('files');setSearchFocused(false);setSearchOffset(0);setNarrowPane('explorer');onWorkspace('guide-plan');requestAnimationFrame(()=>explorer.current?.focus());}
      if(mod&&k==='l'){event.preventDefault();event.stopImmediatePropagation();focusComposer();}
      if(mod&&['1','2','3'].includes(k)){event.preventDefault();event.stopImmediatePropagation();setView((['guide','flow','history'] as const)[Number(k)-1]);onWorkspace('guide-plan');}
      if(event.altKey&&k==='arrowleft'&&current.parentId){event.preventDefault();go(current.parentId);}
    };
    window.addEventListener('keydown',listener,true);window.addEventListener('code-atlas:search',findInRepository);return()=>{window.removeEventListener('keydown',listener,true);window.removeEventListener('code-atlas:search',findInRepository);};
  },[current.id,current.parentId]);
  const composerSearchActive=searchFocused&&[...query.trim()].length>=2;
  const term=(composerSearchActive?query:activity==='search'?repositorySearch:query).trim();
  const searchMode:CodeSearchMode=!composerSearchActive&&activity==='search'&&repositoryRegex?'regex':'code';
  const sourceSearchKey=JSON.stringify([projectId,snapshotId,term,searchMode]);
  function exploreSearch(searchQuery:string,matches:CodeSearchHit[]){
    const intent=searchExploration(searchQuery.trim(),searchMode,matches);
    const references=new Map<string,Context>();
    for(const reference of [...intent.context,...attachments]){
      const key=JSON.stringify([reference.path,reference.from??null,reference.to??null]);
      if(!references.has(key)&&references.size<MAX_CONTEXT_REFERENCES)references.set(key,reference);
    }
    ask(intent.question,undefined,[...references.values()]);
  }
  useEffect(()=>searchCapturedSource({projectId,snapshotId,query:term,mode:searchMode,native:isNativeRuntime(),offset:searchOffset},
    {hits:(hits,append)=>setSourceHits((previous)=>{
      const current=previous.key===sourceSearchKey?previous.hits:[];
      if(!append)return {key:sourceSearchKey,hits};
      const seen=new Set(current.map((hit)=>JSON.stringify([hit.path,hit.line,hit.text])));
      return {key:sourceSearchKey,hits:[...current,...hits.filter((hit)=>{const key=JSON.stringify([hit.path,hit.line,hit.text]);if(seen.has(key))return false;seen.add(key);return true;})]};
    }),limited:setSearchLimited,nextOffset:setSearchNextOffset,error:setSearchError,busy:setSearchBusy},{invoke}),[term,searchMode,projectId,snapshotId,searchOffset,sourceSearchKey]);
  const currentSourceHits=sourceHits.key===sourceSearchKey?sourceHits.hits:[];
  const results=useMemo(()=>workspaceSearchResults({query:term,mode:searchMode,files:rows,symbols:index?.symbols??[],source,captured:currentSourceHits}),[term,searchMode,rows,index,source,currentSourceHits]);
  const graph=useMemo(()=>index?projectProvenRelationshipGraph(index,manifest,new Set(rows.map((r)=>r.path))):null,[index,manifest,rows]);
  const featuredTurn=visibleTurns.find((turn)=>turn.id===featuredTurnId);
  const historyTurnId=current.focusedTurnId??featuredTurnId;
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
  useEffect(()=>{
    if(autoOpenedInitialFile.current||!sourcePaths.length||state.file||routePath||routeRange||routeSelection
      ||workspace!=='guide-plan'||current.id!=='root'||current.turns.length||current.draft||!codeOpen)return;
    // A new workspace should open on useful captured code, not an empty source
    // pane. Persisted work and explicit navigation always take precedence.
    autoOpenedInitialFile.current=true;
    openFile({path:sourcePaths[0]});
  },[sourcePaths,state.file,routePath,routeRange,routeSelection,workspace,current.id,current.turns.length,current.draft,codeOpen,openFile]);
  const fromAnchor=(a:ChapterSourceAnchor):Context=>({path:a.path,sha256:a.sourceSha256,from:a.startLine,to:a.endLine});
  const selectedTurn=featuredTurn;
  const flowOriginals=selectedTurn?.publishedSections?.length?selectedTurn.publishedSections.map(s=>chapters.find(r=>r.revisionId===s.revisionId)).filter((r):r is ChapterRevision=>!!r):chapters.filter(r=>r.revisionId===selectedTurn?.revisionId);
  const flowRevisions=selectedTurn?flowOriginals.map(r=>resolveAnswerRevision(r,selectedTurn,chapters)):[];
  const latestRevision=chooseFlowRevision(flowRevisions,selectedTurn?.reading?.flow?.revisionId,
    (revision)=>revision.sections.some((section)=>inspectInteractiveVisual(revision,section).status==='available'));
  const renderAnswer=(turn:Turn)=><Answer
    key={turn.id} chapters={chapters} turn={turn} loading={chaptersLoading} compact={turn.id!==featuredTurnId}
    revision={chapters.find((revision)=>revision.revisionId===turn.revisionId)}
    branches={state.explorations.filter((exploration)=>exploration.parentId===current.id&&exploration.originTurnId===turn.id)}
    onGo={go} onOpen={(anchor)=>openFile(fromAnchor(anchor))} onDestination={openDestination}
    onBranch={(question,context)=>branch(turn,question,context)} onExploreOmissions={exploreGuideOmissions}
    onStop={()=>stopQuestion(projectId,snapshotId,turn.id)} onRetry={()=>retryTurn(turn)} onRetryGuideUnit={retryGuideUnit}
    onReview={onReview} onFocus={(revisionId,sectionId)=>focusAnswer(turn.id,revisionId,sectionId)}
    onSettings={onSettings}
    onSection={(revisionId,sectionId)=>saveSection(turn.id,'guide',revisionId,sectionId)}
    onStoryProgress={(progress)=>savePresentation(turn.id,'stories',progress)}
    onVersion={(originalId,revisionId)=>selectVersion(turn.id,originalId,revisionId)}
    onGuideSlideConsumed={turn.guideId?((revisionId)=>{if(!generationPaused)void advanceRepositoryGuide(projectId,snapshotId,turn.id,revisionId);}):undefined}
  />;
  const activityButtons=[{id:'files',label:'Explorer',icon:Files},{id:'search',label:'Search',icon:Search},{id:'changes',label:'Changes',icon:GitBranch},{id:'history',label:'Explorations',icon:History}] as const;
  const explorationCounts=explorationSummaryCounts(state.explorations);
  const [expandedExplorations,setExpandedExplorations]=useState<Set<string>>(()=>new Set(['root']));
  const [focusedExplorationId,setFocusedExplorationId]=useState(current.id);
  useEffect(()=>setFocusedExplorationId(current.id),[current.id]);
  const renderExplorations=(parent:string|null):ReactNode=>{
    // Persisted branches can be deeply nested; index siblings once and render
    // bottom-up so history depth does not rescan the full record or recurse.
    const childrenByParent=new Map<string|null,Exploration[]>();
    for(const exploration of state.explorations){
      const siblings=childrenByParent.get(exploration.parentId)??[];
      siblings.push(exploration);childrenByParent.set(exploration.parentId,siblings);
    }
    const roots=childrenByParent.get(parent)??[];
    const currentPathIds=new Set(path.slice(1).map((exploration)=>exploration.id));
    const siblingPositions=new Map<string,{position:number;size:number}>();
    for(const siblings of childrenByParent.values()){
      siblings.forEach((exploration,index)=>siblingPositions.set(exploration.id,{position:index+1,size:siblings.length}));
    }
    const rendered=new Map<string,ReactNode>();
    const visited=new Set<string>();
    const stack=roots.slice().reverse().map((exploration)=>({exploration,depth:0,expanded:false}));
    while(stack.length){
      const frame=stack.pop()!;
      const {exploration:e,depth}=frame;
      if(frame.expanded){
        const title=e.id==='root'?projectName:e.title;
        const displayTitle=e.id==='root'?projectName:questionSummaryText(title);
        const counts=explorationCounts.get(e.id)!;
        const siblingPosition=siblingPositions.get(e.id);
        const expanded=expandedExplorations.has(e.id)||currentPathIds.has(e.id);
        const questionCount=`${counts.questions} ${counts.questions===1?'question':'questions'}`;
        const childCount=`${counts.children} ${counts.children===1?'child exploration':'child explorations'}`;
        const nested=expanded?(childrenByParent.get(e.id)??[]).map((child)=>rendered.get(child.id)):[];
        rendered.set(e.id,<li key={e.id} role="none">
          <div className="dw-exploration-row" style={{paddingLeft:10+depth*12}}>
            <button role="treeitem" data-exploration-row={e.id} data-parent-exploration={e.parentId??''}
              title={`${plainInlineCodeText(displayTitle)} · ${questionCount} · ${childCount}`}
              aria-label={`${plainInlineCodeText(displayTitle)}, ${questionCount}, ${childCount}`}
              aria-current={e.id===current.id?'page':undefined} aria-selected={e.id===current.id}
              aria-expanded={counts.children>0?expanded:undefined} aria-level={depth+1}
              aria-posinset={siblingPosition?.position} aria-setsize={siblingPosition?.size}
              tabIndex={e.id===focusedExplorationId?0:-1} className={e.id===current.id?'active':''}
              onFocus={()=>setFocusedExplorationId(e.id)} onClick={()=>go(e.id)}
              onKeyDown={(event)=>{
                if(event.altKey||event.ctrlKey||event.metaKey||event.nativeEvent.isComposing)return;
                const tree=event.currentTarget.closest('[role="tree"]');
                const items=Array.from(tree?.querySelectorAll<HTMLButtonElement>('[data-exploration-row]')??[]);
                const index=items.indexOf(event.currentTarget);
                if(event.key==='ArrowDown'||event.key==='ArrowUp'){
                  const next=items[index+(event.key==='ArrowDown'?1:-1)];
                  if(next){event.preventDefault();next.focus();}
                }else if(event.key==='ArrowRight'&&counts.children>0){
                  event.preventDefault();
                  if(!expanded)setExpandedExplorations((previous)=>new Set([...previous,e.id]));
                  else items.find((item)=>item.dataset.parentExploration===e.id)?.focus();
                }else if(event.key==='ArrowLeft'){
                  if(counts.children>0&&expanded&&!currentPathIds.has(e.id)){
                    event.preventDefault();setExpandedExplorations((previous)=>{const next=new Set(previous);next.delete(e.id);return next;});
                  }else if(e.parentId){
                    const parentItem=items.find((item)=>item.dataset.explorationRow===e.parentId);
                    if(parentItem){event.preventDefault();parentItem.focus();}
                  }
                }
              }}>
              <MessageSquare size={13}/><span><InlineCodeText text={displayTitle}/></span>
              <small title={`${questionCount} in this exploration`}>{counts.questions} Q</small>
              {counts.children>0&&<small className="dw-child-count" title={childCount}><GitBranch size={11} aria-hidden="true"/>{counts.children}</small>}
            </button>
            {counts.children>0&&<IconButton size="1" variant="ghost" color="gray" className="dw-exploration-toggle"
              aria-label={currentPathIds.has(e.id)?`${plainInlineCodeText(displayTitle)} stays open on the current path`:`${expanded?'Collapse':'Expand'} ${plainInlineCodeText(displayTitle)} explorations`}
              aria-expanded={expanded} title={currentPathIds.has(e.id)?'The current path stays visible':`${expanded?'Collapse':'Expand'} ${childCount}`}
              tabIndex={-1} disabled={currentPathIds.has(e.id)}
              onClick={(event)=>{setExpandedExplorations((previous)=>{const next=new Set(previous);if(expanded)next.delete(e.id);else next.add(e.id);return next;});event.currentTarget.parentElement?.querySelector<HTMLButtonElement>('[data-exploration-row]')?.focus();}}>
              {expanded?<ChevronDown size={13}/>:<ChevronRight size={13}/>}</IconButton>}
          </div>
          {nested.length>0&&<ul role="group">{nested}</ul>}
        </li>);
        continue;
      }
      if(visited.has(e.id))continue;
      visited.add(e.id);
      stack.push({...frame,expanded:true});
      const children=expandedExplorations.has(e.id)||currentPathIds.has(e.id)?childrenByParent.get(e.id)??[]:[];
      for(let index=children.length-1;index>=0;index--){
        const child=children[index];
        if(!visited.has(child.id))stack.push({exploration:child,depth:depth+1,expanded:false});
      }
    }
    return roots.map((exploration)=>rendered.get(exploration.id));
  };
  const sourcePane=source
    ? <CodePane key={`${source.snapshotId}:${source.path}:${source.sha256}`} path={source.path} contents={source.contents} sha256={source.sha256} theme={appearance} reveal={reveal?.sha256 && reveal.sha256!==source.sha256 ? null : reveal} canExplain={!generationPaused} onExplain={explain} onAttach={(c)=>attach({...c,sha256:source.sha256})} onClose={()=>{setCodeOpen(false);setNarrowPane('explanation');}}/>
    : <div className="dw-source-empty">{sourceBusy?<><LoaderCircle size={18}/><p>Opening source…</p></>:sourceError?<p role="alert">{sourceError}</p>:<><Code2 size={24}/><p>Open a file to read its code.</p></>}</div>;
  const resolveConversationConflict=async(choice:'current'|'saved')=>{
    if(conflictResolutionBusy)return;
    setConflictResolutionBusy(true);setConflictResolutionError(null);
    try{await resolveWorkspaceConversationConflict(projectId,snapshotId,choice);}
    catch(failure){setConflictResolutionError(errorMessageOr(failure,'The conversation could not be saved. Your current and saved copies remain available.'));}
    finally{setConflictResolutionBusy(false);}
  };
  return <ReaderProgressContext.Provider value={{
    story:(identity)=>{const owner=presentationOwner(current,identity.revisionId);return owner?{progress:owner.presentation?.stories.find(p=>sameSection(p,identity)),onProgressChange:(progress)=>savePresentation(owner.id,'stories',progress)}:undefined;},
    section:(revisionId)=>{const owner=presentationOwner(current,revisionId);return owner?{sectionId:answerReadingPosition(owner,'guide',revisionId)?.sectionId,onSectionChange:(sectionId)=>{if(chapters.some(r=>r.revisionId===revisionId&&r.sections.some(s=>s.sectionId===sectionId)))saveSection(owner.id,'guide',revisionId,sectionId);}}:undefined;},
  }}><Theme accentColor="blue" grayColor="slate" radius="medium" className={`developer-workspace dw-narrow-${narrowPane}`}>
    <nav className="dw-activity" aria-label="Repository tools">{activityButtons.map(({id,label:caption,icon:Icon})=><Tooltip content={caption} key={id}><button className={(workspace==='changes'?id==='changes':activity===id)?'active':''} aria-label={caption} aria-pressed={workspace==='changes'?id==='changes':activity===id} onClick={()=>{setActivity(id);setSearchFocused(false);setSearchOffset(0);setNarrowPane('explorer');if(id==='changes')onWorkspace('changes');else onWorkspace('guide-plan');if(id==='search')requestAnimationFrame(()=>search.current?.focus());}}><Icon size={21}/></button></Tooltip>)}<div className="dw-activity-bottom"><Tooltip content="Repository sharing"><button aria-label="Repository sharing" onClick={()=>setSharingOpen(true)}><ShieldCheck size={19}/></button></Tooltip></div></nav>
    <Group orientation="horizontal" className="dw-panels">
      <Panel id="repository-sidebar" defaultSize="220px" minSize="160px" maxSize="400px">
        <aside className="dw-sidebar" aria-label="Repository sidebar">
          <header><strong>{showChanges?'Source control':activity==='search'?'Search':activity==='history'?'Explorations':'Explorer'}</strong><Button className="dw-sidebar-back" size="1" variant="ghost" color="gray" aria-label="Back to explanation" onClick={()=>returnToGuide('explanation')}><ArrowLeft size={13}/>Back</Button></header>
          {showChanges?<><div className="dw-sidebar-section">Commit history <span>{commits.length ? `${commits.length} recent`:''}</span><Button size="1" variant="ghost" disabled={gitBusy} onClick={()=>setGitReload((value)=>value+1)}>Refresh</Button></div><div className="dw-commits" role="group" aria-label="Recent commits. Use Up and Down arrows to move between commits.">{gitBusy&&<p role="status">Loading commits… <Button size="1" variant="ghost" onClick={()=>{gitRequest.current?.abort();setGitBusy(false);setGitStopped(true);}}>Stop</Button></p>}{gitStopped&&<p role="status">Loading stopped.{commits.length>0?' Your previous history is still available.':''}</p>}{gitError&&<p role="alert">{gitError}</p>}{commitHistoryEmpty&&<p role="status">{commitHistoryEmpty}</p>}{commits.map((c)=><button key={c.oid} ref={(button)=>{if(button)commitButtons.current.set(c.oid,button);else commitButtons.current.delete(c.oid);}} className="dw-commit" disabled={!c.parents.length} tabIndex={c.oid===rovingCommitId?0:-1} title={c.parents.length?`${c.subject}\n${c.oid}`:`${c.subject} — initial commit; parent comparison unavailable`} onFocus={()=>setFocusedCommitId(c.oid)} onKeyDown={(event)=>{if(event.key!=='ArrowDown'&&event.key!=='ArrowUp')return;event.preventDefault();const direction=event.key==='ArrowDown'?1:-1;const currentIndex=commits.findIndex((commit)=>commit.oid===c.oid);const nextIndex=nextFocusableCommitIndex(commits,currentIndex,direction);const next=commits[nextIndex];if(next&&nextIndex!==currentIndex)commitButtons.current.get(next.oid)?.focus();}} onClick={()=>{if(!c.parents.length)return;onCompare(c.parents[0],c.oid);setNarrowPane('explanation');onWorkspace('changes');}}><span className={`dw-commit-line ${c.parents.length>1?'merge':''}`}><GitCommitHorizontal size={16}/></span><span><strong>{c.subject||'Untitled commit'}</strong><small>{c.author} · {c.oid.slice(0,7)}{c.parents.length>1?' · merge':!c.parents.length?' · initial commit':''}</small></span></button>)}</div></>:activity==='history'?<ul className="dw-explorations" role="tree" aria-label="Explorations">{renderExplorations(null)}</ul>:<>
            <div className="dw-sidebar-search"><TextField.Root ref={search} size="1" value={activity==='search'?repositorySearch:fileFilter} placeholder={activity==='search'?(repositoryRegex?'Code pattern…':'Files, symbols, code…'):'Find a file…'} maxLength={activity==='search'?240:4096} aria-label={activity==='search'?'Search repository':'Filter files'} onFocus={()=>{setSearchFocused(false);setSearchOffset(0);}} onChange={(e)=>{if(activity==='search'){setRepositorySearch(e.target.value);setSearchOffset(0);}else setFileFilter(e.target.value);}} onKeyDown={(event)=>{if(activity==='search'&&event.altKey&&event.code==='KeyR'){event.preventDefault();toggleRepositoryRegex();setSearchOffset(0);return;}if(activity==='files'&&event.key==='Enter'&&openQuickMatch(fileFilter)){event.preventDefault();return;}if(activity==='search'&&event.key==='Enter'&&!event.shiftKey&&!event.nativeEvent.isComposing){if(!searchBusy&&!searchError&&!searchLimited&&searchNextOffset===null&&results.length===1){event.preventDefault();openFile(results[0]);return;}if(!generationPaused&&shouldOfferExploreWithAI(repositorySearch,searchMode,searchBusy,searchError,results.length)){event.preventDefault();exploreSearch(repositorySearch,results);}}}}><TextField.Slot><Search size={13}/></TextField.Slot>{activity==='search'&&<TextField.Slot side="right"><Tooltip content="Use regular expression (⌥R)"><IconButton type="button" size="1" variant="ghost" color={repositoryRegex?'blue':'gray'} aria-label="Use regular expression" aria-pressed={repositoryRegex} onClick={()=>{toggleRepositoryRegex();setSearchOffset(0);}}><span aria-hidden="true">.*</span></IconButton></Tooltip></TextField.Slot>}</TextField.Root></div>
            {activity==='search'?<div className="dw-search-results">{composerSearchActive?<p className="dw-search-count" role="status">Searching your question above. Results appear below the chat box.</p>:<><small>{repositoryRegex?'Captured code · regular expression':'Files, symbols and captured code'}</small>{searchBusy&&<p role="status">Searching…</p>}{repositorySearch.trim()&&!searchBusy&&!searchError&&results.length>0&&<p className="dw-search-count" role="status">{results.length>=80||(searchLimited&&searchNextOffset===null)?searchLimitMessage(results.length):searchResultsMessage(results.length,searchLimited&&searchNextOffset!==null)}</p>}{searchError&&<p role="status">{searchError}</p>}{repositorySearch&&!searchBusy&&!searchError&&results.length===0&&<p>No matching code in this search.</p>}{results.map((r,i)=><button key={i} onClick={()=>openFile(r)}><FileCode2 size={14}/><span><strong>{r.text}</strong><small>{r.path}{r.from?`:${r.from}`:''}</small></span></button>)}{searchLimited&&searchNextOffset!==null&&results.length<80&&<Button size="1" variant="soft" disabled={searchBusy} onClick={()=>setSearchOffset(searchNextOffset)}>Load more matches</Button>}{!generationPaused&&shouldOfferExploreWithAI(repositorySearch,searchMode,searchBusy,searchError,results.length)&&<Button size="1" variant="soft" onClick={()=>exploreSearch(repositorySearch,results)}><MessageSquare size={13}/>Explore with AI</Button>}</>}</div>:<><div className="dw-sidebar-section">{projectName}</div><WorkspaceExplorer paths={sourcePaths} query={fileFilter} selectedPath={state.file} expanded={openFolders} onExpandedChange={setOpenFolders} onOpen={(path)=>openFile({path})} treeRef={explorer}/></>}
          </>}
        </aside>
      </Panel><Separator className="dw-separator" aria-label="Resize repository sidebar"/>
      <Panel id="reading-workspace" minSize="320px"><section className="dw-main" aria-label="Code exploration">
        <nav className="dw-path" aria-label="Exploration path"><IconButton size="1" variant="ghost" color="gray" aria-label="Back one exploration" disabled={!current.parentId} onClick={()=>current.parentId&&go(current.parentId)}><ArrowLeft size={14}/></IconButton>{path.map((e,i)=><span key={e.id}>{i>0&&<ChevronRight size={12}/>}<button aria-current={e.id===current.id?'page':undefined} onClick={()=>go(e.id)}>{e.id==='root'?projectName:<InlineCodeText text={e.title}/>}</button></span>)}{path.length>1&&<Badge size="1" color="gray">Level {path.length}</Badge>}{tool&&<span className="dw-tool-label">{workspace==='changes'?'Changes':workspace==='chapter-studio'?'Edit explanation':'Player'}</span>}</nav>
        {conversationUnavailable&&<div className="dw-conversation-warning" role="status"><MessageSquare size={16}/><p><strong>Saved conversation unavailable.</strong> Your saved work has not been changed. You can browse and search this snapshot; new questions cannot be saved.</p></div>}
        {conversationConflict&&<div className="dw-conversation-warning dw-conversation-conflict" role="alert">
          <MessageSquare size={16}/><div><strong>Changes in two windows</strong><p>Separate edits will merge. Choose which value to keep for {conversationConflict.overlappingChanges} overlapping change{conversationConflict.overlappingChanges===1?'':'s'}. New questions are paused until this is saved.</p>{conflictResolutionError&&<p className="dw-error" role="status">{conflictResolutionError}</p>}</div>
          <Button size="1" variant="soft" disabled={conflictResolutionBusy} title="Keep this window's value only where both windows changed the same field" onClick={()=>void resolveConversationConflict('current')}>{conflictResolutionBusy?'Saving…':'Keep this window'}</Button>
          <Button size="1" variant="soft" color="gray" disabled={conflictResolutionBusy} title="Keep the saved value only where both windows changed the same field" onClick={()=>void resolveConversationConflict('saved')}>Keep saved value</Button>
        </div>}
        <div className="dw-composer" onDragOver={(e)=>{if(!conversationUnavailable&&e.dataTransfer.types.includes('application/x-code-atlas-context'))e.preventDefault();}} onDrop={(e)=>{e.preventDefault();if(conversationUnavailable)return;try{const c=JSON.parse(e.dataTransfer.getData('application/x-code-atlas-context')) as Context;if(rows.some((r)=>r.path===c.path))attach(c);}catch{/* Ignore unrelated drops. */}}}>
          <Flex gap="2" mb="2"><Button size="1" variant="soft" disabled={generationPaused} onClick={()=>{const action=contextualGuideAction(query,attachments,!!current.parentId);if(action.kind==='repository-guide')startRepositoryGuide(action.intent);else ask(action.intent,'guide');}}><BookOpen size={14}/>Guide me</Button><Button size="1" variant="soft" color="gray" disabled={generationPaused&&!!query.trim()} onClick={()=>{if(!query.trim())showView('flow');else ask(query,'flow');}}><Network size={14}/>Show the flow</Button><IconButton className="dw-narrow-toggle" size="1" variant="ghost" color="gray" aria-label={narrowPane==='source'?'Show explanation':'Show source'} onClick={()=>{if(narrowPane==='source')setNarrowPane('explanation');else{setCodeOpen(true);setNarrowPane('source');}}}><Code2 size={15}/></IconButton></Flex>
          <div className="dw-input">{attachments.length>0&&<div className="dw-context">{attachments.map((c,i)=><Badge key={i} color="gray"><FileCode2 size={12}/><code>{label(c)}</code><button aria-label={`Remove ${label(c)}`} onClick={()=>setAttachments((all)=>all.filter((_,n)=>n!==i))}><X size={12}/></button></Badge>)}</div>}
            <textarea ref={composer} data-focus-key="workspace-composer" rows={1} value={query} maxLength={1000} disabled={conversationUnavailable} aria-label="Ask about this codebase" placeholder={conversationUnavailable?'Saved explanations are read-only for now':conversationConflict?'Resolve the saved conversation conflict to send':'Ask about this codebase'} onChange={(e)=>{e.target.setCustomValidity('');setQuery(e.target.value);}} onFocus={()=>{setSearchFocused(true);setSearchOffset(0);}} onKeyDown={(e)=>{if(e.key==='Escape')setSearchFocused(false);if(e.key==='Enter'&&!e.shiftKey&&!e.nativeEvent.isComposing){e.preventDefault();ask(query);}}}/>
            <Tooltip content="Attach open file"><IconButton size="1" variant="ghost" color="gray" disabled={!source||conversationUnavailable} aria-label="Attach open file" onClick={()=>source&&attach({path:source.path,sha256:source.sha256})}><Paperclip size={15}/></IconButton></Tooltip><IconButton size="1" aria-label="Send question" disabled={!query.trim()||generationPaused} onClick={()=>ask(query)}><ArrowUp size={15}/></IconButton>
          </div>
          {connectionNotice&&<div className="dw-conversation-warning" role="status"><MessageSquare size={16}/><p>{connectionNotice}</p><Button size="1" variant="soft" onClick={onSettings}>Connections</Button></div>}
          {draftNotice&&<div className="dw-conversation-warning" role="status"><MessageSquare size={16}/><p>{draftNotice}</p><Button size="1" variant="ghost" onClick={()=>setDraftNotice(null)}>Dismiss</Button></div>}
          {!conversationUnavailable&&searchFocused&&query.trim().length>1&&<div className="dw-suggestions"><small>{searchBusy?'Searching…':searchError?'Search unavailable':results.length?'In your code':'No matching code in this search'}</small>{results.slice(0,4).map((r,i)=><button key={i} onMouseDown={(e)=>e.preventDefault()} onClick={()=>openFile(r)}><FileCode2 size={13}/><span>{r.path}{r.from?`:${r.from}`:''}<small>{r.text}</small></span><ArrowRight size={13}/></button>)}{!generationPaused&&shouldOfferExploreWithAI(query,searchMode,searchBusy,searchError,results.length)&&<button onMouseDown={(e)=>e.preventDefault()} onClick={()=>exploreSearch(query,results)}><MessageSquare size={13}/>Explore with AI<ArrowRight size={13}/></button>}</div>}
        </div>
        {tool?<Group orientation="horizontal" className="dw-reading dw-tool-reading">
          {codeOpen&&<><Panel id="source" defaultSize="43%" minSize="220px">{sourcePane}</Panel><Separator className="dw-separator" aria-label="Resize source and explanation"/></>}
          <Panel id="explanations" minSize="280px"><div className="dw-tool-content"><Button className="dw-return" size="1" variant="ghost" onClick={()=>returnToGuide('explorer')}><ArrowLeft size={13}/>Back to exploration</Button><GitHistoryEmptyContext.Provider value={commitHistoryEmpty}>{children}</GitHistoryEmptyContext.Provider></div></Panel>
        </Group>:<Group orientation="horizontal" className="dw-reading">
          {codeOpen&&<><Panel id="source" defaultSize="43%" minSize="220px">{sourcePane}</Panel><Separator className="dw-separator" aria-label="Resize source and explanation"/></>}
          <Panel id="explanations" minSize="280px"><Tabs.Root className="dw-explanation" value={current.view} onValueChange={(value)=>setView(value as View)}><div className="dw-viewbar"><Tabs.List aria-label="Exploration views"><Tabs.Trigger value="guide"><BookOpen size={13}/>Guide</Tabs.Trigger><Tabs.Trigger value="flow"><Network size={13}/>Flow</Tabs.Trigger><Tabs.Trigger value="history"><MessageSquare size={13}/>Chat history</Tabs.Trigger></Tabs.List>{!codeOpen&&<IconButton size="1" variant="ghost" color="gray" aria-label="Show code pane" onClick={()=>{setCodeOpen(true);setNarrowPane('source');}}><Code2 size={15}/></IconButton>}</div>
            <div className="dw-answer-scroll" ref={answerScroll} onScroll={(event)=>{if(restoringScroll.current)return;const top=event.currentTarget.scrollTop;update((s)=>rememberScroll(s,current.id,current.view,top));}}>
              {chapterError&&<p className="dw-error" role="alert">{chapterError}<button onClick={()=>{void refreshChapters();}}>Retry</button></p>}
              <Tabs.Content value="guide">
                {visibleTurns.length>0?visibleTurns.map(renderAnswer):<section className="dw-welcome"><BookOpen size={25}/><h1>Get to know {projectName}</h1><p>{conversationUnavailable?'This session is read-only. Browse a file or search the captured source.':'Ask a question, or use Guide me through the codebase. Open a file to follow its source.'}</p>{!generationPaused&&source&&<button className="dw-text-action" onClick={()=>explain({path:source.path,sha256:source.sha256})}>Explain {source.path.split('/').at(-1)}</button>}</section>}
              </Tabs.Content>
              <Tabs.Content value="flow">{flowRevisions.length>1&&<div className="dw-flow-choice"><label htmlFor="answer-flow-section">Explanation</label><select id="answer-flow-section" value={latestRevision?.revisionId??''} onChange={(event)=>{const revision=flowRevisions.find((r)=>r.revisionId===event.target.value);if(selectedTurn&&revision?.sections[0])saveSection(selectedTurn.id,'flow',revision.revisionId,revision.sections[0].sectionId);}}>{flowRevisions.map((revision)=><option key={revision.revisionId} value={revision.revisionId}>{plainInlineCodeText(revision.sections[0]?.title??revision.title)}</option>)}</select></div>}{indexError&&!latestRevision&&<p role="status" className="dw-error">{indexError}</p>}{latestRevision?<EvidenceFlow revision={latestRevision} saved={selectedTurn?answerReadingPosition(selectedTurn,'flow',latestRevision.revisionId):undefined} progress={selectedTurn?.presentation?.diagrams} onProgress={(progress)=>{if(selectedTurn)savePresentation(selectedTurn.id,'diagrams',progress);}} onSection={(sectionId)=>{if(selectedTurn)saveSection(selectedTurn.id,'flow',latestRevision.revisionId,sectionId);}} onBranch={(question,context)=>{if(selectedTurn)branch(selectedTurn,question,context);}} theme={appearance} onOpen={(a)=>openFile(fromAnchor(a))}/>:source?<FileFlow index={index} edges={graph?.edges??[]} loading={indexBusy} path={source.path} focus={reveal?.path===source.path&&(!reveal.sha256||reveal.sha256===source.sha256)?reveal:null} showAll={current.fileFlow?.path===source.path&&current.fileFlow.showAll} onShowAllChange={(showAll)=>update((s)=>rememberFileFlow(s,current.id,source.path,showAll))} onOpenFile={(p,_snapshot,range)=>openFile({path:p,from:range?.startLine,to:range?.endLine,sha256:range?.sourceSha256})}/>:<p>Open a file or ask a question to explore its flow.</p>}</Tabs.Content>
              <Tabs.Content value="history"><div className="dw-history">{path.map((e)=><section key={e.id}><button className="dw-history-level" onClick={()=>go(e.id)}><MessageSquare size={13}/>{e.id==='root'?projectName:<InlineCodeText text={e.title}/>}<small>Level {ancestors(state,e.id).length}</small></button>{e.turns.map((turn)=>{const entry=historyEntry(turn,availableRevisionIds,chaptersLoading);const isCurrent=e.id===current.id&&turn.id===historyTurnId;return <article key={turn.id} className={isCurrent?'is-current':''}><details className="dw-history-question"><summary><strong><InlineCodeText text={questionSummaryText(turn.question)}/></strong></summary><p><InlineCodeText text={turn.question}/></p></details><p><InlineCodeText text={entry.text}/></p><button className="dw-text-action" aria-current={isCurrent?'page':undefined} onClick={()=>{update((s)=>openWorkspaceQuestion(s,e.id,turn.id,isCurrent?historyTurnId:undefined));setNarrowPane('explanation');onWorkspace('guide-plan');}}>{entry.action}<ArrowRight size={13}/>{isCurrent&&<small className="dw-history-current">Current</small>}</button>{turn.runIds?.length ? <AnswerDiagnostics project={projectId} snapshot={snapshotId} runId={turn.runIds.at(-1)!}/> : null}</article>;})}</section>)}{!path.some((e)=>e.turns.length>0)&&chapters.length===0&&<p>Your questions and explanations will appear here.</p>}{chapters.length>0&&<details className="dw-library"><summary>Saved explanations <span>{chapters.length}</span></summary>{chapters.slice().sort((a,b)=>b.createdAtUnixMs-a.createdAtUnixMs).map((r)=><button key={r.revisionId} onClick={()=>{update((s)=>openWorkspaceRevision(s,r.revisionId,r.title,chapters));setNarrowPane('explanation');onWorkspace('guide-plan');}}><BookOpen size={13}/><span><InlineCodeText text={r.title}/></span><small>{r.lifecycle}</small></button>)}</details>}</div>{chapters.some((revision)=>revision.lifecycle==='accepted'&&revision.sections.length>0)&&<div className="dw-history-export"><OfflineExport projectId={projectId} revisions={chapters}/></div>}</Tabs.Content>
            </div>
          </Tabs.Root></Panel>
        </Group>}
      </section></Panel>
    </Group>
      <Dialog.Root open={!!codeRequestReview} onOpenChange={(open)=>{if(!open){setCodeRequestReview(null);setCodeRequestReviewError(null);}}}><Dialog.Content maxWidth="620px"><Dialog.Title>Review your code question</Dialog.Title><Dialog.Description size="2">Nothing is sent until you choose Ask about this code.</Dialog.Description>{codeRequestReview&&<><section className="provider-context-picker"><div className="provider-context-heading"><strong>Question</strong></div><p>{codeRequestReview.question}</p><div className="provider-context-heading"><strong>Captured source</strong><span>{codeRequestReview.context.path}</span></div><code>{codeRequestReview.context.path}{codeRequestReview.context.from?`:${codeRequestReview.context.from}–${codeRequestReview.context.to}`:' · entire file'}</code>{codeRequestReview.sourceExcerpt&&<details className="dw-error-details"><summary>Preview selected code{codeRequestReview.excerptTruncated?' · limited':''}</summary><pre style={{maxHeight:220,overflow:'auto',whiteSpace:'pre-wrap',fontFamily:'var(--mono)',fontSize:12}}>{codeRequestReview.sourceExcerpt}{codeRequestReview.excerptTruncated?'\n… preview is limited; see the selected range in the source pane.':''}</pre></details>}<p className="provider-small-note">Your connected Codex account can inspect relevant captured code under this snapshot’s existing sharing permission. The live working copy is not sent. Account quota applies.</p></section>{codeRequestReviewError&&<p className="dw-error" role="alert">{codeRequestReviewError}</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray">Cancel</Button></Dialog.Close><Button disabled={!isCurrentCodeRequestReview(codeRequestReview)||generationPaused} onClick={acceptCodeRequestReview}>Ask about this code</Button></Flex></>}</Dialog.Content></Dialog.Root>
    <Dialog.Root open={!!consentTurn} onOpenChange={(open)=>{if(!open&&!consentBusy){if(consentTurn)stopQuestion(projectId,snapshotId,consentTurn);setConsentTurn(null);}}}><Dialog.Content maxWidth="520px"><Dialog.Title>Explain {projectName} with Codex</Dialog.Title><Dialog.Description size="2">Codex can read relevant code from this captured repository snapshot to answer your questions. Your live working files stay private, and you can revoke access at any time.</Dialog.Description><WorkspaceSharingDetails kind="question"/>{consentError&&<p role="alert" className="dw-error">{errorMessageOr(consentError,'The question could not be started.')}</p>}{generationPaused&&<p role="status">Resolve the saved conversation conflict before starting this question.</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray" disabled={consentBusy}>Cancel</Button></Dialog.Close><Button disabled={consentBusy||generationPaused} onClick={()=>{if(!consentTurn||generationPaused)return;const id=consentTurn;setConsentBusy(true);setConsentError(null);void authorizeWorkspace(projectId,snapshotId).then((grant)=>{setConsentTurn(null);void runQuestion(projectId,snapshotId,id,grant);}).catch((failure)=>setConsentError(errorMessageOr(failure,'Connect Codex in the header, then try again.'))).finally(()=>setConsentBusy(false));}}>{consentBusy?'Connecting…':'Allow and explain'}</Button></Flex>{consentError&&<Button variant="ghost" onClick={()=>{if(consentTurn)stopQuestion(projectId,snapshotId,consentTurn);setConsentTurn(null);requestAnimationFrame(onSettings);}}>Open connection settings</Button>}</Dialog.Content></Dialog.Root>
    <Dialog.Root open={!!guideConsent} onOpenChange={(open)=>{if(!open&&!consentBusy){if(guideConsent&&!guideConsent.resume)update((state)=>discardUnstartedGuide(state,guideConsent.id));setGuideConsent(null);}}}><Dialog.Content maxWidth="520px"><Dialog.Title>{guideConsent?.unitKey?'Retry one guide slide':guideConsent?.resume?'Continue your guide':'Build a guide to '+projectName}</Dialog.Title><Dialog.Description size="2">A clear, illustrated guide will appear here, with each slide linked to its code. Earlier slides stay available if a later part needs attention.</Dialog.Description><p className="dw-permission-note">See the planned topics first. Follow each slide through its code links while the next ones take shape.</p><WorkspaceSharingDetails kind="guide"/>{consentError&&<p role="alert" className="dw-error">{errorMessageOr(consentError,'This guide could not be started.')}</p>}{generationPaused&&<p role="status">Resolve the saved conversation conflict before continuing this guide.</p>}<Flex justify="end" gap="3" mt="4"><Dialog.Close><Button variant="soft" color="gray" disabled={consentBusy}>Cancel</Button></Dialog.Close><Button disabled={consentBusy||generationPaused} onClick={()=>{if(!guideConsent||generationPaused)return;const pending=guideConsent;setConsentBusy(true);setConsentError(null);void (state.grantId?Promise.resolve(state.grantId):authorizeWorkspace(projectId,snapshotId)).then((grant)=>{setGuideConsent(null);void runRepositoryGuide(projectId,snapshotId,pending.id,grant,pending.resume,pending.unitKey);}).catch((error)=>setConsentError(errorMessageOr(error,'Connect Codex in the header, then try again.'))).finally(()=>setConsentBusy(false));}}>{consentBusy?'Connecting…':guideConsent?.unitKey?'Retry slide':guideConsent?.resume?'Continue guide':'Build guide'}</Button></Flex></Dialog.Content></Dialog.Root>
    <Dialog.Root open={sharingOpen} onOpenChange={setSharingOpen}><Dialog.Content maxWidth="460px"><Dialog.Title>Repository sharing</Dialog.Title><Dialog.Description size="2">{state.grantId?'Codex can explore captured code in this snapshot when you ask a question. Your live working copy is not included.':'Codex has no workspace sharing permission for this snapshot.'}</Dialog.Description>{state.grantId&&<Button mt="4" variant="soft" color="red" onClick={()=>{void revokeWorkspace(projectId,snapshotId).then(()=>setSharingOpen(false)).catch((failure)=>setConsentError(errorMessageOr(failure,'Permission could not be revoked.')));}}>Revoke permission and stop questions</Button>}{consentError&&<p role="alert">{consentError}</p>}<Flex justify="end" mt="4"><Dialog.Close><Button variant="soft" color="gray">Done</Button></Dialog.Close></Flex></Dialog.Content></Dialog.Root>
  </Theme></ReaderProgressContext.Provider>;
}

interface AnswerProps {chapters?:ChapterRevision[];hideQuestion?:boolean;turn:Turn; loading?:boolean; revision?:ChapterRevision; branches:Exploration[]; compact?:boolean;onGo:(id:string)=>void;onOpen:(a:ChapterSourceAnchor)=>void;onBranch:(question:string,context:Context[])=>void;onExploreOmissions:(turn:Turn)=>void;onStop:()=>void;onRetry:()=>void;onRetryGuideUnit?:(turnId:string,unitKey:string)=>void;onReview:(r:ChapterRevision)=>void;onFocus:(revisionId:string,sectionId:string)=>void;onSettings:()=>void;onDestination:(d:HotspotDestination)=>void;onSection:(revisionId:string,sectionId:string)=>void;onStoryProgress:(progress:StoryProgress)=>void;onVersion:(originalRevisionId:string,revisionId:string)=>void;onGuideSlideConsumed?:(revisionId:string)=>void;branchLabel?:string;coverageSummary?:string}
function QuestionDisclosure({question}:{question:string}){
  return <details className="dw-user-question"><summary><MessageSquare size={14}/><span><InlineCodeText text={questionSummaryText(question)}/></span><ChevronRight size={13}/></summary><p><InlineCodeText text={question}/></p></details>;
}
function WorkspaceSharingDetails({kind}:{kind:'question'|'guide'}) {
  return <details className="dw-error-details"><summary>What can be shared</summary><p className="dw-permission-note">Relevant captured code, indexed names and documentation hints from this snapshot may be sent through your connected Codex account. Each {kind==='guide'?'guide slide':'question'} is limited to 250 KB across 12 files; large files can be read in portions. Secret-like paths and excluded files stay blocked. Your account quota applies. The live working copy is not sent. Manage or revoke this snapshot's permission in Repository sharing. Code Atlas does not run repository code.</p></details>;
}
function Answer(props:AnswerProps){
  const {turn,chapters,branches}=props;
  const compact=Boolean(props.compact);
  const [guideOutlineState,setGuideOutlineState]=useState<{compact:boolean;open:boolean}|null>(null);
  const guideOutlineOpen=guideOutlineState?.compact===compact?guideOutlineState.open:!compact;
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
    const sectionId=(revision&&answerReadingPosition(turn,'guide',revision.revisionId)?.sectionId)
      ?? revision?.sections[0]?.sectionId;
    if(revision&&sectionId)props.onFocus(revision.revisionId,sectionId);
  };
  const hasNextGuideSlide=selectedGuideSlide<guideSlides.length-1;
  const canBuildNextGuideSlide=Boolean(turn.guideId&&canContinueRepositoryGuide(turn.guide));
  const lastAvailableItemIndex=props.loading?-1:lastAvailableAnswerItemIndex(sectionItems);
  const capacityError=!!turn.error&&/retained locally|saved answers|(?:answer|saved explanations).*storage limit|answer.*directory limit/i.test(turn.error);
  const failureMessage=explanationFailureMessage(turn.failureCode);
  const progressText=questionProgressText(turn);
  const guideCoverageLabel=repositoryGuideCoverageLabel(turn.guide,sections.length>0);
  const completedSequenceLabel=()=>{
    if(turn.guideId){
      if(turn.status==='partial'||guideCoverageLabel==='Partial coverage')return 'Guide partly complete';
      if(turn.status==='stopped')return 'Guide stopped';
      if(turn.status==='interrupted')return 'Guide interrupted';
      if(turn.status==='failed')return 'Guide could not finish';
      return 'Guide complete';
    }
    if(turn.status==='partial')return 'Answer partly complete';
    if(turn.status==='stopped')return 'Answer stopped';
    if(turn.status==='interrupted')return 'Answer interrupted';
    if(turn.status==='failed')return 'Answer could not finish';
    return 'Answer complete';
  };
  const nextGuideSlideLabel=hasNextGuideSlide?`Next ${turn.guideId?'slide':'section'}`
    :canBuildNextGuideSlide?'Build next slide'
    :turn.status==='working'?`Another ${turn.guideId?'slide':'section'} is being prepared`
    :completedSequenceLabel();
  const uncoveredGuideTopics=repositoryGuideUncoveredTopics(turn.guide);
  const canExtendOmissions=!!(turn.guideId&&turn.guide?.canExtend===true&&!canContinueRepositoryGuide(turn.guide)
    &&(turn.status==='ready'||turn.status==='partial')&&guideSlideItemIndex===lastAvailableItemIndex);
  const omissionSummary=canExtendOmissions?repositoryGuideOmissionSummary(uncoveredGuideTopics):null;
  const guideCoverageSummary=guideCoverageLabel==='Partial coverage'
    ?omissionSummary??'Some planned topics were not completed.'
    :null;
  return <section className="dw-progressive-answer" data-question-id={turn.id} tabIndex={-1} aria-label={`Question: ${plainInlineCodeText(questionSummaryText(turn.question))}`}>
    <QuestionDisclosure question={turn.question}/>
    <div className="dw-answer-progress" role="status">{progressText?<><LoaderCircle size={14}/><span>{progressText}</span><Button size="1" variant="ghost" color="gray" onClick={props.onStop}><Square size={12}/>Stop</Button></>:turn.status==='ready'?null:turn.status==='waiting'?<span>{sections.length} {turn.guideId?'slide':'section'}{sections.length===1?'':'s'} ready. The next {turn.guideId?'slide':'section'} builds as you read or listen.</span>:<><span>{turn.status==='stopped'?'Stopped.':turn.status==='interrupted'?'Interrupted.':turn.status==='partial'?'Some details could not be completed.':failureMessage??'This question could not be completed.'} {sections.length>0?'Your saved sections are available below.':''}</span><Button size="1" variant="ghost" onClick={props.onRetry}>{turn.guideId?(canContinueRepositoryGuide(turn.guide)?'Continue guide':'Start a fresh guide'):isRepositoryGuideIntent(turn.question)?'Start a fresh guide':'Try again'}</Button></>}</div>
    {turn.guide&&turn.guide.units.length>0&&<details className="dw-guide-outline" open={guideOutlineOpen} onToggle={(event)=>setGuideOutlineState({compact,open:event.currentTarget.open})}>
      <summary><InlineCodeText text={turn.guide.title}/>{guideCoverageLabel&&<span className={`dw-guide-status${guideCoverageLabel==='Planned topics complete'?' is-complete':''}`} role="status">{guideCoverageLabel==='Planned topics complete'?'Complete':guideCoverageLabel==='Partial coverage'?'Partial':'Incomplete'}</span>}<span className="dw-guide-outline-count">{turn.guide.units.length} topics · {guideSlides.length} slides ready</span></summary>
      <ol aria-label="Planned guide topics">{turn.guide.units.map((unit)=>{const status=unit.status==='complete'?'Ready':unit.status==='working'?'In progress':unit.status==='partial'?'Incomplete':unit.status==='failed'?'Could not build':unit.status==='interrupted'?'Interrupted':unit.status==='stopped'?'Stopped':'Not yet available';return <li key={unit.key}><span><InlineCodeText text={unit.question}/></span><small>{status}</small>{unit.canRetry&&props.onRetryGuideUnit&&<Button size="1" variant="soft" aria-label={`Retry slide: ${plainInlineCodeText(unit.question)}`} onClick={()=>props.onRetryGuideUnit?.(turn.id,unit.key)}>Retry slide</Button>}</li>;})}</ol>
      <p>Follow these parts in order, or open another file to explore a different part of the code.</p>
      {turn.guide.omissions.map((note)=><p key={note}>{note}</p>)}
    </details>}
    {turn.notes?.map((note)=><p className="dw-answer-note" key={note}>{note}</p>)}
    {capacityError&&<AnswerStorage/>}
    {turn.error&&<details className="dw-error-details"><summary>Details</summary><p>{explanationErrorMessage(turn.error)}</p></details>}
    {sectionSequence&&props.loading&&sections.length>0&&<p className="dw-guide-loading" role="status">{turn.guideId?'Restoring saved slides…':'Restoring saved answer sections…'}</p>}
    {sectionSequence&&guideSlide&&<nav className="dw-guide-slide-nav" aria-label={turn.guideId?'Guide slides':'Answer sections'}><Button size="1" variant="ghost" color="gray" disabled={selectedGuideSlide<=0} onClick={()=>selectGuideSlide(selectedGuideSlide-1)}><ArrowLeft size={13}/>Previous</Button><span role="status" aria-live="polite" aria-atomic="true">{turn.guideId?'Slide':'Section'} {selectedGuideSlide+1} of {guideSlides.length}{progressText?' · more on the way':''}</span><Button size="1" variant="soft" disabled={!hasNextGuideSlide&&!canBuildNextGuideSlide} aria-label={nextGuideSlideLabel} onClick={()=>{if(hasNextGuideSlide){if(turn.guideId)props.onGuideSlideConsumed?.(guideSlide.revisionId);selectGuideSlide(selectedGuideSlide+1);}else if(canBuildNextGuideSlide)props.onGuideSlideConsumed?.(guideSlide.revisionId);}}>{nextGuideSlideLabel}<ArrowRight size={13}/></Button></nav>}
    {sectionSequence&&guideSlide&&<AnswerSection {...props} key={`${guideSlide.runId}:${guideSlide.generationId}`} hideQuestion loading={props.loading} revision={guideSlideRevision}
      turn={{...turn,status:'ready',revisionId:guideSlide.revisionId,notes:[],error:undefined}}
      branches={guideSlideItemIndex===lastAvailableItemIndex?branches:[]}
      branchLabel={canExtendOmissions?'Explore uncovered topics':undefined}
      coverageSummary={guideCoverageSummary??undefined}
      onBranch={canExtendOmissions?()=>props.onExploreOmissions(turn):props.onBranch}/>}
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
      <ExplorationBranchLinks branches={branches} onGo={props.onGo}/>
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
function ExplorationBranchLinks({branches,onGo}:{branches:Exploration[];onGo:(id:string)=>void}){
  const recent=branches.slice(-5).reverse();
  const earlier=branches.slice(0,-5).reverse();
  const renderLink=(exploration:Exploration)=><button className="dw-branch-link" key={exploration.id} onClick={()=>onGo(exploration.id)}><span className="dw-branch-connector">↳</span><span><InlineCodeText text={exploration.title}/></span><small>{exploration.turns.length} question{exploration.turns.length===1?'':'s'}</small><ChevronRight size={13}/></button>;
  return <>{recent.map(renderLink)}{earlier.length>0&&<details className="dw-branch-archive"><summary>Show {earlier.length} earlier explorations</summary><div>{earlier.map(renderLink)}</div></details>}</>;
}
function AnswerSection({turn,revision:original,chapters=[],loading,branches,compact,hideQuestion,onGo,onOpen,onBranch,onStop,onRetry,onReview,onSettings,onDestination,onSection,onStoryProgress,onVersion,onFocus,onGuideSlideConsumed,branchLabel,coverageSummary}:AnswerProps){
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
  const terminalStatus=turn.status==='failed'||turn.status==='interrupted'||turn.status==='stopped';
  const terminalMessage=turn.status==='stopped'?'Stopped. Saved explanations are preserved.':turn.status==='interrupted'?'This question was interrupted. Saved explanations are preserved.':failureMessage??(knownEvidenceError?readableError:'This explanation could not be completed. Your saved work is preserved.');
  const terminalNotice=terminalStatus&&<><p className="dw-error" role="status">{terminalMessage}</p>{turn.error&&!knownEvidenceError&&!failureMessage&&<details className="dw-error-details"><summary>Details</summary><p>{explanationErrorMessage(turn.error)}</p></details>}<Button size="1" variant="soft" onClick={onRetry}>Try this question again</Button></>;
  const progressText=questionProgressText(turn);
  const contexts=section?.claims.flatMap((c)=>c.anchors).slice(0,3).map((a)=>({path:a.path,sha256:a.sourceSha256,from:a.startLine,to:a.endLine}))??turn.context;
  return <article className="dw-turn" data-question-id={hideQuestion?undefined:turn.id} data-focus-key={!hideQuestion?`workspace-answer:${turn.id}`:undefined} tabIndex={hideQuestion?undefined:-1} aria-label={unavailable?'Question with unavailable saved explanation':`Question: ${plainInlineCodeText(questionSummaryText(turn.question))}`}>
    {!hideQuestion&&<QuestionDisclosure question={turn.question}/>}
    <div className={`dw-answer-card${unavailable?' dw-answer-card-unavailable':''}`}>
    {!revision && progressText ? (
      <div className="dw-working" role="status"><LoaderCircle size={16}/><span>{progressText}</span><Button size="1" variant="ghost" color="gray" onClick={onStop}><Square size={12}/>Stop</Button></div>
    ) : !revision && terminalStatus ? terminalNotice : !revision ? (
      availability === 'loading' ? <p role="status">Loading saved explanation…</p> : <>
        <p className="dw-unavailable-status" role="status">The answer for this question could not be reopened. Your question is saved; explain it again to create a new answer.</p>
        <Button size="1" variant="soft" onClick={onRetry}>Explain again</Button>
      </>
    ) : section ? (
      <>
      {!hideQuestion&&progressText&&<div className="dw-working" role="status"><LoaderCircle size={16}/><span>{progressText}</span><Button size="1" variant="ghost" color="gray" onClick={onStop}><Square size={12}/>Stop</Button></div>}
      {terminalStatus&&terminalNotice}
      <div className="dw-answer-heading"><span>{compact?'Explanation':`${position+1} / ${revision.sections.length}`}</span>{workspaceAnswerLifecycleBadge(revision.lifecycle)&&<Badge size="1" color="gray">{workspaceAnswerLifecycleBadge(revision.lifecycle)}</Badge>}<div><Tooltip content="Edit this explanation"><IconButton size="1" variant="ghost" color="gray" aria-label={`Edit ${plainInlineCodeText(revision.title)}`} onClick={()=>onReview(revision)}><Pencil size={13}/></IconButton></Tooltip></div></div>
      {versions.length>1&&original&&<label className="dw-answer-version">Version<select aria-label={`Answer version for ${plainInlineCodeText(original.title)}`} value={revision.revisionId} onChange={event=>onVersion(original.revisionId,event.target.value)}>{versions.map((r,index)=><option key={r.revisionId} value={r.revisionId}>{index===0?'Original':'Accepted'} · r{r.revisionNumber}{index===versions.length-1?' · latest accepted':''}</option>)}</select>{revision.revisionId!==versions.at(-1)?.revisionId&&<span>Updated version available</span>}</label>}
      {turn.notes?.map((note)=><p key={note} className="dw-answer-note" role="status">{note}</p>)}
      <h3>{compact?<button type="button" className="dw-answer-expand" aria-label={`Open ${plainInlineCodeText(section.title)} in Guide`} onClick={()=>onFocus(revision.revisionId,section.sectionId)}><InlineCodeText text={section.title}/><ChevronRight size={16}/></button>:<InlineCodeText text={section.title}/>}</h3>
      {!compact ? <>
        <VisualStory
          key={`${revision.revisionId}:${section.sectionRevisionId}`}
          revision={revision}
          section={section}
          progress={turn.presentation?.stories.find((p) => sameSection(p, {
            revisionId: revision.revisionId,
            sectionId: section.sectionId,
            sectionRevisionId: section.sectionRevisionId,
          }))}
          onProgressChange={onStoryProgress}
          onSettings={onSettings}
          onOpenAnchor={onOpen}
          onExploreDetail={(title, anchors)=>onBranch(`Explain how ${title} behaves in the cited implementation.`,contextsFromAnchors(anchors))}
          // Start the next guide slide while this narration plays. Completion
          // also fires for the same slide, so it must not enqueue a second run.
          onSectionPlaybackStarted={turn.guideId
            ? () => onGuideSlideConsumed?.(revision.revisionId)
            : undefined}
          onFollowHotspot={(hotspot) => {
            const destination = hotspot.destination;
            if (destination.kind === 'section') {
              selectSection(revision.sections.findIndex((candidate) => candidate.sectionId === destination.sectionId));
            } else if (destination.kind === 'codeLens') {
              const anchor = revision.sections
                .flatMap((candidate) => candidate.claims.flatMap((claim) => claim.anchors))
                .find((candidate) => candidate.anchorId === destination.anchorId);
              if (anchor) onOpen(anchor);
            } else {
              onDestination(destination);
            }
          }}
        />
        <div className="dw-section-controls"><IconButton size="1" variant="ghost" color="gray" aria-label="Previous section" disabled={position===0} onClick={()=>selectSection(position-1)}><ArrowLeft size={14}/></IconButton><span>{position+1} / {revision.sections.length}</span><Button size="1" variant="soft" disabled={position>=revision.sections.length-1} onClick={()=>selectSection(position+1)}>Next section<ArrowRight size={13}/></Button></div>
      </> : <details className="dw-flow-narration"><summary>Read narration</summary><p className="dw-prose"><InlineCodeText text={section.spokenText || section.prose}/></p><div className="dw-citations">{section.claims.flatMap((c)=>c.anchors).slice(0,5).map((a)=><button key={a.anchorId} onClick={()=>onOpen(a)}><Code2 size={12}/>{a.path.split('/').at(-1)}:{a.startLine}–{a.endLine}</button>)}</div></details>}
      {coverageSummary&&<div className="dw-guide-coverage" role="status"><strong>Partial guide</strong><span>Still uncovered: {coverageSummary}</span></div>}
      <footer className="dw-branches"><Button size="1" variant="ghost" onClick={()=>onBranch(`Explain ${section.title} in more detail.`,contexts)}><Network size={13}/>{branchLabel??'Explore further'}</Button>{branches.length>0&&<span>{branches.length} exploration{branches.length===1?'':'s'}</span>}<ExplorationBranchLinks branches={branches} onGo={onGo}/></footer>
      </>
    ) : <p>This saved explanation has no sections.</p>}
  </div></article>;
}
function EvidenceFlow({revision,saved,progress,onProgress,onSection,onBranch,theme,onOpen}:{revision:ChapterRevision;saved?:ReadingPosition;progress?:DiagramProgress[];onProgress:(progress:DiagramProgress)=>void;onSection:(sectionId:string)=>void;onBranch?:(question:string,context:Context[])=>void;theme:'dark'|'light';onOpen:(a:ChapterSourceAnchor)=>void}){
  const diagrams=revision.sections.map((s)=>inspectInteractiveVisual(revision,s));
  const availableDiagramCount=diagrams.filter((diagram)=>diagram.status==='available').length;
  const remembered=sectionPosition(revision.revisionId,revision.sections,saved);
  const selected=diagrams[remembered]?.status==='available'?remembered:Math.max(0,diagrams.findIndex((d)=>d.status==='available'));
  const result=diagrams[selected];
  const available=diagrams.some((d)=>d.status==='available');
  if(!available)return <div className="dw-flow-empty"><Network size={23}/><h2>No evidenced diagram in this explanation</h2><p>Use the source links in Guide to inspect the implementation. A graph is shown only when it has verified evidence.</p></div>;
  if(result?.status!=='available')return null;
  const identity={revisionId:revision.revisionId,sectionId:revision.sections[selected].sectionId,sectionRevisionId:revision.sections[selected].sectionRevisionId};
  const savedProgress=progress?.find(p=>sameSection(p,identity));
  const viewport=savedProgress?.viewport;
  const {visual,anchorsById}=result;
  const sourceValues=sourceValuesByVisualNode(visual);
  const section=revision.sections[selected];
  if(!section)return null;
  const selectedNode=savedProgress?.focusedNodeId?visual.nodes.find((node)=>node.nodeId===savedProgress.focusedNodeId):undefined;
  const selectedAnchors=selectedNode?selectedNode.evidenceAnchorIds.map((anchorId)=>anchorsById.get(anchorId)).filter((anchor):anchor is ChapterSourceAnchor=>!!anchor):[];
  const selectedAnchorIds=new Set(selectedNode?.evidenceAnchorIds??[]);
  const selectedClaims=section.claims.filter((claim)=>claim.anchors.some((anchor)=>selectedAnchorIds.has(anchor.anchorId))).map((claim)=>claim.prose);
  const selectNode=(nodeId:string)=>onProgress({...identity,...(viewport?{viewport}:{}),focusedNodeId:nodeId,layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION});
  const linearSteps=linearFlowSteps(visual);
  const unlinkedNodes=unlinkedFlowNodes(visual);
  const positions=layoutFlowNodes(visual.nodes,visual.edges,visual.layoutHint.direction);
  const labelPositions=layoutFlowEdgeLabels(visual.nodes,visual.edges,positions);
  const nodes=visual.nodes.map((n)=>{
    const label=<><strong><InlineCodeText text={n.label}/></strong>{n.detail&&<small><InlineCodeText text={n.detail}/></small>}</>;
    return {id:n.nodeId,position:positions.get(n.nodeId)??{x:0,y:0},
      data:{label:<button type="button" className="dw-flow-select nodrag nopan" aria-label={`Show code detail for ${plainInlineCodeText(n.label)}`} aria-pressed={selectedNode?.nodeId===n.nodeId} onClick={(event)=>{event.stopPropagation();selectNode(n.nodeId);}}>{label}</button>},style:{width:250}};
  });
  const edges=visual.edges.map((e)=>({id:e.edgeId,source:e.fromNodeId,target:e.toNodeId,type:'evidenceRelationship' as const,label:e.label??undefined,
    data:{labelPosition:labelPositions.get(e.edgeId)},style:{stroke:theme==='dark'?'#8c96a8':'#6a7486',strokeWidth:1.5},
    markerEnd:{type:MarkerType.ArrowClosed}}));
  return <section className="dw-flow-panel" aria-label={plainInlineCodeText(visual.title)}><div className="dw-flow-heading"><h2><InlineCodeText text={visual.title}/></h2>{availableDiagramCount>1&&<select aria-label="Diagram section" value={selected} onChange={(e)=>{const section=revision.sections[Number(e.target.value)];if(section)onSection(section.sectionId);}}>{diagrams.map((d,i)=>d.status==='available'?<option key={i} value={i}>{plainInlineCodeText(revision.sections[i].title)}</option>:null)}</select>}</div>
    {linearSteps ? <>
      <p>Choose a step to see what the cited code does.</p>
      <ol className="dw-linear-flow" aria-label={`Steps in ${plainInlineCodeText(visual.title)}`}>
        {linearSteps.map(({node, next}, index) => {
          const sourceAnchorId=node.sourceAnchorId&&node.evidenceAnchorIds.includes(node.sourceAnchorId)?node.sourceAnchorId:node.evidenceAnchorIds[0];
          const evidenceAnchor=anchorsById.get(sourceAnchorId);
          const anchor=evidenceAnchor?sourceAnchorForVisualNode(evidenceAnchor,node,node.sourceValue??sourceValues.get(node.nodeId)):undefined;
          if (!anchor) return null;
          const sourceLabel=`${anchor.path}:${anchor.startLine}${anchor.endLine===anchor.startLine?'':`–${anchor.endLine}`}`;
          return <li className="dw-linear-flow-item" key={node.nodeId}>
            <button type="button" className="dw-linear-flow-step" aria-label={`Step ${index+1}: show code detail for ${plainInlineCodeText(node.label)}. Cited source ${sourceLabel}`} aria-pressed={selectedNode?.nodeId===node.nodeId} onClick={()=>selectNode(node.nodeId)}>
              <span className="dw-linear-flow-index">Step {index+1}</span>
              <strong><InlineCodeText text={node.label}/></strong>
              {node.detail&&<small><InlineCodeText text={node.detail}/></small>}
              <span className="dw-linear-flow-source"><Code2 size={13}/>Cited source · {sourceLabel}</span>
            </button>
            {next&&<div className="dw-linear-flow-edge"><ArrowDown size={15}/><span><InlineCodeText text={next.label||'Then'}/></span></div>}
          </li>;
        })}
      </ol>
    </> : unlinkedNodes ? <>
      <p>The evidence does not define a path between these points. Choose one to inspect its cited implementation.</p>
      <ul className="dw-unlinked-flow" aria-label={`Source points in ${plainInlineCodeText(visual.title)}`}>
        {unlinkedNodes.map((node) => {
          const sourceAnchorId=node.sourceAnchorId&&node.evidenceAnchorIds.includes(node.sourceAnchorId)?node.sourceAnchorId:node.evidenceAnchorIds[0];
          const evidenceAnchor=anchorsById.get(sourceAnchorId);
          const anchor=evidenceAnchor?sourceAnchorForVisualNode(evidenceAnchor,node,node.sourceValue??sourceValues.get(node.nodeId)):undefined;
          if(!anchor)return null;
          const sourceLabel=`${anchor.path}:${anchor.startLine}${anchor.endLine===anchor.startLine?'':`–${anchor.endLine}`}`;
          return <li key={node.nodeId}><button type="button" className="dw-unlinked-flow-point" aria-label={`Show code detail for ${plainInlineCodeText(node.label)} at ${sourceLabel}`} aria-pressed={selectedNode?.nodeId===node.nodeId} onClick={()=>selectNode(node.nodeId)}>
            <strong><InlineCodeText text={node.label}/></strong>
            {node.detail&&<small><InlineCodeText text={node.detail}/></small>}
            <span className="dw-linear-flow-source"><Code2 size={13}/>Cited source · {sourceLabel}</span>
          </button></li>;
        })}
      </ul>
    </> : <>
      <p>Choose a point to see what the cited code does.</p>
      <EvidenceFlowCanvas key={`${identity.revisionId}:${identity.sectionRevisionId}:${FLOW_DIAGRAM_LAYOUT_VERSION}`} revisionId={identity.revisionId} sectionId={identity.sectionId} sectionRevisionId={identity.sectionRevisionId} nodes={nodes} edges={edges} viewport={viewport} theme={theme} onViewportChange={(viewport)=>onProgress({...identity,viewport,...(selectedNode?{focusedNodeId:selectedNode.nodeId}:{}),layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION})}/>
    </>}
    {selectedNode&&<CodeDetail title={selectedNode.label} claims={selectedClaims} anchors={selectedAnchors} onOpen={onOpen} onExplore={onBranch?(title,anchors)=>onBranch(`Explain how ${title} behaves in the cited implementation.`,contextsFromAnchors(anchors)):undefined}/>}
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
