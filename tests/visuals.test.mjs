import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { inspectInteractiveVisual } from '../src/lib/visuals.ts';

const anchor = (anchorId = 'anchor:call', path = 'src/main.ts') => ({
  anchorId,
  path,
  symbolId: 'symbol:main',
  snapshotId: 'snapshot:one',
  sourceSha256: 'a'.repeat(64),
  startByte: 0,
  endByteInclusive: 12,
  startLine: 1,
  endLine: 1,
  startColumn: 1,
  endColumn: 13,
  excerpt: 'runWorker();',
});

const visual = () => ({
  schemaVersion: 3,
  kind: 'dataFlow',
  title: 'Request path',
  layoutHint: { direction: 'leftToRight' },
  nodes: [
    { nodeId: 'node:entry', label: 'handleRequest', detail: 'HTTP entry point', hotspotId: 'hotspot:code', evidenceAnchorIds: ['anchor:call'] },
    { nodeId: 'node:worker', label: 'runWorker', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:call'] },
  ],
  edges: [
    { edgeId: 'edge:call', fromNodeId: 'node:entry', toNodeId: 'node:worker', label: 'calls', evidenceAnchorIds: ['anchor:call'] },
  ],
  presentation: { schemaVersion: 3, scenes: [
    { claimIndex: 0, title: 'The handler calls the worker', narration: 'The handler receives the request and calls the worker.', mode: 'graph', elements: [] },
  ] },
});

const chapter = (diagram = visual(), { anchors = [anchor()], hotspots = [{
  hotspotId: 'hotspot:code',
  label: 'Open implementation',
  destination: { kind: 'codeLens', anchorId: 'anchor:call' },
}] } = {}) => {
  const section = {
    sectionId: 'section:request',
    sectionRevisionId: 'section-revision:request',
    order: 0,
    title: 'Handle a request',
    prose: 'The handler delegates to a worker.',
    spokenText: '',
    claims: [{ claimId: 'claim:request', category: { kind: 'codeBehavior' }, prose: 'A handler calls the worker.', anchors, dependencies: [] }],
    dependencies: [],
    visual: diagram,
    hotspots,
  };
  return {
    schemaVersion: 5,
    chapterId: 'chapter:request',
    revisionId: 'revision:request',
    revisionNumber: 1,
    projectId: 'project:one',
    snapshotId: 'snapshot:one',
    title: 'Request flow',
    lifecycle: 'accepted',
    validationStatus: 'valid',
    createdAtUnixMs: 1,
    target: { targetId: 'target:request', scope: { kind: 'file', path: 'src/main.ts' } },
    sections: [section],
  };
};

describe('interactive diagram safety', () => {
  it('accepts a bounded v3 diagram with exact same-section evidence, a scene, and a valid hotspot route', () => {
    const revision = chapter();
    const result = inspectInteractiveVisual(revision, revision.sections[0]);
    assert.equal(result.status, 'available');
    assert.equal(result.status === 'available' && result.anchorsById.get('anchor:call').path, 'src/main.ts');
    assert.equal(result.status === 'available' && result.hotspotsById.get('hotspot:code').destination.kind, 'codeLens');
  });

  it('rejects graph-only visuals from the prior schema while keeping a missing visual distinct', () => {
    const revision = chapter();
    const section = revision.sections[0];
    delete section.visual.presentation;
    assert.deepEqual(inspectInteractiveVisual(revision, section), { status: 'unavailable', reason: 'malformed' });
    section.visual = null;
    assert.deepEqual(inspectInteractiveVisual(revision, section), { status: 'none' });
  });

  it('keeps absent and unsupported diagrams out of the renderer with a clear version reason', () => {
    const revisionWithoutVisual = chapter(null);
    revisionWithoutVisual.sections[0].visual = null;
    assert.deepEqual(inspectInteractiveVisual(revisionWithoutVisual, revisionWithoutVisual.sections[0]), { status: 'none' });
    const old = chapter({ schemaVersion: 2, kind: 'dataFlow', title: 'Old', layoutHint: { direction: 'auto' }, nodes: [], edges: [] });
    assert.deepEqual(inspectInteractiveVisual(old, old.sections[0]), { status: 'unavailable', reason: 'unsupportedVersion' });
  });

  it('rejects unknown, malformed, oversized, or executable-shaped diagram data as a whole', () => {
    const unknown = chapter({ ...visual(), schemaVersion: 7 });
    assert.deepEqual(inspectInteractiveVisual(unknown, unknown.sections[0]), { status: 'unavailable', reason: 'unsupportedVersion' });
    const executable = chapter({ ...visual(), html: '<script>run()</script>' });
    assert.deepEqual(inspectInteractiveVisual(executable, executable.sections[0]), { status: 'unavailable', reason: 'malformed' });
    const tooManyNodes = chapter({ ...visual(), nodes: Array.from({ length: 501 }, (_, index) => ({
      nodeId: `node:${index}`, label: `Step ${index}`, detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:call'],
    })) });
    assert.equal(inspectInteractiveVisual(tooManyNodes, tooManyNodes.sections[0]).status, 'unavailable');
    const tooLong = chapter({ ...visual(), nodes: [{ ...visual().nodes[0], label: 'é'.repeat(121) }, visual().nodes[1]] });
    assert.equal(inspectInteractiveVisual(tooLong, tooLong.sections[0]).status, 'unavailable');
  });

  it('rejects duplicate IDs, unresolved endpoints, missing or foreign evidence, and invalid hotspots', () => {
    const duplicate = visual();
    duplicate.edges[0].edgeId = duplicate.nodes[0].nodeId;
    const badEndpoint = visual();
    badEndpoint.edges[0].toNodeId = 'node:elsewhere';
    const missingEvidence = visual();
    missingEvidence.edges[0].evidenceAnchorIds = [];
    const wrongSectionEvidence = visual();
    wrongSectionEvidence.nodes[0].evidenceAnchorIds = ['anchor:other'];
    const foreignEvidence = chapter(wrongSectionEvidence);
    foreignEvidence.sections.push({
      sectionId: 'section:other',
      sectionRevisionId: 'section-revision:other',
      order: 1,
      title: 'Another section',
      prose: 'This source citation belongs to another section.',
      spokenText: '',
      claims: [{ claimId: 'claim:other', category: { kind: 'codeBehavior' }, prose: 'Other code.', anchors: [anchor('anchor:other', 'src/other.ts')], dependencies: [] }],
      dependencies: [],
      visual: null,
      hotspots: [],
    });
    const invalidHotspot = visual();
    invalidHotspot.nodes[0].hotspotId = 'hotspot:missing';
    const badDestination = chapter(visual(), { hotspots: [{
      hotspotId: 'hotspot:code', label: 'Open implementation', destination: { kind: 'section', sectionId: 'section:unrelated' },
    }] });

    for (const candidate of [duplicate, badEndpoint, missingEvidence, wrongSectionEvidence, invalidHotspot]) {
      const revision = chapter(candidate);
      assert.equal(inspectInteractiveVisual(revision, revision.sections[0]).status, 'unavailable');
    }
    assert.equal(inspectInteractiveVisual(foreignEvidence, foreignEvidence.sections[0]).status, 'unavailable');
    assert.equal(inspectInteractiveVisual(badDestination, badDestination.sections[0]).status, 'unavailable');
  });
});
