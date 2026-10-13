import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { describe, it } from 'node:test';
import { buildInventory } from '../src/lib/inventory.ts';
import { buildInventoryTree, flattenInventoryTree, projectProvenRelationshipGraph } from '../src/lib/atlas.ts';
import {
  codeLensRangeFromIndexedAnchor,
  resolveCodeLensCitation,
  sourceLineTextRanges,
  splitCapturedSourceLines,
} from '../src/lib/codeLens.ts';
import {
  initialNavigationState,
  navigateBack,
  navigateToRoute,
  replaceCurrentRoute,
  sourceRangeMatchesHash,
} from '../src/lib/navigation.ts';

const hash = (contents) => createHash('sha256').update(contents).digest('hex');

const mainSource = "import { decode } from './decode';\nexport function start(input: string) {\n  return decode(input);\n}\n";
const decodeSource = 'export function decode(input: string) {\n  return input.trim();\n}\n';
const testSource = "import { decode } from '../src/decode';\ndescribe('decode', () => {\n  it('trims input', () => {\n    expect(decode(' value ')).toBe('value');\n  });\n});\n";
const sourceHashes = {
  'src/main.ts': hash(mainSource),
  'src/decode.ts': hash(decodeSource),
  'tests/decode.test.ts': hash(testSource),
};

const manifest = {
  schemaVersion: 1,
  summary: { projectId: 'project-demo', snapshotId: 'snapshot-demo' },
  directories: [
    { path: '.', kind: 'directory', sizeBytes: null },
    { path: 'src', kind: 'directory', sizeBytes: null },
    { path: 'tests', kind: 'directory', sizeBytes: null },
  ],
  files: Object.entries(sourceHashes).map(([path, sha256]) => ({
    path, kind: 'file', sizeBytes: 100, sha256, contentKind: 'text',
  })),
  skipped: [],
};

const anchor = (path, contents, startLine, endLine = startLine) => {
  const lineRanges = sourceLineTextRanges(contents);
  const start = lineRanges[startLine - 1].start;
  const end = lineRanges[endLine - 1].end;
  return {
    path,
    sourceSha256: hash(contents),
    startByte: Buffer.byteLength(contents.slice(0, start)),
    endByte: Buffer.byteLength(contents.slice(0, end)),
    startLine,
    endLine,
    startColumn: 1,
    endColumn: 1,
  };
};

const indexedSymbol = (symbolId, path, contents, name, { isTest = false } = {}) => ({
  symbolId,
  path,
  language: 'TypeScript',
  kind: 'function',
  name,
  qualifiedName: name,
  declarationIdentity: symbolId,
  signatureSha256: hash(`signature:${symbolId}`),
  isExported: !isTest,
  isTest,
  anchor: anchor(path, contents, isTest ? 2 : 1),
});

const index = {
  schemaVersion: 1,
  indexerVersion: 'journey-fixture',
  projectId: 'project-demo',
  snapshotId: 'snapshot-demo',
  status: 'complete',
  parsers: [],
  files: Object.entries(sourceHashes).map(([path, sourceSha256]) => ({
    path, sourceSha256, status: 'indexed',
  })),
  symbols: [
    indexedSymbol('start', 'src/main.ts', mainSource, 'start'),
    indexedSymbol('decode', 'src/decode.ts', decodeSource, 'decode'),
    indexedSymbol('test-decode', 'tests/decode.test.ts', testSource, 'decode trims input', { isTest: true }),
  ],
  relationships: [
    {
      relationshipId: 'calls-decode', kind: 'calls', sourceId: 'start', targetId: 'decode',
      targetPath: 'src/decode.ts', targetLabel: null, confidence: 'high', unresolvedReason: null,
      evidence: anchor('src/main.ts', mainSource, 3),
    },
    {
      relationshipId: 'test-covers-decode', kind: 'testReference', sourceId: 'test-decode', targetId: 'decode',
      targetPath: 'src/decode.ts', targetLabel: null, confidence: 'high', unresolvedReason: null,
      evidence: anchor('tests/decode.test.ts', testSource, 3, 5),
    },
    {
      relationshipId: 'unresolved-dynamic', kind: 'calls', sourceId: 'start', targetId: null,
      targetPath: null, targetLabel: 'runtime-selected handler', confidence: 'unresolved',
      unresolvedReason: 'dynamic_callee_not_resolved', evidence: anchor('src/main.ts', mainSource, 3),
    },
  ],
};

