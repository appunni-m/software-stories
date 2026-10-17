import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { validScenePresentation } from '../src/lib/scenePresentation.ts';
import { inspectInteractiveVisual } from '../src/lib/visuals.ts';
import { readFileSync } from 'node:fs';

const claims = [{ claimId: 'claim:one', anchors: [{ anchorId: 'anchor:one', excerpt: 'return `Hello, ${name}`;' }] }];
const nodes = [{ nodeId: 'node:one', label: 'Greeting', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:one'] }];
const element = (kind, value = '', valueOrigin = 'none', parentIndex = null) => ({ kind, parentIndex, nodeIndex: 0, tone: 'accent', value, valueOrigin });
const presentation = (elements = [element('code', 'Hello, ${name}', 'source')]) => ({ schemaVersion: 1, scenes: [{ claimIndex: 0, title: 'An incoming name becomes a greeting', mode: 'composition', elements }] });

describe('AI-authored HTML scene contract', () => {
  it('reads every published version 2 scene in the real partial Fontdone run', () => {
    const artifact = JSON.parse(readFileSync(new URL('../docs/acceptance/artifacts/2026-10-17-fontdone-scene-v2-partial.json', import.meta.url)));
    assert.equal(artifact.answer.outcome, 'partial');
    assert.equal(artifact.answer.sections.length, 3);
    for (const published of artifact.answer.sections) {
      for (const section of published.preview.revision.sections) {
        assert.equal(section.visual.presentation.schemaVersion, 2);
        assert.ok(validScenePresentation(section.visual.presentation, section.visual.nodes, section.claims));
      }
    }
  });
  it('version 2 allows meaningful values and roles while rejecting presentation-only changes', () => {
    const root = { ...element('row'), nodeIndex: null };
    const arrow = { ...element('arrow', '', 'none', 0), nodeIndex: null };
    const same = presentation([root, element('card', '', 'none', 0), arrow, { ...element('code', '', 'none', 0), tone: 'positive' }]);
    same.schemaVersion = 2;
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
    for (const version of ['2', 0, 3, null]) {
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
    tree.schemaVersion = 2;
    const duplicated = [nodes[0], { ...nodes[0], nodeId: 'node:two' }];
    assert.equal(validScenePresentation(tree, duplicated, claims), false);
    tree.schemaVersion = 1;
    assert.equal(validScenePresentation(tree, duplicated, claims), true);
  });
  it('rejects the recorded repeated transformation in new scenes while preserving version 1', () => {
    const artifact = JSON.parse(readFileSync(new URL('../docs/acceptance/artifacts/2026-10-17-fontdone-short-references-partial.json', import.meta.url)));
    const section = artifact.answer.sections[0].preview.revision.sections[0];
    const authored = structuredClone(section.visual.presentation);
    assert.equal(validScenePresentation(authored, section.visual.nodes, section.claims), true);
    authored.schemaVersion = 2;
    assert.equal(validScenePresentation(authored, section.visual.nodes, section.claims), false);
  });
  it('accepts composable layouts with exact source values, illustrations and graph scenes', () => {
    const root = { ...element('row'), nodeIndex: null };
    assert.ok(validScenePresentation(presentation([root, element('bytes', '48 65 6c 6c 6f', 'illustration', 0), element('token', 'Hello, Ada', 'illustration', 0)]), nodes, claims));
    assert.ok(validScenePresentation(presentation(), nodes, claims));
    assert.ok(validScenePresentation({ schemaVersion: 1, scenes: [{ claimIndex: 0, title: 'The actual call path', mode: 'graph', elements: [] }] }, nodes, claims));
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
