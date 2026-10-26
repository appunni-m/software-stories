import { useState } from 'react';
import { CodexChapterDialog, type CodexChapterDialogProps } from './CodexChapterDialog';
import { SourceDiscoveryDialog } from './SourceDiscoveryDialog';
import type { SourceDiscoveryResult } from './lib/sourceDiscovery';
import { isDocumentationPath } from './lib/inventory';

/** Ordinary file explanations discover their own supporting implementation. */
export function CodexContextDialog(props: CodexChapterDialogProps) {
  const [discovery, setDiscovery] = useState<SourceDiscoveryResult | null>(null);
  const paths = discovery?.selectedFiles.map((file) => file.path);
  const mode = props.mode;
  const needsDiscovery = mode !== 'settings' && (mode.kind === 'review' || mode.kind === 'overview')
    && !mode.plannedChapter && !mode.recoveryOperationId && !mode.selectedPaths?.length
    && (!mode.targetPath || !isDocumentationPath(mode.targetPath));
  if (needsDiscovery && !paths) {
    return <SourceDiscoveryDialog scope={{ projectId: mode.projectId, snapshotId: mode.snapshotId, targetPath: mode.targetPath,
      question: mode.targetPath ? `How does ${mode.targetPath} work, including its callers, dependencies, transformations, errors and tests?`.slice(0, 300)
        : 'Explain the main implementation flow from entry points through transformations, state and outputs, including error handling and relevant tests.' }}
      onClose={props.onClose} onReady={setDiscovery} autoDiscover />;
  }
  return <CodexChapterDialog {...props} discoveryResult={discovery ?? undefined} mode={paths && mode !== 'settings' && (mode.kind === 'review' || mode.kind === 'overview') ? { ...mode, selectedPaths: paths } : mode} />;
}
