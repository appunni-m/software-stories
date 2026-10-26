export type RepositorySearchMode='code'|'regex';
export type RepositorySidebarActivity='files'|'search'|'history';

export interface RepositorySearchContext {
  projectId:string;
  snapshotId:string;
  query:string;
  mode:RepositorySearchMode;
  activity:RepositorySidebarActivity;
  updatedAt:number;
}

const STORAGE_KEY='code-atlas:repository-search:v1';
const MAX_ENTRIES=32;
const MAX_STORAGE_CHARACTERS=32_768;
const MAX_QUERY_LENGTH=240;
const MAX_ID_LENGTH=256;

function cleanEntry(value:unknown):RepositorySearchContext|undefined {
  if(!value||typeof value!=='object'||Array.isArray(value))return undefined;
  const entry=value as Partial<RepositorySearchContext>;
  if(typeof entry.projectId!=='string'||entry.projectId.length===0||entry.projectId.length>MAX_ID_LENGTH
    ||typeof entry.snapshotId!=='string'||entry.snapshotId.length===0||entry.snapshotId.length>MAX_ID_LENGTH
    ||typeof entry.query!=='string'||entry.query.length>MAX_QUERY_LENGTH
    ||(entry.mode!=='code'&&entry.mode!=='regex')
    ||(entry.activity!=='files'&&entry.activity!=='search'&&entry.activity!=='history')
    ||typeof entry.updatedAt!=='number'||!Number.isSafeInteger(entry.updatedAt)||entry.updatedAt<0)return undefined;
  return {projectId:entry.projectId,snapshotId:entry.snapshotId,query:entry.query,mode:entry.mode,activity:entry.activity,updatedAt:entry.updatedAt};
}

function readEntries():RepositorySearchContext[] {
  try{
    const raw=localStorage.getItem(STORAGE_KEY);
    if(!raw||raw.length>MAX_STORAGE_CHARACTERS)return [];
    const decoded:unknown=JSON.parse(raw);
    if(!decoded||typeof decoded!=='object'||Array.isArray(decoded)||(decoded as {version?:unknown}).version!==1
      ||!Array.isArray((decoded as {entries?:unknown}).entries))return [];
    const entries=(decoded as {entries:unknown[]}).entries;
    if(entries.length>MAX_ENTRIES)return [];
    const unique=new Map<string,RepositorySearchContext>();
    for(const value of entries){
      const entry=cleanEntry(value);
      if(entry)unique.set(`${entry.projectId}\n${entry.snapshotId}`,entry);
    }
    return [...unique.values()].sort((left,right)=>right.updatedAt-left.updatedAt).slice(0,MAX_ENTRIES);
  }catch{return [];}
}

/** Search text is a local preference, scoped to the immutable repository snapshot. */
export function readRepositorySearchContext(projectId:string,snapshotId:string):RepositorySearchContext|undefined {
  return readEntries().find((entry)=>entry.projectId===projectId&&entry.snapshotId===snapshotId);
}

export function rememberRepositorySearchContext(
  projectId:string,
  snapshotId:string,
  input:Pick<RepositorySearchContext,'query'|'mode'|'activity'>,
):void {
  if(!projectId||projectId.length>MAX_ID_LENGTH||!snapshotId||snapshotId.length>MAX_ID_LENGTH
    ||typeof input.query!=='string'||input.query.length>MAX_QUERY_LENGTH
    ||(input.mode!=='code'&&input.mode!=='regex')
    ||(input.activity!=='files'&&input.activity!=='search'&&input.activity!=='history'))return;
  const entry:RepositorySearchContext={...input,projectId,snapshotId,updatedAt:Date.now()};
  const key=`${projectId}\n${snapshotId}`;
  const entries=[entry,...readEntries().filter((candidate)=>`${candidate.projectId}\n${candidate.snapshotId}`!==key)]
    .slice(0,MAX_ENTRIES);
  const serialize=()=>JSON.stringify({version:1,entries});
  let serialized=serialize();
  while(entries.length>1&&serialized.length>MAX_STORAGE_CHARACTERS){
    entries.pop();
    serialized=serialize();
  }
  if(serialized.length>MAX_STORAGE_CHARACTERS)return;
  try{localStorage.setItem(STORAGE_KEY,serialized);}catch{/* Search remains usable when local preferences cannot be written. */}
}
