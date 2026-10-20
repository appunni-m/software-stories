import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import { viewportClipsNodeBounds } from '../src/lib/flowViewport.ts';
import { readSectionStoryProgress, readStoryProgress, sceneNodeIds, sourceAnchorForVisualNode, sourceValuesByVisualNode, storyLayout, storyScenes } from '../src/lib/visualStory.ts';

const claim = (id, prose, category = { kind: 'codeBehavior' }) => ({ claimId: id, prose, category, anchors: [{ anchorId: `a:${id}` }] });
const claims = [claim('entry', 'A request reaches handleRequest. The bytes are still untrusted.'), claim('check', 'If validate rejects those bytes, the caller receives an error. Otherwise processing continues.')];
const section = { sectionId: 'section:one', title: 'Follow a request', claims, spokenText: claims.map((item) => item.prose).join(' ') };
const visual = {
  schemaVersion: 2, kind: 'dataFlow', title: 'Follow a request', layoutHint: { direction: 'topToBottom' },
  nodes: ['entry', 'check', 'error', 'result'].map((id) => ({ nodeId: id, label: id, evidenceAnchorIds: [id === 'entry' ? 'a:entry' : 'a:check'] })),
  edges: [ ['entry', 'check'], ['check', 'error'], ['check', 'result'] ].map(([fromNodeId, toNodeId], i) => ({ edgeId: `e:${i}`, fromNodeId, toNodeId, label: 'branch' })),
};

