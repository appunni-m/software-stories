import { useEffect, useMemo, useRef, useState } from 'react';
import { Background, BaseEdge, Controls, EdgeLabelRenderer, MarkerType, Position, ReactFlow, getBezierPath, getViewportForBounds, type Edge, type EdgeProps, type ReactFlowInstance } from '@xyflow/react';
import { ArrowLeft, ArrowRight, Code2, Maximize2, Network } from 'lucide-react';
import { InlineCodeText } from './InlineCodeText';
import { NarrationPlayer } from './NarrationPlayer';
import { HtmlScene } from './HtmlScene';
import { isNativeRuntime, type ChapterHotspot, type ChapterRevision, type ChapterSection, type ChapterSourceAnchor } from './lib/commands';
import { defaultNarrationText, parseNarrationPreferences } from './lib/narration';
import { narrationStorageKey } from './lib/narrationScope';
import { readOnlyDiagramAriaLabelConfig } from './lib/readOnlyDiagramAccessibility';
import { inspectInteractiveVisual } from './lib/visuals';
import { readSectionStoryProgress, readStoryProgressRecord, restoreStoryFocus, sceneNodeIds, sourceAnchorForVisualEdge, sourceAnchorForVisualNode, sourceEvidenceAnchorForVisualEdge, sourceEvidenceAnchorForVisualNode, sourceValuesByVisualNode, STORY_EDGE_LABEL_HEIGHT, STORY_EDGE_LABEL_MAX_WIDTH, STORY_NODE_WIDTH, storyEdgeLabelPosition, storyLayoutWithDirection, storyScenes, storyViewportBounds, supportsStoryFullMap, visibleStoryEdgeLabel } from './lib/visualStory';
import { storyViewportNeedsRefit, type FlowViewportInstance } from './lib/flowViewport';
import { cleanFlowViewport, FOCUSED_STORY_LAYOUT_VERSION, sameSection, type FlowViewport, type StoryProgress } from './lib/readerProgress';
import { useReaderProgress } from './lib/ReaderProgressContext';
import './visual-story.css';

interface Props {
  revision: ChapterRevision;
  section: ChapterSection;
  onOpenAnchor: (anchor: ChapterSourceAnchor, focusTarget?: string) => void;
  onFollowHotspot?: (hotspot: ChapterHotspot, focusTarget?: string) => void;
  activeFocusTarget?: string;
  onSettings: () => void;
  restorePosition?: number;
  onPlaybackPositionChange?: (position: number) => void;
  onTranscriptPositionChange?: (position: number) => void;
  onSectionPlaybackStarted?: () => void;
  onSectionPlaybackComplete?: () => void;
  progress?: StoryProgress;
  onProgressChange?: (progress: StoryProgress) => void;
}

interface StoryEdgeData extends Record<string, unknown> {
  label: string;
  focused: boolean;
  vertical: boolean;
  labelSide: number;
  onOpenSource?: () => void;
}

type StoryEdge = Edge<StoryEdgeData, 'storyEvidence'>;

function EvidenceStoryEdge({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, markerEnd, style, data }: EdgeProps<StoryEdge>) {
  const [path, labelX, labelY] = getBezierPath({ sourceX, sourceY, sourcePosition, targetX, targetY, targetPosition });
  const labelPosition = storyEdgeLabelPosition(sourceX, targetX, labelX, labelY, data?.vertical ?? false, data?.labelSide ?? 1);
  const label = data?.label ?? '';
  const focused = data?.focused ?? false;
  const onOpenSource = data?.onOpenSource;
  return <>
    <BaseEdge id={id} path={path} markerEnd={markerEnd} style={style} />
    <EdgeLabelRenderer>
      <button type="button" className={`story-edge-label nodrag nopan${focused ? ' story-edge-label-focused' : ''}`} title={label}
        style={{ position: 'absolute', transform: `translate(-50%, -50%) translate(${labelPosition.x}px,${labelPosition.y}px)`, pointerEvents: 'all' }}
        aria-label={`${label}. Open cited source code`} aria-pressed={focused} disabled={!onOpenSource}
        onClick={(event) => { event.stopPropagation(); onOpenSource?.(); }}>
        <span className="story-edge-label-text">{visibleStoryEdgeLabel(label)}</span><Code2 size={11} aria-hidden="true" />
      </button>
    </EdgeLabelRenderer>
  </>;
}

