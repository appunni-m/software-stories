import { readDirectApiSelection, readGenerationBackend } from './lib/generationPreferences';
import type { AgentGitRecovery } from './lib/agentRecovery';
import { useEffect, useMemo, useRef, useState } from 'react';
import { AlertTriangle, BookOpenCheck, Check, CircleDollarSign, FileDiff, RotateCcw, Send, ShieldCheck, Sparkles } from 'lucide-react';
import { CodexChapterDialog, type CodexChapterDialogMode } from './CodexChapterDialog';
import { ClaudeConnectionDialog } from './ClaudeConnectionDialog';
import type { ClaudeOperationReviewMode } from './lib/claudeReview';
import { PrGuideOperationFence, type PrGuideOperationToken } from './lib/prGuideLifecycle';
import {
  acceptPrGuideDraft,
  buildProviderPrChangeExplanationRequestReview,
  buildProviderPrGuideRequestReview,
  cancelProviderRequest,
  discardProviderRequestReview,
  getProviderCredentialStatus,
  getProviderCatalog,
  isNativeRuntime,
  listPrGuideRevisions,
  rejectPrGuideDraft,
  sendProviderPrChangeExplanationRequest,
  sendProviderPrGuideRequest,
  type GitComparisonResult,
  type GitFileChange,
  type PrGuideRevision,
  type PrChangeExplanationTarget,
  type PrChangeExplanationPreview,
  type ProviderCommandResult,
  type ProviderConsent,
  type ProviderCredentialStatus,
  type ProviderDisclosure,
  type ProviderFailure,
  type ProviderRequestReview,
  type SelectedGitChangeIdentity,
} from './lib/commands';

const NO_CONSENT: ProviderConsent = {
  privacyAndDataUseAcknowledged: false,
  billingUncertaintyAcknowledged: false,
  adultSupportedRegionAcknowledged: false,
};

function readPrGuideBackend(): 'codex' | 'claude-agent' | 'api' | 'handoff' {
  const saved = readGenerationBackend();
  return saved === 'api' ? 'api' : saved === 'claude-setup' ? 'claude-agent' : saved;
}

interface PrGuideStudioProps {
  accountOnly?: boolean;
  projectId: string;
  baseRef: string;
  headRef: string;
  comparison: GitComparisonResult;
  onFocusChange: (path: string | undefined, hunkIndex?: number) => void;
  onOpenProviderSettings: () => void;
  recovery?: AgentGitRecovery | null;
  onRecoveryReviewReady?: (operationId: string) => void;
}

