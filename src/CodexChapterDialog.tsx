import { useEffect, useMemo, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Check, FileCode2, HardDrive, Sparkles, X } from 'lucide-react';
import {
  buildCodexChapterReview, buildCodexCodeLensReview, cancelCodexChapterRequest, cancelCodexLogin,
  configureCodexAgent, discardGeneratedChapterDraft, discardProviderRequestReview,
  getCodexAgentStatus, listChapterRevisions, listCodexModels, loadSnapshotManifest,
  logoutCodexAgent, saveGeneratedChapterDraft, sendCodexChapterRequest, sendCodexCodeLensRequest, startCodexLogin,
  type ChapterDraftPreview, type ChapterRevision, type CodexAgentStatus, type CodexModel, type CodexProgress,
  type ProviderFailure, type ProviderRequestReview, type SectionUpdateReviewTarget, type SnapshotManifest,
  type CodeLensExplanationTarget, type CodeLensExplanationPreview, type CodeLensExplanationCitation,
} from './lib/commands';

export type CodexChapterDialogMode = 'settings' | {
  kind: 'review' | 'overview';
  projectId: string;
  snapshotId: string;
  targetPath?: string;
  selectedPaths?: string[];
  recoveryOperationId?: string;
} | {
  kind: 'section-update';
  projectId: string;
  snapshotId: string;
  targetPath: string;
  selectedPaths: string[];
  sectionUpdate: SectionUpdateReviewTarget;
  sectionTitle?: string;
  recoveryOperationId?: string;
} | {
  kind: 'code-lens-range';
  projectId: string;
  snapshotId: string;
  target: CodeLensExplanationTarget;
};

export interface CodexChapterDialogProps {
  mode: CodexChapterDialogMode;
  native: boolean;
  onClose: () => void;
  onChooseBackend: () => void;
  onSaved?: (revision: ChapterRevision) => void;
  onRecoveryReviewReady?: (operationId: string) => void;
  onOpenCodeLensCitation?: (citation: CodeLensExplanationCitation) => void;
}

type Busy = 'status' | 'configure' | 'login' | 'cancel-login' | 'logout' | 'models' | 'build' | 'send' | 'save' | 'discard';
const preferenceKey = 'code-atlas:codex-chapter-preferences';

function preferences(): { modelId: string; effort: string } {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(preferenceKey) ?? '{}');
    if (saved && typeof saved === 'object' && 'modelId' in saved && 'effort' in saved
      && typeof saved.modelId === 'string' && typeof saved.effort === 'string') {
      return { modelId: saved.modelId, effort: saved.effort };
    }
  } catch { /* Preferences are optional; blocked storage does not block generation. */ }
  return { modelId: '', effort: '' };
}

function remember(modelId: string, effort: string) {
  try { localStorage.setItem(preferenceKey, JSON.stringify({ modelId, effort })); } catch { /* Optional. */ }
}

function safeFailure(failure: ProviderFailure): string {
  // Do not surface raw agent output, JSON-RPC errors, source text, or tokens.
  // This adapter's rejected-request messages are authored by the native app.
  // They include actionable runtime upgrade and configuration instructions.
  if (failure.kind === 'providerRejectedRequest' && failure.message.trim()) return failure.message;
  const messages: Partial<Record<ProviderFailure['kind'], string>> = {
    consentRequired: 'Confirm the exact request and account quota disclosure before generating.',
    reviewExpired: 'This review expired. Build a new review and confirm it again.',
    reviewChanged: 'The reviewed context or settings changed. Build a new review before sending.',
    reviewUnavailable: 'This review is no longer available. Build a new review.',
    contextTooLarge: 'The selected context exceeds the request limit. Select fewer or smaller files.',
    secretLikeContext: 'The selected context contains a blocked secret-like path or pattern. Remove it before reviewing again.',
    unavailableModel: 'The selected model or reasoning setting is unavailable. Refresh the model catalog and choose again.',
    quotaExceeded: 'The Codex account quota is unavailable or exhausted. Check the account before trying again.',
    requestCancelled: 'Generation was cancelled. No response was accepted.',
    requestTimedOut: 'Codex did not finish in time. No draft was accepted; a new attempt needs a new review.',
    networkUnavailable: 'Codex could not complete the request. Check the connection and account, then build a new review.',
    malformedProviderResponse: 'The response did not pass the explanation format or source evidence checks. No response was accepted.',
    providerResponseTooLarge: 'The response exceeded the local size limit. Try a smaller source selection.',
    localStorageUnavailable: 'Local storage is unavailable. Existing chapter revisions were preserved.',
    invalidReview: 'The local request could not be validated. Check the snapshot selection and Codex setup.',
  };
  return messages[failure.kind] ?? 'Codex could not complete this request. Check the executable, account and supported model settings, then build a new review.';
}

function loginHref(value: string): string | null {
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && ['auth.openai.com', 'chatgpt.com'].includes(url.hostname)
      && !url.username && !url.password ? url.href : null;
  } catch { return null; }
}

