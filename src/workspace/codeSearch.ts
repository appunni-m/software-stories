import { errorMessageOr } from '../lib/errorMessage.ts';

export type CodeSearchMode = 'code' | 'regex';
export interface SourceMatch { path:string; line:number; text:string }
export interface CodeSearchHit { path:string; text:string; from?:number; to?:number }

/** Offer AI when a settled search has no hits or did not complete. The visible
 * search error remains, while the user can still ask AI to interpret the query. */
export function shouldOfferExploreWithAI(
  query:string,
  mode:CodeSearchMode,
  busy:boolean,
  error:string|null,
  resultCount:number,
):boolean {
  const length=[...query.trim()].length;
  const hasFallback=Boolean(error)||resultCount===0;
  return length>=(mode==='regex'?1:2)&&length<=240&&!busy&&hasFallback;
}

/** Tell readers how much of a capped result set is currently visible. */
export function searchLimitMessage(resultCount:number):string {
  const count=resultCount===1?'1 result shown':`${resultCount} results shown`;
  return `Search limit reached (${count}). Refine your search to find more.`;
}

/** Report only the number of rows currently visible; the search API is paged. */
export function searchResultsMessage(resultCount:number,moreAvailable:boolean):string {
  const count=resultCount===1?'1 result shown':`${resultCount} results shown`;
  return `${count}${moreAvailable?'; more matches available':''}.`;
}

/** Keep a minified or generated source line from expanding the explorer UI. */
function searchSnippet(text:string,query:string,maxLength=360):string {
  const value=text.trim();
  if(value.length<=maxLength)return value;
  const match=value.toLowerCase().indexOf(query.toLowerCase());
  let start=Math.max(0,Math.min(value.length-maxLength,match<0?0:match-Math.floor((maxLength-query.length)/2)));
  if(start>0&&value.charCodeAt(start)>=0xdc00&&value.charCodeAt(start)<=0xdfff)start++;
  let end=Math.min(value.length,start+maxLength);
  if(end<value.length&&value.charCodeAt(end)>=0xdc00&&value.charCodeAt(end)<=0xdfff)end--;
  return `${start>0?'…':''}${value.slice(start,end)}${end<value.length?'…':''}`;
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
      all.push({path:source.path,text:searchSnippet(text,term),from:line,to:line});if(++count===32)break;
    }
  }
  const seenFiles=new Set<string>();
  const seenLocations=new Map<string,number>();
  const unique:CodeSearchHit[]=[];
  for(const hit of all){
    if(hit.from===undefined){
      const key=JSON.stringify([hit.path,hit.text]);
      if(seenFiles.has(key))continue;
      seenFiles.add(key);
      unique.push(hit);
      continue;
    }
    const key=JSON.stringify([hit.path,hit.from]);
    const existingIndex=seenLocations.get(key);
    if(existingIndex!==undefined){
      const existing=unique[existingIndex];
      const to=Math.max(existing.to??existing.from??hit.from,hit.to??hit.from);
      if(hit.text.length>existing.text.length)unique[existingIndex]={...existing,text:hit.text,to};
      else if(to>(existing.to??existing.from??0))unique[existingIndex]={...existing,to};
      continue;
    }
    seenLocations.set(key,unique.length);
    unique.push(hit);
  }
  return unique.slice(0,80);
}

/** Search matches are starting points for investigation, not proof of an answer. */
export function searchExploration(query:string,mode:CodeSearchMode,matches:CodeSearchHit[]){
  const question=mode==='regex'?`Explain the code matching this regular expression: ${query}`:query.length<8?`Explain ${query}`:query;
  const candidates:{path:string;from?:number;to?:number}[]=[];
  const seen=new Set<string>();
  for(const hit of matches){
    const reference={path:hit.path,...(hit.from!==undefined?{from:hit.from,to:hit.to??hit.from}:{})};
    const key=JSON.stringify(reference);
    if(seen.has(key))continue;
    seen.add(key);candidates.push(reference);
  }
  const context:{path:string;from?:number;to?:number}[]=[];
  const selected=new Set<string>();
  const selectedPaths=new Set<string>();
  for(const reference of candidates){
    if(selectedPaths.has(reference.path))continue;
    context.push(reference);selected.add(JSON.stringify(reference));selectedPaths.add(reference.path);
    if(context.length===8)break;
  }
  if(context.length<8)for(const reference of candidates){
    const key=JSON.stringify(reference);
    if(selected.has(key))continue;
    context.push(reference);selected.add(key);
    if(context.length===8)break;
  }
  return {question,context};
}
