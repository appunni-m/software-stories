import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  ChevronDown,
  Code2,
  FileCode2,
  Layers3,
  Play,
  VolumeX,
  Workflow,
} from 'lucide-react';
import {
  isNativeRuntime,
  listProjectChapterRevisions,
  type ChapterRevision,
  type ChapterSection,
  type ChapterSourceAnchor,
  type ChapterHotspot,
  type HotspotDestination,
} from './lib/commands';
import { orderedSections, latestAcceptedRevisions, parseRememberedProgress, rememberedSectionRecord, resumeStorageKey, chooseSectionId } from './lib/walkthrough';
import { inspectInteractiveVisual } from './lib/visuals';
import type { WorkspaceRoute } from './lib/navigation';
import { NarrationPlayer } from './NarrationPlayer';
import { OfflineExport } from './OfflineExport';
import { errorMessageOr } from './lib/errorMessage';

export interface ReaderRouteContext {
  projectId: string;
  snapshotId: string;
  chapterId: string;
  chapterRevisionId: string;
  sectionId: string;
  playbackPosition: number;
  transcriptPosition: number;
  focusTarget: string;
}

interface WalkthroughProps {
  projectId: string;
  selectedSnapshotId: string | null;
  route: WorkspaceRoute;
  onReplaceRoute: (route: WorkspaceRoute) => void;
  onNavigateRoute: (route: WorkspaceRoute) => void;
  onOpenSource: (anchor: ChapterSourceAnchor, context: ReaderRouteContext) => void;
  onOpenAtlas: (destination: HotspotDestination, context: ReaderRouteContext) => void;
  onGoPlan: () => void;
  onOpenProviderSettings: () => void;
}

