import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {isNativeRuntime} from './commands';
import {appCloseCoordinator,CloseRequestController} from './closeCoordinator';
import {beginWorkspaceClose,flushAllWorkspaceConversations} from '../workspace/store';

let installing: Promise<void> | null = null;
let installed = false;
let listening: Promise<void> | null = null;

/** Installed before mounting the app, so interactive edits always have a native
 * close/quit listener. A failed handshake leaves a retryable startup screen. */
export function installAppCloseGuard(): Promise<void> {
  if (!isNativeRuntime() || installed) return Promise.resolve();
  if (installing) return installing;
  installing = (async () => {
    if (!listening) {
      listening = (async () => {
        const root = document.getElementById('root')!;
        let focus: HTMLElement | null = null;
        let previousInert = false;
        let previousBusy: string | null = null;
        let busy = false;
        let status: HTMLElement | null = null;
        let failureEpisode: string | null = null;
        const controller = new CloseRequestController(appCloseCoordinator, {
          complete: (requestId,saved) => invoke('complete_app_close',{requestId,saved}),
          busy: value => {
            if (value === busy) return;
            busy = value;
            if (value) {
              focus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
              previousInert = root.inert; previousBusy = root.getAttribute('aria-busy');
              root.inert = true; root.setAttribute('aria-busy','true');
              status = document.createElement('div'); status.className = 'app-close-status';
              status.setAttribute('role','status'); status.textContent = 'Saving your place…';
              const theme = getComputedStyle(root.querySelector('.radix-themes')??root);
              status.style.color = theme.color; status.style.backgroundColor = theme.backgroundColor;
              document.body.append(status);
            } else {
              status?.remove(); status = null;
              root.inert = previousInert;
              if (previousBusy === null) root.removeAttribute('aria-busy'); else root.setAttribute('aria-busy',previousBusy);
              if (focus?.isConnected) focus.focus({preventScroll:true});
            }
          },
          failed: () => {
            if (failureEpisode) return;
            failureEpisode = crypto.randomUUID();
            window.dispatchEvent(new CustomEvent('code-atlas:workspace-notice',{detail:{
              title:'Code Atlas stayed open',
              message:'Your place could not be saved yet. Check the save issue in Notifications, then try closing again. Existing explanations are preserved.',
              occurrenceId:failureEpisode,
            }}));
          },
        });
        const removeRequest = await listen<{requestId:string}>('code-atlas:prepare-close',event => {void controller.request(event.payload?.requestId);});
        try {
          await listen<{requestId:string}>('code-atlas:cancel-close',event => controller.cancel(event.payload?.requestId));
        } catch (error) {removeRequest();throw error;}
        appCloseCoordinator.register({freeze:beginWorkspaceClose,flush:flushAllWorkspaceConversations});
      })().catch(error => {listening = null; throw error;});
    }
    await listening;
    await invoke('register_app_close_guard');
    installed = true;
  })().finally(() => {installing = null;});
  return installing;
}