describe('visual story scenes', () => {
  it('opens a visual node on its unique literal expression inside the cited snapshot bytes', () => {
    const excerpt = [
      'impl Font {',
      '    pub(crate) fn memory_face(data: &[u8]) -> Result<Self, FontError> {',
      '        if data.starts_with(b"PFR0") {',
      '            return Self::pfr_face(data, face_index, size_pt);',
      '        }',
      '    }',
      '}',
    ].join('\n');
    const anchor = {
      anchorId: 'anchor:memory-face', path: 'src/font.rs', symbolId: 'symbol:memory-face', snapshotId: 'snapshot:one',
      sourceSha256: 'a'.repeat(64), startByte: 300, endByteInclusive: 299 + Buffer.byteLength(excerpt),
      startLine: 4053, endLine: 4059, startColumn: 1, endColumn: 1, excerpt,
    };
    const narrowed = sourceAnchorForVisualNode(anchor, {
      label: 'PFR0 prefix', detail: 'Branch test: data.starts_with(b"PFR0").',
    });
    const expression = 'data.starts_with(b"PFR0")';
    const expressionOffset = excerpt.indexOf(expression);
    assert.equal(narrowed.startLine, 4055);
    assert.equal(narrowed.endLine, 4055);
    assert.equal(narrowed.startByte, anchor.startByte + Buffer.byteLength(excerpt.slice(0, expressionOffset)));
    assert.equal(narrowed.endByteInclusive, narrowed.startByte + Buffer.byteLength(expression) - 1);
    assert.equal(narrowed.excerpt, expression);
    assert.equal(excerpt.slice(expressionOffset, expressionOffset + expression.length), narrowed.excerpt);
  });

  it('opens a composed source-value cue on the exact expression it displays', () => {
    const excerpt = [
      'impl Font {',
      '    pub(crate) fn memory_face(data: &[u8]) -> Result<Self, FontError> {',
      '        if data.starts_with(b"PFR0") {',
      '            return Self::pfr_face(data, face_index, size_pt);',
      '        }',
      '    }',
      '}',
    ].join('\n');
    const anchor = {
      anchorId: 'anchor:memory-face', path: 'src/font.rs', symbolId: 'symbol:memory-face', snapshotId: 'snapshot:one',
      sourceSha256: 'a'.repeat(64), startByte: 300, endByteInclusive: 299 + Buffer.byteLength(excerpt),
      startLine: 4053, endLine: 4059, startColumn: 1, endColumn: 1, excerpt,
    };
    const value = 'Self::pfr_face(data, face_index, size_pt)';
    const narrowed = sourceAnchorForVisualNode(anchor, {
      label: 'Arguments forwarded', detail: 'The matching branch calls the constructor with the original byte slice, face index, and point size.',
    }, value);
    const expressionOffset = excerpt.indexOf(value);
    assert.equal(narrowed.startLine, 4056);
    assert.equal(narrowed.endLine, 4056);
    assert.equal(narrowed.startByte, anchor.startByte + Buffer.byteLength(excerpt.slice(0, expressionOffset)));
    assert.equal(narrowed.endByteInclusive, narrowed.startByte + Buffer.byteLength(value) - 1);
    assert.equal(narrowed.excerpt, value);
  });

  it('passes only code values from the rendered composition into source navigation', () => {
    const component = readFileSync(new URL('../src/HtmlScene.tsx', import.meta.url), 'utf8');
    assert.match(component, /const sourceValue = e\.valueOrigin === 'source' \? value : undefined;/u);
    assert.match(component, /onClick=\{\(\) => onOpenNode\(node\.nodeId, sourceValue\)\}/u);
    assert.match(component, /onOpenNode\(node\.nodeId, sourceValue\)/u);
  });

  it('uses an unambiguous composed source value when a graph node opens source', () => {
    const expression = 'Self::pfr_face(data, face_index, size_pt)';
    const presentation = (values) => ({ schemaVersion: 3, scenes: values.map((value, claimIndex) => ({
      claimIndex, title: 'Pass the original input', narration: 'The matching branch forwards its input.', mode: 'composition',
      elements: [{ kind: 'code', parentIndex: null, nodeIndex: 2, tone: 'accent', value, valueOrigin: 'source' }],
    })) });
    const graphVisual = { ...visual, presentation: presentation([expression]) };
    assert.equal(sourceValuesByVisualNode(graphVisual).get('error'), expression);

    const conflicting = { ...visual, presentation: presentation([expression, 'Self::other_face(data)']) };
    assert.equal(sourceValuesByVisualNode(conflicting).has('error'), false);

    const component = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    assert.match(component, /onClick=\{\(\) => chooseNode\(node\.id, sourceValues\.get\(node\.id\)\)\}/u);
  });

  it('keeps the full citation when a visual node has no unique exact expression', () => {
    const excerpt = 'if matches(value) { return first(); }\nif matches(value) { return second(); }';
    const anchor = {
      anchorId: 'anchor:ambiguous', path: 'src/example.rs', symbolId: 'symbol:example', snapshotId: 'snapshot:one',
      sourceSha256: 'b'.repeat(64), startByte: 10, endByteInclusive: 9 + Buffer.byteLength(excerpt),
      startLine: 20, endLine: 21, startColumn: 1, endColumn: 1, excerpt,
    };
    assert.equal(sourceAnchorForVisualNode(anchor, { label: 'Select a parser', detail: 'The bytes choose a parser.' }), anchor);
    assert.equal(sourceAnchorForVisualNode(anchor, { label: 'Parser match', detail: 'The code calls `matches(value)`.' }), anchor);
    const malformed = { ...anchor, endByteInclusive: anchor.endByteInclusive + 1 };
    assert.equal(sourceAnchorForVisualNode(malformed, {
      label: 'Parser match', detail: 'The call is `first()`.'
    }), malformed);
  });

  it('keeps the current scene title and player together while the story body scrolls', () => {
    const component = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    assert.match(component, /className="story-sticky-controls"[\s\S]*?className="story-toolbar"[\s\S]*?className="story-scene-title" title=\{scene\.title\}[\s\S]*?className="story-playbar"/u);
    assert.match(component, /aria-label=\{`Scene \$\{i \+ 1\}: \$\{item\.title\}`\}/u);
    assert.match(styles, /\.story-sticky-controls\s*\{[^}]*position:\s*sticky;[^}]*top:\s*0;/u);
    const storyRule=styles.match(/\.visual-story\s*\{([^}]+)\}/u)?.[1]??'';
    assert.doesNotMatch(storyRule,/overflow:\s*(?:clip|hidden|auto|scroll)/u,'the story container must not clip its sticky controls');
    assert.match(styles,/\.visual-story > \.story-stage:not\(\.story-html-stage\)\s*\{[^}]*overflow:\s*hidden/u,'only the diagram stage clips to rounded corners');
  });

  it('gives the visual enough bounded height on large windows and scales down responsively', () => {
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const desktopRule = styles.match(/\.story-stage\s*\{([^}]+)\}/u)?.[1] ?? '';
    const smallScreenRule = styles.match(/@media\s*\(max-width:\s*680px\)\s*\{\s*\.story-stage\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(desktopRule, /height:\s*clamp\(320px,\s*48vh,\s*720px\)/u);
    assert.match(smallScreenRule, /height:\s*clamp\(260px,\s*42vh,\s*520px\)/u);
  });

  it('reflows Guide cards and narration controls in split views and at high zoom', () => {
    const workspace = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const explanationRule = workspace.match(/\.dw-explanation\s*\{([^}]+)\}/u)?.[1] ?? '';
    const answerRule = workspace.match(/\.dw-answer-scroll\s*\{([^}]+)\}/u)?.[1] ?? '';
    const compactRules = styles.match(/@container\s+guide-pane\s*\(max-width:\s*700px\)\s*\{([\s\S]*?)\n\}/u)?.[1] ?? '';
    assert.match(explanationRule, /container-type:\s*inline-size/u);
    assert.match(explanationRule, /container-name:\s*guide-pane/u);
    assert.match(answerRule, /min-width:\s*0/u);
    assert.match(answerRule, /box-sizing:\s*border-box/u);
    assert.match(compactRules, /\.story-scene-title\s*\{[^}]*white-space:\s*normal/u);
    assert.match(compactRules, /\.story-playbar \.is-story-audio\s*\{[^}]*grid-template-areas:\s*"status status" "playback playback" "read edit"/u);
    assert.match(compactRules, /\.is-story-audio \.walkthrough-playback-buttons\s*\{[^}]*flex-wrap:\s*wrap/u);
    assert.match(styles, /\.visual-story\s*\{[^}]*max-width:\s*100%[^}]*box-sizing:\s*border-box/u);
    assert.match(workspace, /html\[data-app-zoom-compact="true"\] \.dw-answer-scroll/u);
    assert.match(styles, /html\[data-app-zoom-compact="true"\] \.story-playbar \.is-story-audio\s*\{[^}]*grid-template-areas:"status status" "playback playback" "read edit"/u);
  });

  it('keeps the sticky Guide player compact and leaves transcript editing available in overlays', () => {
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const component = readFileSync(new URL('../src/NarrationPlayer.tsx', import.meta.url), 'utf8');
    const headingRule = styles.match(/\.is-story-audio \.walkthrough-panel-heading\s*\{([^}]+)\}/u)?.[1] ?? '';
    const repeatedTitleRule = styles.match(/\.is-story-audio \.walkthrough-panel-heading > div\s*\{([^}]+)\}/u)?.[1] ?? '';
    const playerRule = styles.match(/\.story-playbar \.is-story-audio\s*\{([^}]+)\}/u)?.[1] ?? '';
    const playbackRule = styles.match(/\.is-story-audio \.walkthrough-playback\s*\{([^}]+)\}/u)?.[1] ?? '';
    const overlayRule = styles.match(/\.is-story-audio \.walkthrough-narration-editor\[open\] > \.walkthrough-narration-editor-content\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(headingRule, /display:\s*flex/u);
    assert.doesNotMatch(headingRule, /position:\s*absolute/u);
    assert.match(playerRule, /min-width:\s*0/u);
    assert.match(playerRule, /max-width:\s*100%/u);
    assert.match(playerRule, /grid-template-areas:\s*"playback status" "read edit"/u);
    assert.match(playbackRule, /grid-template-columns:\s*auto minmax\(0,1fr\)/u);
    assert.match(repeatedTitleRule, /clip-path:\s*inset\(50%\)/u);
    assert.match(overlayRule, /position:\s*absolute/u);
    assert.match(overlayRule, /max-height:\s*min\(55vh, 480px\)/u);
    assert.match(component, /className="walkthrough-narration-editor-content"/u);
    assert.match(component, /className="walkthrough-narration-script" onToggle=\{\(event\).*querySelector<HTMLDetailsElement>\(':scope > \.walkthrough-narration-editor'\)/u);
    assert.match(component, /className="walkthrough-narration-editor" onToggle=\{\(event\).*querySelector<HTMLDetailsElement>\(':scope > \.walkthrough-narration-script'\)/u);
    assert.match(component, /<summary>Read narration<\/summary>/u);
    assert.match(component, /<summary><span>Edit spoken text<\/span>/u);
    assert.match(component, /walkthrough-transcript-state[^>]+role="status"/u);
    assert.match(component, /narrationAvailabilityLabel\(prewarmState\)/u);
  });

  it('re-fits a saved scene only when its nodes fall outside the current canvas', () => {
    const bounds = { x: 0, y: 0, width: 1_000, height: 300 };
    assert.equal(viewportClipsNodeBounds(bounds, { x: -500, y: 50, zoom: 1 }, 600, 400), true);
    assert.equal(viewportClipsNodeBounds(bounds, { x: 80, y: 50, zoom: .45 }, 600, 400), false);
    assert.equal(viewportClipsNodeBounds({ ...bounds, width: 0 }, { x: 0, y: 0, zoom: 1 }, 600, 400), false);
  });

  it('checks saved viewports in both the Guide player and the exploration Flow canvas', () => {
    const story = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
    for (const source of [story, workspace]) {
      assert.match(source, /viewportClipsNodeBounds\(bounds,\s*\w+\.getViewport\(\),\s*\w+\.width,\s*\w+\.height/u);
      assert.match(source, /onNodesChange=\{\(changes\)\s*=>\s*\{\s*if\s*\(changes\.some\(\(change\)\s*=>\s*change\.type\s*===\s*'dimensions'\)\)/u);
    }
  });

  it('keeps HTML visual values intact at word boundaries before breaking long tokens', () => {
    const styles = readFileSync(new URL('../src/html-scene.css', import.meta.url), 'utf8');
    const valueRule = styles.match(/\.html-scene-value\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(valueRule, /overflow-wrap:\s*break-word/u);
  });

  it('lets horizontal HTML story steps share narrow panes without overflowing their row', () => {
    const styles = readFileSync(new URL('../src/html-scene.css', import.meta.url), 'utf8');
    const scene = styles.match(/\.html-scene\s*\{([^}]+)\}/u)?.[1] ?? '';
    const body = styles.match(/\.html-scene-body\s*\{([^}]+)\}/u)?.[1] ?? '';
    const row = styles.match(/\.html-scene-row, \.html-scene-compare\s*\{([^}]+)\}/u)?.[1] ?? '';
    const rowItems = styles.match(/\.html-scene-row > :not\(\.html-scene-arrow\), \.html-scene-compare > \*\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(scene, /box-sizing:\s*border-box/u);
    assert.match(scene, /width:\s*100%/u);
    assert.match(body, /grid-template-columns:\s*minmax\(0,\s*1fr\)/u);
    assert.match(row, /width:\s*100%/u);
    assert.match(row, /min-width:\s*0/u);
    assert.match(row, /max-width:\s*100%/u);
    assert.match(rowItems, /box-sizing:\s*border-box/u);
    assert.match(rowItems, /flex:\s*1 1 0/u);
    assert.match(rowItems, /min-width:\s*min\(100%,\s*140px\)/u);
    assert.match(rowItems, /max-width:\s*min\(260px,\s*calc\(\(100% - 80px\) \/ 3\)\)/u);
    assert.match(styles, /@container \(max-width:\s*900px\)/u);
    const playerStyles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    assert.match(playerStyles, /\.story-html-stage\s*\{[^}]*height:\s*auto/u);
    assert.match(playerStyles, /\.story-html-stage\s*\{[^}]*max-height:\s*min\(80vh,\s*900px\)/u);
  });

  it('keeps graph node labels readable at word boundaries before breaking long identifiers', () => {
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const component = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    const nodeRule = styles.match(/\.visual-story \.react-flow__node\s*\{([^}]+)\}/u)?.[1] ?? '';
    const buttonRule = styles.match(/\.visual-story \.story-node-button\s*\{([^}]+)\}/u)?.[1] ?? '';
    for (const rule of [nodeRule, buttonRule]) {
      assert.match(rule, /overflow-wrap:\s*break-word/u);
      assert.match(rule, /word-break:\s*normal/u);
      assert.doesNotMatch(rule, /overflow-wrap:\s*anywhere/u);
    }
    assert.match(buttonRule, /white-space:\s*normal/u);
    assert.match(component, /className="story-node-detail">\{data\.detail\}/u);
    const detailRule = styles.match(/\.visual-story \.story-node-detail\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(detailRule, /-webkit-line-clamp:\s*2/u);
    assert.match(detailRule, /color:\s*var\(--text\)/u);
  });

  it('keeps authored speech without a visual whole, with every claim source still reachable', () => {
    const spokenText = 'First the incoming bytes reach a trust check. Rejected input returns an error; only accepted input moves on.';
    const current = { ...section, visual: null, spokenText };
    assert.deepEqual(storyScenes(current), [{ id: section.sectionId, title: section.title, narration: spokenText,
      anchorIds: ['a:entry', 'a:check'], nodeIds: [] }]);
    assert.equal(current.claims, claims);
    assert.notEqual(spokenText, claims.map(c => c.prose).join(' '));
    const edited = 'My revised account keeps both outcomes: rejection returns an error and acceptance allows processing.';
    assert.equal(storyScenes(current, undefined, edited)[0].narration, edited);
    assert.equal(current.spokenText, spokenText);
  });

  it('retains the entire grounded narration, including every condition and later consequence', () => {
    const scenes = storyScenes(section, visual);
    assert.equal(scenes.length, 2);
    assert.equal(scenes.map((scene) => scene.narration).join(' '), section.spokenText);
    assert.deepEqual(scenes.map((scene) => scene.nodeIds), [['entry'], ['check', 'error', 'result']]);
  });

  it('never substitutes complete claims for an older summary or edited recording script', () => {
    const summary = 'Previously reviewed, shorter narration.';
    assert.deepEqual(storyScenes({ ...section, spokenText: summary }, visual).map((scene) => scene.narration), [summary]);
    const edited = 'My edited narration, with a different scene boundary.';
    assert.deepEqual(storyScenes(section, visual, edited).map((scene) => scene.narration), [edited]);
  });

  it('keeps inference labels on every sentence and handles identifier punctuation without dropping names', () => {
    const claims = [claim('context', 'See src/file.rs and Font::memory_face.\nThis may avoid another parse!', { kind: 'inference', visibleLabel: 'Inference' })];
    const spokenText = 'Inference: See src/file.rs and Font::memory_face. Inference: This may avoid another parse!';
    assert.equal(storyScenes({ ...section, claims, spokenText })[0].narration, spokenText);
  });

  it('retains native duplicate-sentence behavior and never manufactures unanchored scenes', () => {
    const claims = [claim('a', 'A call reaches run().'), claim('b', 'A call reaches run(). The result returns.'), { ...claim('c', 'A secret extra sentence.'), anchors: [] }];
    const spokenText = 'A call reaches run(). The result returns.';
    assert.equal(storyScenes({ ...section, claims, spokenText }).map((scene) => scene.narration).join(' '), spokenText);
  });

  it('shows only declared neighbors, and keeps both rejection and success paths', () => {
    const scene = storyScenes(section, visual)[1];
    assert.deepEqual([...sceneNodeIds(visual, scene)].sort(), ['check', 'entry', 'error', 'result']);
    assert.equal(visual.edges.length, 3);
  });

  it('lays out actual branches beside one another instead of inventing a linear call sequence', () => {
    const nodes = new Map(storyLayout(visual).map((node) => [node.id, node.position]));
    assert.equal(nodes.get('error').y, nodes.get('result').y);
    assert.notEqual(nodes.get('error').x, nodes.get('result').x);
    assert.ok(nodes.get('check').y > nodes.get('entry').y);
    assert.ok(nodes.get('error').y > nodes.get('check').y);
  });

  it('fits a focused scene to its visible subgraph instead of hidden chapter branches', () => {
    const full = storyLayout(visual);
    const focused = storyLayout(visual, new Set(['check', 'error', 'result']));
    const fullY = full.map((node) => node.position.y);
    const focusedY = focused.map((node) => node.position.y);

    assert.deepEqual(focused.map((node) => node.id), ['check', 'error', 'result']);
    assert.equal(Math.min(...focusedY), 0);
    assert.ok(Math.max(...focusedY) < Math.max(...fullY));
    assert.equal(focused.find((node) => node.id === 'error').position.y, focused.find((node) => node.id === 'result').position.y);
  });

  it('terminates deterministically for cycles, self loops and disconnected diagrams', () => {
    const cyclic = { ...visual, edges: [...visual.edges, { edgeId: 'cycle', fromNodeId: 'check', toNodeId: 'entry' }, { edgeId: 'self', fromNodeId: 'result', toNodeId: 'result' }] };
    assert.deepEqual(storyLayout(cyclic), storyLayout(cyclic));
    assert.equal(storyLayout(cyclic).length, visual.nodes.length);
    assert.ok(storyLayout(cyclic).every((node) => Number.isFinite(node.position.x) && Number.isFinite(node.position.y)));
    assert.equal(new Set(storyLayout({ ...visual, edges: [] }).map((node) => `${node.position.x},${node.position.y}`)).size, 4);
  });

  it('maps all four preserved Fontdone narration scenes to their declared evidence cues without inventing timing', () => {
    const artifact = JSON.parse(readFileSync(new URL('../docs/acceptance/artifacts/2026-10-17-fontdone-complete-script-partial.json', import.meta.url)));
    const section = artifact.answer.sections[0].preview.revision.sections[0];
    const scenes = storyScenes(section, section.visual);
    assert.equal(scenes.length, 4);
    assert.equal(scenes.map((scene) => scene.narration).join(' '), section.spokenText);
    assert.ok(scenes[1].narration.includes('parse_bdf_constructor_error'));
    assert.ok(scenes[3].narration.includes('shorter inputs return that original error directly'));
    assert.equal(scenes[1].nodeIds.length, 4);
    assert.ok(scenes.every((scene) => !('startSeconds' in scene)));
  });

  it('restores the exact scene and independent audio positions without resuming playback', () => {
    const scenes = storyScenes(section, visual);
    const identity = { revisionId: 'revision:one', sectionId: 'section:one', sectionRevisionId: 'section-revision:one' };
    const saved = { ...identity, sceneId: 'check', positions: { entry: 12.4, check: 3.2, other: 14 }, fullMap: false, focusLayoutVersion: 2 };
    const progress = readStoryProgress(JSON.stringify(saved), identity, scenes);
    assert.deepEqual(progress, saved);
    const restored = readSectionStoryProgress(progress, identity, scenes);
    assert.deepEqual(restored, { index: 1, positions: [['entry', 12.4], ['check', 3.2]], fullMap: false, viewport: undefined });
    assert.equal('autoPlay' in restored, false);
  });

  it('rejects malformed or unscoped progress and never lets it select a foreign scene', () => {
    const scenes = storyScenes(section, visual);
    const identity = { revisionId: 'revision:one', sectionId: 'section:one', sectionRevisionId: 'section-revision:one' };
    for (const raw of [null, '{bad', '"text"', JSON.stringify({ sceneId: 'check', positions: { check: 4 } }), '0'.repeat(16_001)]) {
      assert.equal(readStoryProgress(raw, identity, scenes), undefined);
    }
    const foreign = { ...identity, sceneId: 'other', positions: { entry: 0, check: 0 }, fullMap: false, focusLayoutVersion: 2 };
    assert.equal(readStoryProgress(JSON.stringify(foreign), identity, scenes), undefined);
    const unversioned = { ...identity, sceneId: 'check', positions: { entry: 0, check: 0 }, fullMap: false };
    assert.equal(readStoryProgress(JSON.stringify(unversioned), identity, scenes), undefined);
    const extra = { ...identity, sceneId: 'check', positions: { entry: 0, check: 0 }, fullMap: false, focusLayoutVersion: 2, autoPlay: true };
    assert.equal(readStoryProgress(JSON.stringify(extra), identity, scenes), undefined);
    const bad = { ...identity, sceneId: 'check', positions: { entry: 0, check: 3601 }, fullMap: false, focusLayoutVersion: 2 };
    assert.equal(readStoryProgress(JSON.stringify(bad), identity, scenes), undefined);
  });
});

