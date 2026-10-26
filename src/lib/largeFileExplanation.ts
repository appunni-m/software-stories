import type { CapturedFile } from './commands';

/** Whole-file generation sends source verbatim and shares the native 250 KB request cap. */
export const WHOLE_FILE_EXPLANATION_LIMIT = 250_000;
export const MAX_RANGE_INDEXED_EXPLANATION_BYTES = 1024 * 1024;

export interface PendingLargeFileExplanation {
  id: string;
  projectId: string;
  snapshotId: string;
  targetPath: string;
  paths: string[];
  question: string;
}

type CapturedExplanationFile = Pick<CapturedFile, 'path' | 'kind' | 'contentKind' | 'secretLike' | 'sizeBytes' | 'sha256'>;

/** Larger captured text should use the workspace's bounded source-range reader. */
export function requiresBoundedFileExplanation(file: Pick<CapturedFile, 'kind' | 'contentKind' | 'secretLike' | 'sizeBytes'> | undefined): boolean {
  return file?.kind === 'file'
    && file.contentKind === 'text'
    && !file.secretLike
    && file.sizeBytes > WHOLE_FILE_EXPLANATION_LIMIT;
}

/** Beyond the local parser limit, a whole-file request cannot discover bounded ranges. */
export function requiresExplicitCodeRange(file: Pick<CapturedFile, 'kind' | 'contentKind' | 'secretLike' | 'sizeBytes'> | undefined): boolean {
  return requiresBoundedFileExplanation(file) && file.sizeBytes > MAX_RANGE_INDEXED_EXPLANATION_BYTES;
}

/** Build the handoff into workspace range retrieval without copying file contents. */
export function createLargeFileExplanation(input: {
  id: string;
  projectId: string;
  snapshotId: string;
  targetPath: string;
  selectedPaths?: readonly string[];
  files: readonly CapturedExplanationFile[];
}): PendingLargeFileExplanation | null {
  const target = input.files.find((file) => file.path === input.targetPath);
  if (!requiresBoundedFileExplanation(target)) return null;
  const readablePaths = new Set(input.files
    .filter((file) => file.kind === 'file' && file.contentKind === 'text' && !file.secretLike)
    .map((file) => file.path));
  const paths = [...new Set([input.targetPath, ...(input.selectedPaths ?? [])])]
    .filter((path) => readablePaths.has(path))
    .slice(0, 12);
  if (!paths.includes(input.targetPath)) return null;
  return {
    id: input.id,
    projectId: input.projectId,
    snapshotId: input.snapshotId,
    targetPath: input.targetPath,
    paths,
    question: `Explain ${input.targetPath} from the captured implementation. Trace its inputs, transformations, outputs, errors, and relevant callers using bounded source ranges. Cite exact code and call out unresolved links or anything unverified.`,
  };
}

/** Resolve only snapshot paths and hashes for the workspace; source bytes are fetched through bounded range tools. */
export function largeFileExplanationContexts(
  pending: Pick<PendingLargeFileExplanation, 'paths'>,
  files: readonly CapturedExplanationFile[],
): { path: string; sha256: string }[] {
  const byPath = new Map(files
    .filter((file) => file.kind === 'file' && file.contentKind === 'text' && !file.secretLike)
    .map((file) => [file.path, file] as const));
  return [...new Set(pending.paths)].slice(0, 12).flatMap((path) => {
    const file = byPath.get(path);
    return file ? [{ path: file.path, sha256: file.sha256 }] : [];
  });
}
