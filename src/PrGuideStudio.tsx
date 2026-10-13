import { useEffect, useMemo, useRef, useState } from 'react';
import { AlertTriangle, BookOpenCheck, Check, CircleDollarSign, FileDiff, RotateCcw, Send, ShieldCheck } from 'lucide-react';
import {
  acceptPrGuideDraft,
  buildProviderPrGuideRequestReview,
  cancelProviderRequest,
  discardProviderRequestReview,
  getProviderCredentialStatus,
  getProviderCatalog,
  isNativeRuntime,
  listPrGuideRevisions,
  rejectPrGuideDraft,
  sendProviderPrGuideRequest,
  type GitComparisonResult,
  type GitFileChange,
  type PrGuideRevision,
  type ProviderCommandResult,
  type ProviderConsent,
  type ProviderCredentialStatus,
  type ProviderDisclosure,
  type ProviderFailure,
  type ProviderRequestReview,
} from './lib/commands';

const NO_CONSENT: ProviderConsent = {
  privacyAndDataUseAcknowledged: false,
  billingUncertaintyAcknowledged: false,
  adultSupportedRegionAcknowledged: false,
};

interface PrGuideStudioProps {
  projectId: string;
  baseRef: string;
  headRef: string;
  comparison: GitComparisonResult;
  onFocusChange: (path: string | undefined, hunkIndex?: number) => void;
  onOpenProviderSettings: () => void;
}

