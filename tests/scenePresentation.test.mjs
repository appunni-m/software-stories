import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { sceneChildIndices, sceneNodeAccessibleLabel, validScenePresentation, validStoredScenePresentation } from '../src/lib/scenePresentation.ts';
import { inspectInteractiveVisual } from '../src/lib/visuals.ts';
import { storyScenes } from '../src/lib/visualStory.ts';

const claims = [{ claimId: 'claim:one', anchors: [{ anchorId: 'anchor:one', excerpt: 'return `Hello, ${name}`;' }] }];
const nodes = [{ nodeId: 'node:one', label: 'Greeting', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:one'] }];
const element = (kind, value = '', valueOrigin = 'none', parentIndex = null) => ({ kind, parentIndex, nodeIndex: 0, tone: 'accent', value, valueOrigin });
const presentation = (elements = [element('code', 'Hello, ${name}', 'source')]) => ({ schemaVersion: 3, scenes: [{ claimIndex: 0, title: 'An incoming name becomes a greeting', narration: 'A name becomes a greeting with the same prefix for every input.', mode: 'composition', elements }] });

describe('AI-authored HTML scene contract', () => {
  it('orders stack children top-first for visual and assistive-technology reading', () => {
    const elements = [
      { ...element('stack'), nodeIndex: null },
      element('token', 'oldest item', 'illustration', 0),
      element('token', 'newest item', 'illustration', 0),
    ];
    assert.deepEqual(sceneChildIndices(elements, 0), [2, 1]);
    assert.deepEqual(sceneChildIndices([{ ...element('row'), nodeIndex: null }, ...elements.slice(1)], 0), [1, 2]);
  });

  it('makes each source-linked visual cue meaningful to keyboard and screen-reader users', () => {
    assert.equal(sceneNodeAccessibleLabel({
      label: 'PFR0 prefix check',
      detail: 'The source checks whether data starts with b"PFR0".',
    }, 'PFR0', 'illustration'), 'PFR0 prefix check. Shown value: PFR0. Meaning: The source checks whether data starts with b"PFR0". Illustrative example. Open supporting code');
    assert.equal(sceneNodeAccessibleLabel({ label: 'Return target', detail: null }, '', 'none'), 'Return target. Open supporting code');
    assert.equal(sceneNodeAccessibleLabel({ label: 'Format decision', detail: null }, 'first format check', 'explanation'), 'Format decision. Shown value: first format check. Open supporting code');
    assert.equal(sceneNodeAccessibleLabel({ label: 'Source token', detail: null }, 'PFR0', 'source'), 'Source token. Shown value: PFR0. Source value. Open supporting code');
  });

  it('keeps grounded explanatory copy separate from literal source and hypothetical examples', () => {
    const grounded = presentation([element('text', 'The first matching format selects this decoder', 'explanation')]);
    assert.equal(validScenePresentation(grounded, nodes, claims), true);
    const ungroundedSource = presentation([element('text', 'The first matching format selects this decoder', 'source')]);
    assert.equal(validScenePresentation(ungroundedSource, nodes, claims), false);
  });

  it('allows each claim to choose composition or graph mode within one visual section', () => {
    const twoClaims = [claims[0], {
      claimId: 'claim:two',
      anchors: [{ anchorId: 'anchor:two', excerpt: 'load();' }],
    }];
    const twoNodes = [nodes[0], {
      ...nodes[0], nodeId: 'node:two', label: 'Load', evidenceAnchorIds: ['anchor:two'],
    }];
    const mixed = {
      schemaVersion: 3,
      scenes: [
        ...presentation().scenes,
        {
          claimIndex: 1, title: 'The call reaches its next step',
          narration: 'The accepted value reaches the load call.', mode: 'graph', elements: [],
        },
      ],
    };
    assert.equal(validScenePresentation(mixed, twoNodes, twoClaims), true);
  });

  it('current scenes allow meaningful values and roles while rejecting presentation-only changes', () => {
    const root = { ...element('row'), nodeIndex: null };
    const arrow = { ...element('arrow', '', 'none', 0), nodeIndex: null };
    const same = presentation([root, element('card', '', 'none', 0), arrow, { ...element('code', '', 'none', 0), tone: 'positive' }]);
    assert.equal(validScenePresentation(same, nodes, claims), false);
    for (const [before, after] of [['input', 'output'], ['  Mira  ', 'Mira'], ['a', 'A']]) {
      same.scenes[0].elements[1] = element('token', before, 'illustration', 0);
      same.scenes[0].elements[3] = element('token', after, 'illustration', 0);
      assert.equal(validScenePresentation(same, nodes, claims), true);
    }
    same.scenes[0].elements[3].value = 'a';
    same.scenes[0].elements[3].nodeIndex = 1;
    const duplicateNodes = [nodes[0], { ...nodes[0], nodeId: 'node:two' }];
    assert.equal(validScenePresentation(same, duplicateNodes, claims), false);
    duplicateNodes[1].label = 'Result role';
    assert.equal(validScenePresentation(same, duplicateNodes, claims), true);
    same.scenes[0].elements[2].kind = 'divider';
    duplicateNodes[1].label = nodes[0].label;
    assert.equal(validScenePresentation(same, duplicateNodes, claims), true);
    same.scenes[0].mode = 'graph'; same.scenes[0].elements = [];
    assert.equal(validScenePresentation(same, nodes, claims), true);
    for (const version of ['3', 0, 1, 2, null]) {
      same.schemaVersion = version;
      assert.equal(validScenePresentation(same, nodes, claims), false);
    }
  });
  it('compares nested content rather than wrappers, tones or node IDs', () => {
    const tree = presentation([
      { ...element('row'), nodeIndex: null },
      { ...element('panel', '', 'none', 0), nodeIndex: null },
      element('card', 'same', 'illustration', 1),
      { ...element('arrow', '', 'none', 0), nodeIndex: null },
      { ...element('column', '', 'none', 0), nodeIndex: null },
      { ...element('text', 'same', 'illustration', 4), tone: 'positive', nodeIndex: 1 },
    ]);
    const duplicated = [nodes[0], { ...nodes[0], nodeId: 'node:two' }];
    assert.equal(validScenePresentation(tree, duplicated, claims), false);
  });
  it('rejects a repeated transformation in the current scene contract', () => {
    const root = { ...element('row'), nodeIndex: null };
    const arrow = { ...element('arrow', '', 'none', 0), nodeIndex: null };
    const repeated = presentation([root, element('card', 'same', 'illustration', 0), arrow,
      { ...element('token', 'same', 'illustration', 0), nodeIndex: 0 }]);
    assert.equal(validScenePresentation(repeated, nodes, claims), false);
  });
  it('accepts composable layouts with exact source values, illustrations and graph scenes', () => {
    const root = { ...element('row'), nodeIndex: null };
    assert.ok(validScenePresentation(presentation([root, element('bytes', '48 65 6c 6c 6f', 'illustration', 0), element('token', 'Hello, Ada', 'illustration', 0)]), nodes, claims));
    assert.ok(validScenePresentation(presentation(), nodes, claims));
    assert.ok(validScenePresentation({ schemaVersion: 3, scenes: [{ claimIndex: 0, title: 'The actual call path', narration: 'The graph traces the route through the handler.', mode: 'graph', elements: [] }] }, nodes, claims));
  });
  it('supports distinct semantic layouts for transformations, nesting and FIFO queues', () => {
    const transform = [
      { ...element('transform'), nodeIndex: null },
      element('token', 'raw input', 'illustration', 0),
      element('token', 'normalize whitespace', 'illustration', 0),
      element('token', 'normalized result', 'illustration', 0),
    ];
    assert.ok(validScenePresentation(presentation(transform), nodes, claims));
    for (const count of [2, 4]) {
      const malformed = [{ ...element('transform'), nodeIndex: null }, ...Array.from({ length: count }, () => element('token', 'x', 'illustration', 0))];
      assert.equal(validScenePresentation(presentation(malformed), nodes, claims), false);
    }
    const nested = [
      { ...element('tree'), nodeIndex: null },
      { ...element('panel', '', 'none', 0), nodeIndex: null },
      element('token', 'parent object', 'illustration', 1),
      { ...element('tree', '', 'none', 1), nodeIndex: null },
      element('token', 'nested field', 'illustration', 3),
    ];
    assert.ok(validScenePresentation(presentation(nested), nodes, claims));
    assert.ok(validScenePresentation(presentation([
      { ...element('queue'), nodeIndex: null },
      element('token', 'oldest item', 'illustration', 0),
      element('token', 'newest item', 'illustration', 0),
    ]), nodes, claims));
  });
  it('supports a coordinate-free point-flow composition with three source-linked stages', () => {
    const pointFlow = [
      { ...element('pointFlow'), nodeIndex: null },
      element('token', 'point', 'explanation', 0),
      element('code', 'map_point(point, transform)', 'illustration', 0),
      element('token', 'mapped point', 'explanation', 0),
    ];
    assert.equal(validScenePresentation(presentation(pointFlow), nodes, claims), true);
    for (const count of [2, 4]) {
      const malformed = [
        { ...element('pointFlow'), nodeIndex: null },
        ...Array.from({ length: count }, (_, index) => element('token', `stage ${index}`, 'illustration', 0)),
      ];
      assert.equal(validScenePresentation(presentation(malformed), nodes, claims), false);
    }
    const wrapped = [
      { ...element('pointFlow'), nodeIndex: null },
      { ...element('column', '', 'none', 0), nodeIndex: null },
      element('token', 'point', 'explanation', 1),
      element('code', 'map_point(point, transform)', 'illustration', 0),
      element('token', 'mapped point', 'explanation', 0),
    ];
    assert.equal(validScenePresentation(presentation(wrapped), nodes, claims), false);
  });
  it('requires ordered selectors to contain guarded branches with visible outcomes', () => {
    const candidateClaims = [{ ...claims[0], prose: 'The first matching candidate wins and a miss tries the next candidate.' }];
    const selector = [
      { ...element('selector'), nodeIndex: null },
      { ...element('branch', '', 'none', 0), nodeIndex: null },
      element('text', 'Flag A is set?', 'illustration', 1),
      element('token', 'Return mode A', 'illustration', 1),
      { ...element('branch', '', 'none', 0), nodeIndex: null },
      element('text', 'No earlier flag matched', 'illustration', 4),
      element('token', 'Use default mode', 'illustration', 4),
    ];
    assert.ok(validScenePresentation(presentation(selector), nodes, candidateClaims));
    for (const malformed of [
      selector.slice(0, 4),
      [selector[0], { ...selector[1], kind: 'row' }, ...selector.slice(2)],
      [selector[0], selector[1], selector[2], selector[4], ...selector.slice(5)],
      [selector[0], selector[1], selector[2], { ...selector[3], kind: 'arrow', nodeIndex: null, value: '', valueOrigin: 'none' }, ...selector.slice(4)],
    ]) assert.equal(validScenePresentation(presentation(malformed), nodes, candidateClaims), false);
  });
  it('rejects selectors unless first-match candidate behavior is explicit', () => {
    const firstMatchClaims = [{ ...claims[0], prose: 'The first matching flag wins; later checks run only after a miss.' }];
    const independentGuardClaims = [{ ...claims[0], prose: 'Independent validation guards run in sequence; each failure returns its own error, while a pass falls through to the next validation check.' }];
    const stack = [
      { ...element('stack'), nodeIndex: null },
      element('token', 'fallback mode', 'illustration', 0),
    ];
    assert.equal(validScenePresentation(presentation(stack), nodes, firstMatchClaims), false);
    const oneGuard = [
      { ...element('branch'), nodeIndex: null },
      element('text', 'Index converts?', 'illustration', 0),
      element('token', 'Return conversion error', 'illustration', 0),
      element('token', 'Continue to bounds check', 'illustration', 0),
    ];
    assert.equal(validScenePresentation(presentation(oneGuard), nodes, independentGuardClaims), true);
    const nestedSelector = [
      { ...element('stack'), nodeIndex: null },
      { ...element('selector', '', 'none', 0), nodeIndex: null, parentIndex: 0 },
      { ...element('branch', '', 'none', 1), nodeIndex: null },
      element('text', 'Flag A is set?', 'illustration', 2),
      element('token', 'Return mode A', 'illustration', 2),
      { ...element('branch', '', 'none', 1), nodeIndex: null },
      element('text', 'No earlier flag matched', 'illustration', 5),
      element('token', 'Use default mode', 'illustration', 5),
    ];
    assert.equal(validScenePresentation(presentation(nestedSelector), nodes, firstMatchClaims), false);

    const selector = [
      { ...element('selector'), nodeIndex: null },
      { ...element('branch', '', 'none', 0), nodeIndex: null },
      element('text', 'Flag A is set?', 'illustration', 1),
      element('token', 'Return mode A', 'illustration', 1),
      { ...element('branch', '', 'none', 0), nodeIndex: null },
      element('text', 'No earlier flag matched', 'illustration', 4),
      element('token', 'Use default mode', 'illustration', 4),
    ];
    assert.equal(validScenePresentation(presentation(selector), nodes, firstMatchClaims), true);
    assert.equal(validScenePresentation(presentation(selector), nodes, independentGuardClaims), false);
    const firstMatchGraph = { schemaVersion: 3, scenes: [{
      claimIndex: 0, title: 'The actual ordered call path', narration: 'First match wins; later work follows only after a miss.', mode: 'graph', elements: [],
    }] };
    assert.equal(validScenePresentation(firstMatchGraph, nodes, firstMatchClaims), false);
    assert.equal(validStoredScenePresentation(firstMatchGraph, nodes, firstMatchClaims), true);
  });
  it('replays structurally safe saved scenes when newer proposal-quality rules reject their layout', () => {
    const narration = 'The parser checks signatures in order. The first match selects a constructor; a miss continues to the next probe.';
    const claim = {
      claimId: 'claim:dispatch', category: { kind: 'codeBehavior' },
      prose: 'The first matching signature chooses a constructor, and a miss continues to later probes.',
      anchors: [{ anchorId: 'anchor:dispatch', excerpt: 'probe();' }],
    };
    const dispatchNodes = ['Binary probes', 'Text probes', 'Fallback and errors'].map((label, index) => ({
      nodeId: `node:dispatch-${index}`, label, detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:dispatch'],
    }));
    const oldComposition = presentation([
      { ...element('column'), nodeIndex: null },
      ...dispatchNodes.map((_node, index) => ({ ...element('card', '', 'none', 0), nodeIndex: index })),
    ]);
    oldComposition.scenes[0].title = 'First match, then fallback';
    oldComposition.scenes[0].narration = narration;
    const visual = {
      schemaVersion: 2, kind: 'sequence', title: 'Ordered dispatch and fallback',
      layoutHint: { direction: 'topToBottom' }, nodes: dispatchNodes, edges: [], presentation: oldComposition,
    };
    const section = { sectionId: 'section:dispatch', title: 'Choose a parser', claims: [claim], spokenText: narration, visual, hotspots: [] };
    const revision = { sections: [section] };

    assert.equal(validScenePresentation(oldComposition, dispatchNodes, [claim]), false);
    assert.equal(validStoredScenePresentation(oldComposition, dispatchNodes, [claim]), true);
    const inspection = inspectInteractiveVisual(revision, section);
    assert.equal(inspection.status, 'available');
    assert.deepEqual(storyScenes(section, inspection.visual), [{
      id: claim.claimId, title: oldComposition.scenes[0].title, narration,
      anchorIds: ['anchor:dispatch'], nodeIds: dispatchNodes.map((node) => node.nodeId),
    }]);
  });
  it('keeps stored-scene replay strict about inert fields and claim-owned evidence', () => {
    const safe = presentation();
    assert.equal(validStoredScenePresentation(safe, nodes, claims), true);

    const executableMarkup = presentation();
    executableMarkup.scenes[0].elements[0].html = '<script>run()</script>';
    assert.equal(validStoredScenePresentation(executableMarkup, nodes, claims), false);

    const unknownSceneField = presentation();
    unknownSceneField.scenes[0].source = 'outside the scene contract';
    assert.equal(validStoredScenePresentation(unknownSceneField, nodes, claims), false);

    assert.equal(validStoredScenePresentation(safe, [{
      ...nodes[0], evidenceAnchorIds: ['anchor:foreign'],
    }], claims), false);
  });
  it('does not confuse independent early returns with candidate dispatch', () => {
    const independentGuardClaims = [{ ...claims[0], prose: 'Several independent validation checks run in order; a failure returns its own error and a pass falls through to the next validation check.' }];
    const candidateClaims = [{ ...claims[0], prose: 'Parser signatures are probed in source order; the first matching candidate wins and a miss tries the next candidate.' }];
    const stack = [
      { ...element('stack'), nodeIndex: null },
      element('token', 'fallback mode', 'illustration', 0),
    ];
    const selector = [
      { ...element('selector'), nodeIndex: null },
      { ...element('branch', '', 'none', 0), nodeIndex: null },
      element('text', 'Guard one?', 'illustration', 1),
      element('token', 'Return result one', 'illustration', 1),
      { ...element('branch', '', 'none', 0), nodeIndex: null },
      element('text', 'No earlier guard matched', 'illustration', 4),
      element('token', 'Continue to fallback', 'illustration', 4),
    ];
    assert.equal(validScenePresentation(presentation(selector), nodes, independentGuardClaims), false);
    assert.equal(validScenePresentation(presentation(stack), nodes, candidateClaims), false);
    assert.equal(validScenePresentation(presentation(selector), nodes, candidateClaims), true);
  });
  it('admits the supported representations without executing values as markup', () => {
    for (const [kind, value] of [['card', ''], ['metric', '3'], ['code', '<script>attack()</script>'], ['bytes', '01 02'], ['matrix', '010\n111\n010'], ['table', 'Input|Output\nAda|Hello, Ada'], ['token', 'name'], ['text', '']]) {
      assert.ok(validScenePresentation(presentation([element(kind, value, value ? 'illustration' : 'none')]), nodes, claims), kind);
    }
  });
  it('rejects raw HTML, CSS, scripts, URLs, handlers, unknown kinds and unknown fields', () => {
    for (const field of ['html', 'css', 'script', 'url', 'onClick', 'style']) {
      const bad = presentation(); bad.scenes[0].elements[0][field] = 'untrusted';
      assert.equal(validScenePresentation(bad, nodes, claims), false, field);
    }
    assert.equal(validScenePresentation(presentation([element('iframe')]), nodes, claims), false);
  });
  it('requires a single bounded acyclic tree and content linked to the current claim', () => {
    for (const bad of [
      [{ ...element('column'), nodeIndex: null }],
      [element('text'), { ...element('text'), parentIndex: 0 }],
      [element('text'), element('text')],
      [{ ...element('text'), nodeIndex: 1 }],
      [{ ...element('column'), nodeIndex: null, parentIndex: 0 }],
    ]) assert.equal(validScenePresentation(presentation(bad), nodes, claims), false);
    assert.equal(validScenePresentation(presentation(), [{ ...nodes[0], evidenceAnchorIds: ['foreign'] }], claims), false);
    const badIndex = presentation(); badIndex.scenes[0].claimIndex = 1;
    assert.equal(validScenePresentation(badIndex, nodes, claims), false);
    const deep = Array.from({ length: 8 }, (_, i) => ({ ...element(i === 7 ? 'text' : 'column'), nodeIndex: i === 7 ? 0 : null, parentIndex: i === 0 ? null : i - 1 }));
    assert.equal(validScenePresentation(presentation(deep), nodes, claims), false);
  });
  it('rejects fabricated source values and malformed or oversized data graphics', () => {
    for (const [kind, value, origin] of [
      ['code', 'Hello, fabricated', 'source'], ['token', 'value', 'none'], ['matrix', '01\n1', 'illustration'],
      ['matrix', 'ab', 'illustration'], ['table', 'a|b\nc', 'illustration'], ['bytes', '01 '.repeat(33), 'illustration'],
      ['code', 'é'.repeat(513), 'illustration'],
    ]) assert.equal(validScenePresentation(presentation([element(kind, value, origin)]), nodes, claims), false);
  });
  it('validates at the visual boundary and preserves existing diagrams without presentations', () => {
    const visual = { schemaVersion: 2, kind: 'dataFlow', title: 'An HTML transformation', layoutHint: { direction: 'auto' }, nodes, edges: [], presentation: presentation() };
    const section = { claims, hotspots: [], visual };
    const revision = { sections: [section] };
    assert.equal(inspectInteractiveVisual(revision, section).status, 'available');
    visual.presentation.scenes[0].elements[0].nodeIndex = 9;
    assert.equal(inspectInteractiveVisual(revision, section).status, 'unavailable');
    delete visual.presentation;
    assert.equal(inspectInteractiveVisual(revision, section).status, 'available');
  });
});

it('requires the current version 3 shape and bounded speech for every claim', () => {
  const current = presentation(); current.schemaVersion = 3;
  current.scenes[0].narration = 'For example, a name of Ada becomes Hello, Ada; the same prefix remains for an empty name.';
  assert.ok(validScenePresentation(current, nodes, claims));
  for (const narration of [undefined, null, '', '  ', 'é'.repeat(2001)]) {
    const bad = structuredClone(current); bad.scenes[0].narration = narration;
    assert.equal(validScenePresentation(bad, nodes, claims), false);
  }
  for (const version of [1, 2]) {
    const previousShape = structuredClone(current); previousShape.schemaVersion = version;
    assert.equal(validScenePresentation(previousShape, nodes, claims), false);
  }
  const missing = structuredClone(current); missing.scenes = [];
  assert.equal(validScenePresentation(missing, nodes, claims), false);
  current.scenes[0].claimIndex = 1;
  assert.equal(validScenePresentation(current, nodes, claims), false);
});