export function PrGuideStudio({
  accountOnly = false,
  projectId,
  baseRef,
  headRef,
  comparison,
  onFocusChange,
  onOpenProviderSettings,
  recovery,
  onRecoveryReviewReady,
}: PrGuideStudioProps) {
  const operationScope = {
    projectId,
    baseRef,
    headRef,
    comparisonId: comparison.comparisonId ?? '',
  };
  const operationScopeRef = useRef(operationScope);
  operationScopeRef.current = operationScope;
  const operationFenceRef = useRef<PrGuideOperationFence | null>(null);
  if (operationFenceRef.current === null) operationFenceRef.current = new PrGuideOperationFence(operationScope);
  operationFenceRef.current.setScope(operationScope);
  const sendOperationRef = useRef<PrGuideOperationToken | null>(null);
  const cancelOperationRef = useRef<PrGuideOperationToken | null>(null);
  const [backend, setBackend] = useState<'codex' | 'claude-agent' | 'api' | 'handoff'>(() => readPrGuideBackend());
  const [codexMode, setCodexMode] = useState<CodexChapterDialogMode | null>(null);
  const [claudeMode, setClaudeMode] = useState<ClaudeOperationReviewMode | 'settings' | null>(null);
  const [recoveryHistoryChecked, setRecoveryHistoryChecked] = useState(false);
  const [providerCatalog, setProviderCatalog] = useState<ProviderDisclosure[]>([]);
  const [providerId, setProviderId] = useState(() => readDirectApiSelection().providerId);
  const [disclosure, setDisclosure] = useState<ProviderDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCommandResult<ProviderCredentialStatus> | null>(null);
  const [informationLoading, setInformationLoading] = useState(isNativeRuntime());
  const [informationError, setInformationError] = useState<string | null>(null);
  const [modelId, setModelId] = useState(() => readDirectApiSelection().modelId);
  const [selectedKeys, setSelectedKeys] = useState<string[]>(()=>accountOnly?comparison.changes.filter(isGuideEligible).slice(0,12).map(changeKey):[]);
  const [requestReview, setRequestReview] = useState<ProviderRequestReview | null>(null);
  const requestReviewRef = useRef<ProviderRequestReview | null>(null);
  const reviewLifecycle = useRef(0);
  const [consent, setConsent] = useState<ProviderConsent>(NO_CONSENT);
  const [building, setBuilding] = useState(false);
  const [sending, setSending] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const cancelRequested = useRef(false);
  const [requestError, setRequestError] = useState<ProviderFailure | null>(null);
  const [cancelStatus, setCancelStatus] = useState<string | null>(null);
  const [history, setHistory] = useState<PrGuideRevision[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const historyLookup = useRef(0);
  const historyActive = useRef(false);
  const [currentDraft, setCurrentDraft] = useState<PrGuideRevision | null>(null);
  const [includedClaimIds, setIncludedClaimIds] = useState<string[]>([]);
  const [decisionBusy, setDecisionBusy] = useState(false);
  const [decisionError, setDecisionError] = useState<string | null>(null);
  const [rejectionReason, setRejectionReason] = useState('A claim needs more evidence.');
  const [notice, setNotice] = useState<string | null>(null);
  const [changeExplanationSelection, setChangeExplanationSelection] = useState<{ change: GitFileChange; hunkIndex: number } | null>(null);

  const eligibleChanges = useMemo(() => comparison.changes.filter(isGuideEligible), [comparison.changes]);
  const unavailableChanges = useMemo(() => comparison.changes.filter((change) => !isGuideEligible(change)), [comparison.changes]);
  const selectedChanges = useMemo(
    () => eligibleChanges.filter((change) => selectedKeys.includes(changeKey(change))),
    [eligibleChanges, selectedKeys],
  );
  const recoveryRefs = !recovery ? null : recovery.mode.kind === 'pr-guide' ? recovery.mode : recovery.mode.target;
  const matchingRecovery = recovery && recovery.mode.projectId === projectId
    && recoveryRefs && recoveryRefs.comparisonId === comparison.comparisonId && recoveryRefs.baseRef === baseRef && recoveryRefs.headRef === headRef ? recovery : null;

  useEffect(() => {
    const refreshBackend = () => {
      abandonRequestReview();
      setBackend(readPrGuideBackend());
    };
    const refreshDirectApiSelection = () => {
      abandonRequestReview();
      const saved = readDirectApiSelection();
      setProviderId(saved.providerId);
      setModelId(saved.modelId);
    };
    window.addEventListener('code-atlas:connection-changed', refreshBackend);
    window.addEventListener('code-atlas:direct-api-selection-changed', refreshDirectApiSelection);
    return () => {
      window.removeEventListener('code-atlas:connection-changed', refreshBackend);
      window.removeEventListener('code-atlas:direct-api-selection-changed', refreshDirectApiSelection);
    };
  }, []);

  useEffect(() => { setRecoveryHistoryChecked(false); }, [recovery?.operationId, history.length]);
  const credentialInfo = credential?.status === 'ok' ? credential.data : null;
  const hasApiKey = credentialInfo?.apiKeyConfigured ?? false;
  const selectedModel = disclosure?.models.find((model) => model.modelId === modelId) ?? null;
  const hasApiSelection = Boolean(providerId && selectedModel && disclosure);
  const canBuild = Boolean(
    isNativeRuntime()
      && comparison.status === 'ready'
      && comparison.comparisonId
      && backend === 'api'
      && selectedChanges.length > 0
      && selectedChanges.length <= 12
      && hasApiSelection
      && modelId
      && !requestReview
      && !changeExplanationSelection
      && !building
      && !sending
      && !decisionBusy,
  );

  useEffect(() => {
    let active = true;
    if (!isNativeRuntime() || backend !== 'api') {
      setInformationLoading(false);
      return () => { active = false; };
    }
    setInformationLoading(true);
    void getProviderCatalog()
      .then((catalog) => {
        if (!active) return;
        setProviderCatalog(catalog);
      })
      .catch(() => { if (active) setInformationError('Code Atlas could not load direct API settings. No model request was sent.'); })
      .finally(() => { if (active) setInformationLoading(false); });
    return () => { active = false; };
  }, [backend]);

  useEffect(() => {
    if (!providerId || backend !== 'api') {
      setDisclosure(null);
      setCredential(null);
      return;
    }
    const selected = providerCatalog.find((provider) => provider.providerId === providerId) ?? null;
    setDisclosure(selected);
    setCredential(null);
    if (!selected) return;
    let active = true;
    void getProviderCredentialStatus(providerId)
      .then((status) => { if (active) setCredential(status); })
      .catch(() => { if (active) setCredential(null); });
    return () => { active = false; };
  }, [backend, providerCatalog, providerId]);

  useEffect(() => {
    requestReviewRef.current = requestReview;
  }, [requestReview]);

  useEffect(() => {
    let active = true;
    const refreshCredentials = () => {
      if (providerId && backend === 'api') void getProviderCredentialStatus(providerId)
        .then((status) => { if (active) setCredential(status); })
        .catch(() => {});
    };
    window.addEventListener('code-atlas:provider-credentials-changed', refreshCredentials);
    return () => {
      active = false;
      window.removeEventListener('code-atlas:provider-credentials-changed', refreshCredentials);
    };
  }, [backend, providerId]);

  useEffect(() => {
    operationFenceRef.current?.mount(operationScopeRef.current);
    return () => {
      operationFenceRef.current?.unmount();
      sendOperationRef.current = null;
      cancelOperationRef.current = null;
      reviewLifecycle.current += 1;
      const reviewId = requestReviewRef.current?.reviewId;
      // Unsent previews are disposable. A sent request is already bound to this comparison;
      // its validated late result is retained in that comparison's history by the native store.
      if (reviewId) void discardProviderRequestReview(reviewId).catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (comparison.status !== 'ready' || !comparison.comparisonId) {
      historyActive.current = false;
      setHistory([]);
      setHistoryError(null);
      setHistoryLoading(false);
      return;
    }
    let active = true;
    historyActive.current = true;
    const comparisonId = comparison.comparisonId;
    const reload = () => {
      const request = ++historyLookup.current;
      setHistoryLoading(true);
      setHistoryError(null);
      void listPrGuideRevisions(projectId, comparisonId)
        .then((revisions) => { if (active && historyLookup.current === request) setHistory(revisions); })
        .catch(() => { if (active && historyLookup.current === request) setHistoryError('The local PR Guide history could not be loaded.'); })
        .finally(() => { if (active && historyLookup.current === request) setHistoryLoading(false); });
    };
    const changed = (event: Event) => {
      const detail = (event as CustomEvent<{ projectId: string; comparisonId: string }>).detail;
      if (detail?.projectId === projectId && detail.comparisonId === comparisonId) reload();
    };
    reload();
    window.addEventListener('code-atlas:pr-guides-changed', changed);
    return () => { active = false; historyActive.current = false; historyLookup.current += 1; window.removeEventListener('code-atlas:pr-guides-changed', changed); };
  }, [comparison.comparisonId, comparison.status, projectId]);

  function clearReview() {
    abandonRequestReview();
    setCurrentDraft(null);
    setIncludedClaimIds([]);
    setNotice(null);
  }

  function abandonRequestReview() {
    reviewLifecycle.current += 1;
    setBuilding(false);
    if (requestReviewRef.current) void discardProviderRequestReview(requestReviewRef.current.reviewId).catch(() => {});
    requestReviewRef.current = null;
    setRequestReview(null);
    setConsent(NO_CONSENT);
    setRequestError(null);
    setCancelStatus(null);
  }

  function toggleChange(change: GitFileChange) {
    const key = changeKey(change);
    clearReview();
    setSelectedKeys((current) => {
      if (current.includes(key)) return current.filter((item) => item !== key);
      if (current.length >= 12) return current;
      return [...current, key];
    });
  }

  function reviewCodexGuide() {
    if (building || sending || decisionBusy || !comparison.comparisonId || !selectedChanges.length || selectedChanges.length > 12) return;
    clearReview();
    setCodexMode({ kind: 'pr-guide', projectId, baseRef, headRef, comparisonId: comparison.comparisonId,
      selectedChanges: selectedChanges.map(({ basePath, headPath }) => ({ basePath, headPath })) });
  }

  function previewClaudeGuide() {
    if (building || sending || decisionBusy || !comparison.comparisonId || !selectedChanges.length || selectedChanges.length > 12) return;
    clearReview();
    setClaudeMode({ kind: 'pr-guide', projectId, baseRef, headRef, comparisonId: comparison.comparisonId,
      selectedChanges: selectedChanges.map(({ basePath, headPath }) => ({ basePath, headPath })) });
  }

  function explainHunk(change: GitFileChange, hunkIndex: number) {
    if ((backend === 'codex' || backend === 'claude-agent') && comparison.comparisonId) {
      clearReview();
      const mode = { kind: 'pr-change' as const, projectId, target: { baseRef, headRef,
        comparisonId: comparison.comparisonId, selectedChange: { basePath: change.basePath, headPath: change.headPath }, hunkIndex } };
      if (backend === 'codex') setCodexMode(mode);
      else setClaudeMode(mode);
    } else if (backend === 'api') setChangeExplanationSelection({ change, hunkIndex });
  }

  async function handleBuildReview() {
    if (!canBuild || !comparison.comparisonId) return;
    clearReview();
    const lifecycle = reviewLifecycle.current;
    setBuilding(true);
    setRequestError(null);
    setNotice(null);
    try {
      const result = await buildProviderPrGuideRequestReview(providerId, {
        projectId,
        baseRef,
        headRef,
        comparisonId: comparison.comparisonId,
        selectedChanges: selectedChanges.map(({ basePath, headPath }) => ({ basePath, headPath })),
        modelId,
      });
      if (lifecycle !== reviewLifecycle.current) {
        if (result.status === 'ok') void discardProviderRequestReview(result.data.reviewId).catch(() => {});
        return;
      }
      if (result.status === 'error') {
        setRequestError(result.error);
        return;
      }
      requestReviewRef.current = result.data;
      setRequestReview(result.data);
      setConsent(NO_CONSENT);
    } catch {
      if (lifecycle === reviewLifecycle.current) setRequestError({ kind: 'invalidReview', message: 'The local PR Guide request preview could not be built. No model request was sent.' });
    } finally {
      if (lifecycle === reviewLifecycle.current) setBuilding(false);
    }
  }

  async function handleSend() {
    if (!requestReview || !hasApiKey || sending || !completeConsent(consent)) return;
    const operation = operationFenceRef.current?.begin('send') ?? null;
    if (!operation) return;
    const sendReview = requestReview;
    sendOperationRef.current = operation;
    setSending(true);
    setRequestError(null);
    setCancelStatus(null);
    cancelRequested.current = false;
    try {
      const result = await sendProviderPrGuideRequest({
        reviewId: sendReview.reviewId,
        fingerprint: sendReview.fingerprint,
        consent,
      });
      if (!operationFenceRef.current?.isCurrent(operation)) return;
      requestReviewRef.current = null;
      setRequestReview(null);
      setConsent(NO_CONSENT);
      if (result.status === 'error') {
        setRequestError(result.error);
        return;
      }
      setCurrentDraft(result.data.revision);
      setIncludedClaimIds(allClaimIds(result.data.revision));
      setNotice(cancelRequested.current
        ? 'Cancellation arrived after the response. The validated guide is saved as an unaccepted draft; accepted guide history was not changed.'
        : 'The guide passed local citation validation and is saved as an unaccepted draft. Review its claims before acceptance.');
      await refreshHistory();
    } catch {
      if (!operationFenceRef.current?.isCurrent(operation)) return;
      requestReviewRef.current = null;
      setRequestReview(null);
      setConsent(NO_CONSENT);
      setRequestError({ kind: 'networkUnavailable', message: 'The result could not be confirmed. Check this comparison’s PR Guide history before sending a new reviewed request.' });
    } finally {
      if (operationFenceRef.current?.isCurrent(operation)) {
        operationFenceRef.current.finish(operation);
        sendOperationRef.current = null;
        if (cancelOperationRef.current) {
          operationFenceRef.current.invalidate('cancel');
          cancelOperationRef.current = null;
        }
        setSending(false);
        setCancelBusy(false);
      }
    }
  }

  async function handleCancel() {
    if (!requestReview || !sending || cancelBusy) return;
    const sendOperation = sendOperationRef.current;
    if (!sendOperation || !operationFenceRef.current?.isCurrent(sendOperation)) return;
    const operation = operationFenceRef.current.begin('cancel');
    if (!operation) return;
    cancelOperationRef.current = operation;
    const reviewId = requestReview.reviewId;
    const providerName = disclosure?.displayName ?? 'The provider';
    setCancelBusy(true);
    cancelRequested.current = true;
    try {
      const stopped = await cancelProviderRequest(reviewId);
      if (operationFenceRef.current?.isCurrent(operation) && operationFenceRef.current.isCurrent(sendOperation)) {
        setCancelStatus(stopped
          ? `Cancellation reached the local request. ${providerName} may already have received the selected diff; any response remains an unaccepted draft.`
          : 'The request had already finished locally. Any returned guide remains an unaccepted draft until you review it.');
      }
    } catch {
      if (operationFenceRef.current?.isCurrent(operation) && operationFenceRef.current.isCurrent(sendOperation)) {
        setCancelStatus(`Code Atlas could not confirm a local abort. ${providerName} may already have received the selected diff; check the draft history after the request finishes.`);
      }
    } finally {
      if (operationFenceRef.current?.isCurrent(operation)) {
        operationFenceRef.current.finish(operation);
        cancelOperationRef.current = null;
        setCancelBusy(false);
      }
    }
  }

  async function refreshHistory() {
    if (!comparison.comparisonId || !historyActive.current) return;
    const request = ++historyLookup.current;
    setHistoryLoading(true);
    setHistoryError(null);
    try {
      const revisions = await listPrGuideRevisions(projectId, comparison.comparisonId);
      if (historyActive.current && historyLookup.current === request) setHistory(revisions);
    } catch {
      if (historyActive.current && historyLookup.current === request) setHistoryError('The local PR Guide history could not be refreshed.');
    } finally {
      if (historyActive.current && historyLookup.current === request) setHistoryLoading(false);
    }
  }

  async function handleAccept() {
    if (!currentDraft || !comparison.comparisonId || includedClaimIds.length === 0 || decisionBusy) return;
    const operation = operationFenceRef.current?.begin('decision') ?? null;
    if (!operation) return;
    const revisionId = currentDraft.revisionId;
    const comparisonId = comparison.comparisonId;
    setDecisionBusy(true);
    setDecisionError(null);
    try {
      const accepted = await acceptPrGuideDraft({
        projectId,
        comparisonId,
        revisionId,
        includedClaimIds,
      });
      if (!operationFenceRef.current?.isCurrent(operation)) return;
      setCurrentDraft(null);
      setIncludedClaimIds([]);
      setNotice(`Accepted PR Guide revision ${accepted.revisionNumber}. Its history is separate from repository file chapters.`);
      await refreshHistory();
    } catch {
      if (operationFenceRef.current?.isCurrent(operation)) setDecisionError('The selected guide could not be accepted. The saved draft and existing guide history were preserved.');
    } finally {
      if (operationFenceRef.current?.isCurrent(operation)) {
        operationFenceRef.current.finish(operation);
        setDecisionBusy(false);
      }
    }
  }

  async function handleReject() {
    if (!currentDraft || !comparison.comparisonId || decisionBusy) return;
    const operation = operationFenceRef.current?.begin('decision') ?? null;
    if (!operation) return;
    const revisionId = currentDraft.revisionId;
    const comparisonId = comparison.comparisonId;
    setDecisionBusy(true);
    setDecisionError(null);
    try {
      const rejected = await rejectPrGuideDraft({
        projectId,
        comparisonId,
        revisionId,
        reason: rejectionReason,
      });
      if (!operationFenceRef.current?.isCurrent(operation)) return;
      setCurrentDraft(null);
      setIncludedClaimIds([]);
      setNotice(`Rejected PR Guide draft ${rejected.revisionNumber}. Existing accepted guides and file chapters were not changed.`);
      await refreshHistory();
    } catch {
      if (operationFenceRef.current?.isCurrent(operation)) setDecisionError('The draft could not be rejected. Existing local PR Guide history was preserved.');
    } finally {
      if (operationFenceRef.current?.isCurrent(operation)) {
        operationFenceRef.current.finish(operation);
        setDecisionBusy(false);
      }
    }
  }

  function openDraft(revision: PrGuideRevision) {
    if (decisionBusy || building || sending || operationFenceRef.current?.isActive('decision')) return;
    setCurrentDraft(revision);
    setIncludedClaimIds(allClaimIds(revision));
    setDecisionError(null);
    setNotice(null);
  }

  function toggleClaim(claimId: string, checked: boolean) {
    setIncludedClaimIds((current) => checked
      ? current.includes(claimId) ? current : [...current, claimId]
      : current.filter((id) => id !== claimId));
  }

  if (comparison.status !== 'ready' || !comparison.comparisonId) {
    return (
      <section className="pr-guide-panel" aria-labelledby="pr-guide-title">
        <div className="git-changes-section-heading"><div><span className="git-changes-kicker">PR GUIDE</span><h2 id="pr-guide-title">Explain a complete local comparison</h2></div></div>
        <p className="pr-guide-muted">A PR Guide request is available after both refs resolve and every included diff is within the local evidence limits. Partial comparisons cannot produce a guide.</p>
      </section>
    );
  }

  return (
    <section className="pr-guide-panel" aria-labelledby="pr-guide-title">
      <div className="git-changes-section-heading">
        <div><span className="git-changes-kicker">SEPARATE, REVIEWABLE HISTORY</span><h2 id="pr-guide-title">Build a PR Guide</h2></div>
        <span className="git-readonly-badge"><BookOpenCheck size={14} /> Local draft</span>
      </div>
      <p className="pr-guide-muted">{accountOnly?'Explain this commit using the connection in the header. Files with complete, bounded source changes are included automatically; review the diff before sharing it.':'Select changed files, inspect the exact diff sent, then review every claim against its base/head hunk. Accepting or rejecting saves a PR Guide revision; it never changes Git refs or file chapter history.'}</p>
      {backend === 'handoff' && <p className="pr-guide-notice" role="status">Manual agent handoff is not available for PR Guides yet. Change the AI connection in the header to use a supported option.</p>}
      {matchingRecovery && <section className="provider-review-card" aria-labelledby="recovered-git-title">
        <h3 id="recovered-git-title">Review interrupted {matchingRecovery.backend === 'codex' ? 'Codex' : 'Claude'} work</h3>
        <p>{matchingRecovery.mode.kind === 'pr-guide'
          ? `${matchingRecovery.mode.selectedChanges.length} original changed files are preserved.`
          : `The original file and hunk ${matchingRecovery.mode.target.hunkIndex + 1} are preserved.`} Local validation checked the comparison and patch hashes. Choose current model settings, review the exact request and confirm again. Nothing resumes automatically.</p>
        {matchingRecovery.mode.kind === 'pr-guide' && <>
          <p>A response may already have saved a draft. Review this comparison’s PR Guide history below before requesting another generation.</p>
          {historyError && <button className="secondary-button" onClick={() => { void refreshHistory(); }}>Reload saved guides</button>}
          <label className="provider-checkbox"><input type="checkbox" checked={recoveryHistoryChecked} disabled={historyLoading || Boolean(historyError)} onChange={(event) => setRecoveryHistoryChecked(event.target.checked)} />I checked the saved guides and want a new request review.</label>
        </>}
        <button className="primary-button" disabled={building || sending || decisionBusy || Boolean(codexMode) || Boolean(changeExplanationSelection) || Boolean(claudeMode)
          || (matchingRecovery.mode.kind === 'pr-guide' && (!recoveryHistoryChecked || historyLoading || Boolean(historyError)))}
          onClick={() => {
            clearReview(); setBackend(matchingRecovery.backend);
            if (matchingRecovery.mode.kind === 'pr-guide') setSelectedKeys(matchingRecovery.mode.selectedChanges.map(changeKey));
            if (matchingRecovery.backend === 'codex') setCodexMode(matchingRecovery.mode);
            else setClaudeMode(matchingRecovery.mode);
          }}>Review original scope with {matchingRecovery.backend === 'codex' ? 'Codex' : 'Claude'}</button>
      </section>}
      {!accountOnly && backend === 'codex' && <p className="pr-guide-muted">Use your Codex account without an API key. Choose model and reasoning, review the exact patches, then generate. Plan quota applies.</p>}
      {backend === 'claude-agent' && <div className="pr-guide-notice" role="status"><div><p>Preview a guide or hunk request with Claude’s supported settings and the exact local diff. Automatic Claude generation remains unfinished; this review sends no source to an agent.</p>
        <button className="secondary-button" onClick={() => setClaudeMode('settings')}>Claude installation and defaults</button></div></div>}
      <details className="pr-guide-included-files" open={accountOnly ? eligibleChanges.length === 0 : true}><summary>{selectedChanges.length} changed {selectedChanges.length===1?'file':'files'} included{eligibleChanges.length>12?' · selection limited to 12':''}</summary><fieldset className="pr-guide-file-picker" disabled={building || sending || decisionBusy || Boolean(changeExplanationSelection) || Boolean(codexMode) || Boolean(claudeMode)}>
        <legend>Select changed files · {selectedChanges.length} of 12</legend>
        {eligibleChanges.length === 0 ? <p>No changed file has a complete, bounded source diff for a PR Guide.</p> : eligibleChanges.map((change) => {
          const key = changeKey(change);
          const checked = selectedKeys.includes(key);
          const pathLabel = labelChange(change);
          return (
            <div className="pr-guide-change-option" key={key}>
              <label className={checked ? 'selected' : ''}>
                <input type="checkbox" checked={checked} disabled={!checked && selectedKeys.length >= 12} onChange={() => toggleChange(change)} />
                <span><strong>{change.kind}</strong><code>{pathLabel}</code></span>
                <small>{change.hunks.length} {change.hunks.length === 1 ? 'hunk' : 'hunks'}</small>
              </label>
              <div className="pr-guide-hunk-actions" role="group" aria-label={`Explain a hunk in ${pathLabel}`}>
                {change.hunks.map((_, hunkIndex) => (
                  <button
                    type="button"
                    className="secondary-button"
                    key={hunkIndex}
                    disabled={backend === 'handoff' || (backend === 'api' && !hasApiSelection) || building || sending || decisionBusy || Boolean(requestReview)}
                    onClick={() => explainHunk(change, hunkIndex)}
                  ><Sparkles size={13} /> {backend === 'claude-agent' ? 'Preview hunk request' : 'Explain hunk'} {hunkIndex + 1}</button>
                ))}
              </div>
            </div>
          );
        })}
      </fieldset></details>
      {eligibleChanges.length === 0 && <p className="pr-guide-muted" role="status">No changed files have a complete, bounded source diff for a guide. Review the exclusion reasons below, then open Diff to inspect each change.</p>}
      {unavailableChanges.length > 0 && <details className="pr-guide-included-files pr-guide-unavailable-files" open={eligibleChanges.length === 0}>
        <summary>{unavailableChanges.length} changed {unavailableChanges.length === 1 ? 'file' : 'files'} available only in Diff</summary>
        <div className="pr-guide-unavailable-list">
          {unavailableChanges.map((change) => <div className="pr-guide-unavailable-item" key={changeKey(change)}>
            <code>{labelChange(change)}</code>
            <small>{guideExclusionReason(change)}</small>
          </div>)}
        </div>
      </details>}
      {backend === 'codex' && <div className="pr-guide-actions"><button className="primary-button" onClick={reviewCodexGuide} disabled={!selectedChanges.length || selectedChanges.length > 12 || building || sending || decisionBusy || Boolean(codexMode)}><FileDiff size={15} /> {accountOnly?'Explain changes':'Review with Codex'}</button></div>}
      {backend === 'claude-agent' && <div className="pr-guide-actions"><button className="secondary-button" onClick={previewClaudeGuide} disabled={!selectedChanges.length || selectedChanges.length > 12 || building || sending || decisionBusy || Boolean(claudeMode)}><FileDiff size={15} /> Preview guide request with Claude</button></div>}
      {backend === 'api' && <><div className="pr-guide-actions">
        <p className="pr-guide-muted" role="status">
          {informationLoading ? 'Loading Direct API connection…'
            : informationError ? 'The Direct API connection could not be loaded. See the details below.'
              : selectedModel ? `Using ${disclosure?.displayName} · ${selectedModel.displayName}${hasApiKey ? ' · API key saved in Keychain' : ' · API key not set up'}`
                : 'No provider and model saved. Choose them once in Connections in the header.'}
        </p>
        <button className="secondary-button" type="button" onClick={onOpenProviderSettings} disabled={building || sending || decisionBusy || Boolean(requestReview) || Boolean(changeExplanationSelection)}>
          {hasApiSelection ? 'Change API connection' : 'Set up API connection'}
        </button>
        <button className="primary-button" type="button" onClick={() => { void handleBuildReview(); }} disabled={!canBuild}>
          <FileDiff size={15} /> {building ? 'Building exact review…' : requestReview ? 'Request ready for review below' : 'Review exact provider request'}
        </button>
      </div>
      {!hasApiKey && hasApiSelection && <p className="pr-guide-muted" role="status">Add this provider’s API key in Connections before sending. Keys stay in macOS Keychain.</p>}
      {providerId && providerCatalog.length > 0 && !providerCatalog.some((provider) => provider.providerId === providerId) && <p className="pr-guide-muted" role="status">The saved provider is no longer available. Choose another provider in Connections.</p>}
      {modelId && disclosure && !selectedModel && <p className="pr-guide-muted" role="status">The saved model is no longer available for this provider. Choose another model in Connections.</p>}
      {informationError && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {informationError}</p>}
      {credential?.status === 'error' && <p className="git-error-note" role="alert">{credential.error.message}</p>}
      </>}
      {!isNativeRuntime() && <p className="git-preview-note" role="status">The browser preview can inspect the UI but cannot run coding agents, call provider APIs or read local Git. Generation runs only in the packaged Mac app.</p>}
      {requestError && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {requestError.message}</p>}
      {notice && <p className="pr-guide-notice" role="status"><Check size={15} /> {notice}</p>}

      {codexMode && <CodexChapterDialog mode={codexMode} native={isNativeRuntime()}
        onClose={() => setCodexMode(null)} onChooseBackend={() => setCodexMode(null)}
        onRecoveryReviewReady={onRecoveryReviewReady}
        onPrGuideReady={(preview) => {
          setCurrentDraft(preview.revision);
          setIncludedClaimIds(allClaimIds(preview.revision));
          setDecisionError(null);
          setNotice('Codex saved a validated, unaccepted PR Guide draft. Review each claim below before acceptance.');
        }}
        onOpenPrChangeCitation={(citation) => {
          if (codexMode !== 'settings' && codexMode.kind === 'pr-change') {
            onFocusChange(changeKey(codexMode.target.selectedChange), citation.hunkIndex);
            setCodexMode(null);
          }
        }} />}
      {claudeMode && <ClaudeConnectionDialog native={isNativeRuntime()} reviewMode={claudeMode}
        onClose={() => setClaudeMode(null)} onChooseBackend={() => setClaudeMode(null)}
        onRecoveryResultReady={onRecoveryReviewReady}
        onPrGuideReady={(preview) => {
          setCurrentDraft(preview.revision);
          setIncludedClaimIds(allClaimIds(preview.revision));
          setDecisionError(null);
          setNotice('Claude saved a validated, unaccepted PR Guide draft. Review each claim below before acceptance.');
        }}
        onOpenPrChangeCitation={(citation) => {
          if (claudeMode !== 'settings' && claudeMode.kind === 'pr-change') {
            onFocusChange(changeKey(claudeMode.target.selectedChange), citation.hunkIndex);
            setClaudeMode(null);
          }
        }} />}

      {changeExplanationSelection && disclosure && modelId && (
        <PrChangeExplanationPanel
          projectId={projectId}
          baseRef={baseRef}
          headRef={headRef}
          comparisonId={comparison.comparisonId}
          change={changeExplanationSelection.change}
          hunkIndex={changeExplanationSelection.hunkIndex}
          providerId={providerId}
          modelId={modelId}
          disclosure={disclosure}
          hasApiKey={hasApiKey}
          onFocusChange={onFocusChange}
          onOpenProviderSettings={onOpenProviderSettings}
          onClose={() => setChangeExplanationSelection(null)}
        />
      )}

      {requestReview && disclosure && (
        <section className="pr-guide-request-review" aria-labelledby="pr-guide-request-title">
          <div className="pr-guide-subheading"><div><span className="git-changes-kicker">NO REQUEST SENT YET</span><h3 id="pr-guide-request-title">Review what {disclosure.displayName} will receive</h3></div><ShieldCheck size={18} /></div>
          <div className="provider-request-summary">
            <div><span>Provider and model</span><strong>{requestReview.providerId} · {requestReview.modelId}</strong></div>
            <div><span>Endpoint</span><code>{requestReview.endpointUrl}</code></div>
            <div><span>Purpose</span><strong>PR Guide for {requestReview.prGuideContext?.baseRef} → {requestReview.prGuideContext?.headRef}</strong></div>
            <div><span>Selected evidence</span><strong>{requestReview.selectedFiles.length} changed files · {formatBytes(requestReview.sourceBytes)}</strong></div>
            <div><span>Input estimate</span><strong>About {requestReview.estimatedInputTokens.toLocaleString()} tokens · not a bill</strong></div>
            <div><span>Request body</span><strong>{formatBytes(requestReview.requestBodyBytes)}</strong></div>
          </div>
          <p className="provider-warning-note"><AlertTriangle size={14} /> Review the diff for credentials and confidential or personal data. Local scans cannot find every secret.</p>
          {requestReview.selectedFiles.map((file) => (
            <details className="provider-source-file" open key={`${file.path}:${file.sha256}`}>
              <summary><code>{file.path}</code><span>{formatBytes(file.sizeBytes)} · SHA-256 {file.sha256.slice(0, 16)}…</span></summary>
              <pre><code>{file.contents}</code></pre>
            </details>
          ))}
          <details className="provider-prompt-preview">
            <summary>Inspect exact prompt and encoded provider request</summary>
            <div><strong>System instruction</strong><pre>{requestReview.systemInstruction}</pre><strong>User prompt</strong><pre>{requestReview.userPrompt}</pre><strong>Generation settings</strong><pre>{JSON.stringify(requestReview.generationConfig, null, 2)}</pre><strong>Exact JSON request body · SHA-256 {requestReview.requestBodySha256}</strong><pre>{JSON.stringify(requestReview.requestBodyPreview, null, 2)}</pre></div>
          </details>
          <div className="provider-fingerprint"><span>Consent is bound to this request fingerprint</span><code>{requestReview.fingerprint}</code><small>It includes the comparison identity, provider, endpoint, resolved base/head commits, selected diff hashes, exact encoded request body, prompt, model, settings, and disclosure version.</small></div>
          <div className="pr-guide-disclosures">
            <h4><CircleDollarSign size={15} /> Provider terms and possible charges</h4>
            {selectedModel && <p><strong>{selectedModel.displayName} pricing checked {disclosure.checkedDate}:</strong> {selectedModel.freeTierStatus} Paid input: {selectedModel.paidInputUsdPerMillionTokens} per million tokens. Paid output: {selectedModel.paidOutputUsdPerMillionTokens} per million tokens{selectedModel.paidPriceChangeDate ? `; price changes ${selectedModel.paidPriceChangeDate}` : ''}.</p>}
            <p>{disclosure.dataUseTerms}</p><p>{disclosure.regionRequirements}</p><p>{disclosure.eligibilityRequirements}</p><p>{disclosure.billingUncertainty}</p>
            <p>{disclosure.displayName} receives the reviewed diff if you send. One request is made; failures do not switch models or providers.</p>
            <p><a href={disclosure.termsUrl} target="_blank" rel="noreferrer">Provider API terms</a> · <a href={disclosure.pricingUrl} target="_blank" rel="noreferrer">Pricing</a> · <a href={disclosure.regionsUrl} target="_blank" rel="noreferrer">Availability and regions</a></p>
          </div>
          <div className="pr-guide-consent">
            <label><input type="checkbox" checked={consent.privacyAndDataUseAcknowledged} disabled={sending} onChange={(event) => setConsent((value) => ({ ...value, privacyAndDataUseAcknowledged: event.target.checked }))} /> I reviewed the selected diff and understand this provider’s data-use terms.</label>
            <label><input type="checkbox" checked={consent.billingUncertaintyAcknowledged} disabled={sending} onChange={(event) => setConsent((value) => ({ ...value, billingUncertaintyAcknowledged: event.target.checked }))} /> I understand this model may incur charges or quota limits.</label>
            <label><input type="checkbox" checked={consent.adultSupportedRegionAcknowledged} disabled={sending} onChange={(event) => setConsent((value) => ({ ...value, adultSupportedRegionAcknowledged: event.target.checked }))} /> I meet this provider’s eligibility and supported-region requirements.</label>
          </div>
          {!hasApiKey && <p className="pr-guide-muted">A {disclosure.displayName} API key is not configured. Add it in Settings; this review will remain local.</p>}
          <div className="pr-guide-actions">
            {!hasApiKey && <button type="button" className="secondary-button" onClick={onOpenProviderSettings}>Open provider settings</button>}
            <button type="button" className="secondary-button" onClick={clearReview} disabled={sending}>Discard this review</button>
            <button type="button" className="primary-button" onClick={() => { void handleSend(); }} disabled={!hasApiKey || !completeConsent(consent) || sending}>
              <Send size={15} /> {sending ? 'Sending reviewed request…' : 'Send reviewed request'}
            </button>
            {sending && <button type="button" className="secondary-button" onClick={() => { void handleCancel(); }} disabled={cancelBusy}>{cancelBusy ? 'Cancelling…' : 'Cancel request'}</button>}
          </div>
          {cancelStatus && <p className="pr-guide-muted" role="status">{cancelStatus}</p>}
        </section>
      )}

      {currentDraft && (
        <GuideDraftReview
          revision={currentDraft}
          includedClaimIds={includedClaimIds}
          busy={decisionBusy}
          error={decisionError}
          rejectionReason={rejectionReason}
          onClaimChange={toggleClaim}
          onFocusCitation={(citation) => {
            const change = currentDraft.comparison.selectedChanges[citation.changeIndex];
            if (change) onFocusChange(changeKey(change), citation.hunkIndex);
          }}
          onRejectionReasonChange={setRejectionReason}
          onAccept={() => { void handleAccept(); }}
          onReject={() => { void handleReject(); }}
        />
      )}

      <section className="pr-guide-history" aria-labelledby="pr-guide-history-title">
        <div className="pr-guide-subheading"><div><span className="git-changes-kicker">THIS COMMIT PAIR ONLY</span><h3 id="pr-guide-history-title">PR Guide history</h3></div><button className="icon-button" type="button" onClick={() => { void refreshHistory(); }} aria-label="Refresh PR Guide history"><RotateCcw size={15} /></button></div>
        {historyLoading && <p role="status">Loading local history…</p>}
        {historyError && <p className="git-error-note" role="alert">{historyError}</p>}
        {!historyLoading && history.length === 0 && !historyError && <p className="pr-guide-muted">No PR Guide revisions exist for this comparison yet.</p>}
        {history.length > 0 && <ol className="pr-guide-history-list">{history.map((revision) => {
          const decision = revision.lifecycle === 'draft'
            ? history.find((candidate) => candidate.basedOnRevisionId === revision.revisionId)
            : undefined;
          const statusLabel = decision ? `draft · ${decision.lifecycle}` : revision.lifecycle;
          return (
            <li key={revision.revisionId}>
              <span className={`pr-guide-lifecycle lifecycle-${decision?.lifecycle ?? revision.lifecycle}`}>{statusLabel}</span>
              <div><strong>{revision.title}</strong><small>Revision {revision.revisionNumber} · {countClaims(revision)} claims · {revision.revisionId.slice(-8)}{decision ? ` · decision in revision ${decision.revisionNumber}` : ''}</small></div>
              {revision.lifecycle === 'draft' && !decision && <button type="button" className="secondary-button" disabled={decisionBusy || building || sending} onClick={() => openDraft(revision)}>Review draft</button>}
            </li>
          );
        })}</ol>}
      </section>
    </section>
  );
}

function GuideDraftReview({
  revision,
  includedClaimIds,
  busy,
  error,
  rejectionReason,
  onClaimChange,
  onFocusCitation,
  onRejectionReasonChange,
  onAccept,
  onReject,
}: {
  revision: PrGuideRevision;
  includedClaimIds: string[];
  busy: boolean;
  error: string | null;
  rejectionReason: string;
  onClaimChange: (claimId: string, checked: boolean) => void;
  onFocusCitation: (citation: PrGuideRevision['sections'][number]['claims'][number]['citations'][number]) => void;
  onRejectionReasonChange: (reason: string) => void;
  onAccept: () => void;
  onReject: () => void;
}) {
  const claimCount = countClaims(revision);
  return (
    <section className="pr-guide-draft-review" aria-labelledby="pr-guide-draft-title">
      <div className="pr-guide-subheading"><div><span className="git-changes-kicker">LOCAL CITATION CHECK PASSED · STILL A DRAFT</span><h3 id="pr-guide-draft-title">{revision.title}</h3></div><span className="pr-guide-lifecycle lifecycle-draft">draft</span></div>
      <p className="pr-guide-muted">Keep only the claims you want in the accepted guide. Uncheck an inference you reject; the original generated draft remains in history and acceptance appends a separate revision.</p>
      {revision.sections.map((section) => (
        <section className="pr-guide-section" key={section.sectionId}>
          <h4>{section.title}</h4>
          {section.claims.map((claim) => (
            <article className={`pr-guide-claim${includedClaimIds.includes(claim.claimId) ? '' : ' excluded'}`} key={claim.claimId}>
              <label className="pr-guide-claim-select"><input type="checkbox" checked={includedClaimIds.includes(claim.claimId)} disabled={busy} onChange={(event) => onClaimChange(claim.claimId, event.target.checked)} /><span><strong>{claim.category === 'inference' ? 'Inference' : claim.category === 'repositoryContext' ? 'Repository context' : 'Code change'}</strong> · {claim.aspect}</span></label>
              <p>{claim.prose}</p>
              {claim.category === 'inference' && claim.inferenceBasis && <p className="pr-guide-inference-basis"><strong>Why this is an inference:</strong> {claim.inferenceBasis}</p>}
              <ul className="pr-guide-citations">{claim.citations.map((citation, index) => (
                <li key={`${citation.anchor.revisionId}:${citation.anchor.path}:${citation.anchor.startLine}:${index}`}>
                  <button type="button" className="pr-guide-citation-link" onClick={() => onFocusCitation(citation)}>
                    <span>{citation.side} · {citation.anchor.path}:{citation.anchor.startLine}–{citation.anchor.endLine}</span>
                    <small>commit {citation.anchor.revisionId.slice(0, 12)} · blob {citation.anchor.blobId.slice(0, 12)} · hunk {citation.hunkIndex + 1}</small>
                    <code>{citation.quote}</code>
                  </button>
                </li>
              ))}</ul>
            </article>
          ))}
        </section>
      ))}
      <div className="pr-guide-actions pr-guide-decision-actions">
        <span>{includedClaimIds.length} of {claimCount} claims included</span>
        <button type="button" className="primary-button" disabled={busy || includedClaimIds.length === 0} onClick={onAccept}><Check size={15} /> {busy ? 'Saving decision…' : 'Accept selected claims'}</button>
      </div>
      <label className="pr-guide-rejection-reason">Reason for rejecting this draft<textarea value={rejectionReason} disabled={busy} maxLength={4_000} onChange={(event) => onRejectionReasonChange(event.target.value)} /></label>
      {error && <p className="git-error-note" role="alert">{error}</p>}
      <button type="button" className="secondary-button" disabled={busy || !rejectionReason.trim()} onClick={onReject}>Reject this PR Guide draft</button>
    </section>
  );
}

function PrChangeExplanationPanel({
  projectId,
  baseRef,
  headRef,
  comparisonId,
  change,
  hunkIndex,
  providerId,
  modelId,
  disclosure,
  hasApiKey,
  onFocusChange,
  onOpenProviderSettings,
  onClose,
}: {
  projectId: string;
  baseRef: string;
  headRef: string;
  comparisonId: string;
  change: GitFileChange;
  hunkIndex: number;
  providerId: string;
  modelId: string;
  disclosure: ProviderDisclosure;
  hasApiKey: boolean;
  onFocusChange: (path: string | undefined, hunkIndex?: number) => void;
  onOpenProviderSettings: () => void;
  onClose: () => void;
}) {
  const [requestReview, setRequestReview] = useState<ProviderRequestReview | null>(null);
  const reviewRef = useRef<ProviderRequestReview | null>(null);
  const [consent, setConsent] = useState<ProviderConsent>(NO_CONSENT);
  const [preview, setPreview] = useState<PrChangeExplanationPreview | null>(null);
  const [building, setBuilding] = useState(true);
  const [sending, setSending] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [cancelRequested, setCancelRequested] = useState(false);
  const [error, setError] = useState<ProviderFailure | null>(null);

  useEffect(() => {
    let active = true;
    setBuilding(true);
    setError(null);
    setPreview(null);
    setConsent(NO_CONSENT);
    const target: PrChangeExplanationTarget = {
      baseRef,
      headRef,
      comparisonId,
      selectedChange: { basePath: change.basePath, headPath: change.headPath },
      hunkIndex,
    };
    void buildProviderPrChangeExplanationRequestReview(providerId, { projectId, target, modelId })
      .then((result) => {
        if (!active) {
          if (result.status === 'ok') void discardProviderRequestReview(result.data.reviewId).catch(() => {});
          return;
        }
        if (result.status === 'error') {
          setError(result.error);
          return;
        }
        reviewRef.current = result.data;
        setRequestReview(result.data);
      })
      .catch(() => {
        if (active) setError({ kind: 'invalidReview', message: 'The local hunk review could not be built. No provider request was sent.' });
      })
      .finally(() => { if (active) setBuilding(false); });
    return () => {
      active = false;
      const reviewId = reviewRef.current?.reviewId;
      reviewRef.current = null;
      if (reviewId) void discardProviderRequestReview(reviewId).catch(() => {});
    };
  }, [baseRef, change.basePath, change.headPath, comparisonId, headRef, hunkIndex, modelId, projectId, providerId]);

  async function send() {
    if (!requestReview || !hasApiKey || sending || !completeConsent(consent)) return;
    setSending(true);
    setError(null);
    setCancelRequested(false);
    try {
      const result = await sendProviderPrChangeExplanationRequest({
        reviewId: requestReview.reviewId,
        fingerprint: requestReview.fingerprint,
        consent,
      });
      reviewRef.current = null;
      setRequestReview(null);
      if (result.status === 'error') {
        setError(result.error);
        return;
      }
      setPreview(result.data);
    } catch {
      reviewRef.current = null;
      setRequestReview(null);
      setError({ kind: 'networkUnavailable', message: 'The request result could not be confirmed. Review the hunk again before sending another request.' });
    } finally {
      setSending(false);
      setCancelBusy(false);
    }
  }

  async function cancel() {
    if (!requestReview || cancelBusy) return;
    setCancelBusy(true);
    try {
      const accepted = await cancelProviderRequest(requestReview.reviewId);
      setCancelRequested(accepted);
    } catch {
      setError({ kind: 'networkUnavailable', message: 'The cancellation request could not be confirmed.' });
      setCancelBusy(false);
    }
  }

  const selectedModel = disclosure.models.find((model) => model.modelId === modelId) ?? null;
  return (
    <section className="pr-guide-request-review pr-change-explanation" aria-labelledby="pr-change-explanation-title">
      <div className="pr-guide-subheading">
        <div><span className="git-changes-kicker">TRANSIENT HUNK EXPLANATION</span><h3 id="pr-change-explanation-title">Explain {labelChange(change)} · hunk {hunkIndex + 1}</h3></div>
        <button type="button" className="secondary-button" onClick={onClose} disabled={sending}>Close</button>
      </div>
      <p className="pr-guide-muted">This request explains one selected hunk and returns temporary claims linked to the exact base/head source. It does not create PR Guide history or change Git refs.</p>
      {building && <p className="pr-guide-muted" role="status">Building the exact local hunk review…</p>}
      {error && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {error.message}</p>}
      {requestReview && (
        <>
          <div className="provider-request-summary">
            <div><span>Provider and model</span><strong>{requestReview.providerId} · {requestReview.modelId}</strong></div>
            <div><span>Endpoint</span><code>{requestReview.endpointUrl}</code></div>
            <div><span>Purpose</span><strong>One hunk from {baseRef} → {headRef}</strong></div>
            <div><span>Selected context</span><strong>{requestReview.selectedFiles[0]?.path} · {formatBytes(requestReview.sourceBytes)}</strong></div>
            <div><span>Input estimate</span><strong>About {requestReview.estimatedInputTokens.toLocaleString()} tokens · not a bill</strong></div>
          </div>
          <p className="provider-warning-note"><AlertTriangle size={14} /> Review the exact hunk for credentials and confidential or personal data. Local scans cannot find every secret.</p>
          {requestReview.selectedFiles.map((file) => (
            <details className="provider-source-file" open key={`${file.path}:${file.sha256}`}>
              <summary><code>{file.path}</code><span>{formatBytes(file.sizeBytes)} · SHA-256 {file.sha256.slice(0, 16)}…</span></summary>
              <pre><code>{file.contents}</code></pre>
            </details>
          ))}
          <details className="provider-prompt-preview">
            <summary>Inspect exact prompt and encoded provider request</summary>
            <div><strong>System instruction</strong><pre>{requestReview.systemInstruction}</pre><strong>User prompt</strong><pre>{requestReview.userPrompt}</pre><strong>Generation settings</strong><pre>{JSON.stringify(requestReview.generationConfig, null, 2)}</pre><strong>Exact JSON request body · SHA-256 {requestReview.requestBodySha256}</strong><pre>{JSON.stringify(requestReview.requestBodyPreview, null, 2)}</pre></div>
          </details>
          <div className="provider-fingerprint"><span>Consent is bound to this exact hunk request</span><code>{requestReview.fingerprint}</code><small>It includes the comparison and hunk, provider, model, endpoint, request body, generation settings, and disclosure version.</small></div>
          <div className="pr-guide-disclosures">
            <h4><CircleDollarSign size={15} /> Provider terms and possible charges</h4>
            {selectedModel && <p><strong>{selectedModel.displayName} pricing checked {disclosure.checkedDate}:</strong> {selectedModel.freeTierStatus} Paid input: {selectedModel.paidInputUsdPerMillionTokens} per million tokens. Paid output: {selectedModel.paidOutputUsdPerMillionTokens} per million tokens{selectedModel.paidPriceChangeDate ? `; price changes ${selectedModel.paidPriceChangeDate}` : ''}.</p>}
            <p>{disclosure.dataUseTerms}</p><p>{disclosure.regionRequirements}</p><p>{disclosure.eligibilityRequirements}</p><p>{disclosure.billingUncertainty}</p>
            <p>{disclosure.displayName} receives the reviewed hunk if you send. One request is made; failures do not switch providers or models.</p>
            <p><a href={disclosure.termsUrl} target="_blank" rel="noreferrer">Provider API terms</a> · <a href={disclosure.pricingUrl} target="_blank" rel="noreferrer">Pricing</a> · <a href={disclosure.regionsUrl} target="_blank" rel="noreferrer">Availability and regions</a></p>
          </div>
          <div className="pr-guide-consent">
            <label><input type="checkbox" checked={consent.privacyAndDataUseAcknowledged} disabled={sending} onChange={(event) => setConsent((value) => ({ ...value, privacyAndDataUseAcknowledged: event.target.checked }))} /> I reviewed this hunk and understand the provider’s data-use terms.</label>
            <label><input type="checkbox" checked={consent.billingUncertaintyAcknowledged} disabled={sending} onChange={(event) => setConsent((value) => ({ ...value, billingUncertaintyAcknowledged: event.target.checked }))} /> I understand this model may incur charges or quota limits.</label>
            <label><input type="checkbox" checked={consent.adultSupportedRegionAcknowledged} disabled={sending} onChange={(event) => setConsent((value) => ({ ...value, adultSupportedRegionAcknowledged: event.target.checked }))} /> I meet this provider’s eligibility and supported-region requirements.</label>
          </div>
          {!hasApiKey && <p className="pr-guide-muted">A {disclosure.displayName} API key is not configured. Add it in Settings; this review will remain local.</p>}
          <div className="pr-guide-actions">
            {!hasApiKey && <button type="button" className="secondary-button" onClick={onOpenProviderSettings}>Open provider settings</button>}
            <button type="button" className="primary-button" onClick={() => { void send(); }} disabled={!hasApiKey || !completeConsent(consent) || sending}><Send size={15} /> {sending ? 'Sending reviewed hunk…' : 'Send reviewed request'}</button>
            {sending && <button type="button" className="secondary-button" onClick={() => { void cancel(); }} disabled={cancelBusy}>{cancelBusy ? 'Cancelling…' : 'Cancel request'}</button>}
          </div>
          {cancelRequested && <p className="pr-guide-muted" role="status">Cancellation was requested. The provider may already have received the request.</p>}
        </>
      )}
      {preview && (
        <div className="pr-change-explanation-result" aria-live="polite">
          <div className="pr-guide-subheading"><div><span className="git-changes-kicker">LOCAL CITATION CHECK PASSED · TRANSIENT RESULT</span><h4>{preview.title}</h4></div><span>{preview.providerId} · {preview.modelId}</span></div>
          {preview.claims.map((claim) => (
            <article className="pr-guide-claim" key={claim.claimId}>
              <strong>{claim.category === 'inference' ? 'Inference' : 'Code change'}</strong>
              <p>{claim.prose}</p>
              {claim.inferenceBasis && <p className="pr-guide-inference-basis"><strong>Why this is an inference:</strong> {claim.inferenceBasis}</p>}
              <button type="button" className="pr-guide-citation-link" onClick={() => onFocusChange(changeKey(change), claim.citation.hunkIndex)}>
                <span>{claim.citation.side} · {claim.citation.anchor.path}:{claim.citation.anchor.startLine}–{claim.citation.anchor.endLine}</span>
                <small>commit {claim.citation.anchor.revisionId.slice(0, 12)} · blob {claim.citation.anchor.blobId.slice(0, 12)} · hunk {claim.citation.hunkIndex + 1}</small>
                <code>{claim.citation.quote}</code>
              </button>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function completeConsent(consent: ProviderConsent): boolean {
  return consent.privacyAndDataUseAcknowledged && consent.billingUncertaintyAcknowledged && consent.adultSupportedRegionAcknowledged;
}

function isGuideEligible(change: GitFileChange): boolean {
  return (change.classification === 'text' || change.classification === 'parserUncertain')
    && change.hunks.length > 0
    && !change.skippedReason;
}

function guideExclusionReason(change: GitFileChange): string {
  if (change.classification === 'oversized') return 'The change is too large to include in a guide.';
  if (change.classification === 'binary') return 'This change is not readable text.';
  if (change.classification === 'parserUncertain') return 'The source structure could not be checked reliably.';
  if (change.classification === 'unsupported' && change.skippedReason?.startsWith('No bundled Tree-sitter grammar')) {
    return 'This file type cannot be safely explained yet.';
  }
  if (change.classification === 'unsupported') return 'This file cannot be safely included.';
  return 'A complete line-by-line change is not available.';
}

function changeKey(change: SelectedGitChangeIdentity): string {
  return `${change.basePath ?? ''}\u0000${change.headPath ?? ''}`;
}

function labelChange(change: GitFileChange): string {
  if (change.basePath && change.headPath && change.basePath !== change.headPath) return `${change.basePath} → ${change.headPath}`;
  return change.headPath ?? change.basePath ?? 'Unknown path';
}

function allClaimIds(revision: PrGuideRevision): string[] {
  return revision.sections.flatMap((section) => section.claims.map((claim) => claim.claimId));
}

function countClaims(revision: PrGuideRevision): number {
  return revision.sections.reduce((total, section) => total + section.claims.length, 0);
}

function formatBytes(bytes: number): string {
  return bytes < 1_024 ? `${bytes} B` : `${(bytes / 1_024).toFixed(1)} KB`;
}
