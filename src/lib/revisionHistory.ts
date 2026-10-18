import type {
  ChapterClaim,
  ChapterHotspot,
  ChapterRevision,
  ChapterSection,
  ChapterSourceAnchor,
  SectionDependency,
  VisualSpec,
} from './commands.ts';
import type { PresentationScene, SceneElement, ScenePresentation } from './scenePresentation.ts';

export type RevisionChangeKind = 'added' | 'removed' | 'changed';
export type RevisionChangeEntity =
  | 'chapter'
  | 'section'
  | 'claim'
  | 'anchor'
  | 'dependency'
  | 'visual'
  | 'visualNode'
  | 'visualEdge'
  | 'presentation'
  | 'scene'
  | 'sceneElement'
  | 'hotspot';

/** A content-safe description of one difference between two immutable revisions. */
export interface RevisionChange {
  kind: RevisionChangeKind;
  entity: RevisionChangeEntity;
  path: string;
  id?: string;
  field?: string;
  summary: string;
  beforeSummary?: string;
  afterSummary?: string;
  /** Expandable authored text only; captured excerpts and element values never enter these fields. */
  beforeDetail?: string;
  afterDetail?: string;
}

export interface RevisionDiffCounts {
  added: number;
  removed: number;
  changed: number;
  total: number;
}

export interface ChapterRevisionDiff {
  fromRevisionId: string;
  toRevisionId: string;
  changes: RevisionChange[];
  counts: RevisionDiffCounts;
  identical: boolean;
}

export interface RevisionHistoryMetadata {
  revisionId: string;
  chapterId: string;
  revisionNumber: number;
  lifecycle: ChapterRevision['lifecycle'];
  snapshotId: string;
  parentRevisionId: string | null;
  parentRevisionNumber: number | null;
  parentRevisionTitle: string | null;
  createdAtUnixMs: number | null;
  createdAtIso: string | null;
}

/**
 * Compare chapter content by its stable IDs. Long authored text can be expanded;
 * captured source excerpts and source-backed scene values remain hidden.
 */
export function compareChapterRevisions(
  before: ChapterRevision,
  after: ChapterRevision,
): ChapterRevisionDiff {
  const changes: RevisionChange[] = [];

  compareField(changes, 'chapter', 'chapter', 'title', before.title, after.title);
  compareField(changes, 'chapter', 'chapter', 'targetId', before.target?.targetId, after.target?.targetId);
  compareField(changes, 'chapter', 'chapter', 'targetScope', before.target?.scope, after.target?.scope);

  compareKeyed(
    before.sections ?? [],
    after.sections ?? [],
    (section) => section.sectionId,
    (section) => section.sectionId,
    (section) => sectionSummary(section),
    (section) => sectionSummary(section),
    'section',
    'sections',
    changes,
    (oldSection, newSection, sectionPath) => compareSection(oldSection, newSection, sectionPath, changes),
  );

  const counts = changes.reduce<RevisionDiffCounts>((result, change) => {
    result[change.kind] += 1;
    result.total += 1;
    return result;
  }, { added: 0, removed: 0, changed: 0, total: 0 });

  return {
    fromRevisionId: before.revisionId,
    toRevisionId: after.revisionId,
    changes,
    counts,
    identical: changes.length === 0,
  };
}

/** Resolve recorded parent and creation-time metadata without inferring missing lineage. */
export function revisionHistoryMetadata(
  revision: ChapterRevision,
  history: readonly ChapterRevision[] = [],
): RevisionHistoryMetadata {
  const parentRevisionId = nonEmptyString(revision.basedOnRevisionId);
  const parent = parentRevisionId
    ? history.find((candidate) => candidate.revisionId === parentRevisionId) ?? null
    : null;
  const candidateTime = Number.isFinite(revision.createdAtUnixMs) && revision.createdAtUnixMs >= 0
    ? new Date(revision.createdAtUnixMs)
    : null;
  const createdAt = candidateTime && Number.isFinite(candidateTime.getTime()) ? candidateTime : null;
  const createdAtUnixMs = createdAt?.getTime() ?? null;

  return {
    revisionId: revision.revisionId,
    chapterId: revision.chapterId,
    revisionNumber: revision.revisionNumber,
    lifecycle: revision.lifecycle,
    snapshotId: revision.snapshotId,
    parentRevisionId,
    parentRevisionNumber: parent?.revisionNumber ?? null,
    parentRevisionTitle: parent?.title ?? null,
    createdAtUnixMs,
    createdAtIso: createdAt && Number.isFinite(createdAt.getTime()) ? createdAt.toISOString() : null,
  };
}

