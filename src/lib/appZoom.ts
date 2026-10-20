import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {isNativeRuntime} from './commands';
import {APP_ZOOM_KEY,compactZoomLayout,validZoom,savedZoom,zoomShortcut} from './pageZoom';

let installation:Promise<void>|null=null;
/** Native menus and shortcuts use the same native level. The old injected
 * polyfill is disabled so one key press cannot change two independent levels. */
export function installAppZoom():Promise<void> {
  if(!isNativeRuntime())return Promise.resolve();
  if(installation)return installation;
  installation=(async()=>{
    const remember=(percent:unknown)=>{
      if(!validZoom(percent))return;
      document.documentElement.dataset.appZoom=String(percent);
      document.documentElement.dataset.appZoomCompact=String(compactZoomLayout(percent));
      try{localStorage.setItem(APP_ZOOM_KEY,String(percent));}catch{/* Zoom still works for this window. */}
    };
    let percent=100;try{percent=savedZoom(localStorage.getItem(APP_ZOOM_KEY));}catch{/* Open at the actual default. */}
    const stop=await listen<{percent:number}>('code-atlas:zoom-changed',event=>remember(event.payload?.percent));
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