export function PrGuideStudio({
  projectId,
  baseRef,
  headRef,
  comparison,
  onFocusChange,
  onOpenProviderSettings,
}: PrGuideStudioProps) {
  const [providerCatalog, setProviderCatalog] = useState<ProviderDisclosure[]>([]);
  const [providerId, setProviderId] = useState('');
  const [disclosure, setDisclosure] = useState<ProviderDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCommandResult<ProviderCredentialStatus> | null>(null);
  const [informationLoading, setInformationLoading] = useState(isNativeRuntime());
  const [informationError, setInformationError] = useState<string | null>(null);
  const [modelId, setModelId] = useState('');
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);
  const [requestReview, setRequestReview] = useState<ProviderRequestReview | null>(null);
  const requestReviewRef = useRef<ProviderRequestReview | null>(null);
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
  const [currentDraft, setCurrentDraft] = useState<PrGuideRevision | null>(null);
  const [includedClaimIds, setIncludedClaimIds] = useState<string[]>([]);
  const [decisionBusy, setDecisionBusy] = useState(false);
  const [decisionError, setDecisionError] = useState<string | null>(null);
  const [rejectionReason, setRejectionReason] = useState('A claim needs more evidence.');
  const [notice, setNotice] = useState<string | null>(null);

  const eligibleChanges = useMemo(() => comparison.changes.filter(isGuideEligible), [comparison.changes]);
  const selectedChanges = useMemo(
    () => eligibleChanges.filter((change) => selectedKeys.includes(changeKey(change))),
    [eligibleChanges, selectedKeys],
  );
  const credentialInfo = credential?.status === 'ok' ? credential.data : null;
  const hasApiKey = credentialInfo?.apiKeyConfigured ?? false;
  const selectedModel = disclosure?.models.find((model) => model.modelId === modelId) ?? null;
  const canBuild = Boolean(
    isNativeRuntime()
      && comparison.status === 'ready'
      && comparison.comparisonId
      && selectedChanges.length > 0
      && selectedChanges.length <= 12
      && disclosure
      && providerId
      && modelId
      && !requestReview
      && !building
      && !sending,
  );

  useEffect(() => {
    let active = true;
    if (!isNativeRuntime()) {
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
  }, []);

  useEffect(() => {
    if (!providerId) {
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
  }, [providerCatalog, providerId]);

  useEffect(() => {
    requestReviewRef.current = requestReview;
  }, [requestReview]);

  useEffect(() => {
    const refreshCredentials = () => {
      if (providerId) void getProviderCredentialStatus(providerId).then(setCredential).catch(() => {});
    };
    window.addEventListener('code-atlas:provider-credentials-changed', refreshCredentials);
    return () => window.removeEventListener('code-atlas:provider-credentials-changed', refreshCredentials);
  }, [providerId]);

  useEffect(() => () => {
    const reviewId = requestReviewRef.current?.reviewId;
    if (reviewId) void discardProviderRequestReview(reviewId).catch(() => {});
  }, []);

  useEffect(() => {
    if (comparison.status !== 'ready' || !comparison.comparisonId) {
      setHistory([]);
      setHistoryError(null);
      return;
    }
    let active = true;
    setHistoryLoading(true);
    setHistoryError(null);
    void listPrGuideRevisions(projectId, comparison.comparisonId)
      .then((revisions) => { if (active) setHistory(revisions); })
      .catch(() => { if (active) setHistoryError('The local PR Guide history could not be loaded.'); })
      .finally(() => { if (active) setHistoryLoading(false); });
    return () => { active = false; };
  }, [comparison.comparisonId, comparison.status, projectId]);

  function clearReview() {
    if (requestReviewRef.current) void discardProviderRequestReview(requestReviewRef.current.reviewId).catch(() => {});
    requestReviewRef.current = null;
    setRequestReview(null);
    setConsent(NO_CONSENT);
    setRequestError(null);
    setCancelStatus(null);
    setCurrentDraft(null);
    setIncludedClaimIds([]);
    setNotice(null);
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

  async function handleBuildReview() {
    if (!canBuild || !comparison.comparisonId) return;
    clearReview();
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
      if (result.status === 'error') {
        setRequestError(result.error);
        return;
      }
      requestReviewRef.current = result.data;
      setRequestReview(result.data);
      setConsent(NO_CONSENT);
    } catch {
      setRequestError({ kind: 'invalidReview', message: 'The local PR Guide request preview could not be built. No model request was sent.' });
    } finally {
      setBuilding(false);
    }
  }

  async function handleSend() {
    if (!requestReview || !hasApiKey || sending || !completeConsent(consent)) return;
    setSending(true);
    setRequestError(null);
    setCancelStatus(null);
    cancelRequested.current = false;
    try {
      const result = await sendProviderPrGuideRequest({
        reviewId: requestReview.reviewId,
        fingerprint: requestReview.fingerprint,
        consent,
      });
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
      requestReviewRef.current = null;
      setRequestReview(null);
      setConsent(NO_CONSENT);
      setRequestError({ kind: 'networkUnavailable', message: 'The result could not be confirmed. Check this comparison’s PR Guide history before sending a new reviewed request.' });
    } finally {
      setSending(false);
      setCancelBusy(false);
    }
  }

  async function handleCancel() {
    if (!requestReview || !sending || cancelBusy) return;
    setCancelBusy(true);
    cancelRequested.current = true;
    try {
      const stopped = await cancelProviderRequest(requestReview.reviewId);
      setCancelStatus(stopped
        ? `Cancellation reached the local request. ${disclosure?.displayName ?? 'The provider'} may already have received the selected diff; any response remains an unaccepted draft.`
        : 'The request had already finished locally. Any returned guide remains an unaccepted draft until you review it.');
    } catch {
      setCancelStatus(`Code Atlas could not confirm a local abort. ${disclosure?.displayName ?? 'The provider'} may already have received the selected diff; check the draft history after the request finishes.`);
    } finally {
      setCancelBusy(false);
    }
  }

  async function refreshHistory() {
    if (!comparison.comparisonId) return;
    try {
      setHistory(await listPrGuideRevisions(projectId, comparison.comparisonId));
      setHistoryError(null);
    } catch {
      setHistoryError('The local PR Guide history could not be refreshed.');
    }
  }

  async function handleAccept() {
    if (!currentDraft || !comparison.comparisonId || includedClaimIds.length === 0 || decisionBusy) return;
    setDecisionBusy(true);
    setDecisionError(null);
    try {
      const accepted = await acceptPrGuideDraft({
        projectId,
        comparisonId: comparison.comparisonId,
        revisionId: currentDraft.revisionId,
        includedClaimIds,
      });
      setCurrentDraft(null);
      setIncludedClaimIds([]);
      setNotice(`Accepted PR Guide revision ${accepted.revisionNumber}. Its history is separate from repository file chapters.`);
      await refreshHistory();
    } catch {
      setDecisionError('The selected guide could not be accepted. The saved draft and existing guide history were preserved.');
    } finally {
      setDecisionBusy(false);
    }
  }

  async function handleReject() {
    if (!currentDraft || !comparison.comparisonId || decisionBusy) return;
    setDecisionBusy(true);
    setDecisionError(null);
    try {
      const rejected = await rejectPrGuideDraft({
        projectId,
        comparisonId: comparison.comparisonId,
        revisionId: currentDraft.revisionId,
        reason: rejectionReason,
      });
      setCurrentDraft(null);
      setIncludedClaimIds([]);
      setNotice(`Rejected PR Guide draft ${rejected.revisionNumber}. Existing accepted guides and file chapters were not changed.`);
      await refreshHistory();
    } catch {
      setDecisionError('The draft could not be rejected. Existing local PR Guide history was preserved.');
    } finally {
      setDecisionBusy(false);
    }
  }

  function openDraft(revision: PrGuideRevision) {
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
      <p className="pr-guide-muted">Choose a direct API, select changed files, inspect the exact diff sent, then review every claim against its base/head hunk. Accepting or rejecting saves a PR Guide revision; it never changes Git refs or file chapter history.</p>
      <fieldset className="pr-guide-file-picker" disabled={building || sending || decisionBusy}>
        <legend>Select changed files · {selectedChanges.length} of 12</legend>
        {eligibleChanges.length === 0 ? <p>No changed file has a complete bounded text diff to explain.</p> : eligibleChanges.map((change) => {
          const key = changeKey(change);
          const checked = selectedKeys.includes(key);
          const pathLabel = labelChange(change);
          return (
            <label className={checked ? 'selected' : ''} key={key}>
              <input type="checkbox" checked={checked} disabled={!checked && selectedKeys.length >= 12} onChange={() => toggleChange(change)} />
              <span><strong>{change.kind}</strong><code>{pathLabel}</code></span>
              <small>{change.hunks.length} {change.hunks.length === 1 ? 'hunk' : 'hunks'}</small>
            </label>
          );
        })}
      </fieldset>
      <div className="pr-guide-actions">
        <label className="pr-guide-model">Direct API provider
          <select value={providerId} disabled={building || sending || Boolean(requestReview)} onChange={(event) => { clearReview(); setProviderId(event.target.value); setModelId(''); setCredential(null); }}>
            <option value="">Choose a provider…</option>
            {providerCatalog.map((provider) => <option key={provider.providerId} value={provider.providerId}>{provider.displayName}</option>)}
          </select>
        </label>
        <label className="pr-guide-model">{disclosure?.displayName ?? 'Model'}
          <select value={modelId} disabled={!disclosure || building || sending || Boolean(requestReview)} onChange={(event) => { clearReview(); setModelId(event.target.value); }}>
            <option value="">Choose a model…</option>
            {(disclosure?.models ?? []).map((model) => <option key={model.modelId} value={model.modelId}>{model.displayName}</option>)}
          </select>
        </label>
        <button className="primary-button" type="button" onClick={() => { void handleBuildReview(); }} disabled={!canBuild}>
          <FileDiff size={15} /> {building ? 'Building exact review…' : requestReview ? 'Request ready for review below' : 'Review exact provider request'}
        </button>
      </div>
      {!isNativeRuntime() && <p className="git-preview-note" role="status">The browser preview can inspect the UI but cannot call provider APIs or read local Git. Generation runs only in the packaged Mac app.</p>}
      {!providerId && <p className="pr-guide-muted" role="status">Choose a provider and model explicitly for this PR Guide. Selection makes no network request.</p>}
      {informationLoading && <p className="pr-guide-muted" role="status">Loading direct API catalog locally…</p>}
      {informationError && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {informationError}</p>}
      {credential?.status === 'error' && <p className="git-error-note" role="alert">{credential.error.message}</p>}
      {requestError && <p className="git-error-note" role="alert"><AlertTriangle size={15} /> {requestError.message}</p>}
      {notice && <p className="pr-guide-notice" role="status"><Check size={15} /> {notice}</p>}

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
              {revision.lifecycle === 'draft' && !decision && <button type="button" className="secondary-button" onClick={() => openDraft(revision)}>Review draft</button>}
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

function completeConsent(consent: ProviderConsent): boolean {
  return consent.privacyAndDataUseAcknowledged && consent.billingUncertaintyAcknowledged && consent.adultSupportedRegionAcknowledged;
}

function isGuideEligible(change: GitFileChange): boolean {
  return (change.classification === 'text' || change.classification === 'parserUncertain')
    && change.hunks.length > 0
    && !change.skippedReason;
}

function changeKey(change: GitFileChange): string {
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