function compareSection(
  before: ChapterSection,
  after: ChapterSection,
  path: string,
  changes: RevisionChange[],
): void {
  compareField(changes, 'section', path, 'order', before.order, after.order, before.sectionId);
  compareField(changes, 'section', path, 'title', before.title, after.title, before.sectionId);
  compareField(changes, 'section', path, 'prose', before.prose, after.prose, before.sectionId);
  compareField(changes, 'section', path, 'spokenText', before.spokenText, after.spokenText, before.sectionId);
  compareField(changes, 'section', path, 'claimOrder', before.claims.map(c=>c.claimId), after.claims.map(c=>c.claimId), before.sectionId,
    (_value,side)=>orderedSummary((side==='before'?before:after).claims.map(c=>c.prose)));

  compareKeyed(
    before.claims ?? [], after.claims ?? [],
    (claim) => claim.claimId, (claim) => claim.claimId,
    claimSummary, claimSummary, 'claim', `${path}/claims`, changes,
    (oldClaim, newClaim, claimPath) => compareClaim(oldClaim, newClaim, claimPath, changes),
  );

  compareDependencies(before.dependencies ?? [], after.dependencies ?? [], `${path}/dependencies`, changes);
  compareVisual(before.visual, after.visual, `${path}/visual`, changes, before.claims, after.claims);

  compareKeyed(
    before.hotspots ?? [], after.hotspots ?? [],
    (hotspot) => hotspot.hotspotId, (hotspot) => hotspot.hotspotId,
    hotspotSummary, hotspotSummary, 'hotspot', `${path}/hotspots`, changes,
    (oldHotspot, newHotspot, hotspotPath) => compareHotspot(oldHotspot, newHotspot, hotspotPath, changes),
  );
}

function compareClaim(before: ChapterClaim, after: ChapterClaim, path: string, changes: RevisionChange[]): void {
  compareField(changes, 'claim', path, 'category', before.category.kind, after.category.kind, before.claimId);
  compareField(changes, 'claim', path, 'inferenceLabel', before.category.kind==='inference'?before.category.visibleLabel:null, after.category.kind==='inference'?after.category.visibleLabel:null, before.claimId);
  compareField(changes, 'claim', path, 'inferenceBasis', before.category.kind==='inference'?before.category.basis:null, after.category.kind==='inference'?after.category.basis:null, before.claimId);
  compareField(changes, 'claim', path, 'prose', before.prose, after.prose, before.claimId);
  compareKeyed(
    before.anchors ?? [], after.anchors ?? [],
    (anchor) => anchor.anchorId, (anchor) => anchor.anchorId,
    anchorSummary, anchorSummary, 'anchor', `${path}/anchors`, changes,
    (oldAnchor, newAnchor, anchorPath) => compareAnchor(oldAnchor, newAnchor, anchorPath, changes),
  );
  compareDependencies(before.dependencies ?? [], after.dependencies ?? [], `${path}/dependencies`, changes);
}

function compareAnchor(
  before: ChapterSourceAnchor,
  after: ChapterSourceAnchor,
  path: string,
  changes: RevisionChange[],
): void {
  const fields: Array<keyof ChapterSourceAnchor> = [
    'path', 'symbolId', 'snapshotId', 'sourceSha256', 'startByte', 'endByteInclusive',
    'startLine', 'endLine', 'startColumn', 'endColumn', 'excerpt',
  ];
  for (const field of fields) {
    compareField(changes, 'anchor', path, field, before[field], after[field], before.anchorId);
  }
}

