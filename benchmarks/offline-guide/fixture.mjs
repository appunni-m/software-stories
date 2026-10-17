import { createHash } from 'node:crypto';

export const NARRATION_SCRIPTS = [
  'In greeting, trim removes spaces around the incoming name. For example, the padded name on the left becomes the clean name on the right.',
  'Then greeting chooses its reply from the cleaned name: a blank name gets the stranger greeting, while a nonblank name is inserted into the reply. The table shows those two illustrative outcomes.',
];
export const SOURCE = [
  'export function greeting(name: string) {',
  '  const cleaned = name.trim();',
  '  if (!cleaned) return "Hello, stranger";',
  '  return `Hello, ${cleaned}`;',
  '}',
].join('\n');

/** Explicitly synthetic accepted fixture for export, never inserted into app
 * history and never represented as a fresh agent-generated accepted answer.
 */
export function offlineGuideFixture() {
  const anchor = id => ({
    anchorId: id, path: 'src/greeting.ts', symbolId: 'symbol:greeting', snapshotId: 'snapshot:offline-fixture',
    sourceSha256: createHash('sha256').update(SOURCE).digest('hex'), startByte: 0,
    endByteInclusive: Buffer.byteLength(SOURCE) - 1, startLine: 1, endLine: 5,
    startColumn: 1, endColumn: 2, excerpt: SOURCE,
  });
  const group = kind => ({ kind, parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' });
  const leaf = (kind, nodeIndex, value) => ({ kind, parentIndex: 0, nodeIndex, tone: 'accent', value, valueOrigin: 'illustration' });
  const claims = NARRATION_SCRIPTS.map((prose, index) => ({
    claimId: `claim:scene-${index + 1}`, category: { kind: 'codeBehavior' }, prose,
    anchors: [anchor(`anchor:scene-${index + 1}`)], dependencies: [],
  }));
  return {
    schemaVersion: 4, chapterId: 'chapter:offline-fixture', revisionId: 'revision:offline-fixture', revisionNumber: 1,
    projectId: 'project:offline-fixture', snapshotId: 'snapshot:offline-fixture',
    title: 'Synthetic fixture: a name becomes a greeting', lifecycle: 'accepted', validationStatus: 'valid', createdAtUnixMs: 1,
    target: { targetId: 'target:offline-fixture', scope: { kind: 'file', path: 'src/greeting.ts' } },
    sections: [{
      sectionId: 'section:greeting', sectionRevisionId: 'section-revision:greeting', order: 0,
      title: 'Clean the input, then choose the reply', prose: NARRATION_SCRIPTS.join(' '), spokenText: NARRATION_SCRIPTS.join(' '),
      claims, dependencies: [], hotspots: [], visual: {
        schemaVersion: 2, kind: 'dataFlow', title: 'A greeting in two scenes', layoutHint: { direction: 'leftToRight' },
        nodes: [
          { nodeId: 'node:padded', label: 'Padded name', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:scene-1'] },
          { nodeId: 'node:clean', label: 'Clean name', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:scene-1'] },
          { nodeId: 'node:reply', label: 'Choose the reply', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:scene-2'] },
        ], edges: [], presentation: { schemaVersion: 1, scenes: [
          { claimIndex: 0, title: 'Spaces fall away; the name remains', mode: 'composition', elements: [
            group('compare'), leaf('token', 0, '  Mira  '), leaf('token', 1, 'Mira'),
          ] },
          { claimIndex: 1, title: 'The cleaned name determines the greeting', mode: 'composition', elements: [
            group('column'), leaf('table', 2, 'Cleaned name|Returned greeting\n(blank)|Hello, stranger\nMira|Hello, Mira'),
          ] },
        ] },
      },
    }],
  };
}
