import { useEffect, useRef, useState } from 'react';
import { HardDrive, X } from 'lucide-react';
import {
  cancelClaudeAgentOperation, inspectClaudeAgentConnection,
  type ClaudeConnectionObservation, type ClaudeInspectionFailureKind,
} from './lib/commands';
import { useModalFocusTrap } from './lib/useModalFocusTrap';

const preferenceKey = 'code-atlas:claude-agent-executable';
function savedPath(): string {
  try { const value = localStorage.getItem(preferenceKey) ?? ''; return value.length <= 4096 ? value : ''; }
  catch { return ''; }
}

const failures: Record<ClaudeInspectionFailureKind, string> = {
  busy: 'Another Claude operation is active. Finish or cancel it before checking again.',
  cancelled: 'The check was cancelled. No repository source was included.',
  timedOut: 'The CLI check timed out. Check installation and account setup in the vendor CLI before trying again.',
  runtimeChanged: 'The selected executable changed or could not be read. Check its path and installation.',
  preparationFailed: 'The check could not be prepared. Choose an absolute path to an installed executable and check local app storage.',
  processFailed: 'The CLI metadata command could not finish. Check that the selected executable is Claude Code.',
  protocolRejected: 'The CLI metadata was incomplete or unrecognized. This cannot establish a usable account connection.',
  outputLimit: 'The CLI exceeded the local metadata output limit. No result was accepted.',
  cleanupFailed: 'Native process or temporary-storage cleanup could not be confirmed. Resolve that state before restarting the app.',
  unsupportedPlatform: 'This Claude CLI check is not available on this platform.',
};
function inspectionError(error: unknown): string {
  if (error && typeof error === 'object' && 'kind' in error && typeof error.kind === 'string'
    && Object.prototype.hasOwnProperty.call(failures, error.kind)) return failures[error.kind as ClaudeInspectionFailureKind];
  return 'The native CLI check could not be completed. Existing repository data was preserved.';
}

