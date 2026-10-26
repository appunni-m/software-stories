import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  ChevronDown,
  FileCode2,
  Layers3,
  Play,
  VolumeX,
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
import { orderedSections, readerSelection, readRememberedProgress, saveRememberedProgress, resumeStorageKey, chooseSectionId } from './lib/walkthrough';
import type { WorkspaceRoute } from './lib/navigation';
import { VisualStory } from './VisualStory';
import { plainInlineCodeText } from './lib/inlineCode';
import { InlineCodeText } from './InlineCodeText';
import { ReaderProgressContext } from './lib/ReaderProgressContext';
import { sameSection, type SectionIdentity, type StoryProgress } from './lib/readerProgress';
import { answerReadingPosition } from './workspace/answerVersions';
import { presentationOwner, rememberPresentation, rememberSection, type WorkspaceState } from './workspace/model';
import { getWorkspace, prepareWorkspaceConversation, updateWorkspace, useConversationAvailability, useWorkspace } from './workspace/store';
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

function readerTurnForRevision(state: WorkspaceState, revisionId: string) {
  const current = state.explorations.find((exploration) => exploration.id === state.currentId);
  const explorations = [
    ...(current ? [current] : []),
    ...state.explorations.filter((exploration) => exploration.id !== state.currentId).reverse(),
  ];
  for (const exploration of explorations) {
    const turn = presentationOwner(exploration, revisionId);
    if (turn) return { exploration, turn };
  }
  return null;
}

