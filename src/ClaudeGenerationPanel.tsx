import { GenerationRecord } from './GenerationRecord';
import { InlineCodeText } from './InlineCodeText';
import { AppButton } from './design/AppButton';
import { useEffect, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  cancelClaudeAgentOperation, discardGeneratedChapterDraft, listChapterRevisions,
  saveGeneratedChapterDraft, sendClaudeChapterRequest, sendClaudeCodeLensRequest,
  sendClaudePrChangeRequest, sendClaudePrGuideRequest,
  type ChapterDraftPreview, type ChapterRevision, type CodeLensExplanationCitation,
  type CodeLensExplanationPreview, type PrChangeExplanationCitation,
  type PrChangeExplanationPreview, type PrGuidePreview, type ProviderCommandResult,
  type ProviderFailure, type ProviderRequestReview,
} from './lib/commands';
import { type ClaudeOperationReviewMode } from './lib/claudeReview';
import { sendClaudeRepositoryPlanRequest, type RepositoryPlanPreview } from './lib/repositoryPlans';

export interface ClaudeResultCallbacks {
  onOpenCodeLensCitation?: (citation: CodeLensExplanationCitation) => void;
  onOpenPrChangeCitation?: (citation: PrChangeExplanationCitation) => void;
  onPrGuideReady?: (preview: PrGuidePreview) => void;
  /** Only a validated result adopted by the current view resolves a notice. */
  onRecoveryResultReady?: (operationId: string) => void;
}

type Result = { kind: 'chapter'; value: ChapterDraftPreview }
  | { kind: 'range'; value: CodeLensExplanationPreview }
  | { kind: 'guide'; value: PrGuidePreview }
  | { kind: 'hunk'; value: PrChangeExplanationPreview }
  | { kind: 'plan'; value: RepositoryPlanPreview };
type Progress = { runId: string; phase: string; message: string };

async function send(mode: ClaudeOperationReviewMode, review: ProviderRequestReview): Promise<ProviderCommandResult<Result>> {
  switch (mode.kind) {
    case 'repository-plan': {
      const result = await sendClaudeRepositoryPlanRequest(review.reviewId, review.fingerprint, true);
      return result.status === 'error' ? result : { status: 'ok', data: { kind: 'plan', value: result.data } };
    }
    case 'review': case 'overview': case 'section-update': {
      const result = await sendClaudeChapterRequest(review.reviewId, review.fingerprint, true);
      return result.status === 'error' ? result : { status: 'ok', data: { kind: 'chapter', value: result.data } };
    }
    case 'code-lens-range': {
      const result = await sendClaudeCodeLensRequest(review.reviewId, review.fingerprint, true);
      return result.status === 'error' ? result : { status: 'ok', data: { kind: 'range', value: result.data } };
    }
    case 'pr-guide': {
      const result = await sendClaudePrGuideRequest(review.reviewId, review.fingerprint, true);
      return result.status === 'error' ? result : { status: 'ok', data: { kind: 'guide', value: result.data } };
    }
    case 'pr-change': {
      const result = await sendClaudePrChangeRequest(review.reviewId, review.fingerprint, true);
      return result.status === 'error' ? result : { status: 'ok', data: { kind: 'hunk', value: result.data } };
    }
  }
}

function failure(error: ProviderFailure): string {
  // These messages originate in the native adapter, never raw agent output.
  if (error.kind === 'providerRejectedRequest' || error.kind === 'reviewUnavailable') return error.message;
  const messages: Partial<Record<ProviderFailure['kind'], string>> = {
    consentRequired: 'Confirm the exact source and Claude account usage before generating.',
    reviewExpired: 'This review expired. Prepare a fresh request and confirm it again.',
    reviewChanged: 'The source, settings or Claude connection changed. Prepare a fresh review.',
    requestCancelled: 'Generation stopped. No response was accepted.',
    requestTimedOut: 'Claude did not finish within the operation limit. Review a new request before retrying.',
    malformedProviderResponse: 'The response failed local format or source evidence validation.',
    repairProgress: error.message,
    providerResponseTooLarge: 'The response exceeded the local limit. Try a smaller source selection.',
    localStorageUnavailable: 'The local result or recovery state could not be confirmed. Check pending chapter previews or PR Guide history before generating again.',
    unavailableModel: 'Refresh the Claude catalog and choose a supported model and effort.',
    quotaExceeded: 'Claude account usage is unavailable or exhausted. Check that account before retrying.',
    invalidReview: 'The exact request could not be validated. Reopen its captured source and prepare a fresh review.',
  };
  return messages[error.kind] ?? 'Claude could not complete this request. Check its account and runtime, then review a fresh request. The backend has not changed.';
}

