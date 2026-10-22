import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import { storyViewportNeedsRefit, viewportClipsNodeBounds } from '../src/lib/flowViewport.ts';
import { readSectionStoryProgress, readStoryProgress, restoreStoryFocus, sceneNodeIds, sourceAnchorForVisualEdge, sourceAnchorForVisualNode, sourceEvidenceAnchorForVisualEdge, sourceEvidenceAnchorForVisualNode, sourceValuesByVisualNode, storyEdgeLabelPosition, storyLayout, storyLayoutWithDirection, storyScenes, storyViewportBounds, visibleStoryEdgeLabel } from '../src/lib/visualStory.ts';

const claim = (id, prose, category = { kind: 'codeBehavior' }) => ({ claimId: id, prose, category, anchors: [{ anchorId: `a:${id}` }] });
const claims = [claim('entry', 'A request reaches handleRequest. The bytes are still untrusted.'), claim('check', 'If validate rejects those bytes, the caller receives an error. Otherwise processing continues.')];
const section = { sectionId: 'section:one', title: 'Follow a request', claims, spokenText: claims.map((item) => item.prose).join(' ') };
const visual = {
  schemaVersion: 2, kind: 'dataFlow', title: 'Follow a request', layoutHint: { direction: 'topToBottom' },
  nodes: ['entry', 'check', 'error', 'result'].map((id) => ({ nodeId: id, label: id, evidenceAnchorIds: [id === 'entry' ? 'a:entry' : 'a:check'] })),
  edges: [ ['entry', 'check'], ['check', 'error'], ['check', 'result'] ].map(([fromNodeId, toNodeId], i) => ({ edgeId: `e:${i}`, fromNodeId, toNodeId, label: 'branch' })),
};