function compareDependencies(
  before: readonly SectionDependency[],
  after: readonly SectionDependency[],
  path: string,
  changes: RevisionChange[],
): void {
  compareKeyed(
    before,
    after,
    dependencyIdentity,
    dependencyIdentity,
    dependencySummary,
    dependencySummary,
    'dependency',
    path,
    changes,
    (oldDependency, newDependency, dependencyPath) => {
      compareField(changes, 'dependency', dependencyPath, 'path', oldDependency.path, newDependency.path, dependencyIdentity(oldDependency));
      compareField(changes, 'dependency', dependencyPath, 'sourceSha256', oldDependency.sourceSha256, newDependency.sourceSha256, dependencyIdentity(oldDependency));
      if (oldDependency.kind !== newDependency.kind) {
        compareField(changes, 'dependency', dependencyPath, 'kind', oldDependency.kind, newDependency.kind, dependencyIdentity(oldDependency));
      }
      if (oldDependency.kind === 'symbol' && newDependency.kind === 'symbol') {
        compareField(changes, 'dependency', dependencyPath, 'symbolId', oldDependency.symbolId, newDependency.symbolId, dependencyIdentity(oldDependency));
      }
      if (oldDependency.kind === 'relationship' && newDependency.kind === 'relationship') {
        compareField(changes, 'dependency', dependencyPath, 'relationshipId', oldDependency.relationshipId, newDependency.relationshipId, dependencyIdentity(oldDependency));
      }
    },
  );
}

function compareVisual(
  before: VisualSpec | null | undefined,
  after: VisualSpec | null | undefined,
  path: string,
  changes: RevisionChange[],
  beforeClaims: ChapterClaim[],
  afterClaims: ChapterClaim[],
): void {
  const presentation=(visual:VisualSpec|null|undefined)=>visual&&'presentation' in visual?visual.presentation??null:null;
  // A scene indexes visual nodes, so an unchanged presentation can render a
  // different label/evidence after the node array moves. Compare both together.
  comparePresentation(presentation(before),presentation(after),`${path}/presentation`,changes,beforeClaims,afterClaims,before?.nodes??[],after?.nodes??[]);
  if (sameValue(before ?? null, after ?? null)) return;
  if (!before) {
    changes.push({ kind: 'added', entity: 'visual', path, summary: `Added ${visualSummary(after!)}.` , afterSummary: visualSummary(after!) });
    return;
  }
  if (!after) {
    changes.push({ kind: 'removed', entity: 'visual', path, summary: `Removed ${visualSummary(before)}.`, beforeSummary: visualSummary(before) });
    return;
  }

  compareField(changes, 'visual', path, 'schemaVersion', before.schemaVersion, after.schemaVersion);
  compareField(changes, 'visual', path, 'kind', before.kind, after.kind);
  compareField(changes, 'visual', path, 'title', before.title, after.title);
  compareField(changes, 'visual', path, 'layoutHint', 'layoutHint' in before ? before.layoutHint : null, 'layoutHint' in after ? after.layoutHint : null);
  compareField(changes, 'visual', path, 'nodeOrder', before.nodes.map(n=>n.nodeId), after.nodes.map(n=>n.nodeId), undefined,
    (_value,side)=>orderedSummary((side==='before'?before:after).nodes.map(n=>n.label)));

  compareKeyed(
    before.nodes ?? [], after.nodes ?? [],
    (node) => node.nodeId, (node) => node.nodeId,
    (node) => `node “${safeText(node.label)}”`, (node) => `node “${safeText(node.label)}”`,
    'visualNode', `${path}/nodes`, changes,
    (oldNode, newNode, nodePath) => {
      compareField(changes, 'visualNode', nodePath, 'label', oldNode.label, newNode.label, oldNode.nodeId);
      compareField(changes, 'visualNode', nodePath, 'detail', oldNode.detail, newNode.detail, oldNode.nodeId);
      compareField(changes, 'visualNode', nodePath, 'hotspotId', oldNode.hotspotId, newNode.hotspotId, oldNode.nodeId);
      const oldAnchorIds = 'evidenceAnchorIds' in oldNode ? oldNode.evidenceAnchorIds : [];
      const newAnchorIds = 'evidenceAnchorIds' in newNode ? newNode.evidenceAnchorIds : [];
      compareField(changes, 'visualNode', nodePath, 'evidenceAnchorIds', oldAnchorIds ?? [], newAnchorIds ?? [], oldNode.nodeId);
    },
  );

  compareKeyed(
    before.edges ?? [], after.edges ?? [],
    (edge) => edge.edgeId, (edge) => edge.edgeId,
    (edge) => `edge “${safeText(edge.label)}”`, (edge) => `edge “${safeText(edge.label)}”`,
    'visualEdge', `${path}/edges`, changes,
    (oldEdge, newEdge, edgePath) => {
      compareField(changes, 'visualEdge', edgePath, 'fromNodeId', oldEdge.fromNodeId, newEdge.fromNodeId, oldEdge.edgeId);
      compareField(changes, 'visualEdge', edgePath, 'toNodeId', oldEdge.toNodeId, newEdge.toNodeId, oldEdge.edgeId);
      compareField(changes, 'visualEdge', edgePath, 'label', oldEdge.label, newEdge.label, oldEdge.edgeId);
      const oldAnchorIds = 'evidenceAnchorIds' in oldEdge ? oldEdge.evidenceAnchorIds : [];
      const newAnchorIds = 'evidenceAnchorIds' in newEdge ? newEdge.evidenceAnchorIds : [];
      compareField(changes, 'visualEdge', edgePath, 'evidenceAnchorIds', oldAnchorIds ?? [], newAnchorIds ?? [], oldEdge.edgeId);
    },
  );
}

