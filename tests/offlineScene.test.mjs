import assert from 'node:assert/strict';
import { it } from 'node:test';
import { OFFLINE_SCENE_CSS, renderOfflineScene } from '../src/lib/offlineScene.ts';
import { validScenePresentation } from '../src/lib/scenePresentation.ts';

const anchor = { anchorId: 'anchor:one', excerpt: 'run()' };
const visual = { nodes: [{ label: 'Example <input>', evidenceAnchorIds: ['anchor:one'] }] };
const claims = [{ anchors: [anchor] }];
const scene = (kind, value, parentKind = 'row') => ({ claimIndex: 0, title: 'An illustrative representation', narration: 'This representation shows the value in its supporting context.', mode: 'composition', elements: [
  { kind: parentKind, parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
  { kind, parentIndex: 0, nodeIndex: 0, tone: 'accent', value, valueOrigin: 'illustration' },
] });

it('offline composition preserves every supported data and layout primitive', () => {
  const values = { card: 'A card', token: 'PFR0', code: 'run()', metric: '42', bytes: '50 46 52 30', matrix: '01\n10', table: 'input|output\nA|B', text: 'A useful cue' };
  for (const [kind, value] of Object.entries(values)) {
    for (const group of ['row', 'column', 'grid', 'panel', 'compare', 'stack', 'tree', 'queue']) {
      const authored = scene(kind, value, group);
      assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, visual.nodes, claims));
      for (const schemaVersion of [1, 2]) assert.equal(validScenePresentation({ schemaVersion, scenes: [authored] }, visual.nodes, claims), false);
      const markup = renderOfflineScene(authored, visual, () => '#evidence-1-1');
      assert.ok(markup.includes(`scene-${kind}`));
      assert.ok(markup.includes(`scene-${group}`));
      assert.match(markup, /Illustrative example/);
      assert.match(markup, /href="#evidence-1-1"/);
      assert.doesNotMatch(markup, /<script|onClick|onclick|<input>/);
      if (kind === 'matrix') {
        assert.match(markup, /aria-label="2 by 2 cell representation; row 1: 01; row 2: 10"/);
        assert.equal((markup.match(/class="filled"/g) ?? []).length, 2);
      }
      if (kind === 'table') assert.match(markup, /<th scope="col">input<\/th>/);
      if (kind === 'bytes') assert.match(markup, /<b>30<\/b><small>3<\/small>/);
    }
  }
});

it('offline stacks read and display the next-to-pop item first', () => {
  const stackVisual = { nodes: [
    { label: 'Oldest item', evidenceAnchorIds: ['anchor:one'] },
    { label: 'Newest item', evidenceAnchorIds: ['anchor:two'] },
  ] };
  const stackClaims = [{ anchors: [anchor, { anchorId: 'anchor:two', excerpt: 'run()' }] }];
  const authored = {
    claimIndex: 0, title: 'Stack order', narration: 'The newest item leaves first.', mode: 'composition',
    elements: [
      { kind: 'stack', parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
      { kind: 'token', parentIndex: 0, nodeIndex: 0, tone: 'neutral', value: 'oldest', valueOrigin: 'illustration' },
      { kind: 'token', parentIndex: 0, nodeIndex: 1, tone: 'accent', value: 'newest', valueOrigin: 'illustration' },
    ],
  };
  assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, stackVisual.nodes, stackClaims));
  const markup = renderOfflineScene(authored, stackVisual, index => `#evidence-${index}`);
  assert.ok(markup.indexOf('Newest item') < markup.indexOf('Oldest item'));
  assert.match(markup, /aria-label="Last-in, first-out stack\. Top item is next to pop\."/);
  assert.match(OFFLINE_SCENE_CSS, /\.scene-stack:before\{content:'NEXT TO POP'/);
  assert.doesNotMatch(OFFLINE_SCENE_CSS, /\.scene-stack\{[^}]*column-reverse/u);
});

it('offline scenes keep semantic transform roles and source-linked destinations', () => {
  const authored = scene('token', 'raw input', 'transform');
  authored.elements.push(
    { kind: 'token', parentIndex: 0, nodeIndex: 0, tone: 'neutral', value: 'trim and lowercase', valueOrigin: 'illustration' },
    { kind: 'token', parentIndex: 0, nodeIndex: 0, tone: 'positive', value: 'normalized output', valueOrigin: 'illustration' },
  );
  assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, visual.nodes, claims));
  const markup = renderOfflineScene(authored, visual, () => '#evidence-1-1');
  assert.match(markup, /scene-transform/);
  assert.match(markup, /<small>Input<\/small>/);
  assert.match(markup, /<small>Operation<\/small>/);
  assert.match(markup, /<small>Result<\/small>/);
  assert.equal((markup.match(/scene-transform-arrow/g) ?? []).length, 2);
  assert.equal((markup.match(/href="#evidence-1-1"/g) ?? []).length, 3);
});

