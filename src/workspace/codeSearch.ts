import { errorMessageOr } from '../lib/errorMessage.ts';

export type CodeSearchMode = 'code' | 'regex';
export interface SourceMatch { path:string; line:number; text:string }
export interface CodeSearchHit { path:string; text:string; from?:number; to?:number }
export interface IndexLimitOmission { path:string; reason:'localRangeLimit'|'localSourceByteLimit' }
export interface IndexLimitOmissions {
  sourceByteLimitedFiles:number; rangeLimitedFiles:number;
  files:IndexLimitOmission[]; filesTruncated:boolean;
}

const SEARCH_STOP_WORDS = new Set(['a','an','and','are','does','for','from','how','in','is','of','on','or','the','this','to','what','where','which','why','with','work','works','explain','find','help','show','tell']);
const SEARCH_EXPLORATION_CONTEXT_LIMIT = 4;
const TEST_OR_FIXTURE_DIRECTORIES = new Set(['test','tests','__tests__','spec','specs','fixture','fixtures','testdata','test-data','test_fixtures','test-fixtures']);
const TEST_OR_FIXTURE_QUERY_TERMS = new Set(['test','tests','fixture','fixtures','spec','specs','testcase','testcases','example','examples']);
const TEST_OR_FIXTURE_FILENAME = /(^|[._-])(?:test|spec|fixture|fixtures)(?=$|[._-])/u;

function searchableParts(value:string):string[]{
  return value.replace(/([a-z0-9])([A-Z])/g,'$1 $2').toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
}

function stableTextCompare(left:string,right:string):number{
  return left<right?-1:left>right?1:0;
}

function isTestOrFixturePath(path:string):boolean{
  const parts=path.toLowerCase().split('/');
  const filename=parts.at(-1)??'';
  return parts.slice(0,-1).some(part=>TEST_OR_FIXTURE_DIRECTORIES.has(part))
    || TEST_OR_FIXTURE_FILENAME.test(filename);
}

/** Put executable implementation ahead of supporting fixtures, unless the
 * reader explicitly asks to inspect tests, fixtures, specs, or examples. */
function prioritizeSearchHits<T extends {path:string}>(hits:readonly T[],query:string):T[]{
  const testsFirst=searchableParts(query).some(part=>TEST_OR_FIXTURE_QUERY_TERMS.has(part));
  return hits.map((hit,index)=>({hit,index})).sort((left,right)=>{
    const leftIsTest=isTestOrFixturePath(left.hit.path);
    const rightIsTest=isTestOrFixturePath(right.hit.path);
    const leftPriority=leftIsTest===testsFirst?0:1;
    const rightPriority=rightIsTest===testsFirst?0:1;
    return leftPriority-rightPriority||left.index-right.index;
  }).map(({hit})=>hit);
}

/** Extract useful identifiers from a natural-language query without treating
 * its stop words as required code tokens. */
function localQueryTerms(query:string):string[]{
  const parts=searchableParts(query);
  const useful=[...new Set(parts.filter(part=>!SEARCH_STOP_WORDS.has(part)))];
  return useful.length?useful:parts.length?[query.toLowerCase()]:[];
}

/** Rank a local path, symbol or line by the useful words it contains. Requiring
 * every word makes natural questions such as “how does a font become pixels?”
 * miss the `Font` entry point entirely. Prefix matches cover common inflections
 * while token boundaries avoid matching `font` inside `confront`.
 */
function localMatcher(query:string,terms:readonly string[]):(value:string)=>number{
  const phrase=searchableParts(query).join(' ');
  return (value:string)=>{
    const parts=searchableParts(value);
    if(!parts.length)return 0;
    const matched=terms.filter(term=>parts.some(part=>{
      if(part===term)return true;
      if(Math.min(part.length,term.length)<3)return false;
      if(part.startsWith(term))return true;
      // Permit common plural endings when the user typed the longer form,
      // but never let an unrelated long term match a short source token
      // (for example, "nonexistent" matching the identifier "None").
      if(!term.startsWith(part))return false;
      const suffix=term.slice(part.length);
      return suffix==='s'||suffix==='es';
    }));
    if(!matched.length)return 0;
    const normalized=parts.join(' ');
    const exactPhrase=phrase&&normalized.includes(phrase)?1_000:0;
    return exactPhrase+matched.length*100+matched.reduce((score,term)=>score+Math.min(term.length,12),0);
  };
}

