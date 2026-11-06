export const POCKET_TTS_RUNTIME_READY_EVENT = 'code-atlas:pocket-tts-runtime-ready';

export function notifyPocketTtsRuntimeReady(target: EventTarget) {
  target.dispatchEvent(new Event(POCKET_TTS_RUNTIME_READY_EVENT));
}

export function listenForPocketTtsRuntimeReady(target: EventTarget, onReady: () => void) {
  const listener = () => onReady();
  target.addEventListener(POCKET_TTS_RUNTIME_READY_EVENT, listener);
  return () => target.removeEventListener(POCKET_TTS_RUNTIME_READY_EVENT, listener);
}
