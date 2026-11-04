import { normalizeSourceLineRange, type SourceLineRange } from './navigation.ts';

export interface CapturedSourceNavigation {
  relativePath: string;
  sourceRange?: SourceLineRange;
}

/** Validate untrusted source links before they can change the active route. */
export function normalizeCapturedSourceNavigation(
  relativePath: unknown,
  sourceRange?: unknown,
): CapturedSourceNavigation | null {
  if (typeof relativePath !== 'string'
    || relativePath.length === 0
    || new TextEncoder().encode(relativePath).length > 4096
    || relativePath.startsWith('/')
    || relativePath.includes('\\')
    || /[\u0000-\u001f\u007f-\u009f]/u.test(relativePath)
    || relativePath.split('/').some((part) => part.length === 0 || part === '.' || part === '..')) {
    return null;
  }

  const normalizedRange = sourceRange === undefined ? undefined : normalizeSourceLineRange(sourceRange);
  if (sourceRange !== undefined && !normalizedRange) return null;

  return {
    relativePath,
    ...(normalizedRange ? { sourceRange: normalizedRange } : {}),
  };
}
