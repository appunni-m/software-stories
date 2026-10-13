import assert from 'node:assert/strict';
import test from 'node:test';
import { compareChapterRevisions, revisionHistoryMetadata } from '../src/lib/revisionHistory.ts';

const baseRevision = () => ({
  schemaVersion: 1,
  chapterId: 'chapter:atlas',
  revisionId: 'revision:one',
  revisionNumber: 1,
  projectId: 'project:demo',
  snapshotId: 'snapshot:one',
  target: { targetId: 'target:one', scope: { kind: 'overview' } },
  title: 'How the request moves',
  lifecycle: 'draft',
  validationStatus: 'valid',
  createdAtUnixMs: 1_791_292_800_000,
  sections: [{
    sectionId: 'section:request',
    sectionRevisionId: 'section-revision:one',
    order: 0,
    title: 'Request entry point',
    prose: 'The handler receives the request.',
    spokenText: 'A request enters here.',
    claims: [{
      claimId: 'claim:entry',
      category: { kind: 'codeBehavior' },
      prose: 'The route dispatches the request.',
      anchors: [{
        anchorId: 'anchor:route',
        path: 'src/routes.ts',
        symbolId: 'symbol:route',
        snapshotId: 'snapshot:one',
        sourceSha256: 'a'.repeat(64),
        startByte: 10,
        endByteInclusive: 42,
        startLine: 2,
        endLine: 4,
        startColumn: 1,
        endColumn: 5,
        excerpt: 'SAFE_EXCERPT_SENTINEL: route(request) { return dispatch(request); }',
      }],
      dependencies: [{ kind: 'symbol', path: 'src/routes.ts', sourceSha256: 'b'.repeat(64), symbolId: 'symbol:route' }],
    }],
    dependencies: [
      { kind: 'sourceFile', path: 'src/routes.ts', sourceSha256: 'c'.repeat(64) },
      { kind: 'symbol', path: 'src/routes.ts', sourceSha256: 'd'.repeat(64), symbolId: 'symbol:route' },
      { kind: 'relationship', path: 'src/routes.ts', sourceSha256: 'e'.repeat(64), relationshipId: 'relationship:dispatch' },
    ],
    visual: {
      schemaVersion: 2,
      kind: 'dataFlow',
      title: 'Request flow',
      layoutHint: { direction: 'leftToRight' },
      nodes: [
        { nodeId: 'node:request', label: 'Request', detail: 'Incoming HTTP request', hotspotId: 'hotspot:route', evidenceAnchorIds: ['anchor:route'] },
        { nodeId: 'node:handler', label: 'Handler', detail: null, hotspotId: null, evidenceAnchorIds: [] },
      ],
      edges: [{ edgeId: 'edge:request-handler', fromNodeId: 'node:request', toNodeId: 'node:handler', label: 'dispatches', evidenceAnchorIds: ['anchor:route'] }],
    },
    hotspots: [{
      hotspotId: 'hotspot:route',
      label: 'Open route code',
      destination: { kind: 'codeLens', anchorId: 'anchor:route' },
    }],
  }],
});

function hasField(diff, entity, field) {
  return diff.changes.some((change) => change.entity === entity && change.field === field);
}

test('same stable content compares equal independent of object key order and revision metadata', () => {
  const before = baseRevision();
  const after = structuredClone(before);
  after.revisionId = 'revision:two';
  after.revisionNumber = 2;
  after.createdAtUnixMs += 1_000;
  after.basedOnRevisionId = before.revisionId;
  after.sections[0].sectionRevisionId = 'section-revision:two';
  after.sections[0].claims[0].category = { kind: 'codeBehavior' };
  after.sections[0].claims[0].category = Object.fromEntries(Object.entries(after.sections[0].claims[0].category).reverse());

  const diff = compareChapterRevisions(before, after);
  assert.equal(diff.identical, true);
  assert.deepEqual(diff.counts, { added: 0, removed: 0, changed: 0, total: 0 });
});

