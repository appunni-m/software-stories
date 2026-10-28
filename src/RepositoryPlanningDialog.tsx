import { useEffect, useRef, useState } from 'react';
import { X } from 'lucide-react';
import { AppButton } from './design/AppButton';
import { readGenerationBackend, repositoryPlanningBackend } from './lib/generationPreferences';
import { SourceDiscoveryDialog } from './SourceDiscoveryDialog';
import { sourceDiscoveryLimitNotes, type LargeFileDiscoveryHandoff, type SourceDiscoveryResult } from './lib/sourceDiscovery';
import { CodexChapterDialog } from './CodexChapterDialog';
import { ClaudeConnectionDialog } from './ClaudeConnectionDialog';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import type { PrepareRepositoryPlanInput } from './lib/repositoryPlans';
import type { PlannedChapterScope } from './lib/plannedChapters';

export type RepositoryPlanningScope = PrepareRepositoryPlanInput & ({
  kind?: 'repository-plan';
} | {
  kind: 'planned-chapter';
  plannedChapter: PlannedChapterScope;
}) & {
  recoveryOperationId?: string;
  recoveryBackend?: 'codex' | 'claude-agent';
  discoveryResult?: SourceDiscoveryResult;
};

/** Agent-only planning/chapter entry. Each child owns its existing account, settings,
 * review, consent and cancellation lifecycle. No API or manual fallback. */