function orderedSummary(labels: string[]): string {
  return labels.length?labels.slice(0,6).map(label=>`“${safeText(label,48)}”`).join(' → ')+(labels.length>6?` → … (${labels.length} total)`:''):'(none)';
}

function comparePresentation(
  before:ScenePresentation|null,after:ScenePresentation|null,path:string,changes:RevisionChange[],
  beforeClaims:ChapterClaim[],afterClaims:ChapterClaim[],beforeNodes:VisualSpec['nodes'],afterNodes:VisualSpec['nodes'],
):void {
  if(!before&&!after)return;
  if(!before||!after){
    const value=(before??after)!;
    changes.push({kind:before?'removed':'added',entity:'presentation',path,summary:`${before?'Removed':'Added'} visual presentation (${value.scenes.length} scenes).`,
      ...(before?{beforeSummary:`${value.scenes.length} scenes`}:{afterSummary:`${value.scenes.length} scenes`})});
  }else compareField(changes,'presentation',path,'schemaVersion',before.schemaVersion,after.schemaVersion);
  const sceneKey=(scene:PresentationScene,claims:ChapterClaim[])=>claims[scene.claimIndex]?.claimId??`scene:${scene.claimIndex}`;
  const sceneSummary=(scene:PresentationScene)=>`scene “${safeText(scene.title)}” (${scene.mode}; ${scene.elements.length} elements)`;
  compareKeyed(before?.scenes??[],after?.scenes??[],s=>sceneKey(s,beforeClaims),s=>sceneKey(s,afterClaims),sceneSummary,sceneSummary,'scene',`${path}/scenes`,changes,(oldScene,newScene,scenePath)=>{
    const id=sceneKey(oldScene,beforeClaims);
    for(const field of ['claimIndex','title','mode','narration'] as const)compareField(changes,'scene',scenePath,field,oldScene[field]??null,newScene[field]??null,id);
    // Elements have positional identity in the validated presentation schema.
    // Report their exact positions rather than inventing stable element IDs.
    const indexed=(elements:SceneElement[])=>elements.map((element,index)=>({element,index}));
    const summarize=({element,index}:{element:SceneElement;index:number})=>`element ${index+1} (${element.kind}; ${element.valueOrigin==='source'?'captured value hidden':element.valueOrigin})`;
    compareKeyed(indexed(oldScene.elements),indexed(newScene.elements),e=>String(e.index),e=>String(e.index),summarize,summarize,'sceneElement',`${scenePath}/elements`,changes,(oldEntry,newEntry,elementPath)=>{
      const oldElement=oldEntry.element,newElement=newEntry.element,elementId=String(oldEntry.index);
      for(const field of ['kind','parentIndex','nodeIndex','tone','valueOrigin'] as const)compareField(changes,'sceneElement',elementPath,field,oldElement[field],newElement[field],elementId);
      compareField(changes,'sceneElement',elementPath,'nodeId',oldElement.nodeIndex===null?null:beforeNodes[oldElement.nodeIndex]?.nodeId??null,newElement.nodeIndex===null?null:afterNodes[newElement.nodeIndex]?.nodeId??null,elementId,
        (_value,side)=>{
          const element=side==='before'?oldElement:newElement,nodes=side==='before'?beforeNodes:afterNodes;
          return element.nodeIndex===null?'(none)':`“${safeText(nodes[element.nodeIndex]?.label??'Unavailable node')}”`;
        });
      const hideValue=[oldElement,newElement].some(e=>e.valueOrigin==='source'||e.kind==='code');
      compareField(changes,'sceneElement',elementPath,'value',oldElement.value,newElement.value,elementId,
        hideValue?value=>`value (${typeof value==='string'?value.length:0} characters; contents hidden)`:undefined);
    });
  });
}