it('authored narration follows all claim scenes for the reader and export without speaking identifiers', () => {
  const scripts = ['For example, a request arrives with untrusted bytes.', 'If the check rejects them, the caller gets an error. Otherwise, processing continues.'];
  const authoredTitles = ['The request arrives', 'Validation branches'];
  const authored = { ...visual, presentation: { schemaVersion: 3, scenes: scripts.map((narration, claimIndex) => ({ claimIndex, title: authoredTitles[claimIndex], narration, mode: 'graph', elements: [] })) } };
  const current = { ...section, spokenText: scripts.join(' '), visual: authored };
  const fromReader = storyScenes(current, authored);
  const fromExport = storyScenes(current);
  assert.deepEqual(fromReader, fromExport);
  assert.deepEqual(fromReader.map(s => s.narration), scripts);
  assert.deepEqual(fromReader.map(s => s.id), claims.map(c => c.claimId));
  assert.deepEqual(fromReader.map(s => s.title), authoredTitles);
  assert.deepEqual(fromReader.map(s => s.anchorIds), [['a:entry'], ['a:check']]);
  assert.equal(fromReader[1].nodeIds.length, 3);
  assert.ok(current.claims[0].prose.includes('handleRequest'));
  assert.ok(!fromReader[0].narration.includes('handleRequest'));
  assert.deepEqual(storyScenes(current, authored, 'My edited whole script.').map(s => s.narration), ['My edited whole script.']);
  const bad = structuredClone(current); bad.visual.presentation.scenes.pop();
  assert.deepEqual(storyScenes(bad).map(s => s.narration), [current.spokenText]);
});

it('new authored inferred scenes preserve their visible uncertainty label in speech', () => {
  const current = structuredClone(section);
  current.claims[1].category = { kind: 'inference', visibleLabel: 'Inference' };
  current.visual = { ...visual, presentation: { schemaVersion: 3, scenes: [
    { claimIndex: 0, title: 'Arrival', narration: 'The bytes arrive before any trust decision.', mode: 'graph', elements: [] },
    { claimIndex: 1, title: 'Possible effect', narration: 'This may avoid work on rejected input. This is not a runtime guarantee.', mode: 'graph', elements: [] },
  ] } };
  current.spokenText = 'The bytes arrive before any trust decision. Inference: This may avoid work on rejected input. This is not a runtime guarantee.';
  assert.deepEqual(storyScenes(current).map(s => s.narration), ['The bytes arrive before any trust decision.', 'Inference: This may avoid work on rejected input. This is not a runtime guarantee.']);
});
