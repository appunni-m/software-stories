/** Stable semantic identity for a modal's source scope. Parent renders and
 * copied objects must not cancel an active request or erase its result. */
export function requestScopeKey(scope: unknown): string {
  return JSON.stringify(scope, (_key, value) => value && typeof value === 'object' && !Array.isArray(value)
    ? Object.fromEntries(Object.entries(value).sort(([left], [right]) => left.localeCompare(right))) : value);
}