function savedGuideSection(state: WorkspaceState, revisionId: string): string | null {
  const owner = readerTurnForRevision(state, revisionId);
  return owner ? answerReadingPosition(owner.turn, 'guide', revisionId)?.sectionId ?? null : null;
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
  const [progressRefreshKey, setProgressRefreshKey] = useState(0);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const chapterSelectionToken = useRef(0);
  const readerRouteIdentity = [route.workspace, route.projectId, route.snapshotId, route.chapterId, route.chapterRevisionId, route.sectionId].join('\u0000');
  const latestReaderRouteIdentity = useRef(readerRouteIdentity);
  latestReaderRouteIdentity.current = readerRouteIdentity;
  const readerMounted = useRef(true);
  useEffect(() => {
    readerMounted.current = true;
    return () => {
      readerMounted.current = false;
      chapterSelectionToken.current += 1;
    };
  }, []);

  useEffect(() => {
    let active = true;
    if (!isNativeRuntime()) {
      setRevisions([]);
      setLoading(false);
      setLoadError(null);
      return () => { active = false; };
    }
    setLoading(true);
    setLoadError(null);
    void listProjectChapterRevisions(projectId)
      .then((loaded) => { if (active) setRevisions(loaded); })
      .catch((error) => { if (active) setLoadError(errorMessageOr(error, 'Saved explanations could not be loaded.')); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [projectId, refreshKey]);
  useEffect(() => {
    const refresh = () => setRefreshKey((value) => value + 1);
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => window.removeEventListener('code-atlas:chapters-changed', refresh);
  }, []);

  const {selected:selectedRevision,library:chapterList,exportable:acceptedLibrary} = useMemo(
    () => readerSelection(revisions,route.chapterId,route.chapterRevisionId),
    [revisions,route.chapterId,route.chapterRevisionId],
  );
  const readerSnapshotId = selectedRevision?.snapshotId ?? route.snapshotId ?? selectedSnapshotId ?? '';
  const readerWorkspace = useWorkspace(projectId, readerSnapshotId);
  const readerConversationAvailability = useConversationAvailability(projectId, readerSnapshotId);
  useEffect(() => {
    if (!isNativeRuntime() || !readerSnapshotId) return;
    void prepareWorkspaceConversation(projectId, readerSnapshotId).catch(() => {});
  }, [projectId, readerSnapshotId]);
  const nativeReaderPositionReady = !isNativeRuntime() || !readerSnapshotId || readerConversationAvailability !== 'loading';
  const sections = selectedRevision ? orderedSections(selectedRevision) : [];

  const rememberedRead = useMemo(() => {
    if (!selectedRevision || isNativeRuntime()) return { status: 'missing' } as const;
    try {
      return readRememberedProgress(
        window.localStorage.getItem(resumeStorageKey(projectId, selectedRevision.chapterId, selectedRevision.revisionId)),
        projectId,
        selectedRevision.chapterId,
        selectedRevision.revisionId,
      );
    } catch {
      return { status: 'unavailable' } as const;
    }
  }, [projectId, progressRefreshKey, selectedRevision?.chapterId, selectedRevision?.revisionId]);
  const rememberedProgress = rememberedRead.status === 'available' ? rememberedRead.progress : null;
  const nativeSectionId = selectedRevision ? savedGuideSection(readerWorkspace, selectedRevision.revisionId) : null;
  const selectedSectionId = selectedRevision
    ? chooseSectionId(sections, route.sectionId, nativeSectionId ?? rememberedProgress?.sectionId)
    : null;
  useEffect(() => {
    if (!isNativeRuntime() || !nativeReaderPositionReady || !selectedRevision || !selectedSectionId) return;
    const owner = readerTurnForRevision(readerWorkspace, selectedRevision.revisionId);
    if (owner) updateWorkspace(projectId, selectedRevision.snapshotId, (state) => {
      const current = readerTurnForRevision(state, selectedRevision.revisionId);
      return current ? rememberSection(state, current.exploration.id, current.turn.id, 'guide', {
        revisionId: selectedRevision.revisionId,
        sectionId: selectedSectionId,
      }) : state;
    });
  }, [projectId, nativeReaderPositionReady, readerWorkspace, selectedRevision?.snapshotId, selectedRevision?.revisionId, selectedSectionId]);
  const selectedSectionIndex = selectedSectionId ? sections.findIndex((section) => section.sectionId === selectedSectionId) : -1;
  const selectedSection = selectedSectionIndex >= 0 ? sections[selectedSectionIndex] : null;
  const latestRevision = selectedRevision ? acceptedLibrary.find((revision) => revision.chapterId === selectedRevision.chapterId) : null;
  const isOlderRevision = Boolean(selectedRevision?.lifecycle === 'accepted' && latestRevision && latestRevision.revisionId !== selectedRevision.revisionId);
  const snapshotBaselineId = route.readerSnapshotBaselineId ?? selectedSnapshotId;
  const isOlderSnapshot = Boolean(selectedRevision && snapshotBaselineId && selectedRevision.snapshotId !== snapshotBaselineId);

  useEffect(() => {
    if (!nativeReaderPositionReady || !selectedRevision || !selectedSectionId) return;
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
  }, [projectId, route, rememberedProgress, nativeReaderPositionReady, selectedRevision?.chapterId, selectedRevision?.revisionId, selectedSectionId]);

  useEffect(() => {
    if (!selectedSectionId || !selectedRevision) return;
    const focusTarget = route.focusTarget ?? `reader-section-heading:${selectedSectionId}`;
    const focusable = Array.from(document.querySelectorAll<HTMLElement>('[data-focus-key]'))
      .find((element) => element.dataset.focusKey === focusTarget);
    (focusable ?? headingRef.current)?.focus({ preventScroll: focusTarget.startsWith('reader-citation:') || focusTarget.startsWith('reader-visual-citation:') || focusTarget.startsWith('reader-hotspot:') });
  }, [selectedRevision?.revisionId, selectedSectionId, route.focusTarget]);

  const rememberSection = (revision: ChapterRevision, sectionId: string, playbackPosition = 0, transcriptPosition = 0, replaceUnavailable = false) => {
    if (isNativeRuntime()) return false;
    try {
      const saved = saveRememberedProgress(
        window.localStorage,
        projectId,
        revision.chapterId,
        revision.revisionId,
        { sectionId, playbackPosition, transcriptPosition },
        replaceUnavailable,
      );
      if (replaceUnavailable) setProgressRefreshKey((value) => value + 1);
      return saved;
    } catch {
      // Reader navigation and route history still work when storage is unavailable.
      if (replaceUnavailable) setProgressRefreshKey((value) => value + 1);
      return false;
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
    rememberSection(revision, sectionId, 0, 0, true);
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

  const selectChapter = async (revision: ChapterRevision) => {
    const selectionToken = ++chapterSelectionToken.current;
    const originRouteIdentity = readerRouteIdentity;
    let storedProgress: ReturnType<typeof readRememberedProgress> = { status: 'missing' };
    let nativeSectionId: string | null = null;
    if (isNativeRuntime()) {
      try {
        await prepareWorkspaceConversation(projectId, revision.snapshotId);
        nativeSectionId = savedGuideSection(getWorkspace(projectId, revision.snapshotId), revision.revisionId);
      } catch {
        nativeSectionId = null;
      }
    }
    if (!readerMounted.current || selectionToken !== chapterSelectionToken.current
      || latestReaderRouteIdentity.current !== originRouteIdentity) return;
    if (!isNativeRuntime()) {
      try {
        storedProgress = readRememberedProgress(
          window.localStorage.getItem(resumeStorageKey(projectId, revision.chapterId, revision.revisionId)),
          projectId,
          revision.chapterId,
          revision.revisionId,
        );
      } catch { storedProgress = { status: 'unavailable' }; }
    }
    const rememberedSectionId = storedProgress.status === 'available'
      ? storedProgress.progress.sectionId
      : nativeSectionId;
    const sectionId = chooseSectionId(orderedSections(revision), null, rememberedSectionId);
    if (!sectionId) return;
    const playbackPosition = storedProgress.status === 'available' && storedProgress.progress.sectionId === sectionId ? storedProgress.progress.playbackPosition : 0;
    const transcriptPosition = storedProgress.status === 'available' && storedProgress.progress.sectionId === sectionId ? storedProgress.progress.transcriptPosition : 0;
    rememberSection(revision, sectionId, playbackPosition, transcriptPosition, true);
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
      rememberSection(selectedRevision, selectedSectionId, finitePosition(route.playbackPosition), finitePosition(route.transcriptPosition), true);
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

  const readerProgressAccess = useMemo(() => ({
    story: (identity: SectionIdentity) => {
      const owner = readerTurnForRevision(readerWorkspace, identity.revisionId);
      if (!owner) return undefined;
      return {
        progress: owner.turn.presentation?.stories.find((progress) => sameSection(progress, identity)),
        onProgressChange: (progress: StoryProgress) => updateWorkspace(projectId, readerSnapshotId, (state) => {
          const current = readerTurnForRevision(state, identity.revisionId);
          return current ? rememberPresentation(state, current.exploration.id, current.turn.id, 'stories', progress) : state;
        }),
      };
    },
    section: (revisionId: string) => {
      const revision = revisions.find((candidate) => candidate.revisionId === revisionId);
      if (!revision) return undefined;
      const owner = readerTurnForRevision(readerWorkspace, revisionId);
      if (!owner) return undefined;
      return {
        sectionId: answerReadingPosition(owner.turn, 'guide', revisionId)?.sectionId,
        onSectionChange: (sectionId: string) => {
          if (!revision.sections.some((section) => section.sectionId === sectionId)) return;
          updateWorkspace(projectId, readerSnapshotId, (state) => {
            const current = readerTurnForRevision(state, revisionId);
            return current ? rememberSection(state, current.exploration.id, current.turn.id, 'guide', { revisionId, sectionId }) : state;
          });
        },
      };
    },
  }), [projectId, readerSnapshotId, readerWorkspace, revisions]);

  if (loading || (isNativeRuntime() && readerSnapshotId && readerConversationAvailability === 'loading')) return <div className="empty-workspace snapshot-loading" role="status"><Layers3 size={18} /><span>Loading saved explanations…</span></div>;
  if (loadError) return (
    <div className="empty-workspace simple-empty walkthrough-empty" role="alert">
      <div className="empty-state-icon play-icon"><BookOpen size={22} /></div>
      <div className="step-pill"><span>GUIDES</span> UNAVAILABLE</div>
      <h2>Saved guides could not be loaded.</h2>
      <p>{loadError}</p>
      <button className="secondary-button" onClick={() => setRefreshKey((value) => value + 1)}>Retry <ArrowRight size={15} /></button>
    </div>
  );
  if (acceptedLibrary.length === 0 && !selectedRevision) return (
    <div className="empty-workspace simple-empty walkthrough-empty">
      <div className="empty-state-icon play-icon"><Play size={22} /></div>
      <div className="step-pill"><span>GUIDE</span> READ</div>
      <h2>Explanations stay connected to code.</h2>
      <p>Accepted explanations appear here one section at a time. Open a source link to see the captured lines, then return to the same place.</p>
      <button className="secondary-button" data-focus-key="cta:walkthrough-plan" onClick={onGoPlan}>Back to Guide <ArrowRight size={15} /></button>
      <div className="state-note"><BookOpen size={14} /> No accepted explanations in this project.</div>
    </div>
  );

  return (
    <ReaderProgressContext.Provider value={readerProgressAccess}>
    <div className="walkthrough-screen">
      <aside className="walkthrough-rail" aria-label="Explanation and section navigation">
        <div className="walkthrough-rail-heading">
          <div><span className="walkthrough-kicker">LIBRARY</span><h2>Explanations</h2></div>
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
                  <span className="walkthrough-chapter-copy"><strong><InlineCodeText text={revision.title}/>{revision.lifecycle==='draft'?' · Draft preview':''}</strong><small>{targetLabel(revision)} · {revision.sections.length} {revision.sections.length === 1 ? 'section' : 'sections'}</small></span>
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
                          <span><InlineCodeText text={section.title}/></span>
                        </button>
                      </li>
                    ))}
                  </ol>
                )}
              </div>
            );
          })}
        </div>
        <div className="walkthrough-rail-footer"><span><CheckCircle2 size={13} /> Saved on this Mac</span><OfflineExport projectId={projectId} revisions={acceptedLibrary} /></div>
      </aside>

      {!selectedRevision || !selectedSection ? (
        <main className="walkthrough-reader walkthrough-chapter-home">
          <div className="walkthrough-home-art" aria-hidden="true"><div className="walkthrough-home-orbit"><BookOpen size={28} /></div><span className="orbit-tag tag-source">SOURCE</span><span className="orbit-tag tag-section">SECTION</span><span className="orbit-tag tag-evidence">EVIDENCE</span></div>
          <span className="walkthrough-kicker">A CODE ATLAS READING ROOM</span>
          <h2>Choose an explanation to read.</h2>
          <p>Each explanation is tied to the repository snapshot it describes. Move at your own pace, inspect its source, and return without losing your place.</p>
          <div className="walkthrough-home-facts"><span><FileCode2 size={14} /> Exact source references</span><span><ArrowRight size={14} /> One manual step at a time</span><span><VolumeX size={14} /> Readable with audio off</span></div>
          <button className="text-button" onClick={onGoPlan}>Back to Guide <ArrowRight size={14} /></button>
        </main>
      ) : (
        <main className="walkthrough-reader" data-walkthrough-player onKeyDown={handleReaderKeyDown}>
          <div className="walkthrough-reader-topline">
            <div className="walkthrough-reader-breadcrumb"><span>{targetLabel(selectedRevision)}</span><span>/</span><strong><InlineCodeText text={selectedRevision.title}/></strong></div>
            <div className="walkthrough-revision-labels">
              {selectedRevision.lifecycle==='draft'&&<span className="walkthrough-revision-badge">DRAFT PREVIEW</span>}
              {isOlderRevision && <span className="walkthrough-revision-badge">OLDER ACCEPTED REVISION</span>}
              {isOlderSnapshot && <span className="walkthrough-revision-badge">PINNED TO EARLIER SNAPSHOT</span>}
            </div>
          </div>
          {!isNativeRuntime() && rememberedRead.status === 'unavailable' && (
            <div className="walkthrough-progress-warning" role="status">
              <FileCode2 size={15} aria-hidden="true" />
              <span>Saved reading position is unreadable. Your explanation is intact; choose a section to continue.</span>
            </div>
          )}
          <div className="walkthrough-reader-grid">
            <article className="walkthrough-section-content" aria-label={`Section ${selectedSectionIndex + 1} of ${sections.length}`}>
              <div className="walkthrough-section-meta"><span>SECTION {String(selectedSectionIndex + 1).padStart(2, '0')} / {String(sections.length).padStart(2, '0')}</span><span>MANUAL PACE</span></div>
              <h2 ref={headingRef} tabIndex={-1} aria-label={`Section ${selectedSectionIndex + 1} of ${sections.length}: ${selectedSection.title}`} data-focus-key={`reader-section-heading:${selectedSection.sectionId}`}><InlineCodeText text={selectedSection.title}/></h2>
              <VisualStory key={`${selectedRevision.revisionId}:${selectedSection.sectionRevisionId}`} revision={selectedRevision} section={selectedSection} activeFocusTarget={route.focusTarget} onOpenAnchor={openAnchor} onFollowHotspot={followHotspot} onSettings={onOpenProviderSettings} restorePosition={finitePosition(route.playbackPosition)} onPlaybackPositionChange={rememberPlaybackPosition} onTranscriptPositionChange={rememberTranscriptPosition} />
              <EvidencePanel section={selectedSection} onOpenAnchor={(anchor) => openAnchor(anchor)} />
              <SectionFileLinks section={selectedSection} activeFocusTarget={route.focusTarget} onFollowHotspot={followHotspot} />

              <div className="walkthrough-reader-footer">
                <span>{selectedRevision.lifecycle === 'accepted' ? 'Accepted revision' : 'Draft revision'} <code>{selectedRevision.revisionNumber}</code> · snapshot <code>{selectedRevision.snapshotId.slice(-10)}</code></span>
              </div>

            </article>

          </div>
              <nav className="walkthrough-section-actions" aria-label="Section navigation">
                <button className="secondary-button" disabled={selectedSectionIndex <= 0} onClick={() => setSection(selectedRevision, sections[selectedSectionIndex - 1]?.sectionId)}><ArrowLeft size={15} /> Previous</button>
                <span className="walkthrough-step-status">{selectedSectionIndex + 1} of {sections.length}</span>
                {selectedSectionIndex === sections.length - 1
                  ? <button className="primary-button" onClick={finishChapter}>Finish explanation <CheckCircle2 size={15} /></button>
                  : <button className="primary-button" onClick={() => setSection(selectedRevision, sections[selectedSectionIndex + 1].sectionId)}>Continue <ArrowRight size={15} /></button>}
              </nav>
        </main>
      )}
    </div>
    </ReaderProgressContext.Provider>
  );
}

