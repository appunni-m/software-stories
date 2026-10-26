const NON_TEXT_INPUT_TYPES = new Set([
  'button', 'checkbox', 'color', 'file', 'hidden', 'image', 'radio', 'range', 'reset', 'submit',
]);

/** Keep global navigation shortcuts out of text editors, search fields and IME composition. */
export function isEditableKeyboardTarget(target: EventTarget | null): boolean {
  if (typeof Element === 'undefined' || !(target instanceof Element)) return false;
  if (target.closest('.cm-content[aria-readonly="true"]')) return false;
  if (target.closest('textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="searchbox"]')) {
    return true;
  }
  const input = target.closest('input');
  return input !== null && !NON_TEXT_INPUT_TYPES.has(input.type.toLowerCase());
}
