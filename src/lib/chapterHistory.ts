import type {ChapterRevision} from './commands';
import {createRefreshQueue,type RefreshQueue} from '../workspace/refreshQueue.ts';

export interface ChapterHistory extends RefreshQueue {
  /** Record a successful native write immediately, independently of later reads. */
  record(revision:ChapterRevision):void;
}

/** Local history reads never own editor selection or an unsaved candidate.
 * Native receipts remain visible until a fresh read includes them. This also
 * covers an older in-flight read completing after a successful write. */
export function createChapterHistory({projectId,load,onValue,onLoaded,onError,onIdle}:{
  projectId:string;
  load:()=>Promise<ChapterRevision[]>;
  onValue:(revisions:ChapterRevision[])=>void;
  onLoaded:()=>void;
  onError:(error:unknown)=>void;
  onIdle?:()=>void;
}):ChapterHistory {
  let current:ChapterRevision[]=[];
  let disposed=false;
  const receipts=new Map<string,ChapterRevision>();
  const merge=(saved:ChapterRevision[],pending:Iterable<ChapterRevision>)=>
    [...new Map([...saved,...pending].map(revision=>[revision.revisionId,revision])).values()];
  const queue=createRefreshQueue({
    load,
    onValue:saved=>{
      for(const revision of saved)receipts.delete(revision.revisionId);
      current=merge(saved,receipts.values());
      onValue(current);onLoaded();
    },
    onError,onIdle,
  });
  return {
    refresh:()=>queue.refresh(),
    record(revision){
      if(disposed||revision.projectId!==projectId)return;
      receipts.set(revision.revisionId,revision);
      current=merge(current,[revision]);onValue(current);
      void queue.refresh();
    },
    dispose(){disposed=true;queue.dispose();receipts.clear();},
  };
}