export function CodexChapterDialog({ mode, native, onClose, onChooseBackend, onSaved, onRecoveryReviewReady, onOpenCodeLensCitation }: CodexChapterDialogProps) {
  const projectId = mode === 'settings' ? null : mode.projectId;
  const snapshotId = mode === 'settings' ? null : mode.snapshotId;
  const codeLensTarget = mode !== 'settings' && mode.kind === 'code-lens-range' ? mode.target : null;
  const targetPath = mode === 'settings' || mode.kind === 'overview' ? null
    : mode.kind === 'code-lens-range' ? mode.target.path : mode.targetPath ?? null;
  const sectionUpdate = mode !== 'settings' && mode.kind === 'section-update' ? mode.sectionUpdate : undefined;
  const initialSelectedPaths = mode === 'settings' || mode.kind === 'code-lens-range' ? undefined : mode.selectedPaths;
  const [status, setStatus] = useState<CodexAgentStatus | null>(null);
  const [executablePath, setExecutablePath] = useState('');
  const [login, setLogin] = useState<{ loginId: string; authUrl: string } | null>(null);
  const [models, setModels] = useState<CodexModel[]>([]);
  const [selection, setSelection] = useState(preferences);
  const [manifest, setManifest] = useState<SnapshotManifest | null>(null);
  const [manifestLoading, setManifestLoading] = useState(false);
  const [manifestError, setManifestError] = useState<string | null>(null);
  const [paths, setPaths] = useState<string[]>(() => [...new Set([...(targetPath ? [targetPath] : []), ...(initialSelectedPaths ?? [])])]);
  const [query, setQuery] = useState('');
  const [review, setReview] = useState<ProviderRequestReview | null>(null);
  const [preview, setPreview] = useState<ChapterDraftPreview | null>(null);
  const [codeLensResult, setCodeLensResult] = useState<CodeLensExplanationPreview | null>(null);
  const [saved, setSaved] = useState<ChapterRevision | null>(null);
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState<Busy | null>(native ? 'status' : null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [progress, setProgress] = useState<CodexProgress | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const dialogRef = useRef<HTMLElement>(null);
  const summaryRef = useRef<HTMLDivElement>(null);
  const lifetime = useRef(0);
  const activeBusy = useRef<Busy | null>(null);
  const resources = useRef<{ login: string | null; review: string | null; draft: string | null; run: string | null }>({ login: null, review: null, draft: null, run: null });
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const cancelRequested = useRef(false);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  const eligibleFiles = useMemo(() => (manifest?.files ?? []).filter((file) => file.kind === 'file'
    && (file.contentKind === 'text' || file.contentKind === 'unknown') && !file.secretLike && (Boolean(codeLensTarget) || file.sizeBytes <= 250_000))
    .sort((left, right) => left.path.localeCompare(right.path)), [manifest, codeLensTarget]);
  const filteredFiles = eligibleFiles.filter((file) => (!sectionUpdate || paths.includes(file.path))
    && file.path.toLowerCase().includes(query.trim().toLowerCase()));
  const selectedModel = models.find((model) => model.modelId === selection.modelId);
  const modelReady = Boolean(selectedModel?.supportedReasoningEfforts.includes(selection.effort));
  const sourceReady = Boolean(manifest && paths.length > 0 && paths.length <= 12
    && paths.every((path) => eligibleFiles.some((file) => file.path === path))
    && (mode === 'settings' || mode.kind === 'overview' || targetPath));
  const hasResult = preview !== null || codeLensResult !== null;
  const locked = busy !== null || hasResult;
  const otherRun = Boolean(status?.activeRunId && status.activeRunId !== resources.current.run);

  const releaseReview = () => {
    const id = resources.current.review;
    resources.current.review = null;
    if (id && native) void discardProviderRequestReview(id).catch(() => {});
    setReview(null);
    setConsent(false);
  };

  const begin = (operation: Busy): number | null => {
    if (!native || activeBusy.current) return null;
    activeBusy.current = operation;
    setBusy(operation);
    setError(null);
    setNote(null);
    return lifetime.current;
  };

  const finish = (epoch: number) => {
    if (lifetime.current !== epoch) return;
    activeBusy.current = null;
    setBusy(null);
  };

  const acceptStatus = (value: CodexAgentStatus) => {
    setStatus(value);
    setExecutablePath(value.executablePath ?? '');
    if (value.authenticated || !value.pendingLoginId) {
      resources.current.login = null;
      setLogin(null);
    }
  };

  useEffect(() => {
    const epoch = ++lifetime.current;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const current = () => lifetime.current === epoch;
    dialogRef.current?.focus();
    setStatus(null);
    setModels([]);
    setManifest(null);
    setManifestError(null);
    setPaths([...new Set([...(targetPath ? [targetPath] : []), ...(initialSelectedPaths ?? [])])]);
    setReview(null);
    setPreview(null);
    setCodeLensResult(null);
    setSaved(null);
    setLogin(null);
    setConsent(false);
    setProgress(null);
    setError(null);
    setNote(null);
    cancelRequested.current = false;
    activeBusy.current = native ? 'status' : null;
    setBusy(activeBusy.current);
    if (native) {
      void getCodexAgentStatus().then((value) => {
        if (current()) acceptStatus(value);
      }).catch(() => { if (current()) setError('Codex setup could not be read. Refresh account status to try again.'); })
        .finally(() => { if (current()) finish(epoch); });
      if (projectId && snapshotId) {
        setManifestLoading(true);
        void loadSnapshotManifest(projectId, snapshotId).then((value) => {
          if (current()) setManifest(value);
        }).catch(() => { if (current()) setManifestError('The captured file list could not be loaded. Reopen the snapshot to try again.'); })
          .finally(() => { if (current()) setManifestLoading(false); });
      }
    }
    const keydown = (event: KeyboardEvent) => {
      const dialog = dialogRef.current;
      if (!dialog) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        if (!activeBusy.current) closeRef.current();
      }
      if (event.key !== 'Tab') return;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(
        'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), summary, [tabindex]:not([tabindex="-1"])',
      )).filter((element) => element.getClientRects().length > 0 && element.getAttribute('aria-hidden') !== 'true');
      const index = focusable.indexOf(document.activeElement as HTMLElement);
      if (!focusable.length) { event.preventDefault(); dialog.focus(); }
      else if (index < 0 || (event.shiftKey && index === 0) || (!event.shiftKey && index === focusable.length - 1)) {
        event.preventDefault();
        (event.shiftKey ? focusable[focusable.length - 1] : focusable[0]).focus();
      }
    };
    document.addEventListener('keydown', keydown, true);
    return () => {
      ++lifetime.current;
      cancelRequested.current = true;
      document.removeEventListener('keydown', keydown, true);
      unlistenRef.current?.();
      unlistenRef.current = null;
      const pending = resources.current;
      resources.current = { login: null, review: null, draft: null, run: null };
      if (native) {
        if (pending.login) void cancelCodexLogin(pending.login).catch(() => {});
        if (pending.run) void cancelCodexChapterRequest(pending.run).catch(() => {});
        if (pending.review) void discardProviderRequestReview(pending.review).catch(() => {});
        // An in-flight save owns its draft until the command resolves.
        if (pending.draft && activeBusy.current !== 'save') void discardGeneratedChapterDraft(pending.draft).catch(() => {});
      }
      if (previousFocus?.isConnected) previousFocus.focus();
    };
    // These primitive identifiers define the request lifetime; callback identity does not.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [native, projectId, snapshotId, targetPath, initialSelectedPaths, sectionUpdate, codeLensTarget]);

  useEffect(() => { if (review) summaryRef.current?.focus(); }, [review]);

  const refresh = async () => {
    const epoch = begin('status');
    if (epoch === null) return;
    try {
      const value = await getCodexAgentStatus();
      if (lifetime.current === epoch) acceptStatus(value);
    } catch { if (lifetime.current === epoch) setError('Account status is unavailable. Check that the selected Codex executable is installed and accessible.'); }
    finally { finish(epoch); }
  };

  const configure = async () => {
    if (!executablePath.trim() || hasResult || otherRun || resources.current.login) return;
    const epoch = begin('configure');
    if (epoch === null) return;
    releaseReview();
    try {
      const value = await configureCodexAgent(executablePath.trim());
      if (lifetime.current === epoch) { acceptStatus(value); setModels([]); setNote('Executable saved locally. Connect your account to start Codex.'); }
    } catch { if (lifetime.current === epoch) setError('The executable could not be saved. Use an absolute path to an installed, executable Codex CLI binary.'); }
    finally { finish(epoch); }
  };

  const connect = async () => {
    if (hasResult || otherRun) return;
    const epoch = begin('login');
    if (epoch === null) return;
    releaseReview();
    try {
      const value = await startCodexLogin();
      if (lifetime.current !== epoch) { await cancelCodexLogin(value.loginId).catch(() => {}); return; }
      if (!loginHref(value.authUrl)) { await cancelCodexLogin(value.loginId).catch(() => {}); setError('Codex returned an unsupported sign-in destination.'); return; }
      resources.current.login = value.loginId;
      setLogin(value);
      setModels([]);
      setNote('Codex opened sign-in in your browser. Complete ChatGPT sign-in, then select Refresh account. Use the link below if the browser did not open.');
    } catch { if (lifetime.current === epoch) setError('Codex could not start sign-in. Check the executable path and update Codex if needed. This adapter requires the restricted app-server protocol.'); }
    finally { finish(epoch); }
  };

  const cancelLogin = async () => {
    const id = resources.current.login;
    if (!id) return;
    const epoch = begin('cancel-login');
    if (epoch === null) return;
    try {
      await cancelCodexLogin(id);
      if (lifetime.current === epoch) { resources.current.login = null; setLogin(null); setNote('Sign-in cancelled.'); }
    } catch { if (lifetime.current === epoch) setError('Sign-in cancellation could not be confirmed. Close the sign-in browser page and refresh account status.'); }
    finally { finish(epoch); }
  };

  const logout = async () => {
    if (hasResult || otherRun) return;
    const epoch = begin('logout');
    if (epoch === null) return;
    releaseReview();
    try {
      await logoutCodexAgent();
      if (lifetime.current !== epoch) return;
      resources.current.login = null;
      setLogin(null);
      setModels([]);
      setStatus(null);
      const value = await getCodexAgentStatus();
      if (lifetime.current === epoch) acceptStatus(value);
    } catch { if (lifetime.current === epoch) setError('Account disconnection could not be confirmed. Refresh account status before continuing.'); }
    finally { finish(epoch); }
  };

  const discoverModels = async () => {
    if (hasResult || otherRun || resources.current.login) return;
    const epoch = begin('models');
    if (epoch === null) return;
    releaseReview();
    setModels([]);
    try {
      const value = await listCodexModels();
      if (lifetime.current !== epoch) return;
      setModels(value);
      if (!value.length) setNote('This account returned no supported models. Refresh your account or reconnect.');
      else if (!value.some((model) => model.modelId === selection.modelId)) setNote('Choose a model from this account’s catalog.');
      const account = await getCodexAgentStatus();
      if (lifetime.current === epoch) acceptStatus(account);
    } catch { if (lifetime.current === epoch) setError('Models could not be loaded. Connect your ChatGPT account and check that this Codex version supports the required app-server protocol.'); }
    finally { finish(epoch); }
  };

  const changeModel = (modelId: string) => {
    if (locked) return;
    const model = models.find((item) => item.modelId === modelId);
    const effort = model?.defaultReasoningEffort && model.supportedReasoningEfforts.includes(model.defaultReasoningEffort)
      ? model.defaultReasoningEffort : model?.supportedReasoningEfforts[0] ?? '';
    releaseReview();
    setSelection({ modelId, effort });
    remember(modelId, effort);
  };

  const changePath = (path: string, checked: boolean) => {
    if (locked || activeBusy.current || path === targetPath || sectionUpdate || codeLensTarget) return;
    releaseReview();
    setPaths((current) => checked
      ? current.includes(path) || current.length >= 12 ? current : [...current, path]
      : current.filter((selected) => selected !== path));
  };

  const buildReview = async () => {
    if (!projectId || !snapshotId || !sourceReady || !modelReady || hasResult || otherRun || resources.current.login) return;
    const epoch = begin('build');
    if (epoch === null) return;
    releaseReview();
    setSaved(null);
    try {
      const value = codeLensTarget
        ? await buildCodexCodeLensReview({ projectId, snapshotId, target: codeLensTarget,
          modelId: selection.modelId, effort: selection.effort, approvalPolicy: 'never' })
        : await buildCodexChapterReview({ projectId, snapshotId, targetPath, selectedPaths: paths,
          modelId: selection.modelId, effort: selection.effort, approvalPolicy: 'never', sectionUpdate: sectionUpdate ?? null });
      if (value.status === 'error') { if (lifetime.current === epoch) setError(safeFailure(value.error)); return; }
      if (lifetime.current !== epoch) { await discardProviderRequestReview(value.data.reviewId).catch(() => {}); return; }
      resources.current.review = value.data.reviewId;
      setReview(value.data);
      if (mode !== 'settings' && 'recoveryOperationId' in mode && mode.recoveryOperationId) onRecoveryReviewReady?.(mode.recoveryOperationId);
    } catch { if (lifetime.current === epoch) setError('The local review could not be created. No generation was requested. Check the selected snapshot files and settings.'); }
    finally { finish(epoch); }
  };

  const generate = async () => {
    if (!review || !consent || hasResult || !modelReady || !status?.authenticated || otherRun || resources.current.login) return;
    if (Date.now() >= review.expiresAtUnixMs) { releaseReview(); setError('This review expired. Build a new review and confirm it again.'); return; }
    const epoch = begin('send');
    if (epoch === null) return;
    const runId = review.reviewId;
    resources.current.run = runId;
    cancelRequested.current = false;
    setCancelling(false);
    setProgress({ runId, phase: 'preparing', message: 'Preparing the reviewed Codex request.', threadId: null, turnId: null });
    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<CodexProgress>('codex-agent-progress', ({ payload }) => {
        if (lifetime.current !== epoch || payload.runId !== runId) return;
        setProgress(payload);
        // Covers cancellation before the native worker registered its active run.
        if (cancelRequested.current) void cancelCodexChapterRequest(runId).catch(() => {});
      });
      if (lifetime.current !== epoch || cancelRequested.current) return;
      unlistenRef.current = unlisten;
      if (codeLensTarget) {
        const result = await sendCodexCodeLensRequest(review.reviewId, review.fingerprint, true);
        if (lifetime.current !== epoch || cancelRequested.current) {
          if (lifetime.current === epoch) setNote('Generation was cancelled. The transient explanation was discarded. Account quota may already have been used.');
          return;
        }
        if (result.status === 'error') setError(safeFailure(result.error));
        else setCodeLensResult(result.data);
        return;
      }
      const result = await sendCodexChapterRequest(review.reviewId, review.fingerprint, true);
      if (result.status === 'error') {
        if (lifetime.current === epoch) {
          if (cancelRequested.current) setNote('Generation was cancelled. No response was accepted. Source may already have been shared and account quota used.');
          else setError(safeFailure(result.error));
        }
        return;
      }
      if (lifetime.current !== epoch || cancelRequested.current) {
        await discardGeneratedChapterDraft(result.data.generationId).catch(() => {});
        if (lifetime.current === epoch) setNote('Cancellation requested. The returned draft was discarded. Account quota may already have been used.');
        return;
      }
      resources.current.draft = result.data.generationId;
      setPreview(result.data);
    } catch {
      if (lifetime.current === epoch) {
        if (cancelRequested.current) setNote('Cancellation was requested. No returned response was accepted. Source may already have been shared and account quota used.');
        else setError('The Codex run did not return a usable result. Source may already have been shared. Review a new request before retrying.');
      }
    }
    finally {
      unlisten?.();
      if (unlistenRef.current === unlisten) unlistenRef.current = null;
      if (lifetime.current === epoch) {
        resources.current.run = null;
        releaseReview();
        setCancelling(false);
        setProgress(null);
      }
      finish(epoch);
      // A saved preview may exist even after a failed reply or journal cleanup.
      // Refresh the app's recovery list even if this dialog has been closed.
      if (!codeLensTarget) window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    }
  };

  const cancelRun = async () => {
    const runId = resources.current.run;
    if (!native || !runId || cancelRequested.current) return;
    const epoch = lifetime.current;
    cancelRequested.current = true;
    setCancelling(true);
    setNote('Stopping generation. Any returned response will be discarded. Source may already have been shared and account quota used.');
    try { await cancelCodexChapterRequest(runId); }
    catch { if (lifetime.current === epoch) setNote('The stop could not be confirmed. Any returned response will still be discarded; wait for the active operation to finish.'); }
  };

  const discard = async () => {
    if (!preview || saved) return;
    const epoch = begin('discard');
    if (epoch === null) return;
    try {
      await discardGeneratedChapterDraft(preview.generationId);
      if (lifetime.current === epoch) { resources.current.draft = null; setPreview(null); setNote('Unsaved preview discarded. Existing chapter revisions were preserved.'); }
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    } catch { if (lifetime.current === epoch) setError('The local preview could not be discarded. Try again before closing.'); }
    finally { finish(epoch); }
  };

  const save = async () => {
    if (!preview || saved || preview.validation.status !== 'valid') return;
    const epoch = begin('save');
    if (epoch === null) return;
    const draft = preview;
    try {
      let revision: ChapterRevision;
      try { revision = await saveGeneratedChapterDraft(draft.generationId); }
      catch {
        // A lost response may follow a successful immutable local write.
        const existing = await listChapterRevisions(draft.revision.projectId, draft.revision.snapshotId)
          .then((values) => values.find((value) => value.revisionId === draft.revision.revisionId)).catch(() => undefined);
        if (!existing) throw new Error('Save not confirmed');
        revision = existing;
        await discardGeneratedChapterDraft(draft.generationId).catch(() => {});
      }
      if (lifetime.current === epoch) { resources.current.draft = null; setSaved(revision); onSaved?.(revision); }
      window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision } }));
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    } catch {
      if (lifetime.current === epoch) setError('The draft could not be saved locally. Existing revisions were preserved. You can retry saving this preview.');
      else await discardGeneratedChapterDraft(draft.generationId).catch(() => {});
    } finally { finish(epoch); }
  };

  return (
    <div className="provider-overlay">
      <section ref={dialogRef} className="provider-dialog" role="dialog" aria-modal="true" aria-labelledby="codex-dialog-title" tabIndex={-1}>
        <header className="provider-dialog-header">
          <div><div className="step-pill"><span>CODEX ACCOUNT</span> NO API KEY</div>
            <h2 id="codex-dialog-title">{mode === 'settings' ? 'Connect Codex' : codeLensTarget ? 'Explain this selection with Codex' : sectionUpdate ? 'Update this section with Codex' : targetPath ? 'Explain this file with Codex' : 'Map the repository with Codex'}</h2>
            <p>Use your ChatGPT account through Codex. Generation shares only the reviewed snapshot context and uses your account’s plan quota.</p></div>
          <button className="icon-button" aria-label="Close Codex dialog" disabled={busy !== null} onClick={() => { if (!activeBusy.current) onClose(); }}><X size={17} /></button>
        </header>
        <div className="provider-dialog-content">
          {!native ? <div className="provider-preview-only" role="status"><HardDrive size={18} /><div><strong>Codex runs in the macOS app.</strong><p>This browser preview cannot launch Codex, sign in, read snapshot files, or generate chapters.</p></div></div> : <>
            <details className="provider-key-card" open={mode === 'settings' || !status?.authenticated}>
              <summary>Codex setup · {status?.authenticated ? 'account connected' : 'connect your account'}</summary>
              <div className="provider-key-content">
                <p role="status">{status?.message ?? 'Select an installed Codex CLI executable, then connect your ChatGPT account.'}</p>
                {status?.authenticated && <p>{status.accountEmail ?? 'ChatGPT account'} · {status.planType ?? 'Account plan'} · {status.runtimeRunning ? 'Codex running' : 'Codex stopped'}</p>}
                <form className="provider-key-form" onSubmit={(event) => { event.preventDefault(); void configure(); }}>
                  <label htmlFor="codex-executable">Codex executable · absolute path</label>
                  <input id="codex-executable" value={executablePath} onChange={(event) => setExecutablePath(event.target.value)} placeholder="/opt/homebrew/bin/codex" spellCheck={false} autoComplete="off" disabled={locked || otherRun || Boolean(login)} />
                  <div className="provider-key-actions"><button type="submit" className="secondary-button" disabled={locked || otherRun || Boolean(login) || !executablePath.trim()}>{busy === 'configure' ? 'Saving…' : 'Save executable'}</button>
                    <a href="https://developers.openai.com/codex/cli" target="_blank" rel="noreferrer">Codex installation guide ↗</a></div>
                </form>
                <div className="provider-key-actions">
                  <button className="primary-button" disabled={locked || otherRun || Boolean(login)} onClick={() => { void connect(); }}>{busy === 'login' ? 'Starting sign-in…' : status?.authenticated ? 'Reconnect account' : 'Connect ChatGPT account'}</button>
                  <button className="secondary-button" disabled={busy !== null} onClick={() => { void refresh(); }}>{busy === 'status' ? 'Refreshing…' : 'Refresh account'}</button>
                  {status?.authenticated && <button className="secondary-button" disabled={locked || otherRun} onClick={() => { void logout(); }}>Disconnect account</button>}
                </div>
                {login && <div className="provider-status-note"><a href={loginHref(login.authUrl) ?? undefined} target="_blank" rel="noreferrer">Continue ChatGPT sign-in in your browser ↗</a><p>After sign-in, select Refresh account.</p><button className="secondary-button" disabled={busy !== null} onClick={() => { void cancelLogin(); }}>Cancel sign-in</button></div>}
                <p className="provider-small-note">Codex manages account sign-in. Code Atlas does not ask for your password or an API key. Saving an executable does not start it. Connect and Load models can start the managed Codex runtime.</p>
              </div>
            </details>
            <section className="provider-disclosure-card" aria-labelledby="codex-model-title">
              <div className="provider-section-heading"><div><span>ACCOUNT MODELS</span><h3 id="codex-model-title">Model and reasoning</h3></div><button className="secondary-button" onClick={() => { void discoverModels(); }} disabled={locked || otherRun || Boolean(login)}>{busy === 'models' ? 'Loading…' : 'Load / refresh models'}</button></div>
              <p className="provider-small-note">Models are discovered only when you press this button. Availability and usage limits depend on your Codex account.</p>
              <label className="provider-model-select">Model<select value={selectedModel ? selection.modelId : ''} onChange={(event) => changeModel(event.target.value)} disabled={locked || !models.length}><option value="">{models.length ? 'Choose a model…' : 'Load models first…'}</option>{models.map((model) => <option key={model.modelId} value={model.modelId}>{model.displayName} · {model.modelId}</option>)}</select></label>
              <label className="provider-model-select">Reasoning effort<select value={selectedModel?.supportedReasoningEfforts.includes(selection.effort) ? selection.effort : ''} onChange={(event) => { if (locked) return; releaseReview(); const next = { ...selection, effort: event.target.value }; setSelection(next); remember(next.modelId, next.effort); }} disabled={locked || !selectedModel}><option value="">Choose a supported effort…</option>{selectedModel?.supportedReasoningEfforts.map((effort) => <option key={effort} value={effort}>{effort}</option>)}</select></label>
              <p><strong>Approval: Never.</strong> This initial adapter disables command and web tools and uses a temporary, read-only copy of the reviewed files.</p>
              <p className="provider-small-note">Only model and effort preferences are remembered here. Every generation requires a fresh source review and consent.</p>
            </section>
            {otherRun && <p className="provider-status-note" role="status">Another Codex operation is active. Wait for it to finish, then refresh account status.</p>}
            {mode !== 'settings' && <section className="provider-review-card" aria-labelledby="codex-source-title">
              <div className="provider-section-heading"><div><span>IMMUTABLE SNAPSHOT · {snapshotId}</span><h3 id="codex-source-title">{codeLensTarget ? 'Review the selected code' : 'Choose the code to explain'}</h3></div></div>
              {codeLensTarget ? <div className="provider-context-picker">
                <strong><code>{codeLensTarget.path}:{codeLensTarget.startLine}–{codeLensTarget.endLine}</code></strong>
                <p>Only this exact selection is shared. Supporting files and code outside the range are excluded. The explanation is temporary and is not saved to chapter history.</p>
                {codeLensTarget.startByte !== null && codeLensTarget.startByte !== undefined && <small>UTF-8 bytes [{codeLensTarget.startByte}, {codeLensTarget.endByteExclusive})</small>}
                <small>Captured file SHA-256: {codeLensTarget.sourceSha256}</small>
                {manifestLoading && <p role="status">Checking the captured source…</p>}
                {manifestError && <p className="provider-error-text" role="alert">{manifestError}</p>}
                {!manifestLoading && !manifestError && !sourceReady && <p className="provider-error-text" role="alert">This selection’s source is unavailable. Reopen the captured file and select the code again.</p>}
              </div> : <div className="provider-context-picker">
                <div className="provider-context-heading"><strong>{targetPath ?? 'Repository overview'}</strong><span>{paths.length} of 12 files selected</span></div>
                <p>{sectionUpdate ? 'This repair uses the selected section’s declared evidence. Source selection is fixed; the native review rechecks that only this section is stale and the other sections can carry forward.' : targetPath ? 'The target file stays selected. Add supporting files to explain its real behavior.' : 'Choose 1–12 files that ground the repository overview.'}</p>
                {mode.kind === 'section-update' && <p><strong>{mode.sectionTitle ?? 'Selected stale section'}</strong> · Older source and the previous section are included as context in the exact prompt below. Only current source is citable. The source byte limit counts both snapshots.</p>}
                {manifestLoading && <p role="status">Loading the captured file list…</p>}
                {manifestError && <p className="provider-error-text" role="alert">{manifestError}</p>}
                {manifest && <><label className="provider-context-search">{sectionUpdate ? 'Filter declared source files' : 'Filter captured files'}<input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search snapshot paths" /></label>
                  <div className="provider-context-files">{filteredFiles.map((file) => {
                    const checked = paths.includes(file.path);
                    return <label key={file.path} className={checked ? 'selected' : ''}><input type="checkbox" checked={checked} disabled={locked || Boolean(sectionUpdate) || file.path === targetPath || (!checked && paths.length >= 12)} onChange={(event) => changePath(file.path, event.target.checked)} /><code>{file.path}</code><small>{file.sizeBytes.toLocaleString()} B{file.path === targetPath ? ' · target' : ''}</small></label>;
                  })}{!filteredFiles.length && <p>No eligible captured files match this filter.</p>}</div>
                  <details><summary>Selected paths ({paths.length})</summary><ul>{paths.map((path) => <li key={path}><code>{path}</code></li>)}</ul></details>
                  {!sourceReady && paths.length > 0 && <p className="provider-error-text">{paths.length > 12
                    ? sectionUpdate ? 'This fixed repair scope exceeds 12 files. Reopen the section to correct its declared source scope.' : 'This selection exceeds 12 files. Remove supporting files before reviewing.'
                    : 'The target or a selected file is unavailable for this request. Use an eligible captured text file.'}</p>}
                </>}
                <p className="provider-small-note">Captured text only, up to 250 KB per file. The native review enforces the total request limit and blocks secret-like context.</p>
              </div>}
              <button className="secondary-button" disabled={locked || otherRun || !sourceReady || !modelReady || Boolean(login)} onClick={() => { void buildReview(); }}>{busy === 'build' ? 'Building local review…' : review ? 'Rebuild exact request review' : 'Review exact request'}</button>
              <p className="provider-small-note">Reviewing prepares the exact request locally. Generation starts only after your consent below.</p>
              {review && <>
                <div ref={summaryRef} className="provider-request-summary" role="group" aria-label="Exact Codex request" tabIndex={-1}>
                  <div><span>Source snapshot</span><code>{review.snapshotId}</code></div><div><span>Model</span><strong>{review.modelId}</strong></div>
                  <div><span>{sectionUpdate ? 'Current files · source bytes across both snapshots' : 'Selected source'}</span><strong>{review.selectedFiles.length} files · {review.sourceBytes.toLocaleString()} bytes</strong></div>
                  <div><span>Input estimate</span><strong>About {review.estimatedInputTokens.toLocaleString()} tokens</strong></div>
                  <div><span>Review expiry</span><strong>{new Date(review.expiresAtUnixMs).toLocaleTimeString()}</strong></div>
                </div>
                {review.selectedFiles.map((file) => <details className="provider-source-file" key={file.path}><summary><code>{file.path}</code><span>{codeLensTarget ? 'Captured file SHA-256' : 'SHA-256'} {file.sha256}</span></summary><pre><code>{file.contents}</code></pre></details>)}
                <details className="provider-index-preview"><summary>Exact declarations and relationships</summary><pre>{JSON.stringify({ symbols: review.selectedSymbols, relationships: review.selectedRelationships, truncated: review.indexEvidenceTruncated }, null, 2)}</pre></details>
                <details className="provider-prompt-preview"><summary>Exact prompt, output schema and agent settings</summary><div><strong>System instruction</strong><pre>{review.systemInstruction}</pre><strong>User prompt</strong><pre>{review.userPrompt}</pre><strong>Settings and output schema</strong><pre>{JSON.stringify(review.generationConfig, null, 2)}</pre><strong>Effective agent request · SHA-256 {review.requestBodySha256}</strong><pre>{JSON.stringify(review.requestBodyPreview, null, 2)}</pre></div></details>
                <div className="provider-fingerprint"><span>Consent applies to this fingerprint</span><code>{review.fingerprint}</code><small>Changing source, model, effort or executable clears this review and its consent.</small></div>
                <div className="provider-consent-list"><label><input type="checkbox" checked={consent} disabled={busy !== null} onChange={(event) => setConsent(event.target.checked)} /><span><strong>Send this reviewed context through my Codex account</strong>I have reviewed the exact source, prompt and settings. This shares the selected code with OpenAI through Codex, under my account’s terms and plan quota. No API key is used; availability and remaining quota are not guaranteed.</span></label></div>
              </>}
              {busy === 'send' && <div className="provider-loading" role="status" aria-live="polite"><Sparkles size={16} /><div><strong>{cancelling ? 'Cancelling Codex generation…' : progress?.message ?? 'Generating explanation…'}</strong><p>{progress?.phase ?? 'preparing'} · {codeLensTarget ? 'This explanation will not be saved to chapter history.' : 'No draft is saved until you choose Save.'}</p></div></div>}
              {codeLensResult && <section className="provider-response" aria-labelledby="codex-range-result-title">
                <div><span>TRANSIENT · RANGE GROUNDED</span><h4 id="codex-range-result-title">What this code does</h4></div>
                <p>Every claim cites an exact, unique quote inside your selection. Nothing was saved to chapter history.</p>
                <ol className="code-lens-explanation-claims">{codeLensResult.claims.map((claim) => <li key={claim.claimId}>
                  <p>{claim.prose}</p>
                  {onOpenCodeLensCitation && <button className="text-button" onClick={() => onOpenCodeLensCitation(claim.citation)}><FileCode2 size={13} /> Open {claim.citation.path}:{claim.citation.startLine}–{claim.citation.endLine}</button>}
                  <details className="chapter-evidence"><summary>Inspect cited code</summary><pre><code>{claim.citation.quote}</code></pre></details>
                </li>)}</ol>
                <small>{codeLensResult.modelVersion ?? codeLensResult.modelId}</small>
              </section>}
              {preview && <section className="provider-response" aria-labelledby="codex-result-title">
                <div><span>{saved ? 'SAVED · UNACCEPTED DRAFT' : preview.validation.status === 'valid' ? 'VALIDATED · UNSAVED PREVIEW' : 'VALIDATION FAILED'}</span><h4 id="codex-result-title">{preview.revision.title}</h4></div>
                <p>{saved ? `Draft revision ${saved.revisionNumber} is saved locally. Acceptance is a separate review.` : 'Review the explanation and exact citations. Saving creates a local unaccepted draft. Closing or switching backends discards an unsaved preview.'}</p>
                {preview.updateNotes.length > 0 && <div className="provider-status-note" role="status"><strong>Generation notes</strong><ul>{preview.updateNotes.map((message, index) => <li key={index}>{message}</li>)}</ul></div>}
                {preview.validation.issues.length > 0 && <div className="provider-error-text" role="alert"><strong>Evidence validation issues</strong><ul>{preview.validation.issues.map((issue, index) => <li key={index}>{issue.code}{issue.path ? ` · ${issue.path}` : ''}{issue.sectionId ? ` · ${issue.sectionId}` : ''}</li>)}</ul></div>}
                <div className="generated-chapter-sections">{preview.revision.sections.map((section) => <article className="generated-chapter-section" key={section.sectionId}>
                  {sectionUpdate && <small>{section.sectionId === sectionUpdate.sectionId ? 'Updated section' : 'Preserved section'}</small>}
                  <h5>{section.order + 1}. {section.title}</h5><p>{section.prose}</p>
                  <ul>{section.claims.map((claim) => <li key={claim.claimId}><span className="chapter-claim-category">{claim.category.kind === 'inference' ? claim.category.visibleLabel : claim.category.kind === 'codeBehavior' ? 'Code behavior' : 'Repository context'}</span><p>{claim.prose}</p>{claim.category.kind === 'inference' && <p>Basis: {claim.category.basis}</p>}{claim.anchors.map((anchor) => <details className="chapter-evidence" key={anchor.anchorId}><summary><FileCode2 size={12} /><code>{anchor.path}:{anchor.startLine}–{anchor.endLine}</code></summary><pre><code>{anchor.excerpt}</code></pre><small>Snapshot {anchor.snapshotId} · SHA-256 {anchor.sourceSha256}</small></details>)}</li>)}</ul>
                  <details className="chapter-spoken-preview"><summary>Narration script</summary><p>{section.spokenText}</p></details>
                </article>)}</div>
                <small>{preview.modelVersion ?? preview.modelId} · {preview.inputTokens ?? '—'} input tokens · {preview.outputTokens ?? '—'} output tokens</small>
              </section>}
            </section>}
          </>}
          {error && <p className="provider-error-text" role="alert">{error}</p>}
          {note && <p className="provider-status-note" role="status">{note}</p>}
          <p className="provider-small-note">Choose Change backend for Claude Code or other manual agent handoffs, or a direct API. Switching discards an unsaved preview.</p>
        </div>
        <footer className="provider-dialog-footer">
          <div className="provider-draft-actions"><button className="secondary-button" onClick={() => { if (!activeBusy.current) onClose(); }} disabled={busy !== null}>{saved || codeLensResult ? 'Done' : preview ? 'Close and discard preview' : 'Close'}</button><button className="secondary-button" onClick={() => { if (!activeBusy.current && !resources.current.login) onChooseBackend(); }} disabled={busy !== null || Boolean(login)}>Change backend{preview && !saved ? ' and discard preview' : ''}</button></div>
          {native && mode !== 'settings' && (busy === 'send' ? <button className="secondary-button" onClick={() => { void cancelRun(); }} disabled={cancelling}>{cancelling ? 'Cancelling…' : 'Cancel generation'}</button>
            : codeLensResult ? <button className="secondary-button" disabled={busy !== null} onClick={() => { setCodeLensResult(null); setNote('Temporary explanation discarded. Review a new request to generate again.'); }}>Discard explanation</button>
            : preview ? saved ? <span className="provider-saved-status" role="status"><Check size={15} /> Draft saved locally</span> : <div className="provider-draft-actions"><button className="secondary-button" onClick={() => { void discard(); }} disabled={busy !== null}>Discard preview</button><button className="primary-button" onClick={() => { void save(); }} disabled={busy !== null || preview.validation.status !== 'valid'}>{busy === 'save' ? 'Saving…' : 'Save unaccepted draft'}</button></div>
              : <button className="primary-button" onClick={() => { void generate(); }} disabled={busy !== null || otherRun || !review || !consent || !status?.authenticated || !modelReady || Boolean(login)}><Sparkles size={15} /> {sectionUpdate ? 'Regenerate section with Codex' : 'Generate with Codex'}</button>)}
        </footer>
      </section>
    </div>
  );
}
