import { useEffect, useRef, useState } from 'react';
import { HardDrive, X } from 'lucide-react';
import {
  cancelClaudeAgentOperation, forgetClaudeAgentConnection, inspectClaudeAgentConnection, inspectClaudeAgentModelCatalog,
  type ClaudeConnectionObservation, type ClaudeInspectionFailureKind, type ClaudeModelDiscoveryObservation,
} from './lib/commands';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { ClaudeOperationReview } from './ClaudeOperationReview';
import { claudeReviewScopeKey, type ClaudeOperationReviewMode } from './lib/claudeReview';

const preferenceKey = 'code-atlas:claude-agent-executable';
const nodePreferenceKey = 'code-atlas:claude-agent-node-executable';
const sdkPreferenceKey = 'code-atlas:claude-agent-sdk-module';
function savedPath(key: string): string {
  try { const value = localStorage.getItem(key) ?? ''; return value.length <= 4096 ? value : ''; }
  catch { return ''; }
}

const failures: Record<ClaudeInspectionFailureKind, string> = {
  busy: 'Another Claude operation is active. Finish or cancel it before checking again.',
  cancelled: 'The check was cancelled. No repository source was included.',
  timedOut: 'The metadata check timed out. Check the selected installation and account setup before trying again.',
  runtimeChanged: 'A selected executable or SDK module changed or is incompatible. The catalog check requires the inspected SDK 0.3.300 module; refresh the installation paths.',
  reviewChanged: 'The reviewed context or settings changed. Prepare a fresh request review before generating.',
  preparationFailed: 'The check could not be prepared. Check absolute installation paths, local storage and the existing first-party Claude account. API-key and alternate-provider routes cannot supply this account catalog check.',
  processFailed: 'A metadata process could not finish. Check the installed Claude CLI and, for catalog inspection, the selected Node host and SDK module.',
  protocolRejected: 'Metadata was incomplete or incompatible. Catalog inspection requires the candidate CLI 2.1.300, inspected SDK 0.3.300 and Node 18 or later. This does not establish a qualified generation connection.',
  outputLimit: 'Metadata exceeded a local output limit. No result was accepted.',
  cleanupFailed: 'Native process or temporary-storage cleanup could not be confirmed. Resolve that state before restarting the app.',
  unsupportedPlatform: 'This Claude CLI check is not available on this platform.',
};
function inspectionError(error: unknown): string {
  if (error && typeof error === 'object' && 'kind' in error && typeof error.kind === 'string'
    && Object.prototype.hasOwnProperty.call(failures, error.kind)) return failures[error.kind as ClaudeInspectionFailureKind];
  return 'The native CLI check could not be completed. Existing repository data was preserved.';
}

