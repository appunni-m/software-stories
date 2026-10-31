import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {isNativeRuntime} from './commands';
import {APP_ZOOM_KEY,compactZoomLayout,scaledZoomScrollOffset,validZoom,savedZoom,zoomShortcut} from './pageZoom';

let installation:Promise<void>|null=null;
/** Native menus and shortcuts use the same native level. The old injected
 * polyfill is disabled so one key press cannot change two independent levels. */
export function installAppZoom():Promise<void> {
  if(!isNativeRuntime())return Promise.resolve();
  if(installation)return installation;
  installation=(async()=>{
    const remember=(event:{percent:unknown;persist?:boolean})=>{
      const percent=event.percent;
      if(!validZoom(percent))return;
      const previousPercent=savedZoom(document.documentElement.dataset.appZoom??null);
      if(previousPercent!==percent){
        requestAnimationFrame(()=>{
          document.querySelectorAll<HTMLElement>('.dw-answer-scroll,.dw-tool-content,.dw-sidebar,.dw-file-list,.walkthrough-transcript-scroll,.story-html-stage').forEach((element)=>{
            if(element.scrollTop>0)element.scrollTop=scaledZoomScrollOffset(element.scrollTop,previousPercent,percent);
            if(element.scrollLeft>0)element.scrollLeft=scaledZoomScrollOffset(element.scrollLeft,previousPercent,percent);
          });
        });
      }
      document.documentElement.dataset.appZoom=String(percent);
      document.documentElement.dataset.appZoomCompact=String(compactZoomLayout(percent));
      // The app header contains both repository controls and app-wide actions.
      // It runs out of breathing room before the deeper workspace should switch
      // to its compact single-pane layout, so reflow just the header at 160%.
      document.documentElement.dataset.appZoomHeaderCompact=String(percent>=160);
      if(event.persist===true)try{localStorage.setItem(APP_ZOOM_KEY,String(percent));}catch{/* Zoom still works for this window. */}
    };
    let percent=100;try{percent=savedZoom(localStorage.getItem(APP_ZOOM_KEY));}catch{/* Open at the actual default. */}
    const stop=await listen<{percent:number;persist:boolean}>('code-atlas:zoom-changed',event=>remember(event.payload));
    try{await invoke<number>('restore_app_zoom',{percent});}catch(error){stop();throw error;}
    window.addEventListener('keydown',event=>{
      const action=zoomShortcut(event);if(!action)return;
      event.preventDefault();event.stopImmediatePropagation();
      // Native events are ordered; concurrent RPC replies are not. Remember
      // only the event, so an older reply cannot replace the newest preference.
      void invoke<number>('change_app_zoom',{action}).catch(()=>{
        window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{title:'Text size could not be changed',message:'Try again from the View menu. Your explanation is preserved.',occurrenceId:crypto.randomUUID()}}));
      });
    },true);
  })().catch(error=>{installation=null;throw error;});
  return installation;
}