test('compares chapter, section, claim, anchor, dependency, visual, hotspot and destination fields by identity', () => {
  const before = baseRevision();
  const after = structuredClone(before);
  after.revisionId = 'revision:two';
  after.title = 'How the request is routed';
  after.target.targetId = 'target:two';
  after.target.scope = { kind: 'file', path: 'src/routes.ts' };
  const section = after.sections[0];
  section.order = 1;
  section.title = 'Updated route entry';
  section.prose = 'The updated handler validates then dispatches.';
  section.spokenText = 'First it validates, then dispatches.';
  const claim = section.claims[0];
  claim.category = { kind: 'inference', visibleLabel: 'Routing decision', basis: 'The call chain implies routing.' };
  claim.prose = 'The route validates the request before dispatch.';
  const anchor = claim.anchors[0];
  anchor.path = 'src/http/routes.ts';
  anchor.symbolId = 'symbol:new-route';
  anchor.snapshotId = 'snapshot:two';
  anchor.sourceSha256 = 'f'.repeat(64);
  anchor.startByte = 12;
  anchor.endByteInclusive = 48;
  anchor.startLine = 3;
  anchor.endLine = 5;
  anchor.startColumn = 2;
  anchor.endColumn = 8;
  anchor.excerpt = 'CHANGED_EXCERPT_SENTINEL: handle(request) { validate(request); dispatch(request); }';
  claim.dependencies[0].path = 'src/http/routes.ts';
  claim.dependencies[0].sourceSha256 = '9'.repeat(64);
  section.dependencies[0].sourceSha256 = '8'.repeat(64);
  section.dependencies[1].path = 'src/http/routes.ts';
  section.dependencies[1].sourceSha256 = '7'.repeat(64);
  section.dependencies[2].sourceSha256 = '6'.repeat(64);
  section.visual.kind = 'sequence';
  section.visual.title = 'Validation sequence';
  section.visual.layoutHint.direction = 'topToBottom';
  section.visual.nodes[0].label = 'Validated request';
  section.visual.nodes[0].detail = 'Request after validation';
  section.visual.nodes[0].hotspotId = null;
  section.visual.nodes[0].evidenceAnchorIds = ['anchor:other'];
  section.visual.edges[0].fromNodeId = 'node:handler';
  section.visual.edges[0].toNodeId = 'node:request';
  section.visual.edges[0].label = 'validates';
  section.visual.edges[0].evidenceAnchorIds = ['anchor:other'];
  section.hotspots[0].label = 'Open updated route';
  section.hotspots[0].destination = { kind: 'atlasSymbol', path: 'src/http/routes.ts', symbolId: 'symbol:new-route' };

  const diff = compareChapterRevisions(before, after);
  for (const field of ['title', 'targetId', 'targetScope']) assert.ok(hasField(diff, 'chapter', field), `missing chapter ${field}`);
  for (const field of ['order', 'title', 'prose', 'spokenText']) assert.ok(hasField(diff, 'section', field), `missing section ${field}`);
  for (const field of ['category', 'prose']) assert.ok(hasField(diff, 'claim', field), `missing claim ${field}`);
  for (const field of [
    'path', 'symbolId', 'snapshotId', 'sourceSha256', 'startByte', 'endByteInclusive',
    'startLine', 'endLine', 'startColumn', 'endColumn', 'excerpt',
  ]) assert.ok(hasField(diff, 'anchor', field), `missing anchor ${field}`);
  assert.ok(hasField(diff, 'dependency', 'path'));
  assert.ok(hasField(diff, 'dependency', 'sourceSha256'));
  for (const field of ['kind', 'title', 'layoutHint']) assert.ok(hasField(diff, 'visual', field), `missing visual ${field}`);
  for (const field of ['label', 'detail', 'hotspotId', 'evidenceAnchorIds']) assert.ok(hasField(diff, 'visualNode', field), `missing visual node ${field}`);
  for (const field of ['fromNodeId', 'toNodeId', 'label', 'evidenceAnchorIds']) assert.ok(hasField(diff, 'visualEdge', field), `missing visual edge ${field}`);
  assert.ok(hasField(diff, 'hotspot', 'label'));
  assert.ok(hasField(diff, 'hotspot', 'destination'));

  const serialized = JSON.stringify(diff);
  assert.equal(serialized.includes('CHANGED_EXCERPT_SENTINEL'), false, 'source excerpts must not be copied into diff output');
  assert.match(diff.changes.find((change) => change.entity === 'anchor' && change.field === 'excerpt').summary, /contents hidden/);
  assert.ok(diff.counts.changed > 20);
});

