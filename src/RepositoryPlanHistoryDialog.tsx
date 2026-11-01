import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Check, ChevronRight, Clock3, FileCode2, Route, X } from 'lucide-react';
import { AppButton } from './design/AppButton';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import {
  acceptRepositoryPlanDraft, discardRepositoryPlanDraft, listRepositoryPlanDrafts,
  listRepositoryPlanRevisions, loadRepositoryPlanDraft, loadRepositoryPlanRevision,
  type RepositoryPlanDraftDetail, type RepositoryPlanDraftSummary,
  type RepositoryPlanRevisionDetail, type RepositoryPlanRevisionSummary,
} from './lib/repositoryPlans';
import { preparePlannedChapterScope, type PlannedChapterScope } from './lib/plannedChapters';
import { InlineCodeText } from './InlineCodeText';

type PlanSelection = { kind: 'draft'; id: string } | { kind: 'revision'; id: string };
type PlanDetail = RepositoryPlanDraftDetail | RepositoryPlanRevisionDetail;

function planFor(detail: PlanDetail) {
  return 'preview' in detail ? detail.preview.plan : detail.plan.preview.plan;
}

function detailRecord(detail: PlanDetail): string {
  return 'preview' in detail ? detail.recordSha256 : detail.plan.recordSha256;
}

function planTime(timestamp: number | null | undefined): string {
  if (!timestamp || !Number.isFinite(timestamp)) return 'Date unavailable';
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(timestamp);
}