export function RepositoryPlanningDialog({ scope, native, onClose, onOpenConnections, onLargeFileDiscovered, onRecoveryResultReady, onOpenPendingPreview }: {
  scope: RepositoryPlanningScope; native: boolean; onClose: () => void;
  onOpenConnections: () => void;
  onLargeFileDiscovered: (handoff: LargeFileDiscoveryHandoff) => void;
  onRecoveryResultReady?: (operationId: string) => void;
  onOpenPendingPreview?: (generationId: string) => void;
}) {
  const [backend, setBackend] = useState<'codex' | 'claude-agent' | null>(() => repositoryPlanningBackend(
    scope.recoveryOperationId ? scope.recoveryBackend : readGenerationBackend(),
  ));
  // Chapter recovery has the existing candidate-preview/history check inside
  // each agent. Planning has its separate saved-plan acknowledgement here.
  const [checkedEarlier, setCheckedEarlier] = useState(!scope.recoveryOperationId || scope.kind === 'planned-chapter');
  const dialog = useRef<HTMLElement>(null);
  useModalFocusTrap(dialog);
  useEffect(() => {
    if (scope.recoveryOperationId) return;
    const refresh = () => setBackend(repositoryPlanningBackend(readGenerationBackend()));
    window.addEventListener('code-atlas:connection-changed', refresh);
    return () => window.removeEventListener('code-atlas:connection-changed', refresh);
  }, [scope.recoveryOperationId]);
  useEffect(() => {
    if (backend) return;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); onClose(); }
    };
    document.addEventListener('keydown', key);
    return () => { document.removeEventListener('keydown', key); if (previous?.isConnected) previous.focus(); };
  }, [backend, onClose]);
  const [discovery, setDiscovery] = useState<SourceDiscoveryResult | null>(() => scope.discoveryResult ?? null);
  const discoveredPaths = discovery?.selectedFiles.map((file) => file.path);
  const openConnections = () => { onClose(); requestAnimationFrame(onOpenConnections); };
  const needsRepositoryDiscovery = !scope.selectedPaths.length && !discoveredPaths
    && !scope.recoveryOperationId && scope.kind !== 'planned-chapter';
  if (needsRepositoryDiscovery && backend === 'codex') {
    return <SourceDiscoveryDialog scope={scope} onClose={onClose} onReady={(result, review) => {
      const rangeTarget = result.rangeTargets?.[0];
      if (rangeTarget) {
        onClose();
        const limitNotes = sourceDiscoveryLimitNotes(result, review);
        onLargeFileDiscovered({
          projectId: scope.projectId,
          snapshotId: scope.snapshotId,
          targetPath: rangeTarget.path,
          supportingPaths: result.selectedFiles.map((file) => file.path),
          question: scope.question,
          discoveryNotes: [
            `Range target ${rangeTarget.path}: ${rangeTarget.reason}`,
            ...result.selectedFiles.map((file) => `Supporting source ${file.path}: ${file.reason}`),
            ...limitNotes,
          ],
        });
        return;
      }
      setDiscovery(result);
    }} persistSuccessfulResult autoDiscover />;
  }
  if (needsRepositoryDiscovery && backend === 'claude-agent') {
    return <div className="provider-overlay"><section ref={dialog} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="claude-discovery-title" tabIndex={-1}>
      <header className="provider-dialog-header"><div><h2 id="claude-discovery-title">Repository discovery isn’t available for Claude yet</h2><p>{scope.question}</p></div>
        <AppButton intent="icon" onClick={onClose} aria-label="Close planning"><X size={17} /></AppButton></header>
      <div className="provider-dialog-content"><p>Your question is unchanged. Switch to Codex in Connections to have Code Atlas find the relevant files automatically.</p></div>
      <footer className="provider-dialog-footer"><AppButton intent="secondary" onClick={onClose}>Close</AppButton><AppButton intent="primary" onClick={openConnections}>Change connection</AppButton></footer>
    </section></div>;
  }
  const effectiveScope = discoveredPaths ? { ...scope, selectedPaths: discoveredPaths } : scope;
  const savedBackend = readGenerationBackend();
  const unsupportedSavedConnection = !scope.recoveryOperationId && (savedBackend === null || savedBackend === 'api' || savedBackend === 'handoff');
  const savedConnectionName = savedBackend === null ? 'The saved AI connection'
    : savedBackend === 'api' ? 'Direct API' : 'Claude Code handoff';
  const planned = scope.kind === 'planned-chapter' ? scope.plannedChapter : null;
  const mode = planned ? { kind: 'review' as const, projectId: scope.projectId, snapshotId: scope.snapshotId,
    targetPath: planned.primaryPath, selectedPaths: planned.selectedPaths, plannedChapter: planned,
    recoveryOperationId: scope.recoveryOperationId } : { kind: 'repository-plan' as const, projectId: effectiveScope.projectId,
    snapshotId: effectiveScope.snapshotId, question: effectiveScope.question, selectedPaths: effectiveScope.selectedPaths };
  if (backend === 'codex') return <CodexChapterDialog mode={mode} discoveryResult={discovery ?? undefined} native={native} onClose={onClose} onChooseBackend={openConnections} onRecoveryReviewReady={onRecoveryResultReady} onOpenPendingPreview={onOpenPendingPreview} />;
  if (backend === 'claude-agent') return <ClaudeConnectionDialog reviewMode={mode} native={native} onClose={onClose} onChooseBackend={openConnections} onRecoveryResultReady={onRecoveryResultReady} onOpenPendingPreview={onOpenPendingPreview} />;
  return <div className="provider-overlay"><section ref={dialog} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="plan-agent-title" tabIndex={-1}>
    <header className="provider-dialog-header"><div><h2 id="plan-agent-title">Use the connection in your header</h2><p>{scope.question}</p></div>
      <AppButton intent="icon" onClick={onClose} aria-label={planned ? 'Close chapter explanation' : 'Close planning'}><X size={17} /></AppButton></header>
    <div className="provider-dialog-content">
      <p>{scope.selectedPaths.length} captured files. {planned ? 'This action creates one unaccepted chapter for the accepted step.' : 'This action creates an unaccepted reading plan.'} Source review and account consent stay with the selected connection.</p>
      {unsupportedSavedConnection && <p className="provider-small-note" role="status">{savedConnectionName} is not available for this action. Change the AI connection once from the Connections control in the header.</p>}
      {scope.recoveryOperationId && !planned && <section className="provider-review-card"><h3>Check earlier plans first</h3>
        <p>The original question and files were restored locally. A result may already be saved even if the earlier reply was lost. Close this dialog to inspect Learning plans before another request. Your recovery notice stays available.</p>
        <label className="provider-consent"><input type="checkbox" checked={checkedEarlier} onChange={(event) => setCheckedEarlier(event.target.checked)} />I checked earlier results and want to prepare a fresh request.</label>
      </section>}
      {unsupportedSavedConnection && <AppButton intent="primary" disabled={!native || !checkedEarlier} onClick={openConnections}>Open Connections</AppButton>}
      {!native && <p role="status">Coding agents run in the macOS app. This browser preview cannot start an agent or read saved native plans.</p>}
      {scope.recoveryBackend && <p>The restored request stays with its original agent. Its connection cannot be changed while recovering it.</p>}
    </div>
  </section></div>;
}
