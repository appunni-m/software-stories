import type { CapturedFile } from './commands';

export interface PendingFileExplanation {
  id: string;
  projectId: string;
  snapshotId: string;
  targetPath: string;
  paths: string[];
  question: string;
  sourceDiscoveryNotes?: string[];
}

type CapturedExplanationFile = Pick<CapturedFile, 'path' | 'kind' | 'contentKind' | 'secretLike' | 'sha256'>;

/** Create a handoff without copying source bytes into the renderer or chat record. */
export function pendingFileExplanation(input: {
  id: string;
  projectId: string;
  snapshotId: string;
  targetPath: string;
  paths?: readonly string[];
  question?: string;
  sourceDiscoveryNotes?: readonly string[];
}): PendingFileExplanation {
  const paths = [...new Set([input.targetPath, ...(input.paths ?? [])])].slice(0, 12);
  return {
    id: input.id,
    projectId: input.projectId,
    snapshotId: input.snapshotId,
    targetPath: input.targetPath,
    paths,
    question: input.question?.trim() || `Explain ${input.targetPath} from the captured implementation. Trace its inputs, transformations, outputs, errors, and relevant callers using bounded source ranges. Cite exact code and call out unresolved links or anything unverified.`,
    sourceDiscoveryNotes: input.sourceDiscoveryNotes?.slice(0, 14).map((note) => note.slice(0, 1_500)),
  };
}

type FileExplanationInput = {
  id: string;
  projectId: string;
  snapshotId: string;
  targetPath: string;
  selectedPaths?: readonly string[];
  files: readonly CapturedExplanationFile[];
};

/** Route a captured source-file question into the saved workspace conversation. */
export function createFileExplanation(input: FileExplanationInput): PendingFileExplanation | null {
  const target = input.files.find((file) => file.path === input.targetPath);
  if (!target || target.kind !== 'file' || target.contentKind !== 'text' || target.secretLike) return null;
  const readablePaths = new Set(input.files
    .filter((file) => file.kind === 'file' && file.contentKind === 'text' && !file.secretLike)
    .map((file) => file.path));
  const paths = [...new Set([input.targetPath, ...(input.selectedPaths ?? [])])]
    .filter((path) => readablePaths.has(path))
    .slice(0, 12);
  if (!paths.includes(input.targetPath)) return null;
  return pendingFileExplanation({
    id: input.id,
    projectId: input.projectId,
    snapshotId: input.snapshotId,
    targetPath: input.targetPath,
    paths,
  });
}

/** Resolve only captured path/hash metadata; source bytes come from range reads. */
export function fileExplanationContexts(
  pending: Pick<PendingFileExplanation, 'paths'>,
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
