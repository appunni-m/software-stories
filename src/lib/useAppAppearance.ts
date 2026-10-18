import {useSyncExternalStore} from 'react';

export function resolveAppAppearance(mode:string|undefined,prefersLight:boolean):'light'|'dark' {
  return mode==='light'||(mode!=='dark'&&prefersLight)?'light':'dark';
}

export function readAppAppearance() {
  return resolveAppAppearance(document.documentElement.dataset.theme,matchMedia('(prefers-color-scheme: light)').matches);
}

export function watchAppAppearance(notify:()=>void) {
  const observer=new MutationObserver(notify);
  observer.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
  const media=matchMedia('(prefers-color-scheme: light)');media.addEventListener('change',notify);
  return()=>{observer.disconnect();media.removeEventListener('change',notify);};
}

/** The header, editor and portalled standard components share one appearance. */
export function useAppAppearance() {
  return useSyncExternalStore(watchAppAppearance,readAppAppearance,()=> 'dark' as const);
}
