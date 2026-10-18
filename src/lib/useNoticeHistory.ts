import {useEffect,useState,useSyncExternalStore} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {isNativeRuntime} from './commands';
import {emptyNoticeHistory,NoticeHistory,noticeKey,projectNotices,type NoticeHistoryData} from './noticeHistory';

export function useNoticeHistory() {
  const [history]=useState(()=>{
    // Browser design previews have no native persistence or filesystem access.
    if(!isNativeRuntime()) {
      let saved=emptyNoticeHistory();
      return new NoticeHistory({load:async()=>saved,update:async(operation)=>{
        saved=projectNotices(saved,[{operation,...(operation.kind==='publish'?{item:{id:await noticeKey(operation.input),input:operation.input,read:false,createdAtUnixMs:Date.now()}}:{})}]);return saved;
      }});
    }
    return new NoticeHistory({load:()=>invoke<NoticeHistoryData>('list_app_notices'),
      update:(operation)=>invoke<NoticeHistoryData>('update_app_notices',{operation})});
  });
  useEffect(()=>{history.start();},[history]);
  return {history,state:useSyncExternalStore(history.subscribe,history.getSnapshot,history.getSnapshot)};
}
