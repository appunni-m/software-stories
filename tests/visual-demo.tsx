import { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ArrowLeft, BookOpen, Code2, FileCode2 } from 'lucide-react';
import { VisualPanel } from '../src/Walkthrough';
import type { ChapterHotspot, ChapterRevision, ChapterSection, ChapterSourceAnchor } from '../src/lib/commands';
import type { ReaderRouteContext } from '../src/Walkthrough';
import '../src/App.css';

const anchors: ChapterSourceAnchor[] = [
  {
    anchorId: 'anchor:handler', path: 'src/api/handler.ts', symbolId: 'symbol:handleRequest', snapshotId: 'snapshot:demo', sourceSha256: 'a'.repeat(64),
    startByte: 410, endByteInclusive: 486, startLine: 18, endLine: 20, startColumn: 3, endColumn: 14,
    excerpt: 'const input = parseInput(request.body);\nreturn await runWorker(input);',
  },
  {
    anchorId: 'anchor:parser', path: 'src/api/parser.ts', symbolId: 'symbol:parseInput', snapshotId: 'snapshot:demo', sourceSha256: 'b'.repeat(64),
    startByte: 92, endByteInclusive: 168, startLine: 7, endLine: 9, startColumn: 3, endColumn: 8,
    excerpt: 'if (!isValid(value)) throw new InvalidRequest();\nreturn normalize(value);',
  },
  {
    anchorId: 'anchor:worker', path: 'src/workers/run.ts', symbolId: 'symbol:runWorker', snapshotId: 'snapshot:demo', sourceSha256: 'c'.repeat(64),
    startByte: 220, endByteInclusive: 311, startLine: 12, endLine: 15, startColumn: 3, endColumn: 2,
    excerpt: 'const result = await engine.execute(input);\nawait results.save(result);\nreturn result;',
  },
];

const hotspots: ChapterHotspot[] = [
  { hotspotId: 'hotspot:handler', label: 'Open handler code', destination: { kind: 'codeLens', anchorId: 'anchor:handler' } },
  { hotspotId: 'hotspot:parser', label: 'Open parser in Atlas', destination: { kind: 'atlasFile', path: 'src/api/parser.ts' } },
  { hotspotId: 'hotspot:worker', label: 'Continue to worker section', destination: { kind: 'section', sectionId: 'section:worker' } },
];

const section: ChapterSection = {
  sectionId: 'section:request', sectionRevisionId: 'section-revision:request', order: 0,
  title: 'What happens to an incoming request?',
  prose: 'The handler parses and normalizes the request body, then awaits the worker. The exact source links below let you check each step against the captured code.',
  spokenText: '',
  claims: anchors.map((anchor, index) => ({
    claimId: `claim:${index}`, category: { kind: 'codeBehavior' },
    prose: `Verified implementation step ${index + 1}.`, anchors: [anchor], dependencies: [],
  })),
  dependencies: [],
  visual: {
    schemaVersion: 2, kind: 'dataFlow', title: 'Request to worker', layoutHint: { direction: 'leftToRight' },
    nodes: [
      { nodeId: 'node:handler', label: 'handleRequest', detail: 'HTTP entry point', hotspotId: 'hotspot:handler', evidenceAnchorIds: ['anchor:handler'] },
      { nodeId: 'node:parser', label: 'parseInput', detail: 'Validates and normalizes the body', hotspotId: 'hotspot:parser', evidenceAnchorIds: ['anchor:parser'] },
      { nodeId: 'node:worker', label: 'runWorker', detail: 'Executes work and saves the result', hotspotId: 'hotspot:worker', evidenceAnchorIds: ['anchor:worker'] },
    ],
    edges: [
      { edgeId: 'edge:parse', fromNodeId: 'node:handler', toNodeId: 'node:parser', label: 'parses body', evidenceAnchorIds: ['anchor:handler'] },
      { edgeId: 'edge:execute', fromNodeId: 'node:parser', toNodeId: 'node:worker', label: 'awaits worker', evidenceAnchorIds: ['anchor:worker'] },
    ],
  },
  hotspots,
};