function compareHotspot(before: ChapterHotspot, after: ChapterHotspot, path: string, changes: RevisionChange[]): void {
  compareField(changes, 'hotspot', path, 'label', before.label, after.label, before.hotspotId);
  compareField(changes, 'hotspot', path, 'destination', before.destination, after.destination, before.hotspotId);
}

function compareField(
  changes: RevisionChange[],
  entity: RevisionChangeEntity,
  path: string,
  field: string,
  before: unknown,
  after: unknown,
  id?: string,
  summarize?: (value:unknown,side:'before'|'after')=>string,
): void {
  if (sameValue(before, after)) return;
  const beforeSummary = summarize?summarize(before,'before'):safeValueSummary(field, before);
  const afterSummary = summarize?summarize(after,'after'):safeValueSummary(field, after);
  changes.push({
    kind: 'changed',
    entity,
    path,
    ...(id ? { id } : {}),
    field,
    summary: `${fieldLabel(field)} changed: ${beforeSummary} → ${afterSummary}.`,
    beforeSummary,
    afterSummary,
    ...(['prose','spokenText','narration','inferenceBasis','detail'].includes(field)?{
      ...(typeof before==='string'&&before.length>88?{beforeDetail:before}:{}),
      ...(typeof after==='string'&&after.length>88?{afterDetail:after}:{}),
    }:{}),
  });
}

/** Human locations for comparison rows. Native IDs remain in the model for
 * identity matching, while the reader sees their section and scene names. */
