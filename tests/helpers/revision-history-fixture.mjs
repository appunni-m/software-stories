export const baseRevision = () => ({
  schemaVersion: 5,
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
      schemaVersion: 3,
      kind: 'dataFlow',
      title: 'Request flow',
      layoutHint: { direction: 'leftToRight' },
      nodes: [
        { nodeId: 'node:request', label: 'Request', detail: 'Incoming HTTP request', hotspotId: 'hotspot:route', evidenceAnchorIds: ['anchor:route'] },
        { nodeId: 'node:handler', label: 'Handler', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:route'] },
      ],
      edges: [{ edgeId: 'edge:request-handler', fromNodeId: 'node:request', toNodeId: 'node:handler', label: 'dispatches', evidenceAnchorIds: ['anchor:route'] }],
      presentation: { schemaVersion: 3, scenes: [
        { claimIndex: 0, title: 'The route dispatches the request', narration: 'The route dispatches the request to its handler.', mode: 'graph', elements: [] },
      ] },
    },
    hotspots: [{
      hotspotId: 'hotspot:route',
      label: 'Open route code',
      destination: { kind: 'codeLens', anchorId: 'anchor:route' },
    }],
  }],
});

export function sceneRevision(){
  const revision=baseRevision();revision.schemaVersion=5;
  const section=revision.sections[0];
  section.claims[0].anchors[0].excerpt='SOURCE_VALUE_OLD SOURCE_VALUE_NEW SOURCE_VALUE_ADDED';
  section.visual.nodes[1].evidenceAnchorIds=['anchor:route'];
  section.visual.presentation={schemaVersion:3,scenes:[{
    claimIndex:0,title:'A request arrives',narration:'A request arrives at the route before it reaches the handler.',mode:'composition',
    elements:[
      {kind:'panel',parentIndex:null,nodeIndex:null,tone:'neutral',value:'',valueOrigin:'none'},
      {kind:'code',parentIndex:0,nodeIndex:0,tone:'accent',value:'SOURCE_VALUE_OLD',valueOrigin:'source'},
    ],
  }]};
  return revision;
}