/** Keep only the best few local hits while scanning a large index. */
function topMatches<T>(items:readonly T[],scoreOf:(item:T)=>number,compare:(left:T,right:T)=>number,limit:number):T[]{
  const best:{item:T;score:number}[]=[];
  for(const item of items){
    const score=scoreOf(item);
    if(score<=0)continue;
    let index=0;
    while(index<best.length&&(best[index].score>score
      ||(best[index].score===score&&compare(best[index].item,item)<=0)))index++;
    if(index===best.length&&best.length===limit)continue;
    best.splice(index,0,{item,score});
    if(best.length>limit)best.pop();
  }
  return best.map(({item})=>item);
}

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
  if(mode==='regex'&&error&&/(?:invalid|malformed|syntax).*(?:regular expression|regex)|(?:regular expression|regex).*(?:invalid|malformed|syntax)/iu.test(error))return false;
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
function searchSnippet(text:string,queryTerms:readonly string[],maxLength=360):string {
  const value=text.trim();
  if(value.length<=maxLength)return value;
  const folded=value.toLowerCase();
  let match=-1;
  let matchLength=0;
  for(const term of queryTerms){
    const index=folded.indexOf(term.toLowerCase());
    if(index>=0&&(match<0||index<match)){match=index;matchLength=term.length;}
  }
  let start=Math.max(0,Math.min(value.length-maxLength,match<0?0:match-Math.floor((maxLength-matchLength)/2)));
  if(start>0&&value.charCodeAt(start)>=0xdc00&&value.charCodeAt(start)<=0xdfff)start++;
  let end=Math.min(value.length,start+maxLength);
  if(end<value.length&&value.charCodeAt(end)>=0xdc00&&value.charCodeAt(end)<=0xdfff)end--;
  return `${start>0?'…':''}${value.slice(start,end)}${end<value.length?'…':''}`;
}

interface SearchUpdates {
  hits:(hits:SourceMatch[],append:boolean)=>void; limited:(limited:boolean)=>void;
  nextOffset:(nextOffset:number|null)=>void;
  coverage?:(unavailableFiles:number,omissions:IndexLimitOmissions)=>void;
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
  if(offset===0){updates.hits([],false);updates.limited(false);updates.nextOffset(null);updates.coverage?.(0,{sourceByteLimitedFiles:0,rangeLimitedFiles:0,files:[],filesTruncated:false});}
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
      const result=await runtime.invoke('search_workspace_source',{projectId:request.projectId,snapshotId:request.snapshotId,query:request.query,mode:request.mode,requestId,offset}) as {matches:SourceMatch[];limited:boolean;nextOffset?:number|null;unavailableFiles:number;indexLimitOmissions:IndexLimitOmissions};
      if(active){updates.hits(result.matches,offset>0);updates.limited(result.limited);updates.nextOffset(result.nextOffset??null);updates.coverage?.(result.unavailableFiles,result.indexLimitOmissions);}
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
  const terms=localQueryTerms(query);
  const scoreLocal=localMatcher(query,terms);
  const local:CodeSearchHit[]=[];
  if(mode==='code'){
    const symbolMatches=topMatches(symbols,symbol=>scoreLocal(`${symbol.name} ${symbol.path}`),
      (left,right)=>stableTextCompare(left.name,right.name)||stableTextCompare(left.path,right.path),8);
    local.push(...symbolMatches.map((symbol)=>({path:symbol.path,text:symbol.name,from:symbol.anchor.startLine,to:symbol.anchor.endLine})));
    const fileMatches=topMatches(files,file=>scoreLocal(file.path),(left,right)=>stableTextCompare(left.path,right.path),8);
    local.push(...fileMatches.map((file)=>({path:file.path,text:file.path})));
  }
  // Exact symbol/path hits lead, followed by ranked native text matches and
  // then the currently open file. Regex mode remains exact and native-only.
  const all=mode==='regex'?code:[...local,...code];
  if(mode==='code'&&source){
    // Avoid allocating an array containing every line of a large open file on
    // each query update. Search only slices as we reach them and stop once the
    // bounded fallback has enough hits.
    const contents=source.contents;
    let start=0,line=0,count=0;
    while(start<=contents.length){
      const lf=contents.indexOf('\n',start);
      const cr=contents.indexOf('\r',start);
      const nextBreak=lf<0?cr:cr<0?lf:Math.min(lf,cr);
      const end=nextBreak<0?contents.length:nextBreak;
      const text=contents.slice(start,end);
      line++;
      if(scoreLocal(text)){
        all.push({path:source.path,text:searchSnippet(text,terms.length?terms:[term]),from:line,to:line});
        if(++count===32)break;
      }
      if(nextBreak<0)break;
      start=contents[nextBreak]==='\r'&&contents[nextBreak+1]==='\n'?nextBreak+2:nextBreak+1;
    }
  }
  const seenFiles=new Set<string>();
  const seenLocations=new Map<string,number>();
  const unique:CodeSearchHit[]=[];
  for(const hit of mode==='code'?prioritizeSearchHits(all,query):all){
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
    if(context.length===SEARCH_EXPLORATION_CONTEXT_LIMIT)break;
  }
  if(context.length<SEARCH_EXPLORATION_CONTEXT_LIMIT)for(const reference of candidates){
    const key=JSON.stringify(reference);
    if(selected.has(key))continue;
    context.push(reference);selected.add(key);
    if(context.length===SEARCH_EXPLORATION_CONTEXT_LIMIT)break;
  }
  return {question,context};
}
