import { useEffect, useRef, useState } from 'react';
import { X } from 'lucide-react';
import { CodexChapterDialog } from './CodexChapterDialog';
import { ClaudeConnectionDialog } from './ClaudeConnectionDialog';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import type { PrepareRepositoryPlanInput } from './lib/repositoryPlans';

export type RepositoryPlanningScope = PrepareRepositoryPlanInput & {
  recoveryOperationId?: string;
  recoveryBackend?: 'codex' | 'claude-agent';
};

/** Agent-only planning entry. Each child owns its existing account, settings,
 * review, consent and cancellation lifecycle. No API or manual fallback. */
export function RepositoryPlanningDialog({ scope, native, onClose }: {
  scope: RepositoryPlanningScope; native: boolean; onClose: () => void;
}) {
  const [backend, setBackend] = useState<'codex' | 'claude-agent' | null>(null);
  const [checkedEarlier, setCheckedEarlier] = useState(!scope.recoveryOperationId);
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
  const mode = { ...scope, kind: 'repository-plan' as const };
  if (backend === 'codex') return <CodexChapterDialog mode={mode} native={native} onClose={onClose} onChooseBackend={() => setBackend(null)} />;
  if (backend === 'claude-agent') return <ClaudeConnectionDialog reviewMode={mode} native={native} onClose={onClose} onChooseBackend={() => setBackend(null)} />;
  return <div className="provider-overlay"><section ref={dialog} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="plan-agent-title" tabIndex={-1}>
    <header className="provider-dialog-header"><div><h2 id="plan-agent-title">Choose a planning agent</h2><p>{scope.question}</p></div>
      <button className="icon-button" onClick={onClose} aria-label="Close planning"><X size={17} /></button></header>
    <div className="provider-dialog-content">
      <p>{scope.selectedPaths.length} captured files · source and account settings are reviewed before anything is sent. This makes an unaccepted reading plan; chapters are generated separately.</p>
      {scope.recoveryOperationId && <section className="provider-review-card"><h3>Check earlier plans first</h3>
        <p>The original question and files were restored locally. A result may already be saved even if the earlier reply was lost. Close this dialog to inspect Learning plans before another request. Your recovery notice stays available.</p>
        <label className="provider-consent"><input type="checkbox" checked={checkedEarlier} onChange={(event) => setCheckedEarlier(event.target.checked)} />I checked earlier saved plans and want to prepare a fresh request.</label>
      </section>}
      <section className="provider-review-card"><h3>Codex · existing ChatGPT account</h3><p>Use the managed Codex runtime, its supported models and reasoning settings. No API key is required. Account usage limits apply.</p>
        <button className="primary-button" disabled={!native || !checkedEarlier || !!scope.recoveryBackend && scope.recoveryBackend !== 'codex'} onClick={() => setBackend('codex')}>Continue with Codex</button></section>
      <section className="provider-review-card"><h3>Claude Agent · Claude Code runtime</h3><p>Use the existing Claude installation and account catalog with model, reasoning and turn controls. Automatic generation is unavailable until its native runtime is qualified; setup and local review remain available.</p>
        <button className="secondary-button" disabled={!native || !checkedEarlier || !!scope.recoveryBackend && scope.recoveryBackend !== 'claude-agent'} onClick={() => setBackend('claude-agent')}>Continue with Claude</button></section>
      {!native && <p role="status">Planning agents run in the macOS app. This browser preview cannot start an agent or read saved native plans.</p>}
      {scope.recoveryBackend && <p>The restored request stays with its original agent. Start a separate plan to choose another backend.</p>}
    </div>
  </section></div>;
}