export function WalkthroughScreen({ projectId, selectedSnapshotId, route, onReplaceRoute, onNavigateRoute, onOpenSource, onOpenAtlas, onGoPlan, onOpenProviderSettings }: WalkthroughProps) {
  const [revisions, setRevisions] = useState<ChapterRevision[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const headingRef = useRef<HTMLHeadingElement>(null);

  const loadRevisions = () => {
    if (!isNativeRuntime()) {
      setRevisions([]);
      setLoading(false);
      setLoadError(null);
      return;
    }
    setLoading(true);
    setLoadError(null);
    void listProjectChapterRevisions(projectId)
      .then(setRevisions)
      .catch((error) => setLoadError(errorMessageOr(error, 'Accepted chapter revisions could not be loaded.')))
      .finally(() => setLoading(false));
  };

  useEffect(() => { loadRevisions(); }, [projectId, refreshKey]);
  useEffect(() => {
    const refresh = () => setRefreshKey((value) => value + 1);
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => window.removeEventListener('code-atlas:chapters-changed', refresh);
  }, []);

  const acceptedRevisions = useMemo(
    () => revisions.filter((revision) => revision.lifecycle === 'accepted' && revision.sections.length > 0),
    [revisions],
  );
  const chapterList = useMemo(() => latestAcceptedRevisions(revisions), [revisions]);
  const routedRevision = route.chapterId && route.chapterRevisionId
    ? acceptedRevisions.find((revision) => revision.chapterId === route.chapterId && revision.revisionId === route.chapterRevisionId) ?? null
    : null;
  const selectedRevision = routedRevision
    ?? (route.chapterId ? chapterList.find((revision) => revision.chapterId === route.chapterId) ?? null : null);
  const sections = selectedRevision ? orderedSections(selectedRevision) : [];

  const rememberedProgress = useMemo(() => {
    if (!selectedRevision) return null;
    try {
      return parseRememberedProgress(
        window.localStorage.getItem(resumeStorageKey(projectId, selectedRevision.chapterId, selectedRevision.revisionId)),
        projectId,
        selectedRevision.chapterId,
        selectedRevision.revisionId,
      );
    } catch {
      return null;
    }
  }, [projectId, selectedRevision?.chapterId, selectedRevision?.revisionId]);
  const selectedSectionId = selectedRevision
    ? chooseSectionId(sections, route.sectionId, rememberedProgress?.sectionId)
    : null;
  const selectedSectionIndex = selectedSectionId ? sections.findIndex((section) => section.sectionId === selectedSectionId) : -1;
  const selectedSection = selectedSectionIndex >= 0 ? sections[selectedSectionIndex] : null;
  const latestRevision = selectedRevision ? chapterList.find((revision) => revision.chapterId === selectedRevision.chapterId) : null;
  const isOlderRevision = Boolean(selectedRevision && latestRevision && latestRevision.revisionId !== selectedRevision.revisionId);
  const snapshotBaselineId = route.readerSnapshotBaselineId ?? selectedSnapshotId;
  const isOlderSnapshot = Boolean(selectedRevision && snapshotBaselineId && selectedRevision.snapshotId !== snapshotBaselineId);

  useEffect(() => {
    if (!selectedRevision || !selectedSectionId) return;
    const routeMatches = route.chapterId === selectedRevision.chapterId
      && route.chapterRevisionId === selectedRevision.revisionId
      && route.sectionId === selectedSectionId
      && route.snapshotId === selectedRevision.snapshotId;
    if (routeMatches) return;
    onReplaceRoute({
      ...route,
      workspace: 'walkthrough',
      projectId,
      snapshotId: selectedRevision.snapshotId,
      readerSnapshotBaselineId: route.readerSnapshotBaselineId ?? selectedSnapshotId ?? selectedRevision.snapshotId,
      chapterId: selectedRevision.chapterId,
      chapterRevisionId: selectedRevision.revisionId,
      sectionId: selectedSectionId,
      playbackPosition: route.sectionId == null && rememberedProgress?.sectionId === selectedSectionId
        ? rememberedProgress.playbackPosition
        : finitePosition(route.playbackPosition),
      transcriptPosition: route.sectionId == null && rememberedProgress?.sectionId === selectedSectionId
        ? rememberedProgress.transcriptPosition
        : finitePosition(route.transcriptPosition),
      focusTarget: route.focusTarget ?? `reader-section-heading:${selectedSectionId}`,
    });
  }, [projectId, route, rememberedProgress, selectedRevision?.chapterId, selectedRevision?.revisionId, selectedSectionId]);

  useEffect(() => {
    if (!selectedSectionId || !selectedRevision) return;
    const focusTarget = route.focusTarget ?? `reader-section-heading:${selectedSectionId}`;
    const focusable = Array.from(document.querySelectorAll<HTMLElement>('[data-focus-key]'))
      .find((element) => element.dataset.focusKey === focusTarget);
    (focusable ?? headingRef.current)?.focus({ preventScroll: focusTarget.startsWith('reader-citation:') || focusTarget.startsWith('reader-visual-citation:') || focusTarget.startsWith('reader-hotspot:') });
  }, [selectedRevision?.revisionId, selectedSectionId, route.focusTarget]);

  const rememberSection = (revision: ChapterRevision, sectionId: string, playbackPosition = 0, transcriptPosition = 0) => {
    try {
      window.localStorage.setItem(
        resumeStorageKey(projectId, revision.chapterId, revision.revisionId),
        rememberedSectionRecord(projectId, revision.chapterId, revision.revisionId, sectionId, playbackPosition, transcriptPosition),
      );
    } catch {
      // Reader navigation and route history still work when storage is unavailable.
    }
  };

  const contextFor = (focusTarget: string): ReaderRouteContext | null => {
    if (!selectedRevision || !selectedSectionId) return null;
    const transcriptPosition = finitePosition(route.transcriptPosition);
    return {
      projectId,
      snapshotId: selectedRevision.snapshotId,
      chapterId: selectedRevision.chapterId,
      chapterRevisionId: selectedRevision.revisionId,
      sectionId: selectedSectionId,
      playbackPosition: finitePosition(route.playbackPosition),
      transcriptPosition: finitePosition(transcriptPosition),
      focusTarget,
    };
  };

  const rememberPlaybackPosition = (playbackPosition: number) => {
    if (selectedRevision && selectedSectionId) {
      rememberSection(selectedRevision, selectedSectionId, playbackPosition, finitePosition(route.transcriptPosition));
    }
    onReplaceRoute({ ...route, playbackPosition });
  };

  const rememberTranscriptPosition = (transcriptPosition: number) => {
    if (selectedRevision && selectedSectionId) {
      rememberSection(selectedRevision, selectedSectionId, finitePosition(route.playbackPosition), transcriptPosition);
    }
    onReplaceRoute({ ...route, transcriptPosition });
  };

  const setSection = (revision: ChapterRevision, sectionId: string, pushHistory = false, originFocusTarget?: string) => {
    const section = orderedSections(revision).find((item) => item.sectionId === sectionId);
    if (!section) return;
    rememberSection(revision, sectionId);
    const next: WorkspaceRoute = {
      ...route,
      workspace: 'walkthrough',
      projectId,
      snapshotId: revision.snapshotId,
      readerSnapshotBaselineId: route.readerSnapshotBaselineId ?? route.snapshotId ?? selectedSnapshotId ?? revision.snapshotId,
      chapterId: revision.chapterId,
      chapterRevisionId: revision.revisionId,
      sectionId,
      playbackPosition: 0,
      transcriptPosition: 0,
      focusTarget: `reader-section-heading:${sectionId}`,
    };
    if (pushHistory) {
      if (originFocusTarget) onReplaceRoute({ ...route, focusTarget: originFocusTarget });
      onNavigateRoute(next);
    } else onReplaceRoute(next);
  };

  const selectChapter = (revision: ChapterRevision) => {
    let storedProgress: ReturnType<typeof parseRememberedProgress> = null;
    try {
      storedProgress = parseRememberedProgress(
        window.localStorage.getItem(resumeStorageKey(projectId, revision.chapterId, revision.revisionId)),
        projectId,
        revision.chapterId,
        revision.revisionId,
      );
    } catch { /* Progress is optional; start at the first section. */ }
    const sectionId = chooseSectionId(orderedSections(revision), null, storedProgress?.sectionId);
    if (!sectionId) return;
    const playbackPosition = storedProgress?.sectionId === sectionId ? storedProgress.playbackPosition : 0;
    const transcriptPosition = storedProgress?.sectionId === sectionId ? storedProgress.transcriptPosition : 0;
    rememberSection(revision, sectionId, playbackPosition, transcriptPosition);
    onNavigateRoute({
      ...route,
      workspace: 'walkthrough',
      projectId,
      snapshotId: revision.snapshotId,
      readerSnapshotBaselineId: route.readerSnapshotBaselineId ?? route.snapshotId ?? selectedSnapshotId ?? revision.snapshotId,
      chapterId: revision.chapterId,
      chapterRevisionId: revision.revisionId,
      sectionId,
      playbackPosition,
      transcriptPosition,
      focusTarget: `reader-section-heading:${sectionId}`,
    });
  };

  const finishChapter = () => {
    if (selectedRevision && selectedSectionId) {
      rememberSection(selectedRevision, selectedSectionId, finitePosition(route.playbackPosition), finitePosition(route.transcriptPosition));
    }
    onNavigateRoute({
      ...route,
      workspace: 'walkthrough',
      projectId,
      snapshotId: selectedRevision?.snapshotId ?? route.snapshotId ?? selectedSnapshotId,
      readerSnapshotBaselineId: route.readerSnapshotBaselineId ?? selectedSnapshotId ?? route.snapshotId ?? undefined,
      chapterId: undefined,
      chapterRevisionId: undefined,
      sectionId: undefined,
      playbackPosition: undefined,
      transcriptPosition: undefined,
      focusTarget: 'heading:walkthrough',
    });
  };

  const openAnchor = (anchor: ChapterSourceAnchor, focusTarget = `reader-citation:${anchor.anchorId}`) => {
    const context = contextFor(focusTarget);
    if (context) onOpenSource(anchor, context);
  };

  const followHotspot = (hotspot: ChapterHotspot, focusTarget = `reader-hotspot:${hotspot.hotspotId}`) => {
    if (!selectedRevision || !selectedSection) return;
    const destination = hotspot.destination;
    if (destination.kind === 'section') {
      if (orderedSections(selectedRevision).some((section) => section.sectionId === destination.sectionId)) {
        setSection(selectedRevision, destination.sectionId, true, focusTarget);
      }
      return;
    }
    if (destination.kind === 'codeLens') {
      const anchor = selectedRevision.sections.flatMap((section) => section.claims.flatMap((claim) => claim.anchors))
        .find((candidate) => candidate.anchorId === destination.anchorId);
      if (anchor) {
        const context = contextFor(focusTarget);
        if (context) onOpenSource(anchor, context);
      }
      return;
    }
    const context = contextFor(focusTarget);
    if (context) onOpenAtlas(destination, context);
  };

  const handleReaderKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
    if (event.target instanceof HTMLElement && event.target.closest('button, a, select, summary, pre, [contenteditable="true"], .walkthrough-transcript-scroll')) return;
    if (!selectedRevision || selectedSectionIndex < 0) return;
    if (event.key === 'ArrowLeft' && selectedSectionIndex > 0) {
      event.preventDefault();
      setSection(selectedRevision, sections[selectedSectionIndex - 1].sectionId);
    } else if (event.key === 'ArrowRight' && selectedSectionIndex < sections.length - 1) {
      event.preventDefault();
      setSection(selectedRevision, sections[selectedSectionIndex + 1].sectionId);
    }
  };

  if (loading) return <div className="empty-workspace snapshot-loading" role="status"><Layers3 size={18} /><span>Loading accepted explanations from local storage…</span></div>;
  if (loadError) return (
    <div className="empty-workspace simple-empty walkthrough-empty" role="alert">
      <div className="empty-state-icon play-icon"><BookOpen size={22} /></div>
      <div className="step-pill"><span>LOCAL CHAPTERS</span> UNAVAILABLE</div>
      <h2>Saved explanations could not be loaded.</h2>
      <p>{loadError}</p>
      <button className="secondary-button" onClick={loadRevisions}>Retry <ArrowRight size={15} /></button>
    </div>
  );
  if (acceptedRevisions.length === 0) return (
    <div className="empty-workspace simple-empty walkthrough-empty">
      <div className="empty-state-icon play-icon"><Play size={22} /></div>
      <div className="step-pill"><span>03</span> INTERACTIVE LEARNING</div>
      <h2>Explanations stay connected to code.</h2>
      <p>Accepted chapters appear here as small, inspectable sections. Open a citation to read the exact captured lines, then return to the same point.</p>
      <button className="secondary-button" data-focus-key="cta:walkthrough-plan" onClick={onGoPlan}>View Guide Plan <ArrowRight size={15} /></button>
      <div className="state-note"><BookOpen size={14} /> No accepted chapters in this project.</div>
    </div>
  );

  return (
    <div className="walkthrough-screen">
      <aside className="walkthrough-rail" aria-label="Chapter and section navigation">
        <div className="walkthrough-rail-heading">
          <div><span className="walkthrough-kicker">LIBRARY</span><h2>Chapters</h2></div>
          <span className="walkthrough-count">{chapterList.length}</span>
        </div>
        <div className="walkthrough-chapter-list">
          {chapterList.map((revision) => {
            const isSelectedChapter = selectedRevision?.chapterId === revision.chapterId;
            return (
              <div className="walkthrough-chapter-group" key={revision.revisionId}>
                <button
                  className={`walkthrough-chapter${isSelectedChapter ? ' is-selected' : ''}`}
                  aria-current={isSelectedChapter ? 'page' : undefined}
                  onClick={() => selectChapter(revision)}
                >
                  <span className="walkthrough-chapter-icon"><BookOpen size={14} /></span>
                  <span className="walkthrough-chapter-copy"><strong>{revision.title}</strong><small>{targetLabel(revision)} · {revision.sections.length} {revision.sections.length === 1 ? 'section' : 'sections'}</small></span>
                  <ChevronDown size={13} className={isSelectedChapter ? '' : 'is-collapsed'} />
                </button>
                {isSelectedChapter && selectedRevision && (
                  <ol className="walkthrough-section-list" aria-label={`Sections in ${selectedRevision.title}`}>
                    {sections.map((section, index) => (
                      <li key={section.sectionId}>
                        <button
                          className={`walkthrough-section-link${section.sectionId === selectedSectionId ? ' is-current' : ''}`}
                          aria-current={section.sectionId === selectedSectionId ? 'step' : undefined}
                          data-focus-key={`reader-section-link:${section.sectionId}`}
                          onClick={() => setSection(selectedRevision, section.sectionId)}
                        >
                          <span className="walkthrough-section-index">{String(index + 1).padStart(2, '0')}</span>
                          <span>{section.title}</span>
                        </button>
                      </li>
                    ))}
                  </ol>
                )}
              </div>
            );
          })}
        </div>
        <div className="walkthrough-rail-footer"><span><CheckCircle2 size={13} /> Saved on this Mac</span><OfflineExport projectId={projectId} revisions={chapterList} /></div>
      </aside>

      {!selectedRevision || !selectedSection ? (
        <main className="walkthrough-reader walkthrough-chapter-home">
          <div className="walkthrough-home-art" aria-hidden="true"><div className="walkthrough-home-orbit"><BookOpen size={28} /></div><span className="orbit-tag tag-source">SOURCE</span><span className="orbit-tag tag-section">SECTION</span><span className="orbit-tag tag-evidence">EVIDENCE</span></div>
          <span className="walkthrough-kicker">A CODE ATLAS READING ROOM</span>
          <h2>Choose one chapter to begin.</h2>
          <p>Each chapter is an accepted explanation pinned to the repository snapshot it describes. Move at your own pace, inspect its evidence, and return without losing your place.</p>
          <div className="walkthrough-home-facts"><span><FileCode2 size={14} /> Exact source references</span><span><ArrowRight size={14} /> One manual step at a time</span><span><VolumeX size={14} /> Readable with audio off</span></div>
          <button className="text-button" onClick={onGoPlan}>Review chapter coverage <ArrowRight size={14} /></button>
        </main>
      ) : (
        <main className="walkthrough-reader" data-walkthrough-player onKeyDown={handleReaderKeyDown}>
          <div className="walkthrough-reader-topline">
            <div className="walkthrough-reader-breadcrumb"><span>{targetLabel(selectedRevision)}</span><span>/</span><strong>{selectedRevision.title}</strong></div>
            <div className="walkthrough-revision-labels">
              {isOlderRevision && <span className="walkthrough-revision-badge">OLDER ACCEPTED REVISION</span>}
              {isOlderSnapshot && <span className="walkthrough-revision-badge">PINNED TO EARLIER SNAPSHOT</span>}
            </div>
          </div>
          <div className="walkthrough-reader-grid">
            <article className="walkthrough-section-content" aria-label={`Section ${selectedSectionIndex + 1} of ${sections.length}`}>
              <div className="walkthrough-section-meta"><span>SECTION {String(selectedSectionIndex + 1).padStart(2, '0')} / {String(sections.length).padStart(2, '0')}</span><span>MANUAL PACE</span></div>
              <h2 ref={headingRef} tabIndex={-1} aria-label={`Section ${selectedSectionIndex + 1} of ${sections.length}: ${selectedSection.title}`} data-focus-key={`reader-section-heading:${selectedSection.sectionId}`}>{selectedSection.title}</h2>
              <div className="walkthrough-boundary walkthrough-boundary-start"><span className="walkthrough-boundary-mark" /><span>Section start · Read and inspect at your own pace.</span></div>
              <p className="walkthrough-section-prose">{selectedSection.prose}</p>

              <EvidencePanel section={selectedSection} onOpenAnchor={(anchor) => openAnchor(anchor)} />
              <VisualPanel revision={selectedRevision} section={selectedSection} activeFocusTarget={route.focusTarget} onFollowHotspot={followHotspot} onOpenAnchor={openAnchor} />

              <NarrationPlayer
                key={`${selectedRevision.revisionId}:${selectedSection.sectionId}`}
                projectId={projectId}
                chapterId={selectedRevision.chapterId}
                revisionId={selectedRevision.revisionId}
                section={selectedSection}
                restorePosition={finitePosition(route.playbackPosition)}
                transcriptPosition={finitePosition(route.transcriptPosition)}
                onPlaybackPositionChange={rememberPlaybackPosition}
                onTranscriptPositionChange={rememberTranscriptPosition}
                onOpenProviderSettings={onOpenProviderSettings}
              />
              {selectedSection.claims.some((claim) => claim.anchors.length > 0) && <div className="walkthrough-transcript-citations" role="group" aria-label="Source references for this section">
                  <span>SECTION SOURCES · NOT MAPPED TO INDIVIDUAL TRANSCRIPT LINES</span>
                  {[...new Map(selectedSection.claims.flatMap((claim) => claim.anchors).map((anchor) => [anchor.anchorId, anchor])).values()].map((anchor) => <button key={`transcript-${anchor.anchorId}`} data-focus-key={`reader-transcript-citation:${anchor.anchorId}`} onClick={() => openAnchor(anchor, `reader-transcript-citation:${anchor.anchorId}`)}><FileCode2 size={12} /><code>{anchor.path}:{anchor.startLine}-{anchor.endLine}</code><ArrowRight size={11} /></button>)}
                </div>}

              <div className="walkthrough-reader-footer">
                <span>Accepted revision <code>{selectedRevision.revisionNumber}</code> · snapshot <code>{selectedRevision.snapshotId.slice(-10)}</code></span>
              </div>
              <div className="walkthrough-boundary walkthrough-boundary-end"><span className="walkthrough-boundary-mark" /><span>End of section · Continue when you are ready.</span></div>
              <nav className="walkthrough-section-actions" aria-label="Section navigation">
                <button className="secondary-button" disabled={selectedSectionIndex <= 0} onClick={() => setSection(selectedRevision, sections[selectedSectionIndex - 1]?.sectionId)}><ArrowLeft size={15} /> Previous</button>
                {selectedSectionIndex === sections.length - 1
                  ? <button className="primary-button" onClick={finishChapter}>Finish chapter <CheckCircle2 size={15} /></button>
                  : <button className="primary-button" onClick={() => setSection(selectedRevision, sections[selectedSectionIndex + 1].sectionId)}>Continue <ArrowRight size={15} /></button>}
              </nav>
            </article>
            <aside className="walkthrough-side-notes" aria-label="Reader progress and controls">
              <div className="walkthrough-progress-card"><span className="walkthrough-kicker">YOUR PLACE</span><div className="walkthrough-progress-value">{String(selectedSectionIndex + 1).padStart(2, '0')}<span> / {String(sections.length).padStart(2, '0')}</span></div><div className="walkthrough-progress-track"><span style={{ width: `${((selectedSectionIndex + 1) / sections.length) * 100}%` }} /></div><p>{selectedSectionIndex === sections.length - 1 ? 'Last section. Finish when this idea is clear.' : 'No timer. You choose when to continue.'}</p></div>
              <div className="walkthrough-side-card"><div className="walkthrough-side-icon"><Code2 size={15} /></div><strong>Follow the evidence</strong><p>Open any cited file to inspect the exact captured lines. Return here to continue this section.</p></div>
              <div className="walkthrough-side-card"><div className="walkthrough-side-icon"><VolumeX size={15} /></div><strong>Works without audio</strong><p>Every explanation and transcript stays available when narration is missing or turned off.</p></div>
              <div className="walkthrough-key-hint"><kbd>←</kbd><kbd>→</kbd><span>Move between sections</span></div>
            </aside>
          </div>
        </main>
      )}
    </div>
  );
}