export function ClaudeConnectionDialog({ native, reviewMode, onClose, onChooseBackend }: {
  native: boolean; reviewMode: ClaudeOperationReviewMode | 'settings' | null;
  onClose: () => void; onChooseBackend: () => void;
}) {
  const [path, setPath] = useState(() => savedPath(preferenceKey));
  const [nodePath, setNodePath] = useState(() => savedPath(nodePreferenceKey));
  const [sdkPath, setSdkPath] = useState(() => savedPath(sdkPreferenceKey));
  const [observation, setObservation] = useState<ClaudeConnectionObservation | null>(null);
  const [catalogObservation, setCatalogObservation] = useState<ClaudeModelDiscoveryObservation | null>(null);
  const [busyKind, setBusyKind] = useState<'connection' | 'catalog' | null>(null);
  const busy = busyKind !== null;
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const dialog = useRef<HTMLElement>(null);
  const lifetime = useRef(0);
  const active = useRef<{ id: string; cancelled: boolean; cleanupFailed: boolean } | null>(null);
  const reviewConnection = useRef<string | null>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useModalFocusTrap(dialog);

  useEffect(() => {
    ++lifetime.current;
    setBusyKind(null); setObservation(null); setCatalogObservation(null); setError(null); setNote(null);
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
      const connectionId = reviewConnection.current;
      reviewConnection.current = null;
      if (native && connectionId) void forgetClaudeAgentConnection(connectionId).catch(() => {});
      if (previous?.isConnected) previous.focus();
    };
  }, [native]);

  const inspect = async (kind: 'connection' | 'catalog') => {
    if (!native || active.current || !path.trim() || (kind === 'catalog' && (!nodePath.trim() || !sdkPath.trim()))) return;
    const epoch = lifetime.current;
    let id: string;
    try { id = crypto.randomUUID(); }
    catch { setError('A secure operation identity is unavailable. Reopen the native app before trying again.'); return; }
    const run = { id, cancelled: false, cleanupFailed: false };
    // New native inspections invalidate the old connection before work starts.
    const previousConnection = reviewConnection.current;
    reviewConnection.current = null;
    if (previousConnection) void forgetClaudeAgentConnection(previousConnection).catch(() => {});
    active.current = run;
    setBusyKind(kind); setError(null); setNote(null); setObservation(null); setCatalogObservation(null);
    try {
      let discovery: ClaudeModelDiscoveryObservation | null = null;
      let value: ClaudeConnectionObservation;
      if (kind === 'catalog') {
        discovery = await inspectClaudeAgentModelCatalog(path.trim(), nodePath.trim(), sdkPath.trim(), run.id);
        value = discovery.connection;
      } else value = await inspectClaudeAgentConnection(path.trim(), run.id);
      if (lifetime.current !== epoch || run.cancelled) return;
      setObservation(value);
      setPath(value.executablePath);
      if (discovery) {
        reviewConnection.current = discovery.reviewConnection?.connectionId ?? null;
        setCatalogObservation(discovery);
        setNodePath(discovery.nodeExecutablePath);
        setSdkPath(discovery.sdkModulePath);
      }
      // Persist installation path hints only. Account/catalog metadata and
      // fingerprints stay in memory and cannot authorize a later generation.
      try {
        localStorage.setItem(preferenceKey, value.executablePath);
        if (discovery) {
          localStorage.setItem(nodePreferenceKey, discovery.nodeExecutablePath);
          localStorage.setItem(sdkPreferenceKey, discovery.sdkModulePath);
        }
      } catch { /* Optional. */ }
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
        setBusyKind(null);
        if (run.cancelled && !run.cleanupFailed) setNote('Check cancelled. Its returned metadata was discarded.');
      }
    }
  };

  const cancel = async () => {
    const run = active.current;
    if (!native || !run || run.cancelled) return;
    run.cancelled = true;
    const epoch = lifetime.current;
    setNote('Stopping the metadata check…');
    try { await cancelClaudeAgentOperation(run.id); }
    catch { if (lifetime.current === epoch && active.current?.id === run.id) setNote('The stop could not be confirmed. Any returned metadata will be discarded.'); }
  };

  const clearObservation = () => {
    const connectionId = reviewConnection.current;
    reviewConnection.current = null;
    if (native && connectionId) void forgetClaudeAgentConnection(connectionId).catch(() => {});
    setObservation(null); setCatalogObservation(null); setError(null); setNote(null);
  };
  const reviewScopeKey = claudeReviewScopeKey(reviewMode);

  return <div className="provider-overlay">
    <section className="provider-dialog" ref={dialog} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="claude-setup-title">
      <header className="provider-dialog-header">
        <div><div className="step-pill"><span>CLAUDE AGENT</span> SETUP CHECK</div>
          <h2 id="claude-setup-title">Check Claude installation</h2>
          <p>Check the installed CLI, inspect its catalog, then preview the selected operation locally. Installation and catalog checks exclude repository source.</p></div>
        <button className="icon-button" aria-label="Close Claude setup" onClick={onClose}><X size={17} /></button>
      </header>
      <div className="provider-dialog-content">
        {!native && <div className="provider-preview-only" role="status"><HardDrive size={18} /><p>CLI inspection runs in the macOS app. This browser preview cannot launch it.</p></div>}
        <section className="provider-review-card">
          <form onSubmit={(event) => { event.preventDefault(); void inspect('connection'); }}>
            <label className="provider-model-select">Claude CLI executable · absolute path
              <input value={path} onChange={(event) => { if (!active.current) { setPath(event.target.value); clearObservation(); } }} maxLength={4096} placeholder="/Users/you/.local/bin/claude" spellCheck={false} autoComplete="off" disabled={busy} />
            </label>
            <div className="provider-key-actions"><button className="primary-button" type="submit" disabled={!native || busy || !path.trim()}>{busyKind === 'connection' ? 'Checking CLI…' : 'Check installation and account route'}</button>
              <a href="https://code.claude.com/docs/en/setup" target="_blank" rel="noreferrer">Official installation guide ↗</a></div>
          </form>
          <p className="provider-small-note">The app runs the documented version and authentication-status commands in private temporary storage. Account/network metadata may be accessed by the vendor CLI. This check does not start a model turn or enable generation.</p>
        </section>
        <section className="provider-review-card">
          <details><summary>Inspect the model catalog · advanced setup</summary>
            <p>This candidate adapter requires an existing Node installation and the inspected Claude Agent SDK 0.3.300. They are not installed by Code Atlas. The native check verifies SDK bytes and observes executable identities; it does not qualify the installation for generation.</p>
            <form onSubmit={(event) => { event.preventDefault(); void inspect('catalog'); }}>
              <label className="provider-model-select">Node executable · absolute path
                <input value={nodePath} onChange={(event) => { if (!active.current) { setNodePath(event.target.value); clearObservation(); } }} maxLength={4096} placeholder="/opt/homebrew/bin/node" spellCheck={false} autoComplete="off" disabled={busy} />
              </label>
              <label className="provider-model-select">Installed SDK module · absolute path to sdk.mjs
                <input value={sdkPath} onChange={(event) => { if (!active.current) { setSdkPath(event.target.value); clearObservation(); } }} maxLength={4096} placeholder="/absolute/installation/node_modules/@anthropic-ai/claude-agent-sdk/sdk.mjs" spellCheck={false} autoComplete="off" disabled={busy} />
              </label>
              <p className="provider-small-note">Check catalog starts the selected executables and SDK in private temporary storage. It may contact the vendor using the existing account. The app supplies no repository source or user prompt. Only use installation paths you trust. No API fallback or login is offered.</p>
              <button className="secondary-button" type="submit" disabled={!native || busy || !path.trim() || !nodePath.trim() || !sdkPath.trim()}>{busyKind === 'catalog' ? 'Checking catalog…' : 'Check catalog'}</button>
            </form>
          </details>
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
        {catalogObservation?.reviewConnection && <ClaudeOperationReview
          key={`${catalogObservation.reviewConnection.connectionId}:${reviewScopeKey}`}
          native={native} mode={reviewMode} discovery={catalogObservation} />}
        {catalogObservation && <section className="provider-review-card" aria-labelledby="claude-models-title">
          <h3 id="claude-models-title">Candidate model catalog</h3>
          <p role="status">{catalogObservation.catalog.models.length} exact model IDs observed. Account entitlement and automatic generation remain unqualified.</p>
          {catalogObservation.reviewConnection && <p className="provider-small-note">This native review connection expires at {new Date(catalogObservation.reviewConnection.expiresAtUnixMs).toLocaleTimeString()}. Changing installation paths or leaving setup discards it. It cannot authorize generation.</p>}
          <ul>{catalogObservation.catalog.models.map((model) => <li key={model.modelId}>
            <strong>{model.displayName}</strong> · <code>{model.modelId}</code>
            <p>Advertised effort: {model.supportedEfforts.length ? model.supportedEfforts.join(', ') : 'no explicit levels; runtime default only'}.</p>
          </li>)}</ul>
          {catalogObservation.catalog.omittedRows > 0 && <p>{catalogObservation.catalog.omittedRows} unresolved catalog rows omitted.</p>}
          <details><summary>SDK and host observations</summary>
            <p>SDK {catalogObservation.catalog.sdkVersion} · CLI {catalogObservation.catalog.runtimeVersion} · Node {catalogObservation.nodeVersion}</p>
            <p><code>{catalogObservation.nodeExecutablePath}</code><br />SHA-256 {catalogObservation.nodeExecutableSha256}</p>
            <p><code>{catalogObservation.sdkModulePath}</code><br />SHA-256 {catalogObservation.sdkModuleSha256}</p>
          </details>
        </section>}
        {error && <p className="provider-error-text" role="alert">{error}</p>}
        {note && <p className="provider-status-note" role="status">{note}</p>}
        <p className="provider-small-note">Automatic generation still needs a qualified runtime, permitted account integration and connected sender. Catalog inspection and local previews cannot enable it. API authentication is a separate choice.</p>
      </div>
      <footer className="provider-dialog-footer"><div className="provider-draft-actions">
        <button className="secondary-button" onClick={onClose}>Close</button>
        <button className="secondary-button" onClick={onChooseBackend}>Back to agents</button>
      </div>{busy && <button className="secondary-button" onClick={() => { void cancel(); }}>Cancel check</button>}</footer>
    </section>
  </div>;
}