export function RepositoryPlanHistoryDialog({ projectId, native, initialDraftId, onClose, onOpenChapter, onOpenSource }: {
  projectId: string;
  native: boolean;
  initialDraftId?: string;
  onClose: () => void;
  onOpenChapter: (scope: PlannedChapterScope) => void;
  onOpenSource: (snapshotId: string, path: string, startLine: number, endLine: number) => void;
}) {
  const dialog = useRef<HTMLElement>(null);
  const [drafts, setDrafts] = useState<RepositoryPlanDraftSummary[]>([]);
  const [revisions, setRevisions] = useState<RepositoryPlanRevisionSummary[]>([]);
  const [selection, setSelection] = useState<PlanSelection | null>(null);
  const [detail, setDetail] = useState<PlanDetail | null>(null);
  const [busy, setBusy] = useState<'list' | 'load' | 'accept' | 'discard' | 'step' | null>(native ? 'list' : null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const initialDraftToOpen = useRef(initialDraftId ?? null);
  const selectionRef = useRef(selection);
  selectionRef.current = selection;
  const returnFocus = useRef<HTMLElement | null>(null);
  useModalFocusTrap(dialog);

  useEffect(() => {
    returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.focus();
    return () => { if (returnFocus.current?.isConnected) returnFocus.current.focus(); };
  }, []);

  const refresh = useCallback(async (preserveSelection = true) => {
    if (!native) return;
    setBusy('list');
    setError(null);
    try {
      const [nextDrafts, nextRevisions] = await Promise.all([
        listRepositoryPlanDrafts(projectId), listRepositoryPlanRevisions(projectId),
      ]);
      setDrafts(nextDrafts);
      setRevisions(nextRevisions);
      const current = selectionRef.current;
      const stillExists = current && (current.kind === 'draft'
        ? nextDrafts.some((item) => item.draftId === current.id)
        : nextRevisions.some((item) => item.revision.revisionId === current.id));
      if (!preserveSelection || !stillExists) {
        setSelection(null);
        setDetail(null);
      }
    } catch {
      setError('Saved routes could not be read from this Mac. Your plans have not been changed.');
    } finally { setBusy(null); }
  }, [native, projectId]);

  useEffect(() => { void refresh(false); }, [refresh]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); onClose(); }
    };
    document.addEventListener('keydown', key, true);
    return () => document.removeEventListener('keydown', key, true);
  }, [onClose]);

  const selectedRevision = selection?.kind === 'revision'
    ? revisions.find((item) => item.revision.revisionId === selection.id) ?? null : null;
  const selectedDraft = selection?.kind === 'draft'
    ? drafts.find((item) => item.draftId === selection.id) ?? null : null;
  const headRevision = useMemo(() => revisions.find((item) => item.isHead)?.revision.revisionId ?? null, [revisions]);
  const plan = detail ? planFor(detail) : null;
  const isDraft = detail !== null && 'preview' in detail;

  const openSelection = useCallback(async (next: PlanSelection) => {
    if (busy) return;
    initialDraftToOpen.current = null;
    setSelection(next); setDetail(null); setBusy('load'); setError(null); setStatus(null);
    try {
      const loaded = next.kind === 'draft'
        ? await loadRepositoryPlanDraft(projectId, next.id)
        : await loadRepositoryPlanRevision(projectId, next.id);
      const loadedPlan = planFor(loaded);
      if (loadedPlan.projectId !== projectId) throw new Error('scope mismatch');
      if (next.kind === 'draft' && (!('preview' in loaded) || loaded.preview.plan.draftId !== next.id)) throw new Error('draft mismatch');
      if (next.kind === 'revision' && (!('revision' in loaded) || loaded.revision.revisionId !== next.id)) throw new Error('revision mismatch');
      setDetail(loaded);
    } catch {
      setError('This route could not be revalidated from its saved source. The original record is preserved.');
    } finally { setBusy(null); }
  }, [busy, projectId]);

  useEffect(() => {
    const draftId = initialDraftToOpen.current;
    if (!draftId || busy !== null) return;
    if (drafts.some((item) => item.draftId === draftId)) {
      initialDraftToOpen.current = null;
      void openSelection({kind:'draft',id:draftId});
    } else setStatus('The new draft was not in the saved list. Refresh to check for a late save.');
  }, [busy, drafts, openSelection]);

  const accept = async () => {
    if (!selectedDraft || !detail || !isDraft || busy) return;
    setBusy('accept'); setError(null); setStatus(null);
    try {
      const accepted = await acceptRepositoryPlanDraft({
        projectId,
        draftId: selectedDraft.draftId,
        expectedRecordSha256: detailRecord(detail),
        basedOnRevisionId: headRevision,
      });
      if (accepted.revision.draftId !== selectedDraft.draftId || accepted.plan.preview.plan.projectId !== projectId
        || accepted.revision.recordSha256 !== detailRecord(detail)) throw new Error('acceptance mismatch');
      setDetail(accepted); setSelection({ kind: 'revision', id: accepted.revision.revisionId });
      setStatus('Route accepted. Its chapters remain separate drafts until you review them.');
      await refresh(true);
      setDetail(accepted); setSelection({ kind: 'revision', id: accepted.revision.revisionId });
    } catch {
      setError('The route was not confirmed as accepted. Refresh the saved routes before retrying.');
    } finally { setBusy(null); }
  };

  const discard = async () => {
    if (!selectedDraft || busy) return;
    setBusy('discard'); setError(null); setStatus(null);
    try {
      await discardRepositoryPlanDraft(projectId, selectedDraft.draftId);
      setSelection(null); setDetail(null); setStatus('Draft discarded. Accepted routes are unchanged.');
      await refresh(false);
    } catch {
      setError('The draft could not be discarded. Its saved record remains available.');
    } finally { setBusy(null); }
  };

  const openStep = async (stepId: string) => {
    if (!selectedRevision || busy) return;
    setBusy('step'); setError(null); setStatus(null);
    try {
      const scope = await preparePlannedChapterScope(projectId, {
        revisionId: selectedRevision.revision.revisionId,
        recordSha256: selectedRevision.revision.recordSha256,
        stepId,
      });
      if (scope.target.revisionId !== selectedRevision.revision.revisionId
        || scope.target.recordSha256 !== selectedRevision.revision.recordSha256
        || scope.target.stepId !== stepId || scope.snapshotId !== selectedRevision.revision.snapshotId) throw new Error('step scope mismatch');
      onOpenChapter(scope);
    } catch {
      setError('That step is no longer available from this accepted route. Refresh and choose it again.');
    } finally { setBusy(null); }
  };

  return <div className="provider-overlay repository-plan-overlay">
    <section ref={dialog} className="provider-dialog repository-plan-dialog" role="dialog" aria-modal="true"
      aria-labelledby="repository-plan-history-title" tabIndex={-1}>
      <header className="provider-dialog-header">
        <div><span className="eyebrow"><Route size={13}/> LEARNING ROUTES</span><h2 id="repository-plan-history-title">Plans for this repository</h2>
          <p>Review a saved route, accept it, then open one chapter at a time.</p></div>
        <AppButton intent="icon" onClick={onClose} aria-label="Close learning plans"><X size={17}/></AppButton>
      </header>
      {!native ? <div className="provider-dialog-content"><p>Saved routes are available in the macOS app.</p></div> : <div className="repository-plan-browser">
        <aside className="repository-plan-list" aria-label="Saved learning routes">
          <div className="repository-plan-list-heading"><strong>Unaccepted drafts</strong><span>{drafts.length}</span></div>
          {drafts.map((item)=><AppButton key={`draft:${item.draftId}`} intent="custom" className="repository-plan-list-item"
            aria-pressed={selection?.kind === 'draft' && selection.id === item.draftId}
            onClick={()=>void openSelection({kind:'draft',id:item.draftId})} disabled={busy!==null}>
            <Route size={15}/><span><strong>{item.title}</strong><small>{item.question ?? 'Question unavailable'} · {planTime(item.createdAtUnixMs)}</small></span><ChevronRight size={14}/>
          </AppButton>)}
          {drafts.length===0&&<p className="repository-plan-list-empty">No unaccepted drafts.</p>}
          <div className="repository-plan-list-heading"><strong>Accepted routes</strong><span>{revisions.length}</span></div>
          {revisions.map(({revision,isHead})=><AppButton key={`revision:${revision.revisionId}`} intent="custom" className="repository-plan-list-item"
            aria-pressed={selection?.kind === 'revision' && selection.id === revision.revisionId}
            onClick={()=>void openSelection({kind:'revision',id:revision.revisionId})} disabled={busy!==null}>
            <Check size={15}/><span><strong>{revision.title}{isHead?' · Current':''}</strong><small>{revision.question} · {planTime(revision.acceptedAtUnixMs)}</small></span><ChevronRight size={14}/>
          </AppButton>)}
          {revisions.length===0&&<p className="repository-plan-list-empty">No accepted routes yet.</p>}
          <AppButton size="1" intent="quiet" onClick={()=>void refresh(true)} disabled={busy!==null}><Clock3 size={13}/>Refresh saved routes</AppButton>
        </aside>
        <main className="repository-plan-detail" aria-live="polite">
          {busy==='list'&&<p role="status">Reading saved routes…</p>}
          {busy==='load'&&<p role="status">Revalidating the selected route…</p>}
          {error&&<p className="provider-error-text" role="alert">{error}</p>}
          {status&&<p className="provider-status-note" role="status">{status}</p>}
          {!detail&&!busy&&<div className="repository-plan-empty"><Route size={24}/><h3>Choose a route</h3><p>Unaccepted drafts need your review. Accepting a route does not accept its chapters.</p></div>}
          {detail&&plan&&<>
            <div className="repository-plan-detail-heading"><div><span>{isDraft?'UNACCEPTED DRAFT':'ACCEPTED ROUTE'}</span><h3><InlineCodeText text={plan.title}/></h3><p><InlineCodeText text={plan.question}/></p></div>
              {!isDraft&&selectedRevision&&<small>Revision {selectedRevision.revision.revisionNumber}{selectedRevision.isHead?' · current':''}</small>}
            </div>
            <div className="repository-plan-step-list">
              {plan.steps.map((step,index)=><article className="repository-plan-step" key={step.stepId}>
                <div className="repository-plan-step-heading"><span>STEP {index+1}</span><code>{step.primaryPath}</code></div>
                <h4><InlineCodeText text={step.question}/></h4>
                <p><InlineCodeText text={step.claim.prose}/></p>
                <div className="repository-plan-evidence"><FileCode2 size={13}/>{step.claim.anchors.map((anchor)=><AppButton key={anchor.anchorId} size="1" intent="quiet" aria-label={`Open ${anchor.path}:${anchor.startLine} to ${anchor.endLine}`} onClick={()=>onOpenSource(plan.snapshotId,anchor.path,anchor.startLine,anchor.endLine)}><code>{anchor.path}:{anchor.startLine}–{anchor.endLine}</code></AppButton>)}</div>
                {!isDraft&&<AppButton size="1" intent="secondary" onClick={()=>void openStep(step.stepId)} disabled={busy!==null}>Explain this step<ChevronRight size={13}/></AppButton>}
              </article>)}
            </div>
            {plan.unknowns.length>0&&<details className="repository-plan-unknowns"><summary>Open questions · {plan.unknowns.length}</summary>
              {plan.unknowns.map((item)=><p key={item.claimId}><InlineCodeText text={item.prose}/></p>)}
            </details>}
            <p className="provider-small-note">{plan.omittedSelectedPaths.length} selected files did not receive a route step. Route acceptance does not mark any chapter complete.</p>
          </>}
        </main>
      </div>}
      {detail&&isDraft&&<footer className="provider-dialog-footer repository-plan-footer">
        <AppButton intent="secondary" onClick={()=>void discard()} disabled={busy!==null}>Discard draft</AppButton>
        <AppButton intent="primary" onClick={()=>void accept()} disabled={busy!==null}>{busy==='accept'?'Accepting…':'Accept route'}</AppButton>
      </footer>}
    </section>
  </div>;
}
