export const APP_ZOOM_KEY = 'code-atlas:page-zoom:v1';
export type ZoomAction = 'in'|'out'|'reset';
export function validZoom(value:unknown):value is number {
  return typeof value==='number'&&Number.isInteger(value)&&value>=20&&value<=1000&&value%20===0;
}
export function savedZoom(raw:string|null):number {
  if(raw===null||!/^\d{2,4}$/.test(raw))return 100;
  const value=Number(raw);return validZoom(value)&&String(value)===raw?value:100;
}
export function zoomShortcut(event:Pick<KeyboardEvent,'key'|'metaKey'|'ctrlKey'|'altKey'|'isComposing'|'defaultPrevented'>):ZoomAction|null {
  if(event.defaultPrevented||event.isComposing||event.altKey||!(event.metaKey||event.ctrlKey))return null;
  return event.key==='='||event.key==='+'?'in':event.key==='-'?'out':event.key==='0'?'reset':null;
}
