import { errorMessageOr } from '../lib/errorMessage.ts';

export type CodeSearchMode = 'code' | 'regex';
export interface SourceMatch { path:string; line:number; text:string }
export interface CodeSearchHit { path:string; text:string; from?:number; to?:number }

/** Offer AI only after a valid search settles without any local or captured matches. */
export function shouldOfferExploreWithAI(
  query:string,
  mode:CodeSearchMode,
  busy:boolean,
  error:string|null,
  resultCount:number,
):boolean {
  const length=[...query.trim()].length;
  return length>=(mode==='regex'?1:2)&&length<=240&&!busy&&!error&&resultCount===0;
}

/** Tell readers how much of a capped result set is currently visible. */
export function searchLimitMessage(resultCount:number):string {
  const count=resultCount===1?'1 result shown':`${resultCount} results shown`;
  return `Search limit reached (${count}). Refine your search to find more.`;
}

interface SearchUpdates {
  hits:(hits:SourceMatch[],append:boolean)=>void; limited:(limited:boolean)=>void;
  nextOffset:(nextOffset:number|null)=>void;
  busy:(busy:boolean)=>void; error:(error:string|null)=>void;
}
interface SearchRuntime {
  invoke:(command:string,args:Record<string,unknown>)=>Promise<unknown>;
  createId?:()=>string; defer?:(run:()=>void)=>number; clear?:(timer:number)=>void;
}

/** One debounced owned read. Cleanup invalidates replies before native cancellation. */
export function searchCapturedSource(
  request:{projectId:string;snapshotId:string;query:string;mode:CodeSearchMode;native:boolean;offset?:number},
  updates:SearchUpdates,
  runtime:SearchRuntime,
):()=>void {
  const offset=request.offset??0;
  if(offset===0){updates.hits([],false);updates.limited(false);updates.nextOffset(null);}
  updates.error(null);updates.busy(false);
  const length=[...request.query].length;
  if(!request.native||length<(request.mode==='regex'?1:2)||length>240)return()=>{};
  const requestId=(runtime.createId??(()=>crypto.randomUUID()))();
  const defer=runtime.defer??((run:()=>void)=>window.setTimeout(run,250));
  const clear=runtime.clear??((timer:number)=>window.clearTimeout(timer));
  let active=true,dispatched=false;
  updates.busy(true);
  const timer=defer(()=>{dispatched=true;void(async()=>{
    try{
      const result=await runtime.invoke('search_workspace_source',{projectId:request.projectId,snapshotId:request.snapshotId,query:request.query,mode:request.mode,requestId,offset}) as {matches:SourceMatch[];limited:boolean;nextOffset?:number|null};
      if(active){updates.hits(result.matches,offset>0);updates.limited(result.limited);updates.nextOffset(result.nextOffset??null);}
    }catch(error){if(active)updates.error(request.mode==='regex'?errorMessageOr(error,'This regular expression could not be searched.'):'Captured source search is unavailable. File and symbol results are still shown.');}
    finally{if(active)updates.busy(false);}
  })();});
  return()=>{active=false;clear(timer);if(dispatched)void runtime.invoke('cancel_repository_context',{requestId}).catch(()=>{});};
}

/** Regex results come only from native verified code, never a JavaScript regex or fuzzy local hit. */
export function workspaceSearchResults({query,mode,files,symbols,source,captured}:{
  query:string;mode:CodeSearchMode;files:{path:string}[];
  symbols:{path:string;name:string;anchor:{startLine:number;endLine:number}}[];
  source:{path:string;contents:string}|null;captured:SourceMatch[];
}):CodeSearchHit[]{
  if(!query)return[];
  const code=captured.map(hit=>({path:hit.path,text:hit.text,from:hit.line,to:hit.line}));
  const term=query.toLowerCase();
  const local:CodeSearchHit[]=[];
  if(mode==='code'){
    const symbolMatches=symbols.filter(symbol=>symbol.name.toLowerCase().includes(term)).slice().sort((left,right)=>{
      const rank=(name:string)=>name.toLowerCase()===term?0:name.toLowerCase().startsWith(term)?1:2;
      return rank(left.name)-rank(right.name)||left.name.localeCompare(right.name)||left.path.localeCompare(right.path);
    });
    local.push(...symbolMatches.slice(0,8).map(symbol=>({path:symbol.path,text:symbol.name,from:symbol.anchor.startLine,to:symbol.anchor.endLine})));
    local.push(...files.filter(file=>file.path.toLowerCase().includes(term)).slice(0,8).map(file=>({path:file.path,text:file.path})));
  }
  const all=mode==='regex'?code:[...local,...code];
  if(mode==='code'&&source){
    let line=0,count=0;
    for(const text of source.contents.split('\n')){
      line++;if(!text.toLowerCase().includes(term))continue;
      all.push({path:source.path,text:text.trim(),from:line,to:line});if(++count===32)break;
    }
  }
  const seen=new Set<string>();
  return all.filter(hit=>{const key=JSON.stringify([hit.path,hit.from,hit.text]);if(seen.has(key))return false;seen.add(key);return true;}).slice(0,80);
}

/** Search matches are starting points for investigation, not proof of an answer. */
export function searchExploration(query:string,mode:CodeSearchMode,captured:SourceMatch[]){
  const question=mode==='regex'?`Explain the code matching this regular expression: ${query}`:query.length<8?`Explain ${query}`:query;
  const context:{path:string;from:number;to:number}[]=[];
  if(mode==='regex')for(const hit of captured){if(context.some(item=>item.path===hit.path))continue;context.push({path:hit.path,from:hit.line,to:hit.line});if(context.length===4)break;}
  return {question,context};
}
