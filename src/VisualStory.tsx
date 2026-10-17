import { useEffect, useMemo, useRef, useState } from 'react';
import { Background, Controls, MarkerType, Position, ReactFlow } from '@xyflow/react';
import { ArrowLeft, ArrowRight, Code2, Maximize2, Network } from 'lucide-react';
import { NarrationPlayer } from './NarrationPlayer';
import { HtmlScene } from './HtmlScene';
import type { ChapterHotspot, ChapterRevision, ChapterSection, ChapterSourceAnchor } from './lib/commands';
import { defaultNarrationText } from './lib/narration';
import { inspectInteractiveVisual } from './lib/visuals';
import { readStoryProgress, sceneNodeIds, storyLayout, storyScenes } from './lib/visualStory';
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
}

/** One narrated, evidence-backed scene at a time. Audio completion, not guessed
 * elapsed-time percentages, advances the story. Sections always advance manually.
 */
export function VisualStory({ revision, section, onOpenAnchor, onFollowHotspot, onSettings, activeFocusTarget, restorePosition = 0, onPlaybackPositionChange, onTranscriptPositionChange }: Props) {
  const inspection = useMemo(() => inspectInteractiveVisual(revision, section), [revision, section]);
  const visual = inspection.status === 'available' ? inspection.visual : undefined;
  const baseKey = `code-atlas:narration:${revision.projectId}:${revision.chapterId}:${revision.revisionId}:${section.sectionId}`;
  const sceneKey = `code-atlas:story:${revision.revisionId}:${section.sectionRevisionId}`;
  const savedScript = useMemo(() => {
    try {
      const saved = JSON.parse(localStorage.getItem(baseKey) ?? 'null');
      return typeof saved?.spokenText === 'string' && saved.spokenText !== defaultNarrationText(section.spokenText) ? saved.spokenText : undefined;
    } catch { return undefined; }
  }, [baseKey, section.spokenText]);
  const scenes = useMemo(() => storyScenes(section, visual, savedScript), [section, visual, savedScript]);
  const savedPosition = useMemo(() => {
    try { return readStoryProgress(localStorage.getItem(sceneKey), scenes); } catch { return readStoryProgress(null, scenes); }
  }, [sceneKey, scenes]);
  const [index, setIndex] = useState(savedPosition.index);
  const positions = useRef(new Map(savedPosition.positions));
  const root = useRef<HTMLElement>(null);
  const [autoPlay, setAutoPlay] = useState(false);
  const [manualVisit, setManualVisit] = useState(0);
  const [fullMap, setFullMap] = useState(false);
  const [focusedNode, setFocusedNode] = useState<string | null>(null);
  useEffect(() => {
    const node = visual?.nodes.find((node) => activeFocusTarget === `reader-visual-citation:${node.nodeId}:${node.evidenceAnchorIds[0]}`);
    if (node) setFocusedNode(node.nodeId);
  }, [activeFocusTarget, visual]);
  const scene = scenes[Math.min(index, scenes.length - 1)];
  const authoredScenes = visual?.presentation?.scenes.filter((item) => scenes.length === 1
    || section.claims[item.claimIndex]?.claimId === scene.id) ?? [];
  const htmlScenes = !fullMap && authoredScenes.length > 0 && authoredScenes.every((item) => item.mode === 'composition') ? authoredScenes : [];
  const anchors = section.claims.flatMap((claim) => claim.anchors);
  const anchor = anchors.find((item) => scene.anchorIds.includes(item.anchorId));
  const activeIds = new Set(scene.nodeIds);
  const visibleIds = visual && !fullMap && activeIds.size ? sceneNodeIds(visual, scene) : new Set(visual?.nodes.map((node) => node.nodeId));
  const focus = visual?.nodes.find((node) => node.nodeId === focusedNode) ?? visual?.nodes.find((node) => activeIds.has(node.nodeId));
  const focusAnchor = anchors.find((item) => focus?.evidenceAnchorIds.includes(item.anchorId)) ?? anchor;
  const hotspot = inspection.status === 'available' && focus?.hotspotId ? inspection.hotspotsById.get(focus.hotspotId) : undefined;
  const savePosition = (sceneId: string, position?: number) => {
    if (position !== undefined) positions.current.set(sceneId, position);
    try { localStorage.setItem(sceneKey, JSON.stringify({ sceneId, positions: Object.fromEntries(positions.current) })); } catch { /* Reading works without storage. */ }
  };

  const select = (next: number, playing = false, nodeId: string | null = null) => {
    if (next < 0 || next >= scenes.length) return;
    setAutoPlay(playing); setIndex(next); setFocusedNode(nodeId);
    if (!playing) { setManualVisit((visit) => visit + 1); requestAnimationFrame(() => root.current?.scrollIntoView({ block: 'start', behavior: 'instant' })); }
    if (playing) positions.current.set(scenes[next].id, 0);
    savePosition(scenes[next].id);
  };
  const chooseNode = (id: string) => {
    const next = scene.nodeIds.includes(id) ? index : scenes.findIndex((item) => item.nodeIds.includes(id));
    select(next < 0 ? index : next, false, id);
    const node = visual?.nodes.find((item) => item.nodeId === id);
    const source = anchors.find((item) => node?.evidenceAnchorIds.includes(item.anchorId));
    if (source) onOpenAnchor(source, `reader-visual-citation:${id}:${source.anchorId}`);
  };
  const nodes = visual ? storyLayout(visual).filter((node) => visibleIds.has(node.id)).map((node) => {
    const data = visual.nodes.find((item) => item.nodeId === node.id)!;
    return { ...node, sourcePosition: visual.layoutHint.direction === 'topToBottom' ? Position.Bottom : Position.Right,
      targetPosition: visual.layoutHint.direction === 'topToBottom' ? Position.Top : Position.Left,
      data: { label: <button className="story-node-button nodrag" data-focus-key={`reader-visual-citation:${node.id}:${data.evidenceAnchorIds[0]}`} aria-pressed={focus?.nodeId === node.id} aria-label={`${data.label}${activeIds.has(node.id) ? ', narrated in this scene' : ', related step'}`} onClick={() => chooseNode(node.id)}>{data.label}</button> },
      className: `${activeIds.has(node.id) ? 'story-node-active' : 'story-node-context'}${focus?.nodeId === node.id ? ' story-node-focus' : ''}`, style: { width: 195 } };
  }) : [];
  const edges = visual?.edges.filter((edge) => visibleIds.has(edge.fromNodeId) && visibleIds.has(edge.toNodeId)).map((edge) => ({
    id: edge.edgeId, source: edge.fromNodeId, target: edge.toNodeId, label: edge.label,
    markerEnd: { type: MarkerType.ArrowClosed }, className: activeIds.has(edge.fromNodeId) && activeIds.has(edge.toNodeId) ? 'story-edge-active' : 'story-edge-context',
  })) ?? [];

  return <section ref={root} className="visual-story" aria-label={`Visual story: ${section.title}`}>
    <div className="story-toolbar">
      <span className="story-scene-count" aria-live="polite">Scene {index + 1} of {scenes.length}</span>
      <div className="story-scene-dots" role="group" aria-label="Story scenes">{scenes.map((item, i) => <button key={item.id} aria-label={`Scene ${i + 1}`} aria-current={i === index ? 'step' : undefined} onClick={() => select(i)}><span /></button>)}</div>
      {visual && (visual.edges.length > 0 || !visual.presentation) && <button className="story-map-toggle" aria-pressed={fullMap} onClick={() => setFullMap((value) => !value)}><Maximize2 size={13} />{fullMap ? 'Focus scene' : 'Full map'}</button>}
    </div>
    <div className="story-playbar">
      <button className="story-previous" disabled={index === 0} aria-label="Previous scene" onClick={() => select(index - 1)}><ArrowLeft size={15} /></button>
      <NarrationPlayer key={`${section.sectionRevisionId}:${scene.id}:${manualVisit}`} compact
        projectId={revision.projectId} chapterId={revision.chapterId} revisionId={revision.revisionId} section={section}
        clip={scenes.length > 1 ? { id: scene.id, text: scene.narration } : undefined}
        restorePosition={positions.current.get(scene.id) || (scenes.length === 1 ? restorePosition : 0)} transcriptPosition={0}
        onPlaybackPositionChange={(position) => { savePosition(scene.id, position); onPlaybackPositionChange?.(position); }} onTranscriptPositionChange={(position) => onTranscriptPositionChange?.(position)}
        onOpenProviderSettings={onSettings} autoPlay={autoPlay} onEnded={() => { if (index < scenes.length - 1) select(index + 1, true); }} />
      <button className="story-next" disabled={index === scenes.length - 1} aria-label="Next scene" onClick={() => select(index + 1)}><ArrowRight size={15} /></button>
    </div>
    {visual && htmlScenes.length > 0 ? <div className="story-stage story-html-stage" role="group" aria-label={visual.title}>
      {htmlScenes.map((item) => <HtmlScene key={item.claimIndex} scene={item} visual={visual} onOpenNode={chooseNode} focusedNode={focusedNode}/>)}
    </div> : visual ? <div className="story-stage" role="group" aria-label={visual.title}>
      <ReactFlow key={`${section.sectionRevisionId}:${index}:${fullMap}`} nodes={nodes} edges={edges} fitView fitViewOptions={{ padding: .12, maxZoom: 1.1 }}
        minZoom={.18} maxZoom={1.8} nodesDraggable={false} nodesConnectable={false} nodesFocusable={false} edgesFocusable={false} elementsSelectable={false} deleteKeyCode={null} zoomOnScroll={false} preventScrolling={false} colorMode="system">
        <Background gap={24} /><Controls showInteractive={false} />
      </ReactFlow>
    </div> : <div className="story-source-stage">
      {anchor ? <><div><Code2 size={15} /><span>{anchor.path.split('/').at(-1)} · {anchor.startLine}–{anchor.endLine}</span></div><pre><code>{anchor.excerpt}</code></pre></>
        : <div className="story-no-visual"><Network size={28} /><span>{section.title}</span><small>{inspection.status === 'unavailable' ? 'This saved visual cannot be displayed. Narration and source links are available.' : 'No diagram was saved for this section.'}</small></div>}
    </div>}
    {!visual && <div className="story-focus-bar"><span>{focus?.label ?? (anchor ? 'Captured code' : section.title)}</span>
      {focusAnchor && <button onClick={() => { setAutoPlay(false); setManualVisit((visit) => visit + 1); onOpenAnchor(focusAnchor); }}><Code2 size={13} />Source</button>}
    </div>}
    <details className="story-details"><summary>Scene details & source</summary>
      {focus?.detail && <p>{focus.detail}</p>}
      {hotspot && onFollowHotspot && hotspot.destination.kind !== 'codeLens' && <button data-focus-key={`reader-hotspot:${hotspot.hotspotId}`} onClick={() => onFollowHotspot(hotspot, `reader-hotspot:${hotspot.hotspotId}`)}>{hotspot.label}<ArrowRight size={13}/></button>}
      <div className="story-sources">{[...new Map(anchors.filter((item) => scene.anchorIds.includes(item.anchorId)).map((item) => [item.anchorId, item])).values()].map((item) => <button key={item.anchorId} data-focus-key={`reader-citation:${item.anchorId}`} onClick={() => onOpenAnchor(item, `reader-citation:${item.anchorId}`)}>{item.path}:{item.startLine}–{item.endLine}</button>)}</div>
    </details>
    {section.prose.trim() !== section.spokenText.trim() && <details className="story-details"><summary>Section notes</summary><p>{section.prose}</p></details>}
  </section>;
}
