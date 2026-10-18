import type { CapturedFile } from './commands';

/** Whole-file generation sends source verbatim and shares the native 250 KB request cap. */
export const WHOLE_FILE_EXPLANATION_LIMIT = 250_000;

/** Larger captured text should use the workspace's bounded source-range reader. */
export function requiresBoundedFileExplanation(file: Pick<CapturedFile, 'kind' | 'contentKind' | 'secretLike' | 'sizeBytes'> | undefined): boolean {
  return file?.kind === 'file'
    && file.contentKind === 'text'
    && !file.secretLike
    && file.sizeBytes > WHOLE_FILE_EXPLANATION_LIMIT;
}