describe('visual story scenes', () => {
  it('keeps long edge labels scannable while preserving short labels', () => {
    assert.equal(visibleStoryEdgeLabel('PFR0 prefix matches'), 'PFR0 prefix matches');
    const full = 'Prefix matches; hand off and return this result.';
    const visible = visibleStoryEdgeLabel(full, 24);
    assert.equal(visible, 'Prefix matches; hand…');
    assert.ok(visible.length <= 24);
    assert.equal(visibleStoryEdgeLabel(full), 'Prefix matches; hand off and…');
    assert.ok(visibleStoryEdgeLabel(full).length <= 36);
  });

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

  it('narrows numeric signature and bare-call labels only to unique source text', () => {
    const excerpt = [
      'if matches!(read_u16_le(data, 0), Some(0x0200) | Some(0x0300)) {',
      '    return Self::winfnt_face(data, face_index, size_pt);',
      '}',
    ].join('\n');
    const anchor = {
      anchorId: 'anchor:winfnt', path: 'src/font.rs', symbolId: 'symbol:memory-face', snapshotId: 'snapshot:one',
      sourceSha256: 'c'.repeat(64), startByte: 500, endByteInclusive: 499 + Buffer.byteLength(excerpt),
      startLine: 4064, endLine: 4066, startColumn: 1, endColumn: 1, excerpt,
    };
    const signature = sourceAnchorForVisualNode(anchor, {
      label: '0x0200 or 0x0300', detail: 'Either exact signature matches.',
    });
    assert.equal(signature.startLine, 4064);
    assert.equal(signature.endLine, 4064);
    assert.ok(['0x0200', '0x0300'].includes(signature.excerpt));

    const read = sourceAnchorForVisualNode(anchor, {
      label: 'Read the 16-bit signature', detail: 'The code calls read_u16_le(data, 0).',
    });
    assert.equal(read.startLine, 4064);
    assert.equal(read.excerpt, 'read_u16_le(data, 0)');

    const ambiguous = { ...anchor, excerpt: `${excerpt}\nif fallback == 0x0200 {}`, endLine: 4067,
      endByteInclusive: 499 + Buffer.byteLength(`${excerpt}\nif fallback == 0x0200 {}`) };
    assert.equal(sourceAnchorForVisualNode(ambiguous, { label: '0x0200' }), ambiguous);
  });

  it('uses an exact node source binding before the first broad evidence anchor', () => {
    const excerpt = 'if matches!(read_u16_le(data, 0), Some(0x0200) | Some(0x0300)) {';
    const broad = {
      anchorId: 'anchor:broad', path: 'src/font.rs', symbolId: 'symbol:memory-face', snapshotId: 'snapshot:one',
      sourceSha256: 'd'.repeat(64), startByte: 100, endByteInclusive: 99 + Buffer.byteLength(excerpt),
      startLine: 4053, endLine: 4053, startColumn: 1, endColumn: excerpt.length, excerpt,
    };
    const exact = { ...broad, anchorId: 'anchor:exact', startLine: 4064 };
    const node = {
      label: 'LE 16-bit value at offset 0', detail: 'The branch checks the value read at byte offset zero.',
      evidenceAnchorIds: [broad.anchorId, exact.anchorId], sourceAnchorId: exact.anchorId,
      sourceValue: 'read_u16_le(data, 0)',
    };
    assert.equal(sourceEvidenceAnchorForVisualNode([broad, exact], node), exact);
    assert.equal(sourceAnchorForVisualNode(exact, node, node.sourceValue).excerpt, node.sourceValue);
    assert.equal(sourceEvidenceAnchorForVisualNode([broad, exact], { ...node, sourceAnchorId: 'anchor:foreign' }), undefined);

    const singleTokenAnchor = { ...exact, startByte: 20, endByteInclusive: 20, excerpt: '{', startLine: 4053, endLine: 4053, startColumn: 18, endColumn: 18 };
    assert.equal(sourceAnchorForVisualNode(singleTokenAnchor, node, '{').excerpt, '{');

    const labelAndBindingExcerpt = 'if tag == 0x0200 {';
    const labelAndBinding = { ...exact, startByte: 20, endByteInclusive: 19 + Buffer.byteLength(labelAndBindingExcerpt), excerpt: labelAndBindingExcerpt };
    assert.equal(sourceAnchorForVisualNode(labelAndBinding, { label: 'tag == 0x0200' }, 'tag').excerpt, 'tag');
  });

  it('resolves edge evidence and narrows a code-bearing relationship label to the exact expression', () => {
    const excerpt = [
      'if data.starts_with(b"PFR0") {',
      '    return Self::pfr_face(data, face_index, size_pt);',
      '}',
    ].join('\n');
    const anchor = {
      anchorId: 'anchor:pfr-dispatch', path: 'src/font.rs', symbolId: 'symbol:memory-face', snapshotId: 'snapshot:one',
      sourceSha256: 'e'.repeat(64), startByte: 900, endByteInclusive: 899 + Buffer.byteLength(excerpt),
      startLine: 4061, endLine: 4063, startColumn: 1, endColumn: 1, excerpt,
    };
    const edge = { edgeId: 'edge:pfr', label: 'Prefix matches; hand off and return this result.', evidenceAnchorIds: [anchor.anchorId] };
    assert.equal(sourceEvidenceAnchorForVisualEdge([anchor], edge), anchor);
    const targetAnchor = { ...anchor, anchorId: 'anchor:pfr-function', path: 'src/pfr.rs', excerpt: 'pub(crate) fn pfr_face(data: &[u8], face_index: usize, size_pt: f32) -> Result<Self, FontError> {' };
    const narrowed = sourceAnchorForVisualEdge(anchor, edge, targetAnchor);
    assert.equal(narrowed.startLine, 4062);
    assert.equal(narrowed.excerpt, 'Self::pfr_face(data, face_index, size_pt)');
    assert.equal(sourceEvidenceAnchorForVisualEdge([anchor], { ...edge, evidenceAnchorIds: ['anchor:foreign'] }), undefined);
    assert.equal(sourceAnchorForVisualEdge(anchor, edge), anchor, 'without a verified target definition, retain the cited range');
    const duplicatedCall = { ...anchor, excerpt: `${excerpt}\nSelf::pfr_face(data, face_index, size_pt);`, endLine: 4064,
      endByteInclusive: 899 + Buffer.byteLength(`${excerpt}\nSelf::pfr_face(data, face_index, size_pt);`) };
    assert.equal(sourceAnchorForVisualEdge(duplicatedCall, edge, targetAnchor), duplicatedCall, 'ambiguous call sites retain the cited range');
  });

  it('narrows a natural-language edge through its uniquely cited source branch', () => {
    const excerpt = [
      'if is_pcf(data) {',
      '    return Self::pcf_face(data, face_index, size_pt);',
      '}',
      'if data.starts_with(b"PFR0") {',
      '    return Self::pfr_face(data, face_index, size_pt);',
      '}',
      'if is_winfnt(data) {',
      '    return Self::winfnt_face(data, face_index, size_pt);',
      '}',
    ].join('\n');
    const anchor = {
      anchorId: 'anchor:memory-face', path: 'src/font.rs', symbolId: 'symbol:memory-face', snapshotId: 'snapshot:one',
      sourceSha256: 'f'.repeat(64), startByte: 1200, endByteInclusive: 1199 + Buffer.byteLength(excerpt),
      startLine: 4058, endLine: 4066, startColumn: 1, endColumn: 1, excerpt,
    };
    const predicate = 'if data.starts_with(b"PFR0") {';
    const predicateOffset = excerpt.indexOf(predicate);
    const rawSourceAnchor = {
      ...anchor, anchorId: 'anchor:pfr-prefix', startByte: anchor.startByte + Buffer.byteLength(excerpt.slice(0, predicateOffset)),
      endByteInclusive: anchor.startByte + Buffer.byteLength(excerpt.slice(0, predicateOffset + predicate.length)) - 1,
      startLine: 4061, endLine: 4061, excerpt: predicate,
    };
    const sourceNode = { label: 'PFR0 prefix', detail: 'Branch test: data.starts_with(b"PFR0").' };
    const sourceAnchor = sourceAnchorForVisualNode(rawSourceAnchor, sourceNode);
    const edge = { edgeId: 'edge:pfr', label: 'Prefix matches; hand off and return this result.', evidenceAnchorIds: [anchor.anchorId] };
    const unrelatedTarget = { ...anchor, anchorId: 'anchor:parse', path: 'src/pfr.rs', excerpt: 'pub(crate) fn parse(data: &[u8]) -> Result<Self, Error> {' };

    const narrowed = sourceAnchorForVisualEdge(anchor, edge, unrelatedTarget, sourceAnchor);
    const expression = 'Self::pfr_face(data, face_index, size_pt)';
    assert.equal(narrowed.startLine, 4062);
    assert.equal(narrowed.endLine, 4062);
    assert.equal(narrowed.excerpt, expression);
    assert.equal(narrowed.startByte, anchor.startByte + Buffer.byteLength(excerpt.slice(0, excerpt.indexOf(expression))));

    const duplicatePredicateExcerpt = `${excerpt}\n${predicate}\n    return Self::other_face(data);\n}`;
    const duplicateAnchor = { ...anchor, excerpt: duplicatePredicateExcerpt, endLine: 4069,
      endByteInclusive: anchor.startByte + Buffer.byteLength(duplicatePredicateExcerpt) - 1 };
    assert.equal(sourceAnchorForVisualEdge(duplicateAnchor, edge, unrelatedTarget, sourceAnchor), duplicateAnchor,
      'duplicate source conditions retain the validated broad citation');
    assert.equal(sourceAnchorForVisualEdge(anchor, edge, unrelatedTarget, { ...sourceAnchor, sourceSha256: '0'.repeat(64) }), anchor,
      'source context from another snapshot is not used');
    const unrelatedExcerpt = [
      'let is_pfr = data.starts_with(b"PFR0");',
      'if fallback {',
      '    return Self::pfr_face(data, face_index, size_pt);',
      '}',
    ].join('\n');
    const unrelatedAnchor = { ...anchor, excerpt: unrelatedExcerpt, endLine: 4061,
      endByteInclusive: anchor.startByte + Buffer.byteLength(unrelatedExcerpt) - 1 };
    assert.equal(sourceAnchorForVisualEdge(unrelatedAnchor, edge, unrelatedTarget, sourceAnchor), unrelatedAnchor,
      'a matching expression outside the branch condition cannot identify a return call');
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

  it('renders evidence-backed relationship labels and exposes semantic source controls', () => {
    const component = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    assert.match(component, /function EvidenceStoryEdge\(/u);
    assert.match(component, /<EdgeLabelRenderer>/u);
    assert.match(component, /aria-label=\{`\$\{label\}\. Open cited source code`\}/u);
    assert.match(component, /const sourceLinkedEdges = visibleEdges\.flatMap\(/u);
    assert.match(component, /<div className="story-details-connections">[\s\S]*?<ul>\{sourceLinkedEdges\.map/u);
    assert.match(component, /data-focus-key=\{`reader-visual-edge:\$\{edge\.edgeId\}:\$\{source\.anchorId\}`\}/u);
    assert.ok(component.includes('aria-label={`${from.label} — ${edge.label} — ${to.label}. Open ${source.path}, lines ${source.startLine} through ${source.endLine}.`}'));
    assert.match(component, /onClick=\{\(\) => chooseEdge\(edge\.edgeId\)\}/u);
    assert.match(component, /onOpenAnchor\(sourceAnchorForVisualEdge\(source, edge, targetAnchor, sourceContextAnchor\), `reader-visual-edge:/u);
    assert.match(component, /sourceAnchorForVisualNode\(sourceContext, origin, origin\.sourceValue\)/u);
    assert.match(component, /edgeTypes=\{storyEdgeTypes\}/u);
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

  it('leads with the scene visual and keeps playback reachable at the viewport edge', () => {
    const component = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const headingIndex = component.indexOf('className="story-toolbar"');
    const visualIndex = component.indexOf('className="story-stage');
    const playerIndex = component.indexOf('className="story-playbar"');
    const detailsIndex = component.indexOf('className="story-details"');
    assert.ok(headingIndex >= 0 && headingIndex < visualIndex && visualIndex < playerIndex && playerIndex < detailsIndex,
      'scene heading, visual, player and details should appear in that order');
    assert.match(component, /className="story-sticky-controls"[\s\S]*?className="story-toolbar"[\s\S]*?className="story-scene-title" title=\{scene\.title\}/u);
    assert.match(component, /aria-label=\{`Scene \$\{i \+ 1\}: \$\{item\.title\}`\}/u);
    assert.match(styles, /\.story-sticky-controls\s*\{[^}]*position:\s*sticky;[^}]*top:\s*0;[^}]*z-index:\s*10/u,
      'the dedicated walkthrough can keep scene controls visible while its own view scrolls');
    const playerRule = styles.match(/\.story-playbar\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(playerRule, /position:\s*sticky/u);
    assert.match(playerRule, /bottom:\s*0/u);
    assert.match(playerRule, /background:\s*var\(--surface\)/u);
    const workspace = readFileSync(new URL('../src/workspace/workspace.css', import.meta.url), 'utf8');
    assert.match(workspace, /\.dw-answer-scroll \.visual-story > \.story-sticky-controls,\s*\.dw-answer-scroll \.visual-story > \.story-playbar\s*\{\s*position:\s*static;/u,
      'inline Guide controls must stay in the story flow instead of sticking to the entire chat history');
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
    assert.match(compactRules, /\.story-toolbar\s*\{[^}]*grid-template-columns:\s*minmax\(0,1fr\) auto auto/u);
    assert.match(compactRules, /\.story-map-toggle\s*\{[^}]*grid-column:\s*3;[^}]*grid-row:\s*2/u,
      'the map control should share the second row instead of taking a third toolbar row');
    assert.match(compactRules, /\.story-playbar \.is-story-audio\s*\{[^}]*grid-template-areas:\s*"status status" "playback playback" "read edit"/u);
    assert.match(compactRules, /\.is-story-audio \.walkthrough-playback-buttons\s*\{[^}]*flex-wrap:\s*wrap/u);
    assert.match(styles, /\.visual-story\s*\{[^}]*max-width:\s*100%[^}]*box-sizing:\s*border-box/u);
    assert.match(workspace, /html\[data-app-zoom-compact="true"\] \.dw-answer-scroll/u);
    assert.match(styles, /html\[data-app-zoom-compact="true"\] \.story-toolbar\s*\{[^}]*grid-template-columns:minmax\(0,1fr\) auto auto/u);
    assert.match(styles, /html\[data-app-zoom-compact="true"\] \.story-map-toggle\s*\{[^}]*grid-column:3;[^}]*grid-row:2/u);
    assert.match(styles, /html\[data-app-zoom-compact="true"\] \.story-playbar \.is-story-audio\s*\{[^}]*grid-template-areas:"status status" "playback playback" "read edit"/u);
  });

  it('keeps the Guide transcript in reading flow and leaves text editing available in an overlay', () => {
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const component = readFileSync(new URL('../src/NarrationPlayer.tsx', import.meta.url), 'utf8');
    const headingRule = styles.match(/\.is-story-audio \.walkthrough-panel-heading\s*\{([^}]+)\}/u)?.[1] ?? '';
    const repeatedTitleRule = styles.match(/\.is-story-audio \.walkthrough-panel-heading > div\s*\{([^}]+)\}/u)?.[1] ?? '';
    const playerRule = styles.match(/\.story-playbar \.is-story-audio\s*\{([^}]+)\}/u)?.[1] ?? '';
    const transcriptRule = styles.match(/\.is-story-audio \.walkthrough-narration-script\[open\] \.walkthrough-transcript-scroll\s*\{([^}]+)\}/u)?.[1] ?? '';
    const playbackRule = styles.match(/\.is-story-audio \.walkthrough-playback\s*\{([^}]+)\}/u)?.[1] ?? '';
    const overlayRule = styles.match(/\.is-story-audio \.walkthrough-narration-editor\[open\] > \.walkthrough-narration-editor-content\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(headingRule, /display:\s*flex/u);
    assert.doesNotMatch(headingRule, /position:\s*absolute/u);
    assert.match(playerRule, /min-width:\s*0/u);
    assert.match(playerRule, /max-width:\s*100%/u);
    assert.match(playerRule, /grid-template-areas:\s*"playback status" "read edit"/u);
    assert.match(transcriptRule, /position:\s*relative/u);
    assert.match(transcriptRule, /width:\s*100%/u);
    assert.match(transcriptRule, /max-height:\s*min\(32vh, 280px\)/u);
    assert.doesNotMatch(transcriptRule, /position:\s*absolute/u);
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

  it('loads a completed local prewarm before Play without autoplaying it', () => {
    const component = readFileSync(new URL('../src/NarrationPlayer.tsx', import.meta.url), 'utf8');
    assert.match(component, /prewarmState !== 'ready'/u);
    assert.match(component, /cacheHydrationKeyRef\.current === cacheHydrationKey/u);
    assert.match(component, /void makeReviewRef\.current\(true\)/u);
    assert.match(component, /setLoadedTrack\(cached\.data, result\.data, false, requestedDraft\)/u);
    assert.match(component, /track && savedAudioMatchesDraft && currentTime <= 0 \? narrationAvailabilityLabel\('ready'\)/u);
  });

  it('advances the visual scene only on completed playback and leaves section pacing manual', () => {
    const player = readFileSync(new URL('../src/NarrationPlayer.tsx', import.meta.url), 'utf8');
    const story = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    assert.match(player, /onPlay=\{\(\) => \{ completionArmed\.current = true;/u);
    assert.match(player, /onPause=\{\(event\) => \{[\s\S]*?if \(!event\.currentTarget\.ended\) \{\s*completionArmed\.current = false;/u,
      'pausing before the end must disarm scene progression');
    assert.match(player, /const completedPlayback = completionArmed\.current && mountedRef\.current && audioRef\.current\?\.ended;/u);
    assert.match(player, /if \(completedPlayback\) onEnded\?\.\(\);/u,
      'seeking a paused clip to its end must not advance the story');
    assert.match(story, /onEnded=\{\(\) => \{ if \(index < scenes\.length - 1\) select\(index \+ 1, true\); else if \(!sectionCompletionSent\.current\) \{ sectionCompletionSent\.current = true; onSectionPlaybackComplete\?\.\(\); \} \}\}/u,
      'the last completed narration advances the repository guide buffer once');
    assert.match(story, /Sections always advance manually\./u);
    assert.match(story, /aria-label="Next scene" onClick=\{\(\) => select\(index \+ 1\)\}/u);
  });

  it('re-fits a saved scene only when its nodes fall outside the current canvas', () => {
    const bounds = { x: 0, y: 0, width: 1_000, height: 300 };
    assert.equal(viewportClipsNodeBounds(bounds, { x: -500, y: 50, zoom: 1 }, 600, 400), true);
    assert.equal(viewportClipsNodeBounds(bounds, { x: 80, y: 50, zoom: .45 }, 600, 400), false);
    assert.equal(viewportClipsNodeBounds({ ...bounds, width: 0 }, { x: 0, y: 0, zoom: 1 }, 600, 400), false);
  });

  it('re-fits a Guide graph saved at a zoom too small to read', () => {
    const bounds = { x: 0, y: 0, width: 480, height: 260 };
    assert.equal(storyViewportNeedsRefit(bounds, { x: 80, y: 50, zoom: .18 }, 600, 400), true);
    assert.equal(storyViewportNeedsRefit(bounds, { x: 80, y: 50, zoom: .45 }, 600, 400), false);
    assert.equal(storyViewportNeedsRefit({ ...bounds, width: 0 }, { x: 0, y: 0, zoom: .18 }, 600, 400), false);
  });

  it('fits Guide edge-label lanes even when all node cards already fit', () => {
    const nodeBounds = { x: 100, y: 0, width: 195, height: 650 };
    const verticalBounds = storyViewportBounds(nodeBounds, [
      { x: -92, y: 200, width: 180, height: 30 },
      { x: 307, y: 400, width: 180, height: 30 },
    ]);
    assert.deepEqual(verticalBounds, { x: -92, y: 0, width: 579, height: 650 });
    const viewport = { x: 52.5, y: 30, zoom: 1 };
    assert.equal(storyViewportNeedsRefit(nodeBounds, viewport, 400, 720, 28), false,
      'the cards fit inside the narrow pane');
    assert.equal(storyViewportNeedsRefit(verticalBounds, viewport, 400, 720, 28), true,
      'the expanded edge-label lane forces a reframe before labels are clipped');
    assert.deepEqual(storyViewportBounds(nodeBounds, []), nodeBounds,
      'graphs without visible edges keep their established framing');
  });

  it('checks saved viewports in both the Guide player and the exploration Flow canvas', () => {
    const story = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
    assert.match(story, /storyViewportNeedsRefit\(bounds,\s*\w+\.getViewport\(\),\s*\w+\.width,\s*\w+\.height/u,
      'Guide restoration also rejects saved zoom levels that make nodes unreadable');
    assert.match(story, /storyViewportBounds\(nodeBounds,\s*edgeLabelBoundsFor\(instance\)\)/u,
      'Guide reframing includes the rendered edge-label rectangles, not only their node endpoints');
    assert.match(story, /getBezierPath\(\{[\s\S]*?sourceX, sourceY,[\s\S]*?targetX, targetY/u,
      'edge-label bounds follow the same path geometry as the rendered labels');
    assert.match(story, /getViewportForBounds\(bounds,\s*size\.width,\s*size\.height,\s*\.18,\s*1\.1,\s*\.08\)/u,
      'the viewport centers the expanded bounds rather than letting node-only fitView clip edge labels');
    assert.match(workspace, /viewportClipsNodeBounds\(bounds,\s*\w+\.getViewport\(\),\s*\w+\.width,\s*\w+\.height/u,
      'Flow restoration continues to preserve its existing clipping behavior');
    for (const source of [story, workspace]) {
      assert.match(source, /onNodesChange=\{\(changes\)\s*=>\s*\{\s*if\s*\(changes\.some\(\(change\)\s*=>\s*change\.type\s*===\s*'dimensions'\)\)/u);
    }
    assert.match(story, /instance\.setViewport\(saved,\s*\{\s*duration:\s*0\s*\}\)/u,
      'a saved Guide viewport is applied after React Flow initializes, not only passed as its initial default');
    assert.match(story, /onInit=\{\(instance\)\s*=>\s*\{[\s\S]*?instance\.setNodes\(graphData\.current\.nodes\);\s*instance\.setEdges\(graphData\.current\.edges\);/u,
      'the Guide canvas hydrates its nodes and edges after React Flow initializes');
  });

  it('rehydrates the Guide diagram after data arrives and after the first layout frame', () => {
    const story = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    assert.match(story, /if \(!visual \|\| focusInitialized\.current\) return;[\s\S]*?setFocusedNode\(restoredFocus\.nodeId\);\s*setFocusedEdge\(restoredFocus\.edgeId\);/u,
      'a saved focus is restored if the source-backed diagram arrives after the reader mounts');
    assert.match(story, /if \(flowFitInProgress\.current\) \{[\s\S]*?flowFitCheckPending\.current = true;\s*return;/u,
      'a node measurement that arrives during fitView schedules a second bounds check');
    assert.match(story, /finally\(\(\) => \{\s*if \(generation === flowFitGeneration\.current\) \{\s*flowFitInProgress\.current = false;\s*if \(flowFitCheckPending\.current\) \{\s*flowFitCheckPending\.current = false;\s*scheduleViewportCheck\.current\(\);/u,
      'the pending first-mount bounds check runs after fitView settles');
    assert.match(story, /<ReactFlow key=\{`\$\{section\.sectionRevisionId\}:\$\{index\}:\$\{fullMap\}:\$\{visibleNodeKey\}`\}/u,
      'the canvas remounts when a restored section hydrates its actual visible node set');
    assert.match(story, /useEffect\(\(\)\s*=>\s*\{[\s\S]*?const instance = flowInstance\.current;\s*if \(!instance\?\.viewportInitialized\) return;[\s\S]*?const current = graphData\.current;\s*instance\.setNodes\(current\.nodes\);\s*instance\.setEdges\(current\.edges\);[\s\S]*?\}, \[visual, visibleNodeKey, scene\.id, fullMap, revision\.revisionId, section\.sectionRevisionId\]\)/u,
      'newly restored visual data replaces the graph initialized from older saved state');
    assert.match(story, /requestAnimationFrame\(\(\)\s*=>\s*\{\s*if\s*\(flowInstance\.current\s*!==\s*instance\)\s*return;\s*const latest = graphData\.current;\s*instance\.setNodes\(latest\.nodes\);\s*instance\.setEdges\(latest\.edges\);\s*scheduleViewportCheck\.current\(\);\s*\}\)/u,
      'the first Guide mount reconciles the controlled graph after React Flow has completed a browser layout frame');
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

  it('keeps graph context compact but expands the narrated and focused node details', () => {
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
    assert.ok(component.includes("title={`${data.label}${data.detail ? ` — ${data.detail}` : ''}`}"));
    const detailRule = styles.match(/\.visual-story \.story-node-detail\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(detailRule, /-webkit-line-clamp:\s*2/u);
    assert.match(detailRule, /color:\s*var\(--text\)/u);
    const narratedRule = styles.match(/\.visual-story \.story-node-active \.story-node-detail\s*\{([^}]+)\}/u)?.[1] ?? '';
    const focusedRule = styles.match(/\.visual-story \.story-node-focus \.story-node-detail\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(narratedRule, /-webkit-line-clamp:\s*4/u);
    assert.match(focusedRule, /-webkit-line-clamp:\s*6/u);
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

  it('reserves a clear label lane between horizontal graph columns', () => {
    const nodes = new Map(storyLayout({ ...visual, layoutHint: { direction: 'leftToRight' } }).map((node) => [node.id, node.position]));
    assert.equal(nodes.get('check').x - nodes.get('entry').x, 340);
  });

  it('keeps a deep route readable in the Guide canvas even when its preferred direction is horizontal', () => {
    const routeNodes = ['input', 'records', 'tag', 'offset', 'range', 'slice', 'none']
      .map((nodeId) => ({ nodeId, label: nodeId, evidenceAnchorIds: ['a:entry'] }));
    const routeEdges = routeNodes.slice(1).map((node, index) => ({
      edgeId: `route:${index}`,
      fromNodeId: routeNodes[index].nodeId,
      toNodeId: node.nodeId,
      label: 'continues',
    }));
    const positions = new Map(storyLayout({
      ...visual,
      nodes: routeNodes,
      edges: routeEdges,
      layoutHint: { direction: 'leftToRight' },
    }).map((node) => [node.id, node.position]));

    assert.equal(positions.get('records').x, positions.get('input').x);
    assert.equal(positions.get('records').y - positions.get('input').y, 94);
    assert.equal(positions.get('none').y - positions.get('slice').y, 94);
  });

  it('uses the adapted deep-route direction for both node handles and edge-label lanes', () => {
    const routeNodes = ['input', 'records', 'tag', 'offset', 'range', 'slice', 'none']
      .map((nodeId) => ({ nodeId, label: nodeId, evidenceAnchorIds: ['a:entry'] }));
    const routeEdges = routeNodes.slice(1).map((node, index) => ({
      edgeId: `route:${index}`,
      fromNodeId: routeNodes[index].nodeId,
      toNodeId: node.nodeId,
      label: 'continues',
    }));
    const result = storyLayoutWithDirection({
      ...visual,
      nodes: routeNodes,
      edges: routeEdges,
      layoutHint: { direction: 'leftToRight' },
    });
    assert.equal(result.direction, 'vertical');

    const lane = storyEdgeLabelPosition(0, 0, 0, 47, true, 1);
    assert.equal(lane.x, 199.5);
    assert.equal(lane.y, 47);
    assert.ok(lane.x - 180 / 2 >= 195 / 2 + 12,
      'the label begins at least twelve pixels beyond the centered node card');

    const branch = storyEdgeLabelPosition(0, 118, 59, 47, true, 1);
    assert.ok(branch.x - 180 / 2 >= 118 + 195 / 2 + 12,
      'a diagonal branch label clears both endpoint cards');
    assert.deepEqual(storyEdgeLabelPosition(0, 340, 170, 0, false, 1), { x: 170, y: 0 },
      'the existing horizontal label lane is unchanged');

    const component = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
    assert.match(component, /sourcePosition: graphLayout!\.direction === 'vertical' \? Position\.Bottom : Position\.Right/u);
    assert.match(component, /targetPosition: graphLayout!\.direction === 'vertical' \? Position\.Top : Position\.Left/u);
  });

  it('keeps Guide edge labels readable at the node and canvas scale', () => {
    const styles = readFileSync(new URL('../src/visual-story.css', import.meta.url), 'utf8');
    const labelRule = styles.match(/\.visual-story \.story-edge-label\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(labelRule, /max-width:\s*180px/u);
    assert.match(labelRule, /max-height:\s*40px/u);
    assert.match(labelRule, /font:\s*500 10px\/1\.2/u);
    const textRule = styles.match(/\.visual-story \.story-edge-label-text\s*\{([^}]+)\}/u)?.[1] ?? '';
    assert.match(textRule, /-webkit-line-clamp:\s*2/u);
    assert.match(textRule, /white-space:\s*normal/u);
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
    const saved = { ...identity, sceneId: 'check', positions: { entry: 12.4, check: 3.2, other: 14 }, fullMap: false, focusLayoutVersion: 4 };
    const progress = readStoryProgress(JSON.stringify(saved), identity, scenes);
    assert.deepEqual(progress, saved);
    const restored = readSectionStoryProgress(progress, identity, scenes);
    assert.deepEqual(restored, { index: 1, positions: [['entry', 12.4], ['check', 3.2]], fullMap: false, viewport: undefined });
    assert.equal('autoPlay' in restored, false);
  });

  it('restores the selected source-backed graph node and edge after the reader remounts', () => {
    const scenes = storyScenes(section, visual);
    const identity = { revisionId: 'revision:one', sectionId: 'section:one', sectionRevisionId: 'section-revision:one' };
    const saved = { ...identity, sceneId: 'check', positions: { entry: 12.4, check: 3.2 }, fullMap: false,
      focusedNodeId: 'check', focusedEdgeId: 'e:1', focusLayoutVersion: 4 };

    const progress = readStoryProgress(JSON.stringify(saved), identity, scenes);
    const restored = readSectionStoryProgress(progress, identity, scenes);

    assert.equal(restored.index, 1);
    assert.equal(restored.focusedNodeId, 'check');
    assert.equal(restored.focusedEdgeId, 'e:1');
    assert.deepEqual(restored.positions, [['entry', 12.4], ['check', 3.2]]);
    assert.equal('autoPlay' in restored, false);
  });

  it('validates saved visual focus and can restore it after diagram data arrives', () => {
    assert.deepEqual(restoreStoryFocus(visual, 'check', 'e:1'), { nodeId: 'check', edgeId: 'e:1' });
    assert.deepEqual(restoreStoryFocus(visual, undefined, 'e:1'), { nodeId: 'check', edgeId: 'e:1' });
    assert.deepEqual(restoreStoryFocus(visual, 'error', 'missing'), { nodeId: 'error', edgeId: null });
    assert.deepEqual(restoreStoryFocus(visual, 'missing', 'missing'), { nodeId: null, edgeId: null });
    assert.deepEqual(restoreStoryFocus(visual, 'error', 'e:0'), { nodeId: 'error', edgeId: null });
  });

  it('drops invalid visual focus metadata without losing the saved scene or narration position', () => {
    const scenes = storyScenes(section, visual);
    const identity = { revisionId: 'revision:one', sectionId: 'section:one', sectionRevisionId: 'section-revision:one' };
    const saved = { ...identity, sceneId: 'check', positions: { entry: 4, check: 9 }, fullMap: false,
      focusedNodeId: 'x'.repeat(257), focusedEdgeId: {}, focusLayoutVersion: 4 };

    const progress = readStoryProgress(JSON.stringify(saved), identity, scenes);
    const restored = readSectionStoryProgress(progress, identity, scenes);

    assert.ok(progress, 'untrusted optional focus data does not invalidate owned playback progress');
    assert.equal(restored.index, 1);
    assert.deepEqual(restored.positions, [['entry', 4], ['check', 9]]);
    assert.equal(restored.focusedNodeId, undefined);
    assert.equal(restored.focusedEdgeId, undefined);
  });

  it('rejects malformed or unscoped progress and never lets it select a foreign scene', () => {
    const scenes = storyScenes(section, visual);
    const identity = { revisionId: 'revision:one', sectionId: 'section:one', sectionRevisionId: 'section-revision:one' };
    for (const raw of [null, '{bad', '"text"', JSON.stringify({ sceneId: 'check', positions: { check: 4 } }), '0'.repeat(16_001)]) {
      assert.equal(readStoryProgress(raw, identity, scenes), undefined);
    }
    const foreign = { ...identity, sceneId: 'other', positions: { entry: 0, check: 0 }, fullMap: false, focusLayoutVersion: 4 };
    assert.equal(readStoryProgress(JSON.stringify(foreign), identity, scenes), undefined);
    const unversioned = { ...identity, sceneId: 'check', positions: { entry: 0, check: 0 }, fullMap: false };
    assert.equal(readStoryProgress(JSON.stringify(unversioned), identity, scenes), undefined);
    const extra = { ...identity, sceneId: 'check', positions: { entry: 0, check: 0 }, fullMap: false, focusLayoutVersion: 4, autoPlay: true };
    assert.equal(readStoryProgress(JSON.stringify(extra), identity, scenes), undefined);
    const bad = { ...identity, sceneId: 'check', positions: { entry: 0, check: 3601 }, fullMap: false, focusLayoutVersion: 4 };
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