const nextSection: ChapterSection = {
  sectionId: 'section:worker', sectionRevisionId: 'section-revision:worker', order: 1,
  title: 'What does the worker do?', prose: 'The worker executes the request and saves its result.', spokenText: '',
  claims: [{ claimId: 'claim:worker', category: { kind: 'codeBehavior' }, prose: 'The worker executes and persists the result.', anchors: [anchors[2]], dependencies: [] }],
  dependencies: [], visual: null, hotspots: [],
};

const revision: ChapterRevision = {
  schemaVersion: 2, chapterId: 'chapter:request', revisionId: 'revision:demo', revisionNumber: 3,
  projectId: 'project:demo', snapshotId: 'snapshot:demo', target: { targetId: 'target:handler', scope: { kind: 'file', path: 'src/api/handler.ts' } },
  title: 'How request processing works', lifecycle: 'accepted', validationStatus: 'valid', createdAtUnixMs: 1,
  sections: [section, nextSection],
};

type DemoScreen = { kind: 'walkthrough'; sectionId: string } | { kind: 'destination'; title: string; path: string; excerpt: string };
type DemoReturnEntry = { screen: DemoScreen; context: ReaderRouteContext; focusTarget: string };

function InteractiveDiagramFixture() {
  const [screen, setScreen] = useState<DemoScreen>({ kind: 'walkthrough', sectionId: section.sectionId });
  const [invalidHotspot, setInvalidHotspot] = useState(false);
  const [focusTarget, setFocusTarget] = useState<string | undefined>();
  const [readerContext, setReaderContext] = useState<ReaderRouteContext>({
    projectId: revision.projectId, snapshotId: revision.snapshotId, chapterId: revision.chapterId,
    chapterRevisionId: revision.revisionId, sectionId: section.sectionId,
    playbackPosition: 18, transcriptPosition: 243, focusTarget: '',
  });
  const [returnEntry, setReturnEntry] = useState<DemoReturnEntry | null>(null);

  useEffect(() => {
    if (screen.kind !== 'walkthrough' || !focusTarget) return;
    const target = Array.from(document.querySelectorAll<HTMLElement>('[data-focus-key]'))
      .find((element) => element.dataset.focusKey === focusTarget);
    target?.focus();
  }, [screen.kind, screen.kind === 'walkthrough' ? screen.sectionId : '', focusTarget]);

  const openAnchor = (anchor: ChapterSourceAnchor, returnFocus: string) => {
    setReturnEntry({ screen, context: readerContext, focusTarget: returnFocus });
    setReaderContext({ ...readerContext, focusTarget: returnFocus });
    setFocusTarget(undefined);
    setScreen({ kind: 'destination', title: 'Code Lens · exact source range', path: `${anchor.path}:${anchor.startLine}-${anchor.endLine}`, excerpt: anchor.excerpt });
  };
  const followHotspot = (hotspot: ChapterHotspot, returnFocus: string) => {
    const destination = hotspot.destination;
    if (destination.kind === 'codeLens') {
      const anchor = revision.sections.flatMap((item) => item.claims.flatMap((claim) => claim.anchors)).find((candidate) => candidate.anchorId === destination.anchorId);
      if (anchor) openAnchor(anchor, returnFocus);
      return;
    }
    if (destination.kind === 'section') {
      const next = revision.sections.find((item) => item.sectionId === destination.sectionId);
      if (!next) return;
      setReturnEntry({ screen, context: readerContext, focusTarget: returnFocus });
      setReaderContext({ ...readerContext, sectionId: next.sectionId, focusTarget: returnFocus });
      setFocusTarget(undefined);
      setScreen({ kind: 'walkthrough', sectionId: next.sectionId });
      return;
    }
    setReturnEntry({ screen, context: readerContext, focusTarget: returnFocus });
    setReaderContext({ ...readerContext, focusTarget: returnFocus });
    setFocusTarget(undefined);
    setScreen({ kind: 'destination', title: 'Atlas · selected file', path: destination.kind === 'atlasFile' ? destination.path : 'src/workers/run.ts', excerpt: 'The destination is selected within the same snapshot.' });
  };
  const goBack = () => {
    if (!returnEntry) return;
    setReaderContext(returnEntry.context);
    setFocusTarget(returnEntry.focusTarget);
    setScreen(returnEntry.screen);
    setReturnEntry(null);
  };
  const activeSectionId = screen.kind === 'walkthrough' ? screen.sectionId : section.sectionId;
  const activeSection = revision.sections.find((item) => item.sectionId === activeSectionId) ?? section;
  const sectionIndex = revision.sections.findIndex((item) => item.sectionId === activeSection.sectionId);
  const activeVisualSection: ChapterSection = invalidHotspot && activeSection.sectionId === section.sectionId
    ? {
      ...activeSection,
      visual: {
        ...activeSection.visual!,
        nodes: activeSection.visual!.nodes.map((node) => node.nodeId === 'node:worker' ? { ...node, hotspotId: 'hotspot:missing' } : node),
      },
    }
    : activeSection;
  const continueFocusKey = 'reader-hotspot:demo-continue-worker-section';

  return (
    <main className="visual-demo-shell">
      <header className="visual-demo-heading">
        <div><span className="walkthrough-kicker">LOCAL PLAYER FIXTURE</span><h1>Interactive code walkthrough</h1></div>
        <span className="visual-demo-badge"><BookOpen size={13} /> Accepted revision 3</span>
      </header>
      <p className="visual-demo-intro">A small repo-level story, grounded in exact evidence. Click a source chip or hotspot, then return to the exact control you used.</p>
      {screen.kind === 'walkthrough' ? (
        <div className="visual-demo-reader">
          <div className="visual-demo-section-meta"><span>SECTION {String(sectionIndex + 1).padStart(2, '0')} / {String(revision.sections.length).padStart(2, '0')}</span><span>MANUAL PACE · NO AUTO-ADVANCE</span></div>
          <h2>{activeSection.title}</h2>
          <p className="walkthrough-section-prose">{activeSection.prose}</p>
          {activeSection.sectionId === section.sectionId && (
            <button className="visual-demo-toggle secondary-button" onClick={() => setInvalidHotspot((value) => !value)}>
              {invalidHotspot ? 'Show valid diagram' : 'Show invalid hotspot fallback'}
            </button>
          )}
          {activeSection.sectionId === section.sectionId && (
            <VisualPanel
              revision={revision}
              section={activeVisualSection}
              activeFocusTarget={focusTarget}
              onOpenAnchor={(anchor, focus) => openAnchor(anchor, focus ?? `reader-citation:${anchor.anchorId}`)}
              onFollowHotspot={(hotspot, focus) => followHotspot(hotspot, focus ?? `reader-hotspot:${hotspot.hotspotId}`)}
            />
          )}
          <div className="visual-demo-context"><Code2 size={13} /><span>Current reader state: {readerContext.sectionId} · <code>00:{String(readerContext.playbackPosition).padStart(2, '0')}</code> playback · transcript scroll <code>{readerContext.transcriptPosition}px</code>.</span></div>
          <div className="visual-demo-actions">
            <button disabled={!returnEntry} className="secondary-button" onClick={goBack}><ArrowLeft size={14} /> Previous</button>
            {activeSection.sectionId === section.sectionId && (
              <button className="primary-button" data-focus-key={continueFocusKey} onClick={() => followHotspot(hotspots[2], continueFocusKey)}>Continue to worker section <ArrowLeft size={14} className="visual-demo-rotate" /></button>
            )}
          </div>
        </div>
      ) : (
        <section className="visual-demo-destination" aria-labelledby="visual-demo-destination-heading">
          <span className="walkthrough-kicker">{screen.title}</span>
          <h2 id="visual-demo-destination-heading">{screen.path}</h2>
          <pre>{screen.excerpt}</pre>
          <p>Saved reader state: {returnEntry?.context.sectionId} · playback at {returnEntry?.context.playbackPosition}s · transcript at {returnEntry?.context.transcriptPosition}px. Back restores that state and the control that opened this destination.</p>
          <button className="primary-button" onClick={goBack}><ArrowLeft size={14} /> Back to the exact walkthrough control</button>
        </section>
      )}
      <footer className="visual-demo-footer"><FileCode2 size={12} /> Demo fixture; the production player reads accepted local chapter revisions.</footer>
    </main>
  );
}

const root = document.getElementById('visual-demo-root');
if (root) createRoot(root).render(<InteractiveDiagramFixture />);