function EvidencePanel({ section, onOpenAnchor }: { section: ChapterSection; onOpenAnchor: (anchor: ChapterSourceAnchor) => void }) {
  return (
    <details className="walkthrough-evidence"><summary>Inspect the evidence ({section.claims.reduce((total, claim) => total + claim.anchors.length, 0)} source links)</summary>
    <section aria-labelledby="walkthrough-evidence-title">
      <div className="walkthrough-panel-heading"><div><span className="walkthrough-kicker">SOURCE EVIDENCE</span><h3 id="walkthrough-evidence-title">What in the code supports this?</h3></div><span className="walkthrough-evidence-count">{section.claims.reduce((total, claim) => total + claim.anchors.length, 0)} REFERENCES</span></div>
      {section.claims.length === 0 ? <p className="walkthrough-muted">This section has no separate claims. Open its linked source to inspect the explanation.</p> : (
        <div className="walkthrough-claims">
          {section.claims.map((claim) => (
            <article className="walkthrough-claim" key={claim.claimId}>
              <div className="walkthrough-claim-label"><span>{claim.category.kind === 'inference' ? claim.category.visibleLabel : claim.category.kind === 'repositoryContext' ? 'REPOSITORY CONTEXT' : 'CODE BEHAVIOR'}</span></div>
              <p><InlineCodeText text={claim.prose}/></p>
              {claim.category.kind === 'inference' && claim.category.basis && <p className="walkthrough-inference-basis"><strong>Inference basis:</strong> <InlineCodeText text={claim.category.basis}/></p>}
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
    </section></details>
  );
}

function SectionFileLinks({
  section,
  activeFocusTarget,
  onFollowHotspot,
}: {
  section: ChapterSection;
  activeFocusTarget?: string;
  onFollowHotspot: (hotspot: ChapterHotspot, focusTarget?: string) => void;
}) {
  const links = section.hotspots.filter((hotspot) => hotspot.destination.kind === 'atlasFile');
  if (links.length === 0) return null;
  return (
    <section className="walkthrough-transcript-citations" aria-label="Explore cited files">
      <span>EXPLORE CITED FILES</span>
      {links.map((hotspot) => {
        const focusTarget = `reader-hotspot:${hotspot.hotspotId}`;
        return (
          <button
            className={activeFocusTarget === focusTarget ? 'is-return-focus' : undefined}
            key={hotspot.hotspotId}
            data-focus-key={focusTarget}
            aria-label={plainInlineCodeText(hotspot.label)}
            onClick={() => onFollowHotspot(hotspot, focusTarget)}
          >
            <FileCode2 size={12} />
            <InlineCodeText text={hotspot.label}/>
            <ArrowRight size={11} />
          </button>
        );
      })}
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
