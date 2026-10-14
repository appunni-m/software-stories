import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { AlertTriangle, Download, FileCode2, X } from 'lucide-react';
import {
  buildNarrationRequestReview,
  discardNarrationRequestReview,
  isNativeRuntime,
  loadReviewedNarrationAudio,
  type ChapterRevision,
} from './lib/commands';
import { parseNarrationPreferences } from './lib/narration';
import { errorMessageOr } from './lib/errorMessage';
import {
  buildOfflineExportPreview,
  buildOfflineGuideHtml,
  buildOfflineNarrationCoverage,
  isOfflineExportableRevision,
  offlineNarrationSectionKey,
  type OfflineNarrationAudio,
  type OfflineNarrationCacheScanState,
  type OfflineNarrationOmission,
  type OfflineExportPreview,
} from './lib/offlineExport';

interface OfflineExportProps {
  projectId: string;
  revisions: ChapterRevision[];
}

interface CachedNarrationPreview extends OfflineNarrationAudio {
  chapterTitle: string;
  sectionTitle: string;
}

export function OfflineExport({ projectId, revisions }: OfflineExportProps) {
  const [open, setOpen] = useState(false);
  const [selectedRevisionIds, setSelectedRevisionIds] = useState<string[]>([]);
  const [includeExcerpts, setIncludeExcerpts] = useState(true);
  const [includeNarrationAudio, setIncludeNarrationAudio] = useState(false);
  const [cachedNarration, setCachedNarration] = useState<CachedNarrationPreview[]>([]);
  const [narrationCacheScan, setNarrationCacheScan] = useState<OfflineNarrationCacheScanState>('idle');
  const [narrationOmissions, setNarrationOmissions] = useState<OfflineNarrationOmission[]>([]);
  const [privacyReviewed, setPrivacyReviewed] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  const acceptedRevisions = useMemo(
    () => revisions.filter((revision) => revision.lifecycle === 'accepted' && revision.sections.length > 0),
    [revisions],
  );
  const exportableRevisions = useMemo(
    () => acceptedRevisions.filter(isOfflineExportableRevision),
    [acceptedRevisions],
  );
  const exportChoices = useMemo(() => exportableRevisions.map((revision) => ({
    revision,
    preview: buildOfflineExportPreview([revision], [revision.revisionId], false).chapters[0],
  })), [exportableRevisions]);
  const exportableIds = exportableRevisions.map((revision) => revision.revisionId);
  const exportableKey = exportableIds.join('\u0000');
  useEffect(() => {
    setSelectedRevisionIds(exportableIds);
  }, [exportableKey]);

  const selectedSet = new Set(selectedRevisionIds);
  const selected = exportableRevisions.filter((revision) => selectedSet.has(revision.revisionId));
  const selectedRevisionKey = selected.map((revision) => revision.revisionId).join('\u0000');
  const preview: OfflineExportPreview = useMemo(
    () => buildOfflineExportPreview(selected, selectedRevisionIds, includeExcerpts, cachedNarration),
    [selected, selectedRevisionIds, includeExcerpts, cachedNarration],
  );
  const narrationCoverage = useMemo(
    () => buildOfflineNarrationCoverage(selected, selectedRevisionIds, includeNarrationAudio, narrationCacheScan, cachedNarration, narrationOmissions),
    [selected, selectedRevisionIds, includeNarrationAudio, narrationCacheScan, cachedNarration, narrationOmissions],
  );
  const sectionsWithoutUsableAudio = [...narrationCoverage.values()].filter((entry) =>
    entry.status === 'transcript-only' || entry.status === 'unavailable' || entry.status === 'no-transcript',
  ).length;
  const hiddenTargetCount = acceptedRevisions.length - exportableRevisions.length;

  useEffect(() => {
    let active = true;
    if (!open || !includeNarrationAudio) {
      setCachedNarration([]);
      setNarrationOmissions([]);
      setNarrationCacheScan('idle');
      return () => { active = false; };
    }
    if (!isNativeRuntime()) {
      setCachedNarration([]);
      setNarrationOmissions([]);
      setNarrationCacheScan('desktop-only');
      return () => { active = false; };
    }

    const selectedIds = new Set(selectedRevisionKey ? selectedRevisionKey.split('\u0000') : []);
    const revisionsToScan = exportableRevisions.filter((revision) => selectedIds.has(revision.revisionId));
    setCachedNarration([]);
    setNarrationOmissions([]);
    setNarrationCacheScan('loading');
    void (async () => {
      const found: CachedNarrationPreview[] = [];
      const omissions: OfflineNarrationOmission[] = [];
      let unavailable = false;
      for (const revision of revisionsToScan) {
        for (const section of revision.sections) {
          if (!active) return;
          const storageKey = `code-atlas:narration:${projectId}:${revision.chapterId}:${revision.revisionId}:${section.sectionId}`;
          let serialized: string | null = null;
          try { serialized = window.localStorage.getItem(storageKey); } catch {
            omissions.push({
              chapterRevisionId: revision.revisionId,
              sectionId: section.sectionId,
              status: 'unavailable',
              message: 'Saved narration settings could not be read; cached audio cannot be safely matched.',
            });
            unavailable = true;
            continue;
          }
          const preferences = parseNarrationPreferences(serialized, section.spokenText);
          if (!preferences.spokenText.trim()) continue;
          let reviewId: string | null = null;
          try {
            const reviewResult = await buildNarrationRequestReview(preferences);
            if (reviewResult.status !== 'ok') {
              omissions.push({
                chapterRevisionId: revision.revisionId,
                sectionId: section.sectionId,
                status: 'unavailable',
                message: 'Audio cache could not be checked; this section will remain transcript-only.',
              });
              unavailable = true;
              continue;
            }
            const review = reviewResult.data;
            reviewId = review.reviewId;
            if (!active) continue;
            if (!review.cachedAudioAvailable) {
              omissions.push({
                chapterRevisionId: revision.revisionId,
                sectionId: section.sectionId,
                status: 'omitted',
                message: 'No matching cached audio was found; this section will remain transcript-only.',
              });
              continue;
            }
            const audioResult = await loadReviewedNarrationAudio({ reviewId: review.reviewId, fingerprint: review.fingerprint });
            if (!active) continue;
            if (audioResult.status === 'ok' && audioResult.data.fingerprint === review.fingerprint) {
              found.push({
                chapterRevisionId: revision.revisionId,
                sectionId: section.sectionId,
                exactText: review.exactText,
                audio: audioResult.data,
                chapterTitle: revision.title,
                sectionTitle: section.title,
              });
            } else {
              omissions.push({
                chapterRevisionId: revision.revisionId,
                sectionId: section.sectionId,
                status: 'unavailable',
                message: 'Saved audio could not be verified; it will be omitted and the transcript retained.',
              });
              unavailable = true;
            }
          } catch {
            omissions.push({
              chapterRevisionId: revision.revisionId,
              sectionId: section.sectionId,
              status: 'unavailable',
              message: 'Audio cache could not be read; audio is omitted and the transcript retained.',
            });
            unavailable = true;
          } finally {
            if (reviewId) await discardNarrationRequestReview(reviewId).catch(() => {});
          }
        }
      }
      if (!active) return;
      setCachedNarration(found);
      setNarrationOmissions(omissions);
      setNarrationCacheScan(unavailable ? 'unavailable' : 'ready');
    })();
    return () => { active = false; };
  }, [open, includeNarrationAudio, projectId, selectedRevisionKey]);

  useEffect(() => {
    if (!open) return;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const dialog = dialogRef.current;
    (dialog?.querySelector<HTMLElement>('button:not(:disabled)') ?? dialog)?.focus();
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === 'Escape' && !exporting) {
        event.preventDefault();
        setOpen(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, [open, exporting]);

  const close = () => {
    if (exporting) return;
    setOpen(false);
    setError(null);
  };

  const toggleRevision = (revisionId: string) => {
    setSelectedRevisionIds((current) => current.includes(revisionId)
      ? current.filter((item) => item !== revisionId)
      : [...current, revisionId]);
    setError(null);
    setPrivacyReviewed(false);
  };

  const download = async () => {
    if (!privacyReviewed || selectedRevisionIds.length === 0 || exporting) return;
    setExporting(true);
    setError(null);
    try {
      const result = await buildOfflineGuideHtml(acceptedRevisions, {
        selectedRevisionIds,
        includeExcerpts,
        includeNarrationAudio,
        narrationAudio: cachedNarration,
      });
      const blob = new Blob([result.html], { type: 'text/html;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = result.filename;
      anchor.hidden = true;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 0);
      setOpen(false);
    } catch (exportError) {
      setError(errorMessageOr(exportError, 'The offline guide could not be created.'));
    } finally {
      setExporting(false);
    }
  };

  const handleDialogKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== 'Tab') return;
    const dialog = dialogRef.current;
    if (!dialog) return;
    const controls = Array.from(dialog.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), summary, a[href], [tabindex="0"]'));
    if (controls.length === 0) return;
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <>
      <button
        ref={triggerRef}
        className="offline-export-trigger"
        onClick={() => {
          setOpen(true);
          setPrivacyReviewed(false);
          setError(null);
        }}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label="Export accepted chapters for offline reading"
      >
        <Download size={13} /> <span>Offline export</span>
      </button>
      {open && (
        <div className="provider-overlay offline-export-overlay" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}>
          <section
            ref={dialogRef}
            className="provider-dialog offline-export-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="offline-export-title"
            aria-describedby="offline-export-description"
            tabIndex={-1}
            onKeyDown={handleDialogKeyDown}
          >
            <header className="provider-dialog-header">
              <div>
                <span className="provider-kicker">WALKTHROUGH · STATIC PACKAGE</span>
                <h2 id="offline-export-title">Export for offline reading</h2>
                <p id="offline-export-description">Choose accepted chapter revisions, review the source material that will travel with them, then download one self-contained HTML file.</p>
              </div>
              <button className="icon-button" onClick={close} disabled={exporting} aria-label="Close offline export"><X size={16} /></button>
            </header>

            <div className="provider-dialog-content offline-export-content">
              <section className="offline-export-callout" aria-label="Offline feature limits">
                <FileCode2 size={16} />
                <p><strong>What works offline:</strong> written sections, cited source excerpts when included, section navigation, supported typed diagrams, and cached narration when selected. <strong>Not included:</strong> live Atlas or Code Lens destinations. Legacy or malformed diagrams show a message instead of running chapter-provided markup.</p>
              </section>

              <section className="offline-export-chapters" aria-labelledby="offline-export-chapters-title">
                <div className="offline-export-section-heading">
                  <div><span className="provider-kicker">ACCEPTED REVISIONS</span><h3 id="offline-export-chapters-title">Choose chapters</h3></div>
                  {exportableRevisions.length > 0 && <button className="text-button" onClick={() => { setSelectedRevisionIds(selectedRevisionIds.length === exportableIds.length ? [] : exportableIds); setPrivacyReviewed(false); }}> {selectedRevisionIds.length === exportableIds.length ? 'Clear selection' : 'Select all'} </button>}
                </div>
                {acceptedRevisions.length === 0 ? <p className="offline-export-empty">There are no accepted chapter revisions to export.</p> : (
                  <div className="offline-export-option-list">
                    {exportChoices.map(({ revision, preview: choicePreview }) => (
                      <label className="offline-export-option" key={revision.revisionId}>
                        <input type="checkbox" checked={selectedSet.has(revision.revisionId)} onChange={() => toggleRevision(revision.revisionId)} />
                        <span><strong>{choicePreview?.title ?? 'Untitled chapter'}</strong><small>{choicePreview?.targetLabel ?? 'Source target withheld'} · revision {revision.revisionNumber}</small></span>
                      </label>
                    ))}
                  </div>
                )}
                {hiddenTargetCount > 0 && <p className="offline-export-muted" role="status">{hiddenTargetCount} accepted {hiddenTargetCount === 1 ? 'chapter uses' : 'chapters use'} a hidden or unsafe target path and cannot be bundled. Its title and path are omitted here.</p>}
                {acceptedRevisions.length > 0 && exportableRevisions.length === 0 && <p className="offline-export-muted">No accepted chapter has an exportable target path. Hidden path details are deliberately withheld.</p>}
              </section>

              <section className="offline-export-excerpts" aria-labelledby="offline-export-excerpts-title">
                <div className="offline-export-section-heading"><div><span className="provider-kicker">SOURCE EVIDENCE</span><h3 id="offline-export-excerpts-title">Cited excerpts</h3></div><span>{preview.includedExcerptCount} included</span></div>
                <label className="offline-export-include-option"><input type="checkbox" checked={includeExcerpts} onChange={(event) => { setIncludeExcerpts(event.target.checked); setPrivacyReviewed(false); }} /><span><strong>Bundle cited source excerpts</strong><small>On by default. Turn this off to include only citation paths and line ranges.</small></span></label>
                {selected.length === 0 ? <p className="offline-export-empty">Select at least one accepted chapter to preview its names, paths, and cited excerpts.</p> : (
                  <div className="offline-export-preview-list">
                    {preview.chapters.map((chapter) => (
                      <article className="offline-export-preview-chapter" key={chapter.revisionId}>
                        <h4>{chapter.title}</h4>
                        <p className="offline-export-target">{chapter.targetLabel} · snapshot {chapter.snapshotId}</p>
                        {chapter.sections.map((section) => {
                          const transcript = section.narrationTranscript ?? section.spokenText;
                          const audioCoverage = narrationCoverage.get(offlineNarrationSectionKey(chapter.revisionId, section.sectionId));
                          return (
                            <div className="offline-export-preview-section" key={section.sectionId}>
                              <strong>{section.title}</strong>
                              <p className="offline-export-muted">Typed diagram asset: {section.typedVisual === 'included' ? 'included' : section.typedVisual === 'unavailable' ? 'unsupported diagram omitted with an explanation' : 'none attached'}.</p>
                              {audioCoverage !== undefined && <p className="offline-export-muted">
                                Narration: {audioCoverage.message}
                              </p>}
                              {transcript.trim() && <details>
                                <summary>{section.narrationTranscript ? 'Transcript matching cached narration' : 'Saved narration transcript preview'}{transcript.includes('[redacted credential]') ? ' · credential redacted' : ''}</summary>
                                <pre>{transcript}</pre>
                              </details>}
                              {section.references.map((reference) => (
                                <details key={reference.anchorId}>
                                  <summary><code>{reference.path}:{reference.startLine}-{reference.endLine}</code>{reference.redactedCredential && <span>credential redacted</span>}</summary>
                                  {includeExcerpts
                                    ? <pre>{reference.excerpt || '[Excerpt is empty after privacy filtering]'}</pre>
                                    : <p className="offline-export-muted">Excerpt omitted by your selection.</p>}
                                </details>
                              ))}
                              {section.references.length === 0 && <p className="offline-export-muted">No safe cited excerpt in this section.</p>}
                            </div>
                          );
                        })}
                      </article>
                    ))}
                  </div>
                )}
                {preview.omittedHiddenReferences > 0 && <p className="offline-export-muted">{preview.omittedHiddenReferences} citation{preview.omittedHiddenReferences === 1 ? '' : 's'} to hidden or sensitive paths will be left out.</p>}
                {preview.redactedCredentials > 0 && <p className="offline-export-muted">Likely credential values were redacted from {preview.redactedCredentials} text field{preview.redactedCredentials === 1 ? '' : 's'}.</p>}
              </section>

              <section className="offline-export-audio" aria-labelledby="offline-export-audio-title">
                <div className="offline-export-section-heading"><div><span className="provider-kicker">OPTIONAL LOCAL ASSET</span><h3 id="offline-export-audio-title">Cached narration</h3></div><span>{includeNarrationAudio ? `${cachedNarration.length} available` : 'not selected'}</span></div>
                <label className="offline-export-include-option">
                  <input type="checkbox" checked={includeNarrationAudio} disabled={!isNativeRuntime()} onChange={(event) => { setIncludeNarrationAudio(event.target.checked); setPrivacyReviewed(false); setError(null); }} />
                  <span><strong>Include audio already saved on this Mac</strong><small>This only reads local audio caches. It never generates speech or sends a provider request.</small></span>
                </label>
                {!isNativeRuntime() && <p className="offline-export-muted">Cached narration can be bundled from the desktop app after it has been generated there.</p>}
                {includeNarrationAudio && narrationCacheScan === 'loading' && <p className="offline-export-muted" role="status">Checking selected sections for locally cached audio…</p>}
                {includeNarrationAudio && narrationCacheScan === 'ready' && cachedNarration.length === 0 && <p className="offline-export-muted" role="status">No cached narration was found. The guide will remain text-only.</p>}
                {includeNarrationAudio && narrationCacheScan === 'unavailable' && <p className="offline-export-muted" role="status">Some saved audio could not be read. Only verified cached tracks will be included; no new narration request was sent.</p>}
                {includeNarrationAudio && narrationCacheScan === 'desktop-only' && <p className="offline-export-muted" role="status">No desktop audio cache is available in this browser preview.</p>}
                {includeNarrationAudio && narrationCacheScan !== 'loading' && narrationCacheScan !== 'idle' && sectionsWithoutUsableAudio > 0 && <p className="offline-export-muted">{sectionsWithoutUsableAudio} selected section{sectionsWithoutUsableAudio === 1 ? ' has' : 's have'} no usable audio; each section shows whether its audio is omitted or unavailable.</p>}
                {includeNarrationAudio && cachedNarration.length > 0 && (
                  <div className="offline-export-audio-list" role="group" aria-label="Cached narration selected for export">
                    {cachedNarration.map((entry) => <p key={`${entry.chapterRevisionId}:${entry.sectionId}`}><strong>{entry.chapterTitle}</strong><span>{entry.sectionTitle}</span></p>)}
                  </div>
                )}
              </section>

              <div className="offline-export-privacy" role="note">
                <AlertTriangle size={16} />
                <p><strong>Privacy review:</strong> cited excerpts are real source code and may contain sensitive material. Common credentials, hidden paths, and local checkout paths are filtered from text, but automatic filtering cannot catch every secret. Text filters do not alter cached audio; listen for sensitive content before sharing. Review both the text and audio selections. The package does not include provider configuration or the repository checkout.</p>
              </div>
              <label className="offline-export-confirm"><input type="checkbox" checked={privacyReviewed} onChange={(event) => setPrivacyReviewed(event.target.checked)} /><span>I reviewed the selected chapter content and understand what will be included.</span></label>
              {error && <div className="provider-error" role="alert"><AlertTriangle size={15} /><div><strong>Export failed</strong><span>{error}</span></div></div>}
            </div>

            <footer className="provider-dialog-footer offline-export-footer">
              <span>{selected.length} accepted {selected.length === 1 ? 'chapter' : 'chapters'} · {includeExcerpts ? `${preview.includedExcerptCount} excerpts` : 'excerpts omitted'}{includeNarrationAudio ? ` · ${cachedNarration.length} audio tracks` : ''}</span>
              <div><button className="secondary-button" onClick={close} disabled={exporting}>Cancel</button><button className="primary-button" onClick={() => void download()} disabled={!privacyReviewed || selected.length === 0 || exporting || (includeNarrationAudio && narrationCacheScan === 'loading')}>{exporting ? 'Preparing…' : 'Download HTML'} <Download size={14} /></button></div>
            </footer>
          </section>
        </div>
      )}
    </>
  );
}

