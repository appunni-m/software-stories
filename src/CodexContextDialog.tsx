import { useState } from 'react';
import { CodexChapterDialog, type CodexChapterDialogProps } from './CodexChapterDialog';
import { SourceDiscoveryDialog } from './SourceDiscoveryDialog';
import { sourceDiscoveryLimitNotes, sourceDiscoveryQuestion, type LargeFileDiscoveryHandoff, type SourceDiscoveryResult, type SourceDiscoveryScope } from './lib/sourceDiscovery';
import { isDocumentationPath } from './lib/inventory';

/** Ordinary file explanations discover their own supporting implementation. */
export function CodexContextDialog(props: CodexChapterDialogProps & {
  onLargeFileDiscovered: (handoff: LargeFileDiscoveryHandoff) => void;
}) {
  const [discovery, setDiscovery] = useState<SourceDiscoveryResult | null>(null);
  const paths = discovery?.selectedFiles.map((file) => file.path);
  const mode = props.mode;
  const documentationTargetPath = mode !== 'settings'
    && (mode.kind === 'review' || mode.kind === 'overview')
    && mode.targetPath && isDocumentationPath(mode.targetPath) ? mode.targetPath : null;
  const discoveryScope: SourceDiscoveryScope | null = mode !== 'settings'
    && (mode.kind === 'review' || mode.kind === 'overview')
    && !mode.plannedChapter && !mode.recoveryOperationId && !mode.selectedPaths?.length
    ? { projectId: mode.projectId, snapshotId: mode.snapshotId,
      targetPath: documentationTargetPath ? null : mode.targetPath,
      question: sourceDiscoveryQuestion(mode.targetPath) }
    : null;
  if (discoveryScope && !paths) {
    return <SourceDiscoveryDialog scope={discoveryScope} onClose={props.onClose} onReady={(result, review) => {
      const rangeTarget = result.rangeTargets?.[0];
      if (rangeTarget) {
        const limitNotes = sourceDiscoveryLimitNotes(result, review);
        props.onLargeFileDiscovered({
          projectId: discoveryScope.projectId,
          snapshotId: discoveryScope.snapshotId,
          targetPath: rangeTarget.path,
          supportingPaths: result.selectedFiles.map((file) => file.path),
          question: discoveryScope.question,
          discoveryNotes: [
            `Range target ${rangeTarget.path}: ${rangeTarget.reason}`,
            ...result.selectedFiles.map((file) => `Supporting source ${file.path}: ${file.reason}`),
            ...limitNotes,
          ],
        });
        return;
      }
      setDiscovery(result);
    }} persistSuccessfulResult={false} autoDiscover />;
  }
  const discoveredMode = paths && mode !== 'settings' && (mode.kind === 'review' || mode.kind === 'overview')
    ? documentationTargetPath
      ? { kind: 'overview' as const, projectId: mode.projectId, snapshotId: mode.snapshotId, selectedPaths: paths,
        question: sourceDiscoveryQuestion(documentationTargetPath) }
      : { ...mode, selectedPaths: paths }
    : mode;
  return <CodexChapterDialog {...props} discoveryResult={discovery ?? undefined} mode={discoveredMode} />;
}