/** Automatic UI uses only reviewed identities. Native admission remains
 * authoritative; the current empty release registry cannot start inference. */
export function ClaudeGenerationPanel({ native, mode, review, available, unavailableMessage,
  onHoldChange, onReset, onOpenCodeLensCitation, onOpenPrChangeCitation, onPrGuideReady, onRecoveryResultReady,
}: ClaudeResultCallbacks & {
  native: boolean; mode: ClaudeOperationReviewMode; review: ProviderRequestReview;
  available: boolean; unavailableMessage: string;
  onHoldChange: (held: boolean) => void; onReset: () => void;
}) {
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState<'send' | 'save' | 'discard' | null>(null);
  const [attempted, setAttempted] = useState(false);
  const [result, setResult] = useState<Result | null>(null);
  const [saved, setSaved] = useState<ChapterRevision | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const output = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onHoldChange, onPrGuideReady, onRecoveryResultReady });
  callbacks.current = { onHoldChange, onPrGuideReady, onRecoveryResultReady };
  const state = useRef({ mounted: false, version: 0, active: false, attempted: false,
    run: null as string | null, draft: null as string | null, saving: false, cancelled: false,
    unlisten: null as UnlistenFn | null });

  const notify = () => {
    if (mode.kind !== 'repository-plan') window.dispatchEvent(new CustomEvent('code-atlas:agent-recovery-changed'));
    if (mode.kind === 'repository-plan') window.dispatchEvent(new CustomEvent('code-atlas:repository-plans-changed', { detail: { projectId: mode.projectId } }));
    else if (mode.kind === 'pr-guide') window.dispatchEvent(new CustomEvent('code-atlas:pr-guides-changed', {
      detail: { projectId: mode.projectId, comparisonId: mode.comparisonId },
    }));
    else if (mode.kind === 'review' || mode.kind === 'overview' || mode.kind === 'section-update') {
      window.dispatchEvent(new CustomEvent('code-atlas:pending-chapter-previews-changed'));
    }
  };

  useEffect(() => {
    const current = state.current;
    current.mounted = true;
    return () => {
      current.mounted = false; ++current.version;
      current.cancelled = true;
      current.unlisten?.(); current.unlisten = null;
      if (native && current.run) void cancelClaudeAgentOperation(current.run).catch(() => {});
      // A save may already have written an immutable draft. Let its completion
      // reconcile history; a lost reply must not erase its recoverable preview.
      if (native && current.draft && !current.saving) {
        void discardGeneratedChapterDraft(current.draft).catch(() => {}).finally(notify);
        current.draft = null;
      }
      callbacks.current.onHoldChange(false);
    };
  }, [native, review.reviewId]);

  useEffect(() => { if (result || error) output.current?.focus(); }, [result, error]);

  const generate = async () => {
    const current = state.current;
    if (!native || !available || !consent || !current.mounted || current.active || current.attempted) return;
    if (Date.now() >= review.expiresAtUnixMs) { setError('This review expired. Prepare a fresh request.'); return; }
    const version = ++current.version;
    current.active = true; current.attempted = true; current.cancelled = false;
    current.run = review.reviewId;
    callbacks.current.onHoldChange(true);
    setAttempted(true); setBusy('send'); setError(null); setNote(null);
    setProgress({ runId: review.reviewId, phase: 'preparing', message: 'Preparing the exact reviewed Claude request…' });
    let unlisten: UnlistenFn | null = null;
    let invoked = false;
    let unclaimedDraft: string | null = null;
    try {
      unlisten = await listen<Progress>('claude-agent-progress', ({ payload }) => {
        if (!current.mounted || current.version !== version || payload.runId !== review.reviewId) return;
        setProgress(payload);
        if (current.cancelled) void cancelClaudeAgentOperation(review.reviewId).catch(() => {});
      });
      if (!current.mounted || current.version !== version || current.cancelled) return;
      current.unlisten = unlisten;
      invoked = true;
      const response = await send(mode, review);
      if (response.status === 'error') {
        if (current.mounted && current.version === version) {
          if (response.error.kind === 'repairProgress') setNote(response.error.message);
          else setError(mode.kind === 'repository-plan'
            ? `${failure(response.error)} Check Learning plans and recovery before another request; an unaccepted draft may already exist.`
            : failure(response.error));
        }
        return;
      }
      const value = response.data;
      if (value.kind === 'chapter') unclaimedDraft = value.value.generationId;
      const wanted = current.mounted && current.version === version;
      if (value.kind === 'chapter' && (!wanted || current.cancelled)) {
        await discardGeneratedChapterDraft(value.value.generationId).catch(() => {});
        unclaimedDraft = null;
        if (current.mounted && current.version === version) setNote('The returned chapter preview was discarded after cancellation. Account usage may already have occurred.');
        return;
      }
      if (!wanted) return;
      if (value.kind !== 'guide' && value.kind !== 'plan' && current.cancelled) {
        setNote('The temporary explanation was discarded after cancellation. Account usage may already have occurred.');
        return;
      }
      if (value.value.modelId !== review.modelId) throw new Error('Mismatched result model');
      if (value.kind === 'chapter') {
        if (value.value.revision.projectId !== review.projectId || value.value.revision.snapshotId !== review.snapshotId) {
          throw new Error('Mismatched result scope');
        }
        current.draft = value.value.generationId;
        unclaimedDraft = null;
      }
      if (value.kind === 'guide') {
        if (value.value.revision.projectId !== review.projectId || value.value.revision.comparison.comparisonId !== review.snapshotId) {
          throw new Error('Mismatched guide scope');
        }
        callbacks.current.onPrGuideReady?.(value.value);
        if (current.cancelled) setNote('Cancellation arrived after an unaccepted PR Guide draft was saved. Review it in PR Guide history.');
      }
      if (value.kind === 'plan') {
        if (mode.kind !== 'repository-plan' || !value.value.savedLocally || value.value.providerId !== 'claude-agent'
          || value.value.plan.question !== mode.question || value.value.reviewId !== review.reviewId || value.value.recoveryOperationId !== review.reviewId || value.value.fingerprint !== review.fingerprint
          || value.value.plan.projectId !== review.projectId || value.value.plan.snapshotId !== review.snapshotId) {
          throw new Error('Mismatched saved plan');
        }
        if (current.cancelled) setNote('Cancellation arrived after an unaccepted plan was saved. Review it in Learning plans before generating again.');
      }
      setResult(value);
      // Plan recovery remains until the app's explicit notice resolution.
      if (mode.kind !== 'repository-plan' && mode.recoveryOperationId) callbacks.current.onRecoveryResultReady?.(mode.recoveryOperationId);
    } catch {
      if (current.mounted && current.version === version) setError(!invoked
        ? 'Progress tracking could not be prepared. No agent request was started. Prepare a fresh review.'
        : mode.kind === 'repository-plan' ? 'The plan reply could not be confirmed. Check Learning plans and recovery before generating again; an unaccepted plan may already be saved.'
        : mode.kind === 'pr-guide'
          ? 'The reply could not be confirmed. Check PR Guide history before generating again; an unaccepted draft may already be saved.'
          : 'The reply could not be confirmed. Check pending previews and local recovery before generating again. Source may already have been shared.');
    } finally {
      unlisten?.();
      if (unclaimedDraft) await discardGeneratedChapterDraft(unclaimedDraft).catch(() => {});
      if (current.unlisten === unlisten) current.unlisten = null;
      current.run = null; current.active = false;
      if (current.mounted && current.version === version) {
        setBusy(null); setProgress(null); setCancelling(false); setConsent(false);
        if (!invoked && current.cancelled) setNote('Cancelled before sending. Prepare a fresh review before another attempt.');
      }
      notify();
    }
  };

  const cancel = async () => {
    const current = state.current;
    if (!native || !current.run || current.cancelled) return;
    current.cancelled = true; setCancelling(true);
    setNote(mode.kind === 'repository-plan' ? 'Stopping Claude. Any plan already saved remains an unaccepted draft in Learning plans. Account usage may already have occurred.'
      : mode.kind === 'pr-guide'
      ? 'Stopping Claude. Any guide already saved remains unaccepted in PR Guide history. Account usage may already have occurred.'
      : 'Stopping Claude. Returned previews or temporary explanations will be discarded. Account usage may already have occurred.');
    try { await cancelClaudeAgentOperation(current.run); }
    catch { if (current.mounted) setNote('The stop could not be confirmed. Wait for the operation to finish, then inspect local recovery and pending results.'); }
  };

  const save = async () => {
    const current = state.current;
    if (current.active || result?.kind !== 'chapter' || saved || result.value.validation.status !== 'valid') return;
    const preview = result.value;
    current.active = true; current.saving = true; setBusy('save'); setError(null);
    try {
      let revision: ChapterRevision;
      try { revision = await saveGeneratedChapterDraft(preview.generationId); }
      catch {
        const existing = await listChapterRevisions(preview.revision.projectId, preview.revision.snapshotId)
          .then((values) => values.find((value) => value.revisionId === preview.revision.revisionId));
        if (!existing) throw new Error('Save not confirmed');
        revision = existing;
        await discardGeneratedChapterDraft(preview.generationId).catch(() => {});
      }
      if (revision.revisionId !== preview.revision.revisionId || revision.projectId !== preview.revision.projectId
        || revision.snapshotId !== preview.revision.snapshotId) throw new Error('Saved identity mismatch');
      current.draft = null;
      if (current.mounted) setSaved(revision);
      window.dispatchEvent(new CustomEvent('code-atlas:chapters-changed', { detail: { revision } }));
    } catch {
      if (current.mounted) setError('Save could not be confirmed. Keep this preview and check chapter history before retrying.');
    } finally {
      current.active = false; current.saving = false;
      if (current.mounted) setBusy(null);
      notify();
    }
  };

  const reset = async () => {
    const current = state.current;
    if (current.active) return;
    current.active = true; setBusy('discard');
    try {
      if (current.draft) await discardGeneratedChapterDraft(current.draft);
      current.draft = null;
      if (current.mounted) onReset();
    } catch { if (current.mounted) setError('The preview could not be discarded. Try again before preparing another request.'); }
    finally { current.active = false; if (current.mounted) setBusy(null); notify(); }
  };

  return <section className="provider-review-card" aria-labelledby="claude-generate-title">
    <h4 id="claude-generate-title">Generate with Claude Agent</h4>
    {!available && <p role="status">{unavailableMessage}</p>}
    {!attempted && <>
      <label className="provider-consent"><input type="checkbox" checked={consent} disabled={!native || !available}
        onChange={(event) => setConsent(event.target.checked)} />
        I reviewed this exact source, prompt and settings. Send it through my existing Claude account; its usage limits and terms apply. This does not authorize tools or a different billing route.
      </label>
      <AppButton intent="primary" disabled={!native || !available || !consent || busy !== null}
        onClick={() => { void generate(); }}>Generate with Claude Agent</AppButton>
    </>}
    {busy === 'send' && <><p role="status">{progress?.message ?? 'Waiting for Claude and local result validation…'}</p>
      <AppButton intent="secondary" disabled={cancelling} onClick={() => { void cancel(); }}>{cancelling ? 'Stopping…' : 'Cancel generation'}</AppButton></>}
    <div ref={output} tabIndex={-1} role="group" aria-label="Claude generation result">
      {result?.kind === 'chapter' && <>
        <GenerationRecord origin={result.value.revision.generationOrigin} />
        <h4><InlineCodeText text={result.value.revision.title}/></h4><p>Validated chapter preview · {saved ? 'saved as an unaccepted draft' : 'not saved or accepted'}</p>
        {result.value.updateNotes.map((text, index) => <p key={index}>{text}</p>)}
        {result.value.revision.sections.map((section) => <article className="provider-chapter-section" key={section.sectionId}>
          <h5>{section.order + 1}. <InlineCodeText text={section.title}/></h5><p><InlineCodeText text={section.prose}/></p>
          {section.claims.map((claim) => <div key={claim.claimId}>
            <span className="chapter-claim-category">{claim.category.kind === 'inference' ? claim.category.visibleLabel
              : claim.category.kind === 'codeBehavior' ? 'Code behavior' : 'Repository context'}</span><p><InlineCodeText text={claim.prose}/></p>
            {claim.category.kind === 'inference' && <p>Inference basis: <InlineCodeText text={claim.category.basis}/></p>}
            {claim.anchors.map((anchor) => <details key={anchor.anchorId}><summary><code>{anchor.path}:{anchor.startLine}–{anchor.endLine}</code></summary>
              <pre><code>{anchor.excerpt}</code></pre><small>Snapshot {anchor.snapshotId} · SHA-256 {anchor.sourceSha256}</small></details>)}
          </div>)}
          <details><summary>Narration script</summary><p><InlineCodeText text={section.spokenText}/></p></details>
        </article>)}
        {!saved && <AppButton intent="primary" disabled={busy !== null || result.value.validation.status !== 'valid'}
          onClick={() => { void save(); }}>{busy === 'save' ? 'Saving…' : 'Save unaccepted draft'}</AppButton>}
      </>}
      {result?.kind === 'range' && <><h4>Selected code explained</h4>{result.value.claims.map((claim) => <article key={claim.claimId}>
        <p><InlineCodeText text={claim.prose}/></p><blockquote>{claim.citation.quote}</blockquote>
        <AppButton intent="quiet" disabled={!onOpenCodeLensCitation} onClick={() => onOpenCodeLensCitation?.(claim.citation)}>
          Open {claim.citation.path}:{claim.citation.startLine}–{claim.citation.endLine}</AppButton>
      </article>)}</>}
      {result?.kind === 'hunk' && <><h4><InlineCodeText text={result.value.title}/></h4>{result.value.claims.map((claim) => <article key={claim.claimId}>
        <span className="chapter-claim-category">{claim.category}</span><p><InlineCodeText text={claim.prose}/></p>{claim.inferenceBasis && <p>Inference basis: <InlineCodeText text={claim.inferenceBasis}/></p>}
        <blockquote>{claim.citation.quote}</blockquote>
        <AppButton intent="quiet" disabled={!onOpenPrChangeCitation} onClick={() => onOpenPrChangeCitation?.(claim.citation)}>
          Open {claim.citation.side} evidence · hunk {claim.citation.hunkIndex + 1}</AppButton>
      </article>)}</>}
      {result?.kind === 'guide' && <><h4><InlineCodeText text={result.value.revision.title}/></h4>
        <p>Saved in PR Guide history as an unaccepted draft. Close this dialog to review and accept individual claims.</p>
        {result.value.revision.sections.map((section) => <article key={section.sectionId}><h5><InlineCodeText text={section.title}/></h5>
          {section.claims.map((claim) => <div key={claim.claimId}><span className="chapter-claim-category">{claim.category}</span><p><InlineCodeText text={claim.prose}/></p>
            {claim.inferenceBasis && <p>Inference basis: <InlineCodeText text={claim.inferenceBasis}/></p>}</div>)}</article>)}
      </>}
      {result?.kind === 'plan' && <><h4><InlineCodeText text={result.value.plan.title}/></h4>
        <p>Saved locally as an unaccepted learning plan with {result.value.plan.steps.length} steps. Close this dialog and open it in Learning plans to review each rationale and citation.</p>
        <p>Files without a planned step: {result.value.plan.omittedSelectedPaths.length}. No chapter or file coverage was accepted.</p>
      </>}
      {error && <p className="provider-error-text" role="alert">{error}</p>}
      {note && <p role="status">{note}</p>}
    </div>
    {attempted && busy === null && <AppButton intent="secondary" onClick={() => { void reset(); }}>
      {result?.kind === 'chapter' && !saved ? 'Discard preview and review again' : 'Review another request'}</AppButton>}
    <p className="provider-small-note">Failures keep Claude selected. Accepted chapter and guide history change only through their separate acceptance flows.</p>
  </section>;
}