const storyEdgeTypes = { storyEvidence: EvidenceStoryEdge };

/** One narrated, evidence-backed scene at a time. Audio completion, not guessed
 * elapsed-time percentages, advances the story. Sections always advance manually.
 */
export function VisualStory({ revision, section, onOpenAnchor, onFollowHotspot, onSettings, activeFocusTarget, restorePosition = 0, onPlaybackPositionChange, onTranscriptPositionChange, onSectionPlaybackStarted, onSectionPlaybackComplete, progress:explicitProgress, onProgressChange:explicitOnProgressChange }: Props) {
  const inspection = useMemo(() => inspectInteractiveVisual(revision, section), [revision, section]);
  const visual = inspection.status === 'available' ? inspection.visual : undefined;
  const baseKey = narrationStorageKey(revision.projectId,revision.chapterId,revision.revisionId,section.sectionId);
  const sceneKey = `code-atlas:story:v2:${revision.revisionId}:${section.sectionRevisionId}`;
  const identity = {revisionId:revision.revisionId,sectionId:section.sectionId,sectionRevisionId:section.sectionRevisionId};
  const shared = useReaderProgress(identity);
  const progress = explicitProgress ?? shared?.progress;
  const onProgressChange = explicitOnProgressChange ?? shared?.onProgressChange;
  const nativeProgress = progress && sameSection(progress,identity) ? progress : undefined;
  const savedScript = useMemo(() => {
    try {
      const defaultScript = defaultNarrationText(section.spokenText);
      const saved = parseNarrationPreferences(localStorage.getItem(baseKey), defaultScript);
      return saved.spokenText !== defaultScript ? saved.spokenText : undefined;
    } catch { return undefined; }
  }, [baseKey, section.spokenText]);
  const scenes = useMemo(() => storyScenes(section, visual, savedScript), [section, visual, savedScript]);
  const savedPositionRead = useMemo(() => {
    let stored: string | null = null;
    let readFailed = false;
    if (!nativeProgress && !isNativeRuntime()) {
      try { stored = localStorage.getItem(sceneKey); } catch { readFailed = true; }
    }
    const browserRecord = nativeProgress || isNativeRuntime()
      ? { status: 'missing' as const }
      : readFailed ? { status: 'unavailable' as const } : readStoryProgressRecord(stored, identity, scenes);
    const browserProgress = browserRecord.status === 'available' ? browserRecord.progress : undefined;
    return {
      position: readSectionStoryProgress(nativeProgress ?? browserProgress, identity, scenes, restorePosition),
      unavailable: !nativeProgress && !isNativeRuntime() && browserRecord.status === 'unavailable',
    };
  }, [sceneKey, scenes, nativeProgress, identity.revisionId, identity.sectionId, identity.sectionRevisionId, restorePosition]);
  const savedPosition = savedPositionRead.position;
  const browserProgressScopeKey = JSON.stringify([sceneKey, scenes.map((item) => item.id)]);
  const unavailableBrowserProgressRef = useRef(savedPositionRead.unavailable);
  useEffect(() => { unavailableBrowserProgressRef.current = savedPositionRead.unavailable; }, [browserProgressScopeKey, savedPositionRead.unavailable]);
  const reportedUnavailableProgressRef = useRef<string | null>(null);
  useEffect(() => {
    if (!savedPositionRead.unavailable || isNativeRuntime()
      || reportedUnavailableProgressRef.current === browserProgressScopeKey) return;
    reportedUnavailableProgressRef.current = browserProgressScopeKey;
    window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice', { detail: {
      projectId: revision.projectId, snapshotId: revision.snapshotId,
      title: 'Saved scene position is unavailable',
      message: 'Its stored browser position is malformed, unreadable or unsupported and was left unchanged. Choose a scene or open the map to save a new position.',
      tone: 'warning', occurrenceId: crypto.randomUUID(),
    } }));
  }, [browserProgressScopeKey, onProgressChange, revision.projectId, revision.snapshotId, savedPositionRead.unavailable]);
  const [index, setIndex] = useState(savedPosition.index);
  const activeScene = useRef(scenes[savedPosition.index].id);
  const positions = useRef(new Map(savedPosition.positions));
  const root = useRef<HTMLElement>(null);
  const storyStage = useRef<HTMLDivElement>(null);
  const flowFitGeneration = useRef(0);
  const flowFitInProgress = useRef(false);
  const flowFitCheckPending = useRef(false);
  const scheduleViewportCheck = useRef<() => void>(() => {});
  const [autoPlay, setAutoPlay] = useState(false);
  const [manualVisit, setManualVisit] = useState(0);
  const sectionCompletionSent = useRef(false);
  const canShowFullMap = Boolean(visual && supportsStoryFullMap(visual)
    && (visual.edges.length > 0 || !visual.presentation));
  const [fullMap, setFullMap] = useState(savedPosition.fullMap && canShowFullMap);
  const canvasViewport = useRef<FlowViewport|undefined>(savedPosition.viewport);
  const mapView = useRef(fullMap);
  const restoredFocus = restoreStoryFocus(visual, savedPosition.focusedNodeId, savedPosition.focusedEdgeId);
  const [focusedNode, setFocusedNode] = useState<string | null>(restoredFocus.nodeId);
  const [focusedEdge, setFocusedEdge] = useState<string | null>(restoredFocus.edgeId);
  const focusInitialized = useRef(Boolean(visual));
  const scene = scenes[Math.min(index, scenes.length - 1)];
  const authoredScenes = visual?.presentation?.scenes.filter((item) => scenes.length === 1
    || section.claims[item.claimIndex]?.claimId === scene.id) ?? [];
  const htmlScenes = !fullMap && authoredScenes.length > 0 && authoredScenes.every((item) => item.mode === 'composition') ? authoredScenes : [];
  const anchors = section.claims.flatMap((claim) => claim.anchors);
  const sourceValues = useMemo(() => visual ? sourceValuesByVisualNode(visual) : new Map<string, string>(), [visual]);
  const anchor = anchors.find((item) => scene.anchorIds.includes(item.anchorId));
  const activeIds = new Set(scene.nodeIds);
  const visibleIds = visual && !fullMap && activeIds.size ? sceneNodeIds(visual, scene) : new Set(visual?.nodes.map((node) => node.nodeId));
  const focus = visual?.nodes.find((node) => node.nodeId === focusedNode) ?? visual?.nodes.find((node) => activeIds.has(node.nodeId));
  const focusSourceValue = focus?.sourceValue ?? (focus ? sourceValues.get(focus.nodeId) : undefined);
  const focusAnchor = focus
    ? sourceEvidenceAnchorForVisualNode(anchors, focus, focusSourceValue)
    : anchor;
  const focusSourceAnchor = focus && focusAnchor
    ? sourceAnchorForVisualNode(focusAnchor, focus, focusSourceValue)
    : focusAnchor;
  const hotspot = inspection.status === 'available' && focus?.hotspotId ? inspection.hotspotsById.get(focus.hotspotId) : undefined;
  const savePosition = (sceneId: string, position?: number, focusSelection?: {nodeId:string|null;edgeId:string|null}, replaceUnavailable = false) => {
    if (position !== undefined && Number.isFinite(position)) positions.current.set(sceneId, Math.max(0,Math.min(3600,position)));
    const selection=focusSelection??{nodeId:focusedNode,edgeId:focusedEdge};
    const saved={...identity,sceneId,positions:Object.fromEntries(positions.current),fullMap:mapView.current,focusLayoutVersion:FOCUSED_STORY_LAYOUT_VERSION,
      ...(selection.nodeId?{focusedNodeId:selection.nodeId}:{}),...(selection.edgeId?{focusedEdgeId:selection.edgeId}:{}),
      ...(canvasViewport.current?{viewport:canvasViewport.current}:{})};
    if (unavailableBrowserProgressRef.current && !replaceUnavailable) return;
    if (onProgressChange) { onProgressChange(saved); return; }
    if (!isNativeRuntime()) {
      try {
        localStorage.setItem(sceneKey, JSON.stringify(saved));
        unavailableBrowserProgressRef.current = false;
      } catch { /* Standalone reading works without browser storage. */ }
    }
  };
  useEffect(() => {
    if (!visual || focusInitialized.current) return;
    focusInitialized.current = true;
    setFocusedNode(restoredFocus.nodeId);
    setFocusedEdge(restoredFocus.edgeId);
  }, [visual, restoredFocus.nodeId, restoredFocus.edgeId]);
  useEffect(() => {
    const node = visual?.nodes.find((node) => activeFocusTarget === `reader-visual-citation:${node.nodeId}:${node.sourceAnchorId ?? node.evidenceAnchorIds[0]}`);
    if (node) { setFocusedNode(node.nodeId); setFocusedEdge(null); savePosition(scene.id, undefined, {nodeId:node.nodeId,edgeId:null}, true); return; }
    const edge = visual?.edges.find((item) => activeFocusTarget?.startsWith(`reader-visual-edge:${item.edgeId}:`));
    if (edge) { setFocusedNode(edge.fromNodeId); setFocusedEdge(edge.edgeId); savePosition(scene.id, undefined, {nodeId:edge.fromNodeId,edgeId:edge.edgeId}, true); }
  }, [activeFocusTarget, visual]);

  const select = (next: number, playing = false, nodeId: string | null = null, edgeId: string | null = null) => {
    if (next < 0 || next >= scenes.length) return;
    activeScene.current=scenes[next].id;
    if(next!==index)canvasViewport.current=undefined;
    setAutoPlay(playing); setIndex(next); setFocusedNode(nodeId); setFocusedEdge(edgeId);
    if (!playing) { setManualVisit((visit) => visit + 1); requestAnimationFrame(() => root.current?.scrollIntoView({ block: 'start', behavior: 'instant' })); }
    if (playing) positions.current.set(scenes[next].id, 0);
    savePosition(scenes[next].id, undefined, {nodeId,edgeId}, true);
  };
  const chooseNode = (id: string, sourceValue?: string) => {
    const next = scene.nodeIds.includes(id) ? index : scenes.findIndex((item) => item.nodeIds.includes(id));
    select(next < 0 ? index : next, false, id);
    const node = visual?.nodes.find((item) => item.nodeId === id);
    const exactValue = node?.sourceValue ?? sourceValue;
    const source = node ? sourceEvidenceAnchorForVisualNode(anchors, node, exactValue) : undefined;
    if (source && node) onOpenAnchor(sourceAnchorForVisualNode(source, node, exactValue), `reader-visual-citation:${id}:${source.anchorId}`);
  };
  const chooseEdge = (edgeId: string) => {
    const edge = visual?.edges.find((item) => item.edgeId === edgeId);
    if (!edge) return;
    const source = sourceEvidenceAnchorForVisualEdge(anchors, edge);
    if (!source) return;
    const origin = visual?.nodes.find((item) => item.nodeId === edge.fromNodeId);
    const target = visual?.nodes.find((item) => item.nodeId === edge.toNodeId);
    const sourceContext = origin ? sourceEvidenceAnchorForVisualNode(anchors, origin, origin.sourceValue) : undefined;
    const sourceContextAnchor = origin && sourceContext
      ? sourceAnchorForVisualNode(sourceContext, origin, origin.sourceValue)
      : undefined;
    const targetAnchor = target ? sourceEvidenceAnchorForVisualNode(anchors, target, target.sourceValue) : undefined;
    const relatedScene = scenes.findIndex((item) => item.nodeIds.includes(edge.fromNodeId) && item.nodeIds.includes(edge.toNodeId));
    select(relatedScene < 0 ? index : relatedScene, false, edge.fromNodeId, edge.edgeId);
    onOpenAnchor(sourceAnchorForVisualEdge(source, edge, targetAnchor, sourceContextAnchor), `reader-visual-edge:${edge.edgeId}:${source.anchorId}`);
  };
  const graphLayout = visual ? storyLayoutWithDirection(visual, visibleIds) : undefined;
  const nodes = visual ? graphLayout!.nodes.map((node) => {
    const data = visual.nodes.find((item) => item.nodeId === node.id)!;
    return { ...node, sourcePosition: graphLayout!.direction === 'vertical' ? Position.Bottom : Position.Right,
      targetPosition: graphLayout!.direction === 'vertical' ? Position.Top : Position.Left,
      data: { label: <button className="story-node-button nodrag" data-focus-key={`reader-visual-citation:${node.id}:${data.sourceAnchorId ?? data.evidenceAnchorIds[0]}`} aria-pressed={focusedNode === node.id} aria-label={`${data.label}${data.detail ? `: ${data.detail}` : ''}${activeIds.has(node.id) ? ', narrated in this scene' : ', related step'}. Open cited source code`} title={`${data.label}${data.detail ? ` — ${data.detail}` : ''}`} onClick={() => chooseNode(node.id, sourceValues.get(node.id))}>
        <span className="story-node-title">{data.label}</span>
        {data.detail && <small className="story-node-detail">{data.detail}</small>}
      </button> },
      className: `${activeIds.has(node.id) ? 'story-node-active' : 'story-node-context'}${focus?.nodeId === node.id ? ' story-node-focus' : ''}`, style: { width: STORY_NODE_WIDTH } };
  }) : [];
  const visibleEdges = visual?.edges.filter((edge) => visibleIds.has(edge.fromNodeId) && visibleIds.has(edge.toNodeId)) ?? [];
  const sourceLinkedEdges = visibleEdges.flatMap((edge) => {
    const source = sourceEvidenceAnchorForVisualEdge(anchors, edge);
    const from = visual?.nodes.find((node) => node.nodeId === edge.fromNodeId);
    const to = visual?.nodes.find((node) => node.nodeId === edge.toNodeId);
    return source && from && to ? [{ edge, source, from, to }] : [];
  });
  const edges: StoryEdge[] = visibleEdges.map((edge, edgeIndex) => ({
    id: edge.edgeId, type: 'storyEvidence', source: edge.fromNodeId, target: edge.toNodeId,
    data: { label: edge.label, focused: focusedEdge === edge.edgeId, vertical: graphLayout?.direction === 'vertical', labelSide: edgeIndex % 2 === 0 ? 1 : -1, onOpenSource: () => chooseEdge(edge.edgeId) },
    markerEnd: { type: MarkerType.ArrowClosed }, className: activeIds.has(edge.fromNodeId) && activeIds.has(edge.toNodeId) ? 'story-edge-active' : 'story-edge-context',
  }));
  const flowInstance = useRef<FlowViewportInstance & Pick<ReactFlowInstance<(typeof nodes)[number], StoryEdge>, 'setNodes' | 'setEdges'> | null>(null);
  const graphData = useRef({ nodes, edges });
  graphData.current = { nodes, edges };
  const edgeLabelBoundsFor = (instance: FlowViewportInstance) => {
    const nodeById = new Map(instance.getNodes().map((node) => [node.id, node]));
    return graphData.current.edges.flatMap((edge) => {
      const source = nodeById.get(edge.source);
      const target = nodeById.get(edge.target);
      if (!source || !target) return [];
      const sourceWidth = source.measured?.width ?? source.width ?? STORY_NODE_WIDTH;
      const targetWidth = target.measured?.width ?? target.width ?? STORY_NODE_WIDTH;
      const sourceHeight = source.measured?.height ?? source.height;
      const targetHeight = target.measured?.height ?? target.height;
      if (!Number.isFinite(sourceHeight) || !Number.isFinite(targetHeight) || !sourceHeight || !targetHeight) return [];
      const vertical = edge.data?.vertical ?? false;
      const sourceX = source.position.x + (vertical ? sourceWidth / 2 : sourceWidth);
      const sourceY = source.position.y + (vertical ? sourceHeight : sourceHeight / 2);
      const targetX = target.position.x + (vertical ? targetWidth / 2 : 0);
      const targetY = target.position.y + (vertical ? 0 : targetHeight / 2);
      const [, labelX, labelY] = getBezierPath({
        sourceX, sourceY,
        sourcePosition: vertical ? Position.Bottom : Position.Right,
        targetX, targetY,
        targetPosition: vertical ? Position.Top : Position.Left,
      });
      const position = storyEdgeLabelPosition(sourceX, targetX, labelX, labelY, vertical, edge.data?.labelSide ?? 1);
      return [{
        x: position.x - STORY_EDGE_LABEL_MAX_WIDTH / 2,
        y: position.y - STORY_EDGE_LABEL_HEIGHT / 2,
        width: STORY_EDGE_LABEL_MAX_WIDTH,
        height: STORY_EDGE_LABEL_HEIGHT,
      }];
    });
  };

  const visibleNodeKey = JSON.stringify([...visibleIds]);
  useEffect(() => {
    // Saved chapter data can hydrate after the native reader mounts its Guide
    // canvas. Reconcile that new graph even when React Flow initialized with
    // an empty or stale set of nodes.
    const instance = flowInstance.current;
    if (!instance?.viewportInitialized) return;
    const current = graphData.current;
    instance.setNodes(current.nodes);
    instance.setEdges(current.edges);
    requestAnimationFrame(() => {
      if (flowInstance.current !== instance) return;
      const latest = graphData.current;
      instance.setNodes(latest.nodes);
      instance.setEdges(latest.edges);
      scheduleViewportCheck.current();
    });
  }, [visual, visibleNodeKey, scene.id, fullMap, revision.revisionId, section.sectionRevisionId]);
  useEffect(() => {
    const stage = storyStage.current;
    if (!stage) return;
    const generation = ++flowFitGeneration.current;
    let frame: number | undefined;
    const schedule = () => {
      if (frame !== undefined) return;
      frame = requestAnimationFrame(() => {
        frame = undefined;
        const instance = flowInstance.current;
        const currentStage = storyStage.current;
        if (!instance?.viewportInitialized || !currentStage) return;
        if (flowFitInProgress.current) {
          // Node measurements can arrive while fitView is running. Preserve
          // that invalidation so its final viewport is checked against the
          // settled node bounds instead of leaving the initial canvas stale.
          flowFitCheckPending.current = true;
          return;
        }
        const nodeBounds = instance.getNodesBounds([...visibleIds]);
        const bounds = storyViewportBounds(nodeBounds, edgeLabelBoundsFor(instance));
        const size = currentStage.getBoundingClientRect();
        if (!bounds.width || !bounds.height) return;
        if (!storyViewportNeedsRefit(bounds, instance.getViewport(), size.width, size.height, 28)) return;

        flowFitInProgress.current = true;
        const fittedViewport = getViewportForBounds(bounds, size.width, size.height, .18, 1.1, .08);
        void instance.setViewport(fittedViewport, { duration: 0 }).then((fitted) => {
          if (!fitted || generation !== flowFitGeneration.current || instance !== flowInstance.current
            || activeScene.current !== scene.id || mapView.current !== fullMap) return;
          const clean = cleanFlowViewport(instance.getViewport());
          if (!clean) return;
          canvasViewport.current = clean;
          savePosition(scene.id);
        }).catch(() => {
          // The saved scene remains readable if React Flow cannot refit it.
        }).finally(() => {
          if (generation === flowFitGeneration.current) {
            flowFitInProgress.current = false;
            if (flowFitCheckPending.current) {
              flowFitCheckPending.current = false;
              scheduleViewportCheck.current();
            }
          }
        });
      });
    };
    scheduleViewportCheck.current = schedule;
    const observer = typeof ResizeObserver === 'undefined' ? undefined : new ResizeObserver(schedule);
    observer?.observe(stage);
    window.addEventListener('resize', schedule);
    schedule();
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', schedule);
      if (frame !== undefined) cancelAnimationFrame(frame);
      if (scheduleViewportCheck.current === schedule) scheduleViewportCheck.current = () => {};
      flowFitGeneration.current += 1;
      flowFitInProgress.current = false;
      flowFitCheckPending.current = false;
    };
  }, [scene.id, fullMap, visibleNodeKey, revision.revisionId, section.sectionRevisionId]);

  return <section ref={root} className="visual-story" aria-label={`Visual story: ${section.title}`}>
    <div className="story-sticky-controls">
      <div className="story-toolbar">
        <span className="story-scene-title" title={scene.title}>{scene.title}</span>
        <span className="story-scene-count" aria-live="polite">Scene {index + 1} of {scenes.length}</span>
        <div className="story-scene-dots" role="group" aria-label="Story scenes">{scenes.map((item, i) => <button key={item.id} aria-label={`Scene ${i + 1}: ${item.title}`} aria-current={i === index ? 'step' : undefined} onClick={() => select(i)}><span /></button>)}</div>
        {canShowFullMap && <button className="story-map-toggle" aria-pressed={fullMap} onClick={() => {mapView.current=!fullMap;canvasViewport.current=undefined;setFullMap(!fullMap);savePosition(scene.id, undefined, undefined, true);}}><Maximize2 size={13} />{fullMap ? 'Focus scene' : 'Full map'}</button>}
      </div>
    </div>
    {visual && htmlScenes.length > 0 ? <div className="story-stage story-html-stage" role="group" aria-label={visual.title}>
      {htmlScenes.map((item) => <HtmlScene key={item.claimIndex} scene={item} visual={visual} onOpenNode={chooseNode} focusedNode={focusedNode}/>)}
    </div> : visual ? <div ref={storyStage} className="story-stage" role="group" aria-label={visual.title}>
      <ReactFlow key={`${section.sectionRevisionId}:${index}:${fullMap}:${visibleNodeKey}`} nodes={nodes} edges={edges} edgeTypes={storyEdgeTypes} defaultViewport={canvasViewport.current} fitView={!canvasViewport.current} fitViewOptions={{ padding: .06, maxZoom: 1.1 }}
        aria-label={`Read-only visual explanation: ${visual.title}`} ariaLabelConfig={readOnlyDiagramAriaLabelConfig} disableKeyboardA11y
        onInit={(instance) => {
          flowInstance.current = instance;
          // Reapply the controlled arrays after React Flow initializes. On a
          // fresh native workspace restore, its first canvas can otherwise
          // mount without painting nodes until the user changes scenes.
          instance.setNodes(graphData.current.nodes);
          instance.setEdges(graphData.current.edges);
          // A freshly restored Guide can mount before its split-pane layout has
          // settled. Reconcile the controlled graph after that first frame so
          // React Flow cannot retain the pre-layout empty canvas.
          requestAnimationFrame(() => {
            if (flowInstance.current !== instance) return;
            const latest = graphData.current;
            instance.setNodes(latest.nodes);
            instance.setEdges(latest.edges);
            scheduleViewportCheck.current();
          });
          const saved = canvasViewport.current;
          if (saved) {
            void instance.setViewport(saved, { duration: 0 }).then(() => {
              if (flowInstance.current === instance) scheduleViewportCheck.current();
            }).catch(() => {
              if (flowInstance.current === instance) scheduleViewportCheck.current();
            });
          }
          scheduleViewportCheck.current();
        }}
        onNodesChange={(changes) => { if (changes.some((change) => change.type === 'dimensions')) scheduleViewportCheck.current(); }}
        onMoveEnd={(_event,viewport)=>{const clean=cleanFlowViewport(viewport);if(clean&&activeScene.current===scene.id&&mapView.current===fullMap){canvasViewport.current=clean;savePosition(scene.id);}}}
        minZoom={.18} maxZoom={1.8} nodesDraggable={false} nodesConnectable={false} nodesFocusable={false} edgesFocusable={false} elementsSelectable={false} deleteKeyCode={null} zoomOnScroll={false} preventScrolling={false} colorMode="system">
        <Background gap={24} /><Controls showInteractive={false} />
      </ReactFlow>
    </div> : <div className="story-source-stage">
      {anchor ? <div className="story-no-visual story-source-fallback">
        <Code2 size={28} aria-hidden="true" />
        <span><InlineCodeText text={section.title}/></span>
        <small>Open the cited code beside this explanation to inspect this step.</small>
        <button type="button" data-focus-key={`reader-citation:${anchor.anchorId}`}
          aria-label={`Open ${anchor.path}, lines ${anchor.startLine} through ${anchor.endLine}`}
          onClick={() => onOpenAnchor(anchor, `reader-citation:${anchor.anchorId}`)}>
          <span>{anchor.path.split('/').at(-1)} · {anchor.startLine}–{anchor.endLine}</span><ArrowRight size={14} aria-hidden="true" />
        </button>
      </div> : <div className="story-no-visual">
        <Network size={28} aria-hidden="true" /><span><InlineCodeText text={section.title}/></span>
        <small>{inspection.status === 'unavailable' ? 'This saved visual cannot be displayed. Narration and source links are available.' : 'No diagram was saved for this section.'}</small>
      </div>}
    </div>}
    <div className="story-playbar">
      <button className="story-previous" disabled={index === 0} aria-label="Previous scene" onClick={() => select(index - 1)}><ArrowLeft size={15} /></button>
      <NarrationPlayer key={`${section.sectionRevisionId}:${scene.id}:${manualVisit}`} compact
        projectId={revision.projectId} chapterId={revision.chapterId} revisionId={revision.revisionId} section={section}
        clip={scenes.length > 1 ? { id: scene.id, text: scene.narration } : undefined}
        restorePosition={positions.current.get(scene.id) ?? (scenes.length === 1 ? restorePosition : 0)} transcriptPosition={0}
        onPlaybackPositionChange={(position) => { savePosition(scene.id, position, undefined, true); onPlaybackPositionChange?.(position); }} onTranscriptPositionChange={(position) => onTranscriptPositionChange?.(position)}
        onOpenProviderSettings={onSettings} onPlaybackStarted={onSectionPlaybackStarted} autoPlay={autoPlay} onEnded={() => { if (index < scenes.length - 1) select(index + 1, true); else if (!sectionCompletionSent.current) { sectionCompletionSent.current = true; onSectionPlaybackComplete?.(); } }} />
      <button className="story-next" disabled={index === scenes.length - 1} aria-label="Next scene" onClick={() => select(index + 1)}><ArrowRight size={15} /></button>
    </div>
    {!visual && <div className="story-focus-bar"><span>{focus?.label ?? (anchor ? 'Captured code' : section.title)}</span>
      {focusSourceAnchor && <button onClick={() => { setAutoPlay(false); setManualVisit((visit) => visit + 1); onOpenAnchor(focusSourceAnchor); }}><Code2 size={13} />Source</button>}
    </div>}
    <details className="story-details"><summary>Scene details & source</summary>
      {focus?.detail && <p><InlineCodeText text={focus.detail}/></p>}
      {hotspot && onFollowHotspot && hotspot.destination.kind !== 'codeLens' && <button data-focus-key={`reader-hotspot:${hotspot.hotspotId}`} onClick={() => onFollowHotspot(hotspot, `reader-hotspot:${hotspot.hotspotId}`)}>{hotspot.label}<ArrowRight size={13}/></button>}
      {sourceLinkedEdges.length > 0 && <div className="story-details-connections">
        <h3>Connections</h3>
        <ul>{sourceLinkedEdges.map(({ edge, source, from, to }) => <li key={edge.edgeId}>
          <button data-focus-key={`reader-visual-edge:${edge.edgeId}:${source.anchorId}`} aria-pressed={focusedEdge === edge.edgeId}
            aria-label={`${from.label} — ${edge.label} — ${to.label}. Open ${source.path}, lines ${source.startLine} through ${source.endLine}.`}
            onClick={() => chooseEdge(edge.edgeId)}>
            <span>{from.label}</span><span aria-hidden="true">→</span><span>{edge.label}</span><span aria-hidden="true">→</span><span>{to.label}</span>
            <small>{source.path}:{source.startLine}–{source.endLine}</small>
          </button>
        </li>)}</ul>
      </div>}
      <div className="story-sources">{[...new Map(anchors.filter((item) => scene.anchorIds.includes(item.anchorId)).map((item) => [item.anchorId, item])).values()].map((item) => <button key={item.anchorId} data-focus-key={`reader-citation:${item.anchorId}`} onClick={() => onOpenAnchor(item, `reader-citation:${item.anchorId}`)}>{item.path}:{item.startLine}–{item.endLine}</button>)}</div>
    </details>
    {section.prose.trim() !== section.spokenText.trim() && <details className="story-details"><summary>Section notes</summary><p><InlineCodeText text={section.prose}/></p></details>}
  </section>;
}
