import type { ChapterSection, ChapterSourceAnchor, SectionDependency } from './commands.ts';

const key = (d: SectionDependency) => JSON.stringify([d.kind,d.path,d.sourceSha256,
  d.kind === 'symbol' ? d.symbolId : d.kind === 'relationship' ? d.relationshipId : null]);
const unique = (values: SectionDependency[]) => values.filter((d,i) => values.findIndex(other => key(other) === key(d)) === i);
const anchorDependencies = (a: ChapterSourceAnchor): SectionDependency[] => [
  {kind:'sourceFile',path:a.path,sourceSha256:a.sourceSha256},
  ...(a.symbolId ? [{kind:'symbol' as const,path:a.path,sourceSha256:a.sourceSha256,symbolId:a.symbolId}] : []),
];

/** Remove only this claim's cues. Unrelated claims, source links and authored
 * scenes survive, including their stable IDs and narration. */
export function removeClaimAndReferences(section: ChapterSection, claimId: string): ChapterSection {
  const removed = section.claims.findIndex(c => c.claimId === claimId);
  if (removed < 0 || section.claims.length <= 1) return section;
  const claim = section.claims[removed];
  const claims = section.claims.filter(c => c.claimId !== claimId);
  const anchors = new Set(claims.flatMap(c => c.anchors.map(a => a.anchorId)));
  const removedDependencies = new Set([...claim.dependencies,...claim.anchors.flatMap(anchorDependencies)].map(key));
  const retainedDependencies = new Set(claims.flatMap(c => [...c.dependencies,...c.anchors.flatMap(anchorDependencies)]).map(key));
  const dependencies = section.dependencies.filter(d => !removedDependencies.has(key(d)) || retainedDependencies.has(key(d)));
  const hotspots = section.hotspots.filter(h => h.destination.kind !== 'codeLens' || anchors.has(h.destination.anchorId));
  const hotspotIds = new Set(hotspots.map(h => h.hotspotId));
  let visual = section.visual;
  const currentPresentation = visual?.presentation;
  const editable = visual && Array.isArray(visual.nodes) && Array.isArray(visual.edges)
    && visual.schemaVersion === 3
    && visual.nodes.every(n => n && typeof n === 'object' && 'evidenceAnchorIds' in n && Array.isArray(n.evidenceAnchorIds))
    && visual.edges.every(e => e && typeof e === 'object' && 'evidenceAnchorIds' in e && Array.isArray(e.evidenceAnchorIds))
    && currentPresentation?.schemaVersion === 3 && Array.isArray(currentPresentation.scenes)
    && currentPresentation.scenes.every(s => s && Array.isArray(s.elements) && s.elements.every(e => e && typeof e === 'object'));
  if (editable && visual && currentPresentation) {
    const nodeIndices = new Map<number,number>();
    const nodes = visual.nodes.flatMap((node,index) => {
      const evidenceAnchorIds = node.evidenceAnchorIds.filter(id => anchors.has(id));
      if (!evidenceAnchorIds.length) return [];
      nodeIndices.set(index,nodeIndices.size);
      return [{...node,evidenceAnchorIds,hotspotId:node.hotspotId && hotspotIds.has(node.hotspotId) ? node.hotspotId : null}];
    });
    const nodeIds = new Set(nodes.map(n => n.nodeId));
    const edges = visual.edges.flatMap(edge => {
      const evidenceAnchorIds = edge.evidenceAnchorIds.filter(id => anchors.has(id));
      return nodeIds.has(edge.fromNodeId) && nodeIds.has(edge.toNodeId) && evidenceAnchorIds.length ? [{...edge,evidenceAnchorIds}] : [];
    });
    const presentation = {...currentPresentation,scenes:currentPresentation.scenes
      .filter(scene => scene.claimIndex !== removed).map(scene => ({...scene,claimIndex:scene.claimIndex > removed ? scene.claimIndex - 1 : scene.claimIndex,
        elements:scene.elements.map(element => ({...element,nodeIndex:element.nodeIndex === null ? null : nodeIndices.get(element.nodeIndex) ?? -1}))}))};
    visual = nodes.length ? {...visual,nodes,edges,presentation} : null;
  }
  return {...section,claims,dependencies,hotspots,visual};
}

/** Keep anchor identity so linked cues still open the repaired source. Never
 * invent a replacement relationship or rewrite authored prose/visual values. */
export function replaceCitation(section: ChapterSection, anchorId: string, replacement: ChapterSourceAnchor): ChapterSection {
  const previous = section.claims.flatMap(c => c.anchors).find(a => a.anchorId === anchorId);
  if (!previous || replacement.anchorId !== anchorId) return section;
  const replaceDependency = (d: SectionDependency): SectionDependency => {
    if (d.path !== previous.path || d.sourceSha256 !== previous.sourceSha256) return d;
    if (d.kind === 'sourceFile') return {kind:'sourceFile',path:replacement.path,sourceSha256:replacement.sourceSha256};
    if (d.kind === 'symbol' && d.symbolId === previous.symbolId) return replacement.symbolId
      ? {kind:'symbol',path:replacement.path,sourceSha256:replacement.sourceSha256,symbolId:replacement.symbolId}
      : {kind:'sourceFile',path:replacement.path,sourceSha256:replacement.sourceSha256};
    return d;
  };
  const claims = section.claims.map(c => c.anchors.some(a => a.anchorId === anchorId) ? {...c,
    anchors:c.anchors.map(a => a.anchorId === anchorId ? replacement : a),dependencies:unique(c.dependencies.map(replaceDependency))} : c);
  const stillRequired = new Set(claims.flatMap(c => [...c.dependencies,...c.anchors.flatMap(anchorDependencies)]).map(key));
  const dependencies = unique(section.dependencies.flatMap(d => {
    const next = replaceDependency(d);
    return key(next) !== key(d) && stillRequired.has(key(d)) ? [d,next] : [next];
  }));
  return {...section,claims,dependencies};
}