export function revisionChangeContext(change:RevisionChange,before:ChapterRevision,after:ChapterRevision):string {
  const revisions=change.kind==='removed'?[before,after]:[after,before];
  for(const revision of revisions){
    const section=revision.sections.find(s=>change.path===`sections/${s.sectionId}`||change.path.startsWith(`sections/${s.sectionId}/`));
    if(!section)continue;
    const context=[section.title];
    const visual=section.visual;
    if(visual&&'presentation' in visual){
      const scene=visual.presentation?.scenes.find(s=>{
        const key=section.claims[s.claimIndex]?.claimId??`scene:${s.claimIndex}`;
        const prefix=`sections/${section.sectionId}/visual/presentation/scenes/${key}`;
        return change.path===prefix||change.path.startsWith(`${prefix}/`);
      });
      if(scene)context.push(scene.title);
    }
    if(change.entity==='sceneElement'&&change.id!==undefined&&/^\d+$/.test(change.id))context.push(`Element ${Number(change.id)+1}`);
    else if(change.entity==='visualNode'){
      const node=visual?.nodes.find(n=>n.nodeId===change.id);if(node)context.push(node.label);
    }
    else if(change.entity==='claim'||change.entity==='anchor'||change.entity==='dependency'){
      const claim=section.claims.findIndex(c=>change.path===`sections/${section.sectionId}/claims/${c.claimId}`||change.path.startsWith(`sections/${section.sectionId}/claims/${c.claimId}/`));
      if(claim>=0)context.push(`Claim ${claim+1}`);
    }
    return context.map(value=>safeText(value,120)).join(' / ');
  }
  return 'Chapter';
}

function compareKeyed<T>(
  before: readonly T[],
  after: readonly T[],
  beforeKey: (item: T) => string,
  afterKey: (item: T) => string,
  summarizeBefore: (item: T) => string,
  summarizeAfter: (item: T) => string,
  entity: RevisionChangeEntity,
  path: string,
  changes: RevisionChange[],
  compareShared: (before: T, after: T, itemPath: string) => void,
): void {
  const beforeMap = indexedItems(before, beforeKey);
  const afterMap = indexedItems(after, afterKey);
  const keys = [...new Set([...beforeMap.keys(), ...afterMap.keys()])].sort();
  for (const key of keys) {
    const oldItem = beforeMap.get(key);
    const newItem = afterMap.get(key);
    const itemPath = `${path}/${key}`;
    if (oldItem && !newItem) {
      const summary = summarizeBefore(oldItem);
      changes.push({ kind: 'removed', entity, path: itemPath, id: key, summary: `Removed ${summary}.`, beforeSummary: summary });
    } else if (!oldItem && newItem) {
      const summary = summarizeAfter(newItem);
      changes.push({ kind: 'added', entity, path: itemPath, id: key, summary: `Added ${summary}.`, afterSummary: summary });
    } else if (oldItem && newItem) {
      compareShared(oldItem, newItem, itemPath);
    }
  }
}

function indexedItems<T>(items: readonly T[], keyOf: (item: T) => string): Map<string, T> {
  const result = new Map<string, T>();
  const duplicateCounts = new Map<string, number>();
  for (const item of items) {
    const id = keyOf(item) || '(missing-id)';
    const occurrence = duplicateCounts.get(id) ?? 0;
    duplicateCounts.set(id, occurrence + 1);
    result.set(occurrence === 0 ? id : `${id}#${occurrence + 1}`, item);
  }
  return result;
}

function dependencyIdentity(dependency: SectionDependency): string {
  if (dependency.kind === 'symbol') return `symbol:${dependency.symbolId}`;
  if (dependency.kind === 'relationship') return `relationship:${dependency.relationshipId}`;
  return `sourceFile:${dependency.path}`;
}

function sectionSummary(section: ChapterSection): string {
  return `section “${safeText(section.title)}”`;
}

function claimSummary(claim: ChapterClaim): string {
  const evidenceCount = claim.anchors?.length ?? 0;
  const dependencyCount = claim.dependencies?.length ?? 0;
  return `claim “${safeText(claim.prose)}” (${evidenceCount} source reference${evidenceCount === 1 ? '' : 's'}, ${dependencyCount} dependenc${dependencyCount === 1 ? 'y' : 'ies'})`;
}

function anchorSummary(anchor: ChapterSourceAnchor): string {
  return `source reference ${anchor.path}:${anchor.startLine}-${anchor.endLine} (anchor ${anchor.anchorId})`;
}

function dependencySummary(dependency: SectionDependency): string {
  return `dependency ${dependency.kind} at ${dependency.path}${dependency.kind === 'symbol' ? ` (${dependency.symbolId})` : dependency.kind === 'relationship' ? ` (${dependency.relationshipId})` : ''}`;
}