export function ClaudeConnectionDialog({ native, onClose, onChooseBackend }: {
  native: boolean; onClose: () => void; onChooseBackend: () => void;
}) {
  const [path, setPath] = useState(savedPath);
  const [observation, setObservation] = useState<ClaudeConnectionObservation | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const dialog = useRef<HTMLElement>(null);
  const lifetime = useRef(0);
  const active = useRef<{ id: string; cancelled: boolean; cleanupFailed: boolean } | null>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useModalFocusTrap(dialog);

  useEffect(() => {
    ++lifetime.current;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.focus();
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); closeRef.current(); }
    };
    document.addEventListener('keydown', escape, true);
    return () => {
      ++lifetime.current;
      document.removeEventListener('keydown', escape, true);
      const run = active.current;
      active.current = null;
      if (native && run) void cancelClaudeAgentOperation(run.id).catch(() => {});
      if (previous?.isConnected) previous.focus();
    };
  }, [native]);

  const inspect = async () => {
    if (!native || active.current || !path.trim()) return;
    const epoch = lifetime.current;
    let id: string;
    try { id = crypto.randomUUID(); }
    catch { setError('A secure operation identity is unavailable. Reopen the native app before trying again.'); return; }
    const run = { id, cancelled: false, cleanupFailed: false };
    active.current = run;
    setBusy(true); setError(null); setNote(null); setObservation(null);
    try {
      const value = await inspectClaudeAgentConnection(path.trim(), run.id);
      if (lifetime.current !== epoch || run.cancelled) return;
      setObservation(value);
      setPath(value.executablePath);
      // Only the non-secret executable preference is persisted. Account
      // observations are short-lived and must be inspected again explicitly.
      try { localStorage.setItem(preferenceKey, value.executablePath); } catch { /* Optional. */ }
    } catch (error: unknown) {
      // Uncertain native cleanup takes priority over a concurrent cancel.
      // Suppressing abandoned metadata must not hide this actionable failure.
      run.cleanupFailed = !!(error && typeof error === 'object' && 'kind' in error && error.kind === 'cleanupFailed');
      if (lifetime.current === epoch && (!run.cancelled || run.cleanupFailed)) {
        setNote(null);
        setError(inspectionError(error));
      }
    } finally {
      if (active.current?.id === run.id) active.current = null;
      if (lifetime.current === epoch) {
        setBusy(false);
        if (run.cancelled && !run.cleanupFailed) setNote('Check cancelled. Its returned metadata was discarded.');
      }
    }
  };

  const cancel = async () => {
    const run = active.current;
    if (!native || !run || run.cancelled) return;
    run.cancelled = true;
    const epoch = lifetime.current;
    setNote('Stopping the CLI check…');
    try { await cancelClaudeAgentOperation(run.id); }
    catch { if (lifetime.current === epoch && active.current?.id === run.id) setNote('The stop could not be confirmed. Any returned metadata will be discarded.'); }
  };

  return <div className="provider-overlay">
    <section className="provider-dialog" ref={dialog} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="claude-setup-title">
      <header className="provider-dialog-header">
        <div><div className="step-pill"><span>CLAUDE AGENT</span> SETUP CHECK</div>
          <h2 id="claude-setup-title">Check Claude installation</h2>
          <p>Inspect the installed CLI version and its reported sign-in metadata. Repository source is excluded from this check.</p></div>
        <button className="icon-button" aria-label="Close Claude setup" onClick={onClose}><X size={17} /></button>
      </header>
      <div className="provider-dialog-content">
        {!native && <div className="provider-preview-only" role="status"><HardDrive size={18} /><p>CLI inspection runs in the macOS app. This browser preview cannot launch it.</p></div>}
        <section className="provider-review-card">
          <form onSubmit={(event) => { event.preventDefault(); void inspect(); }}>
            <label className="provider-model-select">Claude CLI executable · absolute path
              <input value={path} onChange={(event) => { if (!active.current) { setPath(event.target.value); setObservation(null); setError(null); setNote(null); } }} maxLength={4096} placeholder="/Users/you/.local/bin/claude" spellCheck={false} autoComplete="off" disabled={busy} />
            </label>
            <div className="provider-key-actions"><button className="primary-button" type="submit" disabled={!native || busy || !path.trim()}>{busy ? 'Checking CLI…' : 'Check installation and account route'}</button>
              <a href="https://code.claude.com/docs/en/setup" target="_blank" rel="noreferrer">Official installation guide ↗</a></div>
          </form>
          <p className="provider-small-note">The app runs the documented version and authentication-status commands in private temporary storage. Account/network metadata may be accessed by the vendor CLI. This check does not start a model turn or enable generation.</p>
        </section>
        {observation && <section className="provider-review-card" aria-labelledby="claude-observation-title">
          <h3 id="claude-observation-title">CLI metadata observed</h3>
          <p role="status">{observation.message}</p>
          <dl><dt>Version</dt><dd>{observation.runtimeVersion} · {observation.candidateVersionMatches ? 'candidate matches; qualification pending' : 'candidate differs'}</dd>
            <dt>Account metadata</dt><dd>{observation.account ? observation.account.loggedIn ? 'CLI reports signed in' : 'CLI reports signed out' : 'Not inspected'}</dd>
            {observation.account && <><dt>Authentication route</dt><dd>{observation.account.authMethod}</dd><dt>Reported account</dt><dd>{observation.account.accountEmail ?? 'Identity unavailable'}</dd><dt>Reported plan</dt><dd>{observation.account.subscriptionType ?? 'Unrecognized or unavailable'}</dd></>}
          </dl>
          <details><summary>Installation fingerprint</summary><code>{observation.executablePath}</code><p>SHA-256 {observation.executableSha256}</p></details>
        </section>}
        {error && <p className="provider-error-text" role="alert">{error}</p>}
        {note && <p className="provider-status-note" role="status">{note}</p>}
        <p className="provider-small-note">Automatic generation still needs model discovery, the permitted account integration, source review and dispatch. API authentication is a separate choice. No login flow is offered by this setup check.</p>
      </div>
      <footer className="provider-dialog-footer"><div className="provider-draft-actions">
        <button className="secondary-button" onClick={onClose}>Close</button>
        <button className="secondary-button" onClick={onChooseBackend}>Back to agents</button>
      </div>{busy && <button className="secondary-button" onClick={() => { void cancel(); }}>Cancel check</button>}</footer>
    </section>
  </div>;
}