test('reports stable-ID additions and removals while treating source-reference reordering as unchanged', () => {
  const before = baseRevision();
  const after = structuredClone(before);
  const section = after.sections[0];
  section.claims[0].anchors.push({
    ...structuredClone(section.claims[0].anchors[0]),
    anchorId: 'anchor:new',
    excerpt: 'PRIVATE_NEW_SOURCE_EXCERPT',
  });
  section.claims[0].dependencies.push({ kind: 'sourceFile', path: 'src/new.ts', sourceSha256: '1'.repeat(64) });
  section.dependencies = section.dependencies.filter((item) => item.kind !== 'sourceFile');
  section.dependencies.push({ kind: 'sourceFile', path: 'src/added.ts', sourceSha256: '2'.repeat(64) });
  section.visual.nodes = section.visual.nodes.filter((item) => item.nodeId !== 'node:handler');
  section.visual.nodes.push({ nodeId: 'node:new', label: 'New step', detail: null, hotspotId: null, evidenceAnchorIds: [] });
  section.visual.edges = [];
  section.hotspots = [{ hotspotId: 'hotspot:new', label: 'New destination', destination: { kind: 'section', sectionId: 'section:other' } }];

  const diff = compareChapterRevisions(before, after);
  const identityChanges = diff.changes.map(({ entity, kind, id }) => `${entity}:${kind}:${id}`);
  for (const expected of [
    'anchor:added:anchor:new',
    'dependency:added:sourceFile:src/added.ts',
    'dependency:removed:sourceFile:src/routes.ts',
    'visualNode:added:node:new',
    'visualNode:removed:node:handler',
    'visualEdge:removed:edge:request-handler',
    'hotspot:added:hotspot:new',
    'hotspot:removed:hotspot:route',
  ]) assert.ok(identityChanges.includes(expected), `missing ${expected}`);
  assert.equal(JSON.stringify(diff).includes('PRIVATE_NEW_SOURCE_EXCERPT'), false);

  const reordered = baseRevision();
  const withSecondAnchor = structuredClone(reordered);
  withSecondAnchor.sections[0].claims[0].anchors.push({
    ...structuredClone(withSecondAnchor.sections[0].claims[0].anchors[0]),
    anchorId: 'anchor:second',
    startLine: 7,
    endLine: 7,
  });
  const reverse = structuredClone(withSecondAnchor);
  reverse.sections[0].claims[0].anchors.reverse();
  assert.equal(compareChapterRevisions(withSecondAnchor, reverse).identical, true);
});

test('reports added and removed visual definitions and sections as stable-ID records', () => {
  const before = baseRevision();
  const withoutVisual = structuredClone(before);
  withoutVisual.sections[0].visual = null;
  const removed = compareChapterRevisions(before, withoutVisual);
  assert.equal(removed.changes[0].entity, 'visual');
  assert.equal(removed.changes[0].kind, 'removed');

  const withoutSection = structuredClone(before);
  withoutSection.sections = [];
  const sectionDiff = compareChapterRevisions(before, withoutSection);
  assert.deepEqual(sectionDiff.changes.map(({ entity, kind, id }) => ({ entity, kind, id })), [
    { entity: 'section', kind: 'removed', id: 'section:request' },
  ]);
});

test('resolves explicit parent and valid creation time, without guessing missing lineage', () => {
  const parent = baseRevision();
  parent.revisionNumber = 4;
  parent.title = 'Parent revision';
  const child = structuredClone(parent);
  child.revisionId = 'revision:child';
  child.revisionNumber = 5;
  child.basedOnRevisionId = parent.revisionId;
  const metadata = revisionHistoryMetadata(child, [parent]);
  assert.equal(metadata.parentRevisionId, parent.revisionId);
  assert.equal(metadata.parentRevisionNumber, 4);
  assert.equal(metadata.parentRevisionTitle, 'Parent revision');
  assert.equal(metadata.createdAtUnixMs, child.createdAtUnixMs);
  assert.equal(metadata.createdAtIso, new Date(child.createdAtUnixMs).toISOString());

  const orphan = revisionHistoryMetadata({ ...child, basedOnRevisionId: 'revision:missing', createdAtUnixMs: Number.MAX_SAFE_INTEGER + 1 });
  assert.equal(orphan.parentRevisionId, 'revision:missing');
  assert.equal(orphan.parentRevisionNumber, null);
  assert.equal(orphan.parentRevisionTitle, null);
  assert.equal(orphan.createdAtUnixMs, null);
  assert.equal(orphan.createdAtIso, null);

  const legacy = revisionHistoryMetadata({ ...child, basedOnRevisionId: null, createdAtUnixMs: -1 });
  assert.equal(legacy.parentRevisionId, null);
  assert.equal(legacy.createdAtUnixMs, null);
});
