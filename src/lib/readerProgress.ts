/** Presentation metadata only. None of these records grants access or starts playback. */
export interface SectionIdentity { revisionId:string; sectionId:string; sectionRevisionId:string }
export interface FlowViewport { x:number; y:number; zoom:number }
export const FOCUSED_STORY_LAYOUT_VERSION = 7 as const;
export const FLOW_DIAGRAM_LAYOUT_VERSION = 8 as const;
export interface StoryProgress extends SectionIdentity {
  sceneId:string; positions:Record<string,number>; fullMap:boolean; viewport?:FlowViewport;
  focusedNodeId?:string; focusedEdgeId?:string; focusLayoutVersion:typeof FOCUSED_STORY_LAYOUT_VERSION;
}
export interface DiagramProgress extends SectionIdentity { viewport?:FlowViewport; focusedNodeId?:string; focusedEdgeId?:string; layoutVersion:typeof FLOW_DIAGRAM_LAYOUT_VERSION }
export interface ReaderProgress { stories:StoryProgress[]; diagrams:DiagramProgress[] }

const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const only=(value:Record<string,unknown>,keys:readonly string[])=>Object.keys(value).every(key=>keys.includes(key));
const id=(value:unknown):value is string=>typeof value==='string'&&/^[A-Za-z0-9_.:-]{1,256}$/.test(value);
const identity=(value:Record<string,unknown>):value is Record<string,unknown>&SectionIdentity=>id(value.revisionId)&&id(value.sectionId)&&id(value.sectionRevisionId);
export function sameSection(a:SectionIdentity,b:SectionIdentity):boolean {
  return a.revisionId===b.revisionId&&a.sectionId===b.sectionId&&a.sectionRevisionId===b.sectionRevisionId;
}
export function cleanFlowViewport(value:unknown):FlowViewport|undefined {
  if(!object(value)||!only(value,['x','y','zoom'])||Object.keys(value).length!==3
    ||typeof value.x!=='number'||typeof value.y!=='number'||typeof value.zoom!=='number'
    ||!Number.isFinite(value.x)||Math.abs(value.x)>10_000_000||!Number.isFinite(value.y)||Math.abs(value.y)>10_000_000
    ||!Number.isFinite(value.zoom)||value.zoom<.1||value.zoom>4)return undefined;
  return {x:value.x,y:value.y,zoom:value.zoom};
}
export function cleanStoryProgress(value:unknown):StoryProgress|undefined {
  const required=['revisionId','sectionId','sectionRevisionId','sceneId','positions','fullMap','focusLayoutVersion'];
  const allowed=[...required,'viewport','focusedNodeId','focusedEdgeId'];
  if(!object(value)||!only(value,allowed)||!required.every(key=>Object.hasOwn(value,key))
    ||!identity(value)||!id(value.sceneId)||typeof value.fullMap!=='boolean'||!object(value.positions))return undefined;
  // Older layouts may frame the wider v7 label lanes incorrectly. Preserve
  // the selected scene, narration positions and focus, then refit the graph.
  if(value.focusLayoutVersion!==4 && value.focusLayoutVersion!==5 && value.focusLayoutVersion!==6 && value.focusLayoutVersion!==FOCUSED_STORY_LAYOUT_VERSION)return undefined;
  if(Object.hasOwn(value,'focusedNodeId')&&!id(value.focusedNodeId)
    ||Object.hasOwn(value,'focusedEdgeId')&&!id(value.focusedEdgeId))return undefined;
  const entries=Object.entries(value.positions);
  if(entries.length>128||entries.some(([key,n])=>!id(key)||typeof n!=='number'||!Number.isFinite(n)||n<0||n>3600))return undefined;
  const viewport=value.focusLayoutVersion===FOCUSED_STORY_LAYOUT_VERSION&&Object.hasOwn(value,'viewport')?cleanFlowViewport(value.viewport):undefined;
  if(value.focusLayoutVersion===FOCUSED_STORY_LAYOUT_VERSION&&Object.hasOwn(value,'viewport')&&!viewport)return undefined;
  const focusedNodeId=value.focusedNodeId as string|undefined;
  const focusedEdgeId=value.focusedEdgeId as string|undefined;
  return {revisionId:value.revisionId,sectionId:value.sectionId,sectionRevisionId:value.sectionRevisionId,
    sceneId:value.sceneId,positions:Object.fromEntries(entries) as Record<string,number>,fullMap:value.fullMap,
    focusLayoutVersion:FOCUSED_STORY_LAYOUT_VERSION,
    ...(focusedNodeId?{focusedNodeId}:{}),...(focusedEdgeId?{focusedEdgeId}:{}),
    ...(viewport?{viewport}:{})};
}
export function cleanDiagramProgress(value:unknown):DiagramProgress|undefined {
  const required=['revisionId','sectionId','sectionRevisionId','layoutVersion'];
  const keys=[...required,'viewport','focusedNodeId','focusedEdgeId'];
  if(!object(value)||!only(value,keys)||!required.every(key=>Object.hasOwn(value,key))
    ||!identity(value)||value.layoutVersion!==FLOW_DIAGRAM_LAYOUT_VERSION)return undefined;
  const viewport=Object.hasOwn(value,'viewport')?cleanFlowViewport(value.viewport):undefined;
  const focusedNodeId=value.focusedNodeId as string|undefined;
  const focusedEdgeId=value.focusedEdgeId as string|undefined;
  if(Object.hasOwn(value,'viewport')&&!viewport||Object.hasOwn(value,'focusedNodeId')&&!id(focusedNodeId)
    ||Object.hasOwn(value,'focusedEdgeId')&&!id(focusedEdgeId)
    ||!viewport&&!focusedNodeId&&!focusedEdgeId)return undefined;
  return {revisionId:value.revisionId,sectionId:value.sectionId,sectionRevisionId:value.sectionRevisionId,layoutVersion:FLOW_DIAGRAM_LAYOUT_VERSION,
    ...(viewport?{viewport}:{}),...(focusedNodeId?{focusedNodeId}: {}),...(focusedEdgeId?{focusedEdgeId}: {})};
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
