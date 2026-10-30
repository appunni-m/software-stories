import { useState } from 'react';
import { CodexChapterDialog, type CodexChapterDialogProps } from './CodexChapterDialog';
import { SourceDiscoveryDialog } from './SourceDiscoveryDialog';
import { sourceDiscoveryLimitNotes, type LargeFileDiscoveryHandoff, type SourceDiscoveryResult, type SourceDiscoveryScope } from './lib/sourceDiscovery';
import { isDocumentationPath } from './lib/inventory';

/** Ordinary file explanations discover their own supporting implementation. */
export function CodexContextDialog(props: CodexChapterDialogProps & {
  onLargeFileDiscovered: (handoff: LargeFileDiscoveryHandoff) => void;
}) {
  const [discovery, setDiscovery] = useState<SourceDiscoveryResult | null>(null);
  const paths = discovery?.selectedFiles.map((file) => file.path);
  const mode = props.mode;
  const discoveryScope: SourceDiscoveryScope | null = mode !== 'settings'
    && (mode.kind === 'review' || mode.kind === 'overview')
    && !mode.plannedChapter && !mode.recoveryOperationId && !mode.selectedPaths?.length
    && (!mode.targetPath || !isDocumentationPath(mode.targetPath))
    ? { projectId: mode.projectId, snapshotId: mode.snapshotId, targetPath: mode.targetPath,
      question: mode.targetPath ? `How does ${mode.targetPath} work, including its callers, dependencies, transformations, errors and tests?`.slice(0, 300)
        : 'Explain the main implementation flow from entry points through transformations, state and outputs, including error handling and relevant tests.' }
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
  return <CodexChapterDialog {...props} discoveryResult={discovery ?? undefined} mode={paths && mode !== 'settings' && (mode.kind === 'review' || mode.kind === 'overview') ? { ...mode, selectedPaths: paths } : mode} />;
}
