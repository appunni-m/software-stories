/** Presentation metadata only. None of these records grants access or starts playback. */
export interface SectionIdentity { revisionId:string; sectionId:string; sectionRevisionId:string }
export interface FlowViewport { x:number; y:number; zoom:number }
export const FOCUSED_STORY_LAYOUT_VERSION = 4 as const;
export const FLOW_DIAGRAM_LAYOUT_VERSION = 4 as const;
export interface StoryProgress extends SectionIdentity {
  sceneId:string; positions:Record<string,number>; fullMap:boolean; viewport?:FlowViewport;
  focusedNodeId?:string; focusedEdgeId?:string; focusLayoutVersion:typeof FOCUSED_STORY_LAYOUT_VERSION;
}
export interface DiagramProgress extends SectionIdentity { viewport:FlowViewport; layoutVersion:typeof FLOW_DIAGRAM_LAYOUT_VERSION }
export interface ReaderProgress { stories:StoryProgress[]; diagrams:DiagramProgress[] }

const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const id=(value:unknown):value is string=>typeof value==='string'&&/^[A-Za-z0-9_.:-]{1,256}$/.test(value);
const identity=(value:Record<string,unknown>):value is Record<string,unknown>&SectionIdentity=>id(value.revisionId)&&id(value.sectionId)&&id(value.sectionRevisionId);
export function sameSection(a:SectionIdentity,b:SectionIdentity):boolean {
  return a.revisionId===b.revisionId&&a.sectionId===b.sectionId&&a.sectionRevisionId===b.sectionRevisionId;
}
export function cleanFlowViewport(value:unknown):FlowViewport|undefined {
  if(!object(value)||typeof value.x!=='number'||typeof value.y!=='number'||typeof value.zoom!=='number'
    ||!Number.isFinite(value.x)||Math.abs(value.x)>10_000_000||!Number.isFinite(value.y)||Math.abs(value.y)>10_000_000
    ||!Number.isFinite(value.zoom)||value.zoom<.1||value.zoom>4)return undefined;
  return {x:value.x,y:value.y,zoom:value.zoom};
}
export function cleanStoryProgress(value:unknown):StoryProgress|undefined {
  if(!object(value)||!identity(value)||!id(value.sceneId)||typeof value.fullMap!=='boolean'||!object(value.positions))return undefined;
  const previousLayout = value.focusLayoutVersion === FOCUSED_STORY_LAYOUT_VERSION - 1;
  if(value.focusLayoutVersion!==FOCUSED_STORY_LAYOUT_VERSION&&!previousLayout)return undefined;
  const entries=Object.entries(value.positions);
  if(entries.length>128||entries.some(([key,n])=>!id(key)||typeof n!=='number'||!Number.isFinite(n)||n<0||n>3600))return undefined;
  // Preserve scene/audio progress across a renderer layout change, but never
  // apply coordinates produced by the older graph layout.
  const viewport=previousLayout||value.viewport===undefined?undefined:cleanFlowViewport(value.viewport);
  if(!previousLayout&&value.viewport!==undefined&&!viewport)return undefined;
  const focusedNodeId=id(value.focusedNodeId)?value.focusedNodeId:undefined;
  const focusedEdgeId=id(value.focusedEdgeId)?value.focusedEdgeId:undefined;
  return {revisionId:value.revisionId,sectionId:value.sectionId,sectionRevisionId:value.sectionRevisionId,
    sceneId:value.sceneId,positions:Object.fromEntries(entries) as Record<string,number>,fullMap:value.fullMap,
    focusLayoutVersion:FOCUSED_STORY_LAYOUT_VERSION,
    ...(focusedNodeId?{focusedNodeId}:{}),...(focusedEdgeId?{focusedEdgeId}:{}),
    ...(viewport?{viewport}:{})};
}
export function cleanDiagramProgress(value:unknown):DiagramProgress|undefined {
  if(!object(value)||!identity(value)||value.layoutVersion!==FLOW_DIAGRAM_LAYOUT_VERSION)return undefined;
  const viewport=cleanFlowViewport(value.viewport);
  return viewport?{revisionId:value.revisionId,sectionId:value.sectionId,sectionRevisionId:value.sectionRevisionId,viewport,layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION}:undefined;
}
export function cleanReaderProgress(value:unknown):ReaderProgress {
  const result:ReaderProgress={stories:[],diagrams:[]};
  if(!object(value))return result;
  for(const [key,clean] of [['stories',cleanStoryProgress],['diagrams',cleanDiagramProgress]] as const) {
    const list=value[key];if(!Array.isArray(list)||list.length>256)continue;
    for(const item of list){
      const next=clean(item);if(!next||result[key].some(saved=>sameSection(saved,next)))continue;
      if(key==='stories')result.stories.push(next as StoryProgress);else result.diagrams.push(next as DiagramProgress);
    }
  }
  return result;
}

/** Keep one explicit position for each exact immutable section; oldest metadata
 * may age out at the bound without removing a question or generated artifact. */
export function withReaderProgress<T extends StoryProgress|DiagramProgress>(list:readonly T[],next:T):T[] {
  return [...list.filter(saved=>!sameSection(saved,next)),next].slice(-256);
}