describe('Atlas to Code Lens repository journey', () => {
  it('keeps a supported test relationship, exact evidence citation, and Atlas selection across Back', () => {
    const rows = buildInventory(manifest);
    const tree = flattenInventoryTree(buildInventoryTree(rows));
    assert.deepEqual(tree.map(({ row }) => row.path), [
      '.', 'src', 'src/decode.ts', 'src/main.ts', 'tests', 'tests/decode.test.ts',
    ]);

    // Atlas draws only relationships proven against the captured snapshot and selected files.
    const selectedPaths = new Set(rows.filter((row) => row.disposition === 'captured').map((row) => row.path));
    const graph = projectProvenRelationshipGraph(index, manifest, selectedPaths);
    assert.deepEqual(graph.edges.map(({ relationship }) => relationship.relationshipId), [
      'calls-decode', 'test-covers-decode',
    ]);

    const edge = graph.edges.find(({ relationship }) => relationship.relationshipId === 'test-covers-decode');
    const testLineRanges = sourceLineTextRanges(testSource);
    assert.ok(edge);
    assert.equal(edge.relationship.kind, 'testReference');
    assert.equal(edge.relationship.confidence, 'high');
    assert.equal(index.symbols.find(({ symbolId }) => symbolId === edge.sourceId)?.isTest, true);
    assert.equal(index.symbols.find(({ symbolId }) => symbolId === edge.targetId)?.name, 'decode');
    assert.deepEqual(edge.evidence, {
      path: 'tests/decode.test.ts',
      sha256: sourceHashes['tests/decode.test.ts'],
      startLine: 3,
      endLine: 5,
      startByte: Buffer.byteLength(testSource.slice(0, testLineRanges[2].start)),
      endByte: Buffer.byteLength(testSource.slice(0, testLineRanges[4].end)),
    });

    // Simulate the Atlas root view selecting the supported edge and opening its exact source.
    const rootAtlas = navigateToRoute(initialNavigationState, {
      workspace: 'atlas',
      projectId: manifest.summary.projectId,
      snapshotId: manifest.summary.snapshotId,
      treeSelection: '.',
      focusTarget: 'atlas-map-edge:test-covers-decode',
    });
    const selectedAtlas = replaceCurrentRoute(rootAtlas, {
      ...rootAtlas.current,
      atlasSelection: { kind: 'relationship', relationshipId: edge.relationship.relationshipId },
      treeSelection: `relationship:${edge.relationship.relationshipId}`,
      mapViewport: { x: -64, y: 28, scale: 1.35 },
      focusTarget: 'atlas-open-edge:test-covers-decode',
    });
    const codeLens = navigateToRoute(selectedAtlas, {
      workspace: 'code-lens',
      projectId: manifest.summary.projectId,
      snapshotId: edge.snapshotId,
      relativePath: edge.evidence.path,
      sourceRange: {
        ...codeLensRangeFromIndexedAnchor({ ...edge.evidence, sourceSha256: edge.evidence.sha256 }),
      },
      focusTarget: 'heading:code-lens',
    });

    assert.equal(codeLens.current.relativePath, 'tests/decode.test.ts');
    assert.equal(codeLens.current.snapshotId, manifest.summary.snapshotId);
    assert.equal(sourceRangeMatchesHash(codeLens.current.sourceRange, sourceHashes['tests/decode.test.ts']), true);
    assert.deepEqual({ startByte: codeLens.current.sourceRange.startByte, endByteExclusive: codeLens.current.sourceRange.endByteExclusive }, {
      startByte: edge.evidence.startByte, endByteExclusive: edge.evidence.endByte,
    });
    const citation = resolveCodeLensCitation(codeLens.current.sourceRange, undefined, sourceHashes['tests/decode.test.ts'], splitCapturedSourceLines(testSource).length, testSource);
    assert.equal(citation.status, 'verified');
    assert.equal(testSource.slice(citation.textRange.start, citation.textRange.end), [
      "  it('trims input', () => {",
      "    expect(decode(' value ')).toBe('value');",
      '  });',
    ].join('\n'));
    const sourceLines = splitCapturedSourceLines(testSource);
    assert.deepEqual(sourceLines.slice(codeLens.current.sourceRange.startLine - 1, codeLens.current.sourceRange.endLine), [
      "  it('trims input', () => {",
      "    expect(decode(' value ')).toBe('value');",
      '  });',
    ]);

    // Back restores the exact Atlas selection, snapshot, root tree context, and return-focus target.
    const returned = navigateBack(codeLens);
    assert.deepEqual(returned.current, selectedAtlas.current);
    assert.equal(returned.current.snapshotId, manifest.summary.snapshotId);
    assert.deepEqual(returned.current.atlasSelection, {
      kind: 'relationship', relationshipId: 'test-covers-decode',
    });
    assert.equal(returned.current.treeSelection, 'relationship:test-covers-decode');
    assert.deepEqual(returned.current.mapViewport, { x: -64, y: 28, scale: 1.35 });
    assert.equal(returned.current.focusTarget, 'atlas-open-edge:test-covers-decode');
  });
});