it('offline pointFlow scenes show point symbols without implying coordinates', () => {
  const authored = scene('token', 'pt', 'pointFlow');
  authored.elements.push(
    { kind: 'code', parentIndex: 0, nodeIndex: 0, tone: 'accent', value: 'transform_point(pt, comp)', valueOrigin: 'illustration' },
    { kind: 'token', parentIndex: 0, nodeIndex: 0, tone: 'positive', value: 'transformed', valueOrigin: 'explanation' },
  );
  assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, visual.nodes, claims));
  const markup = renderOfflineScene(authored, visual, () => '#evidence-1-1');
  assert.match(markup, /scene-pointFlow/);
  assert.match(markup, /Coordinate-free point transformation; symbolic only, no coordinates shown/);
  assert.match(markup, /Symbolic point flow · no coordinates shown/);
  assert.match(markup, /<small>Point in<\/small>/);
  assert.match(markup, /<small>Operation<\/small>/);
  assert.match(markup, /<small>Point out<\/small>/);
  assert.equal((markup.match(/scene-pointFlow-arrow/g) ?? []).length, 2);
  assert.equal((markup.match(/href="#evidence-1-1"/g) ?? []).length, 3);
});

it('offline scenes do not badge grounded explanatory copy as a hypothetical example', () => {
  const authored = scene('text', 'The first matching format selects this decoder');
  authored.elements[1].valueOrigin = 'explanation';
  assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, visual.nodes, claims));
  const markup = renderOfflineScene(authored, visual, () => '#evidence-1-1');
  assert.match(markup, /The first matching format selects this decoder/);
  assert.doesNotMatch(markup, /Illustrative example/);
});

it('offline scenes show ordered selector checks and explicit fall-through', () => {
  const candidateClaims = [{ ...claims[0], prose: 'Parser candidates are tried in order; the first matching candidate wins and a miss tries the next candidate.' }];
  const authored = scene('token', 'first output');
  authored.elements = [
    { kind: 'selector', parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
    { kind: 'branch', parentIndex: 0, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
    { kind: 'text', parentIndex: 1, nodeIndex: 0, tone: 'accent', value: 'Flag A is set?', valueOrigin: 'illustration' },
    { kind: 'token', parentIndex: 1, nodeIndex: 0, tone: 'positive', value: 'Use mode A', valueOrigin: 'illustration' },
    { kind: 'branch', parentIndex: 0, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
    { kind: 'text', parentIndex: 4, nodeIndex: 0, tone: 'accent', value: 'Otherwise', valueOrigin: 'illustration' },
    { kind: 'token', parentIndex: 4, nodeIndex: 0, tone: 'positive', value: 'Use default mode', valueOrigin: 'illustration' },
  ];
  assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, visual.nodes, candidateClaims));
  const markup = renderOfflineScene(authored, visual, () => '#evidence-1-1');
  assert.equal((markup.match(/scene-selector-step/g) ?? []).length, 2);
  assert.equal((markup.match(/scene-selector-label/g) ?? []).length, 2);
  assert.match(markup, /Check 1/);
  assert.match(markup, /Check 2/);
  assert.match(markup, /No match · continue/);
  assert.equal((markup.match(/class="scene-branch-item"/g) ?? []).length, 4);
  assert.equal((markup.match(/href="#evidence-1-1"/g) ?? []).length, 4);
});

it('payload markup stays escaped and only generated package destinations become links', () => {
  const authored = scene('code', '<script>bad()</script><img src=x onerror="bad()">');
  assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [authored] }, visual.nodes, claims));
  for (const destination of ['javascript:bad()', 'file:///private/source.rs', 'https://example.com', '#evidence" onclick="bad()']) {
    const markup = renderOfflineScene(authored, visual, () => destination);
    assert.doesNotMatch(markup, /<script|<img|href=|onclick=/);
    assert.match(markup, /&lt;script&gt;/);
    assert.match(markup, /Supporting source omitted/);
  }
});
