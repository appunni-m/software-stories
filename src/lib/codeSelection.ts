import type { Context } from '../workspace/model.ts';

/** Bind a selection to the immutable source identity shown in the editor. */
export function codeSelectionContext(path: string, sha256: string | undefined, from: number, to: number): Context {
  return { path, sha256, from, to };
}

/** Fill a missing hash from the displayed file, but never replace a captured identity. */
export function bindSelectionToDisplayedSource(
  context: Context,
  source: { path: string; sha256: string } | null | undefined,
): Context {
  if (context.sha256 || source?.path !== context.path) return context;
  return { ...context, sha256: source.sha256 };
}
