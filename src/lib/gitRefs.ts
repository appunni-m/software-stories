/** Keep commit identifiers compact in the UI while retaining named refs. */
export function shortGitRef(ref: string): string {
  return /^[a-f\d]{40,64}$/i.test(ref) ? ref.slice(0, 8) : ref;
}
