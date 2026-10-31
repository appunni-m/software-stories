import { useMemo } from 'react';
import { ArrowRight, Code2, Sparkles } from 'lucide-react';
import { AppButton } from '../design/AppButton';
import { HtmlScene } from '../HtmlScene';
import { InlineCodeText } from '../InlineCodeText';
import type { ChapterRevision, ChapterSection, ChapterSourceAnchor, EvidenceLinkedVisualSpec } from '../lib/commands';
import { validStoredScenePresentation } from '../lib/scenePresentation';
import { inspectInteractiveVisual } from '../lib/visuals';
import { sourceAnchorForVisualNode, sourceEvidenceAnchorForVisualNodeInScene } from '../lib/visualStory';
import { plainInlineCodeText } from '../lib/inlineCode';
import { sharedSourceBranch } from '../lib/answerVisualLayout';

/** Keep earlier stacked answers visual without mounting their players or full canvases. */
export function AnswerVisualPreview({ revision, section, onOpenAnchor, onRefreshVisual }: {
  revision: ChapterRevision;
  section: ChapterSection;
  onOpenAnchor: (anchor: ChapterSourceAnchor) => void;
  onRefreshVisual?: () => void;
}) {
  const inspection = useMemo(() => inspectInteractiveVisual(revision, section), [revision, section]);
  const visual = inspection.status === 'available' ? inspection.visual : undefined;
  const anchorsById = inspection.status === 'available' ? inspection.anchorsById : undefined;
  const firstScene = visual && validStoredScenePresentation(visual.presentation, visual.nodes, section.claims)
    ? visual.presentation.scenes[0]
    : undefined;
  const anchors = anchorsById ? [...anchorsById.values()] : [];

  const evidenceAnchorForNode = (node: EvidenceLinkedVisualSpec['nodes'][number], sourceValue?: string, preferredAnchorIds?: readonly string[]) => {
    return sourceEvidenceAnchorForVisualNodeInScene(anchors, node, sourceValue ?? node.sourceValue, preferredAnchorIds);
  };
  const sourceAnchorIdForNode = (nodeId: string, sourceValue?: string, preferredAnchorIds?: readonly string[]) => {
    const node = visual?.nodes.find((candidate) => candidate.nodeId === nodeId);
    return node ? evidenceAnchorForNode(node, sourceValue, preferredAnchorIds)?.anchorId : undefined;
  };

  const openNode = (visual: EvidenceLinkedVisualSpec, nodeId: string, sourceValue?: string, sourceAnchorId?: string, preferredAnchorIds?: readonly string[]) => {
    const node = visual.nodes.find((candidate) => candidate.nodeId === nodeId);
    if (!node || !anchorsById) return;
    const requestedAnchor = sourceAnchorId && node.evidenceAnchorIds.includes(sourceAnchorId)
      && (!preferredAnchorIds || preferredAnchorIds.includes(sourceAnchorId))
      ? anchorsById.get(sourceAnchorId)
      : undefined;
    const evidenceAnchor = requestedAnchor ?? evidenceAnchorForNode(node, sourceValue, preferredAnchorIds);
    if (evidenceAnchor) onOpenAnchor(sourceAnchorForVisualNode(evidenceAnchor, node, sourceValue ?? node.sourceValue));
  };

  if (!visual || !firstScene) {
    const anchor = section.claims.flatMap((claim) => claim.anchors)[0];
    const unavailable = inspection.status === 'unavailable' || Boolean(visual);
    const needsRefresh = inspection.status === 'unavailable' && inspection.reason === 'unsupportedVersion';
    const canRefresh = inspection.status === 'unavailable' && Boolean(onRefreshVisual);
    return <div className="dw-answer-visual-preview dw-answer-visual-preview-empty" role="group"
      aria-label={`Visual preview for ${plainInlineCodeText(section.title)}`}>
      <span>{needsRefresh
        ? canRefresh ? 'This saved visual needs refreshing.' : 'This visual cannot be shown here.'
        : canRefresh ? 'This saved visual cannot be shown.'
        : unavailable ? 'This visual cannot be previewed here.' : 'No visual was saved for this section.'}</span>
      {(canRefresh || anchor) && <div className="dw-answer-visual-actions">
        {canRefresh && onRefreshVisual && <AppButton intent="custom" size="1" type="button" onClick={onRefreshVisual} aria-label={`Create a fresh visual explanation for ${plainInlineCodeText(section.title)}`}>
          <Sparkles size={13} aria-hidden="true" />Create a fresh visual
        </AppButton>}
        {anchor && <AppButton intent="custom" size="1" type="button" onClick={() => onOpenAnchor(anchor)} aria-label={`Open cited code in ${anchor.path} at line ${anchor.startLine}`}>
          <Code2 size={13} />{anchor.path.split('/').at(-1)} · {anchor.startLine}–{anchor.endLine}
        </AppButton>}
      </div>}
    </div>;
  }

  if (firstScene.mode === 'composition') {
    const firstSceneAnchorIds = section.claims[firstScene.claimIndex]?.anchors.map((anchor) => anchor.anchorId) ?? [];
    return <div className="dw-answer-visual-preview dw-answer-visual-preview-composition" role="group"
      aria-label={`Visual preview: ${plainInlineCodeText(firstScene.title)}`}>
      <HtmlScene scene={firstScene} visual={visual} onOpenNode={(nodeId, sourceValue, sourceAnchorId) => openNode(visual, nodeId, sourceValue, sourceAnchorId, firstSceneAnchorIds)}
        sourceAnchorIdForNode={sourceAnchorIdForNode} sourceAnchorIds={firstSceneAnchorIds} />
    </div>;
  }

  const claim = section.claims[firstScene.claimIndex];
  const claimAnchorIds = new Set(claim?.anchors.map((anchor) => anchor.anchorId) ?? []);
  const focusedNodes = visual.nodes.filter((node) => node.evidenceAnchorIds.some((id) => claimAnchorIds.has(id)));
  const visibleIds = new Set(focusedNodes.map((node) => node.nodeId));
  const edges = visual.edges.filter((edge) => visibleIds.has(edge.fromNodeId) || visibleIds.has(edge.toNodeId));
  for (const edge of edges) {
    visibleIds.add(edge.fromNodeId);
    visibleIds.add(edge.toNodeId);
  }
  const visibleNodes = visual.nodes.filter((node) => visibleIds.has(node.nodeId));
  const shownEdges = edges.slice(0, 4);
  const nodeById = new Map(visibleNodes.map((node) => [node.nodeId, node]));
  const sourceBranch = sharedSourceBranch(shownEdges, visibleNodes, anchorsById ?? new Map());
  const branchSource = sourceBranch ? nodeById.get(sourceBranch.sourceNodeId) : undefined;

  return <div className="dw-answer-visual-preview dw-answer-visual-preview-graph" role="group"
    aria-label={`Source-linked relationship preview: ${plainInlineCodeText(firstScene.title)}`}>
    <strong>{plainInlineCodeText(firstScene.title)}</strong>
    {sourceBranch && branchSource ? <div className="dw-answer-visual-fork" role="group" aria-label="One source with multiple outcomes">
      <AppButton intent="custom" size="1" type="button" className="dw-answer-visual-fork-source"
        onClick={() => openNode(visual, branchSource.nodeId)} aria-label={`Open source for ${plainInlineCodeText(branchSource.label)}`}>
        <InlineCodeText text={branchSource.label} />
      </AppButton>
      <ul aria-label={`${plainInlineCodeText(branchSource.label)} outcomes`}>{sourceBranch.edges.map((edge) => {
        const to = nodeById.get(edge.toNodeId);
        if (!to) return null;
        return <li key={edge.edgeId}>
          <span className="dw-answer-visual-fork-label"><InlineCodeText text={edge.label} /></span>
          <ArrowRight size={13} aria-hidden="true" />
          <AppButton intent="custom" size="1" type="button" onClick={() => openNode(visual, to.nodeId)} aria-label={`Open source for ${plainInlineCodeText(to.label)}`}>
            <InlineCodeText text={to.label} />
          </AppButton>
        </li>;
      })}</ul>
    </div> : shownEdges.length > 0 ? <ul>{shownEdges.map((edge) => {
      const from = nodeById.get(edge.fromNodeId);
      const to = nodeById.get(edge.toNodeId);
      if (!from || !to) return null;
      return <li key={edge.edgeId}>
        <AppButton intent="custom" size="1" type="button" onClick={() => openNode(visual, from.nodeId)} aria-label={`Open source for ${plainInlineCodeText(from.label)}`}>
          <InlineCodeText text={from.label} />
        </AppButton>
        <span className="dw-answer-visual-edge"><InlineCodeText text={edge.label} /><ArrowRight size={13} aria-hidden="true" /></span>
        <AppButton intent="custom" size="1" type="button" onClick={() => openNode(visual, to.nodeId)} aria-label={`Open source for ${plainInlineCodeText(to.label)}`}>
          <InlineCodeText text={to.label} />
        </AppButton>
      </li>;
    })}</ul> : visibleNodes.length > 0 ? <div className="dw-answer-visual-points">{visibleNodes.slice(0, 4).map((node) => <AppButton intent="custom" size="1" type="button" key={node.nodeId}
      onClick={() => openNode(visual, node.nodeId)} aria-label={`Open source for ${plainInlineCodeText(node.label)}`}><InlineCodeText text={node.label} /></AppButton>)}</div>
      : <span className="dw-answer-visual-no-links">No direct relationships are attached to this scene.</span>}
    {edges.length > shownEdges.length && <small>{edges.length - shownEdges.length} more source links in the full view</small>}
  </div>;
}