function hotspotSummary(hotspot: ChapterHotspot): string {
  return `hotspot “${safeText(hotspot.label)}” → ${safeValueSummary('destination', hotspot.destination)}`;
}

function visualSummary(visual: VisualSpec): string {
  return `${visual.kind} visual “${safeText(visual.title)}” (${visual.nodes.length} nodes, ${visual.edges.length} edges)`;
}

function fieldLabel(field: string): string {
  if (field === 'sourceSha256') return 'Source hash';
  if (field === 'excerpt') return 'Source excerpt';
  if (field === 'spokenText') return 'Spoken text';
  if (field === 'evidenceAnchorIds') return 'Source reference IDs';
  return field.replace(/[A-Z]/g, (letter) => ` ${letter.toLowerCase()}`);
}

function safeValueSummary(field: string, value: unknown): string {
  if (value === undefined) return '(missing)';
  if (value === null) return '(none)';
  if (field === 'excerpt' && typeof value === 'string') return `source excerpt (${value.length} characters; contents hidden)`;
  if (field === 'sourceSha256' && typeof value === 'string') return `SHA-256 ${value.slice(0, 12)}${value.length > 12 ? '…' : ''}`;
  if (typeof value === 'string') return `“${safeText(value)}”`;
  if (typeof value === 'number' && ['claimIndex','nodeIndex','parentIndex'].includes(field)) return String(value+1);
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (Array.isArray(value)) {
    const identifiers = value.filter((item): item is string => typeof item === 'string');
    return identifiers.length === value.length
      ? `${value.length} item${value.length === 1 ? '' : 's'}${identifiers.length ? ` [${identifiers.slice(0, 4).join(', ')}${identifiers.length > 4 ? ', …' : ''}]` : ''}`
      : `${value.length} item${value.length === 1 ? '' : 's'}`;
  }
  if (isRecord(value)) {
    if (typeof value.kind === 'string' && ['codeBehavior', 'repositoryContext', 'inference'].includes(value.kind)) {
      const label = typeof value.visibleLabel === 'string' ? ` (${safeText(value.visibleLabel)})` : '';
      return `${value.kind}${label}`;
    }
    if (field === 'layoutHint' && typeof value.direction === 'string') return value.direction;
    if (field === 'targetScope' && typeof value.kind === 'string') {
      if (value.kind === 'file' && typeof value.path === 'string') return `file “${safeText(value.path)}”`;
      if (value.kind === 'group' && Array.isArray(value.paths)) return `group (${value.paths.length} paths)`;
      if (value.kind === 'overview') return 'overview';
    }
    if (typeof value.kind === 'string' && 'destination' !== field) return value.kind;
    if (typeof value.kind === 'string' && field === 'destination') {
      const detail = typeof value.path === 'string' ? value.path
        : typeof value.sectionId === 'string' ? value.sectionId
          : typeof value.anchorId === 'string' ? value.anchorId
            : typeof value.symbolId === 'string' ? value.symbolId
              : typeof value.relationshipId === 'string' ? value.relationshipId : '';
      return `“${value.kind}${detail ? ` → ${safeText(detail)}` : ''}”`;
    }
    return `${Object.keys(value).length} fields`;
  }
  return String(value);
}

function safeText(value: string, limit = 88): string {
  const compact = value.replace(/\s+/g, ' ').trim();
  return compact.length > limit ? `${compact.slice(0, limit - 1)}…` : compact;
}

function sameValue(left: unknown, right: unknown): boolean {
  return stableJson(left) === stableJson(right);
}

function stableJson(value: unknown): string {
  return JSON.stringify(canonicalize(value)) ?? 'undefined';
}

function canonicalize(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (isRecord(value)) {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]));
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function nonEmptyString(value: string | null | undefined): string | null {
  return typeof value === 'string' && value.trim().length > 0 ? value : null;
}
