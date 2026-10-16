import { useEffect, useRef, useState } from 'react';
import { X } from 'lucide-react';
import { readGenerationBackend, rememberGenerationBackend } from './lib/generationPreferences';
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
};

/** Agent-only planning/chapter entry. Each child owns its existing account, settings,
 * review, consent and cancellation lifecycle. No API or manual fallback. */
export function RepositoryPlanningDialog({ scope, native, onClose, onRecoveryResultReady, onOpenPendingPreview }: {
  scope: RepositoryPlanningScope; native: boolean; onClose: () => void;
  onRecoveryResultReady?: (operationId: string) => void;
  onOpenPendingPreview?: (generationId: string) => void;
}) {
  const [backend, setBackend] = useState<'codex' | 'claude-agent' | null>(() => scope.recoveryOperationId ? null : readGenerationBackend() === 'claude-setup' ? 'claude-agent' : 'codex');
  // Chapter recovery has the existing candidate-preview/history check inside
  // each agent. Planning has its separate saved-plan acknowledgement here.
  const [checkedEarlier, setCheckedEarlier] = useState(!scope.recoveryOperationId || scope.kind === 'planned-chapter');
  const dialog = useRef<HTMLElement>(null);
  useModalFocusTrap(dialog);
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
  const planned = scope.kind === 'planned-chapter' ? scope.plannedChapter : null;
  const mode = planned ? { kind: 'review' as const, projectId: scope.projectId, snapshotId: scope.snapshotId,
    targetPath: planned.primaryPath, selectedPaths: planned.selectedPaths, plannedChapter: planned,
    recoveryOperationId: scope.recoveryOperationId } : { ...scope, kind: 'repository-plan' as const };
  if (backend === 'codex') return <CodexChapterDialog mode={mode} native={native} onClose={onClose} onChooseBackend={() => setBackend(null)} onRecoveryReviewReady={onRecoveryResultReady} onOpenPendingPreview={onOpenPendingPreview} />;
  if (backend === 'claude-agent') return <ClaudeConnectionDialog reviewMode={mode} native={native} onClose={onClose} onChooseBackend={() => setBackend(null)} onRecoveryResultReady={onRecoveryResultReady} onOpenPendingPreview={onOpenPendingPreview} />;
  return <div className="provider-overlay"><section ref={dialog} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="plan-agent-title" tabIndex={-1}>
    <header className="provider-dialog-header"><div><h2 id="plan-agent-title">{planned ? 'Choose a chapter agent' : 'Choose a planning agent'}</h2><p>{scope.question}</p></div>
      <button className="icon-button" onClick={onClose} aria-label={planned ? 'Close chapter agent selection' : 'Close planning'}><X size={17} /></button></header>
    <div className="provider-dialog-content">
      <p>{scope.selectedPaths.length} captured files · source and account settings are reviewed before anything is sent. {planned ? 'Generate one chapter answering this accepted step. Review and acceptance remain separate.' : 'This makes an unaccepted reading plan; chapters are generated separately.'}</p>
      {scope.recoveryOperationId && !planned && <section className="provider-review-card"><h3>Check earlier plans first</h3>
        <p>The original question and files were restored locally. A result may already be saved even if the earlier reply was lost. Close this dialog to inspect Learning plans before another request. Your recovery notice stays available.</p>
        <label className="provider-consent"><input type="checkbox" checked={checkedEarlier} onChange={(event) => setCheckedEarlier(event.target.checked)} />I checked earlier results and want to prepare a fresh request.</label>
      </section>}
      <section className="provider-review-card"><h3>Codex · existing ChatGPT account</h3><p>Use the managed Codex runtime, its supported models and reasoning settings. No API key is required. Account usage limits apply.</p>
        <button className="primary-button" disabled={!native || !checkedEarlier || !!scope.recoveryBackend && scope.recoveryBackend !== 'codex'} onClick={() => { rememberGenerationBackend('codex'); setBackend('codex'); }}>Continue with Codex</button></section>
      <section className="provider-review-card"><h3>Claude Agent · Claude Code runtime</h3><p>Use the existing Claude installation and account catalog with model, reasoning and turn controls. Automatic generation is unavailable until its native runtime is qualified; setup and local review remain available.</p>
        <button className="secondary-button" disabled={!native || !checkedEarlier || !!scope.recoveryBackend && scope.recoveryBackend !== 'claude-agent'} onClick={() => { rememberGenerationBackend('claude-setup'); setBackend('claude-agent'); }}>Continue with Claude</button></section>
      {!native && <p role="status">Coding agents run in the macOS app. This browser preview cannot start an agent or read saved native plans.</p>}
      {scope.recoveryBackend && <p>The restored request stays with its original agent. Start a separate request to choose another backend.</p>}
    </div>
  </section></div>;
}