function EvidencePanel({ section, onOpenAnchor }: { section: ChapterSection; onOpenAnchor: (anchor: ChapterSourceAnchor) => void }) {
  return (
    <section className="walkthrough-evidence" aria-labelledby="walkthrough-evidence-title">
      <div className="walkthrough-panel-heading"><div><span className="walkthrough-kicker">SOURCE EVIDENCE</span><h3 id="walkthrough-evidence-title">What in the code supports this?</h3></div><span className="walkthrough-evidence-count">{section.claims.reduce((total, claim) => total + claim.anchors.length, 0)} REFERENCES</span></div>
      {section.claims.length === 0 ? <p className="walkthrough-muted">This accepted section has no separate claims. Use its chapter source to inspect the explanation.</p> : (
        <div className="walkthrough-claims">
          {section.claims.map((claim) => (
            <article className="walkthrough-claim" key={claim.claimId}>
              <div className="walkthrough-claim-label"><span>{claim.category.kind === 'inference' ? claim.category.visibleLabel : claim.category.kind === 'repositoryContext' ? 'REPOSITORY CONTEXT' : 'CODE BEHAVIOR'}</span></div>
              <p>{claim.prose}</p>
              {claim.category.kind === 'inference' && claim.category.basis && <p className="walkthrough-inference-basis"><strong>Inference basis:</strong> {claim.category.basis}</p>}
              {claim.anchors.length > 0 ? claim.anchors.map((anchor) => (
                <div className="walkthrough-source-citation" key={anchor.anchorId}>
                  <div className="walkthrough-citation-heading"><FileCode2 size={13} /><code>{anchor.path}:{anchor.startLine}-{anchor.endLine}</code></div>
                  <pre>{anchor.excerpt}</pre>
                  <button className="walkthrough-citation-link" data-focus-key={`reader-citation:${anchor.anchorId}`} onClick={() => onOpenAnchor(anchor)}>Open exact source <ArrowRight size={13} /></button>
                </div>
              )) : <p className="walkthrough-muted walkthrough-no-anchor">No source anchor is attached to this claim.</p>}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

export function VisualPanel({ revision, section, activeFocusTarget, onFollowHotspot, onOpenAnchor }: {
  revision: ChapterRevision;
  section: ChapterSection;
  activeFocusTarget?: string;
  onFollowHotspot: (hotspot: ChapterHotspot, focusTarget?: string) => void;
  onOpenAnchor: (anchor: ChapterSourceAnchor, focusTarget?: string) => void;
}) {
  const inspection = inspectInteractiveVisual(revision, section);
  if (inspection.status !== 'available') {
    const message = inspection.status === 'none'
      ? 'No diagram is attached to this section. The explanation and source evidence remain available.'
      : inspection.reason === 'legacy'
        ? 'This older diagram is retained with the accepted chapter, but it does not have interactive source links. The explanation and evidence remain available.'
        : 'This diagram does not match the safe interactive format, so it is unavailable. The explanation and source evidence remain available.';
    return (
    <section className="walkthrough-visual-slot" aria-label="Section visual">
      <div className="walkthrough-panel-heading"><div><span className="walkthrough-kicker">VISUAL MAP</span><h3>Source stays in view</h3></div></div>
      <div className="walkthrough-no-visual" role={inspection.status === 'none' ? undefined : 'status'}><Workflow size={17} /><span>{message}</span></div>
    </section>
  );
  }
  const { visual, hotspotsById, anchorsById } = inspection;
  const nodesById = new Map(visual.nodes.map((node) => [node.nodeId, node]));
  const directionClass = visual.layoutHint.direction === 'leftToRight'
    ? 'is-left-to-right'
    : visual.layoutHint.direction === 'topToBottom' ? 'is-top-to-bottom' : 'is-auto';
  return (
    <section className="walkthrough-visual-slot" aria-labelledby={`visual-title-${section.sectionId}`}>
      <div className="walkthrough-panel-heading"><div><span className="walkthrough-kicker">VISUAL MAP · {visual.kind.toUpperCase()}</span><h3 id={`visual-title-${section.sectionId}`}>{visual.title}</h3></div><span className="walkthrough-evidence-count">{visual.nodes.length} NODES · {visual.edges.length} LINKS</span></div>
      <div className={`walkthrough-diagram ${directionClass}`} role="group" aria-label={`${visual.kind} diagram for ${visual.title}`}>
        {visual.nodes.map((node, index) => {
          const hotspot = node.hotspotId ? hotspotsById.get(node.hotspotId) : undefined;
          const hotspotFocusTarget = hotspot ? `reader-hotspot:${hotspot.hotspotId}:${node.nodeId}` : undefined;
          const outgoing = visual.edges.filter((edge) => edge.fromNodeId === node.nodeId);
          return (
            <div className="walkthrough-diagram-step" key={node.nodeId}>
              <article className={`walkthrough-diagram-node${hotspot ? ' is-linked' : ''}`}>
                <span className="walkthrough-diagram-node-number">{String(index + 1).padStart(2, '0')}</span>
                <strong>{node.label}</strong>
                {node.detail && <small>{node.detail}</small>}
                {node.evidenceAnchorIds.length > 0 && <div className="walkthrough-visual-evidence" role="group" aria-label={`Evidence for ${node.label}`}>
                  {node.evidenceAnchorIds.map((anchorId) => {
                    const anchor = anchorsById.get(anchorId)!;
                    const focusKey = `reader-visual-citation:${node.nodeId}:${anchorId}`;
                    return <button className={activeFocusTarget === focusKey ? 'is-return-focus' : undefined} key={anchorId} data-focus-key={focusKey} onClick={() => onOpenAnchor(anchor, focusKey)}><FileCode2 size={11} /><code>{anchor.path}:{anchor.startLine}-{anchor.endLine}</code></button>;
                  })}
                </div>}
                {hotspot && hotspotFocusTarget && <button className={`walkthrough-diagram-action${activeFocusTarget === hotspotFocusTarget ? ' is-return-focus' : ''}`} data-focus-key={hotspotFocusTarget} onClick={() => onFollowHotspot(hotspot, hotspotFocusTarget)}>{hotspot.label} <ArrowRight size={12} /></button>}
              </article>
              {outgoing.map((edge) => {
                const target = nodesById.get(edge.toNodeId)!;
                return <div className="walkthrough-diagram-edge" key={edge.edgeId}>
                  <div className="walkthrough-diagram-edge-route"><span>{edge.label}</span><ArrowRight size={13} /><strong>{target.label}</strong></div>
                  <div className="walkthrough-visual-evidence" role="group" aria-label={`Evidence for ${edge.label}`}>
                    {edge.evidenceAnchorIds.map((anchorId) => {
                      const anchor = anchorsById.get(anchorId)!;
                      const focusKey = `reader-visual-citation:${edge.edgeId}:${anchorId}`;
                      return <button className={activeFocusTarget === focusKey ? 'is-return-focus' : undefined} key={anchorId} data-focus-key={focusKey} onClick={() => onOpenAnchor(anchor, focusKey)}><FileCode2 size={11} /><code>{anchor.path}:{anchor.startLine}-{anchor.endLine}</code></button>;
                    })}
                  </div>
                </div>;
              })}
            </div>
          );
        })}
      </div>
      <p className="walkthrough-visual-footnote">Part of accepted revision {revision.revisionNumber}. Source links open the exact evidence; hotspot actions follow destinations in this revision.</p>
    </section>
  );
}

function targetLabel(revision: ChapterRevision): string {
  if (revision.target.scope.kind === 'overview') return 'Repository overview';
  return revision.target.scope.kind === 'file' ? revision.target.scope.path : `${revision.target.scope.paths.length} files`;
}

function finitePosition(value: number | undefined): number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : 0;
}
