import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildInventory } from '../src/lib/inventory.ts';
import { ATLAS_MAX_ZOOM, ATLAS_MIN_ZOOM, ATLAS_ZOOM_STEP, atlasRowIndexStatus, atlasRowIndexStatusLabel, atlasTreeKeyboardAction, buildInventoryTree, filterInventoryTreeRows, filterProvenRelationshipEdges, fitAtlasViewport, flattenInventoryTree, normalizeAtlasViewport, panAtlasViewport, projectProvenRelationshipGraph, summarizeAtlasPathIndexCounts, virtualRowWindow, zoomAtlasViewport } from '../src/lib/atlas.ts';

const sha = (letter) => letter.repeat(64);
const manifest = {
  schemaVersion: 1,
  summary: { snapshotId: 'snap-1' },
  directories: [
    { path: '.', kind: 'directory', sizeBytes: null },
    { path: 'src', kind: 'directory', sizeBytes: null },
    { path: 'assets', kind: 'directory', sizeBytes: null },
  ],
  files: [
    { path: 'src/main.ts', kind: 'file', sizeBytes: 10, sha256: sha('a'), contentKind: 'text' },
    { path: 'src/odd.zzz', kind: 'file', sizeBytes: 4, sha256: sha('b'), contentKind: 'text' },
    { path: 'assets/image.bin', kind: 'file', sizeBytes: 2, sha256: sha('c'), contentKind: 'binary' },
    { path: 'src/helper.ts', kind: 'file', sizeBytes: 12, sha256: sha('d'), contentKind: 'text' },
  ],
  skipped: [
    { path: 'src/large.ts', kind: 'file', sizeBytes: 400, reason: 'fileByteLimit', details: 'too large' },
    { path: 'vendor', kind: 'directory', sizeBytes: null, reason: 'excludedByPolicy', details: 'excluded' },
  ],
};

const anchor = (path, hash) => ({
  path, sourceSha256: hash, startByte: 1, endByte: 5,
  startLine: 2, endLine: 2, startColumn: 1, endColumn: 5,
});
const symbol = (symbolId, path, hash) => ({
  symbolId, path, language: 'TypeScript', kind: 'function', name: symbolId,
  qualifiedName: symbolId, declarationIdentity: symbolId, signatureSha256: sha('f'),
  isExported: true, isTest: false, anchor: anchor(path, hash),
});

const index = {
  schemaVersion: 1, indexerVersion: 'test', projectId: 'project-1', snapshotId: 'snap-1',
  status: 'complete', parsers: [],
  files: [
    { path: 'src/main.ts', sourceSha256: sha('a'), status: 'indexed' },
    { path: 'src/helper.ts', sourceSha256: sha('d'), status: 'indexed' },
    { path: 'src/large.ts', sourceSha256: sha('e'), status: 'indexed' },
  ],
  symbols: [symbol('main', 'src/main.ts', sha('a')), symbol('helper', 'src/helper.ts', sha('d'))],
  relationships: [
    {
      relationshipId: 'r-good', kind: 'calls', sourceId: 'main', targetId: 'helper',
      targetPath: 'src/helper.ts', targetLabel: null, confidence: 'high', unresolvedReason: null,
      evidence: anchor('src/main.ts', sha('a')),
    },
    {
      relationshipId: 'r-dynamic', kind: 'calls', sourceId: 'main', targetId: null,
      targetPath: null, targetLabel: 'dynamic target', confidence: 'unresolved',
      unresolvedReason: 'dynamic_callee_not_resolved', evidence: anchor('src/main.ts', sha('a')),
    },
    {
      relationshipId: 'r-inconsistent', kind: 'imports', sourceId: 'main', targetId: 'helper',
      targetPath: 'src/helper.ts', targetLabel: null, confidence: 'high',
      unresolvedReason: 'unresolved_marker', evidence: anchor('src/main.ts', sha('a')),
    },
  ],
};

describe('Atlas data helpers', () => {
  it('retains every inventory row exactly once and flattens deterministically', () => {
    const rows = buildInventory(manifest);
    const tree = buildInventoryTree(rows);
    const flat = flattenInventoryTree(tree);
    assert.equal(flat.length, rows.length);
    assert.equal(new Set(flat.map((item) => item.row)).size, rows.length);
    assert.deepEqual(flat.map((item) => item.row.path), [
      '.', 'assets', 'assets/image.bin', 'src', 'src/helper.ts', 'src/large.ts', 'src/main.ts', 'src/odd.zzz', 'vendor',
    ]);
    assert.equal(flat.find((item) => item.row.path === 'src/odd.zzz').row.parserReadiness, 'unsupported');
    assert.equal(flat.find((item) => item.row.path === 'src/large.ts').row.disposition, 'skipped');
    assert.equal(flat.find((item) => item.row.path === 'vendor').row.disposition, 'excluded');
    assert.deepEqual(
      [flat.find((item) => item.row.path === 'src').positionInSet, flat.find((item) => item.row.path === 'src').setSize],
      [2, 3],
    );
    assert.deepEqual(flattenInventoryTree(tree, new Set(['src'])).map((item) => item.row.path), [
      '.', 'assets', 'assets/image.bin', 'src', 'vendor',
    ]);
    assert.deepEqual(flattenInventoryTree(tree).map((item) => item.row.path), flattenInventoryTree(buildInventoryTree(rows)).map((item) => item.row.path));
  });

  it('maps tree keys to roving focus and folder expansion actions', () => {
    const tree = buildInventoryTree(buildInventory(manifest));
    const flat = flattenInventoryTree(tree);
    const root = flat.findIndex((item) => item.row.path === '.');
    const assets = flat.findIndex((item) => item.row.path === 'assets');
    const image = flat.findIndex((item) => item.row.path === 'assets/image.bin');
    assert.deepEqual(atlasTreeKeyboardAction(flat, assets, 'ArrowRight'), { kind: 'focus', index: image });
    assert.deepEqual(atlasTreeKeyboardAction(flat, image, 'ArrowLeft'), { kind: 'focus', index: assets });
    assert.deepEqual(atlasTreeKeyboardAction(flat, assets, 'ArrowLeft'), { kind: 'collapse', path: 'assets' });
    assert.deepEqual(atlasTreeKeyboardAction(flat, root, 'ArrowLeft'), { kind: 'collapse', path: '.' });
    assert.deepEqual(atlasTreeKeyboardAction(flat, assets, 'ArrowDown'), { kind: 'focus', index: image });
    assert.deepEqual(atlasTreeKeyboardAction(flat, image, 'Home'), { kind: 'focus', index: 0 });
    assert.deepEqual(atlasTreeKeyboardAction(flat, root, 'End'), { kind: 'focus', index: flat.length - 1 });
    assert.deepEqual(atlasTreeKeyboardAction(flat, assets, 'Enter'), { kind: 'activate' });
    assert.deepEqual(atlasTreeKeyboardAction(flat, assets, ' '), { kind: 'select' });

    const collapsed = flattenInventoryTree(tree, new Set(['assets']));
    const collapsedAssets = collapsed.findIndex((item) => item.row.path === 'assets');
    assert.deepEqual(atlasTreeKeyboardAction(collapsed, collapsedAssets, 'ArrowRight'), { kind: 'expand', path: 'assets' });
    const collapsedRoot = flattenInventoryTree(tree, new Set(['.']));
    assert.deepEqual(atlasTreeKeyboardAction(collapsedRoot, 0, 'ArrowLeft'), { kind: 'handled' });
  });

  it('filters tree rows with ancestor retention and bounded virtual windows', () => {
    const rows = buildInventory(manifest);
    const matching = filterInventoryTreeRows(rows, null, 'helper');
    assert.deepEqual(new Set(matching.map((row) => row.path)), new Set(['.', 'src', 'src/helper.ts']));
    assert.deepEqual(filterInventoryTreeRows(rows, 'src', 'large').map((row) => row.path), ['src', 'src/large.ts']);

    const window = virtualRowWindow(25_000, 14_500 * 29, 510, 29, 10);
    assert.deepEqual(window, {
      startIndex: 14_490,
      endIndex: 14_528,
      topSpacerHeight: 420_210,
      bottomSpacerHeight: (25_000 - 14_528) * 29,
    });
    const offsetWindow = virtualRowWindow(25_000, 14_500 * 29 + 28, 510, 29, 10);
    assert.equal(offsetWindow.endIndex - offsetWindow.startIndex, 39);
    assert.ok(offsetWindow.endIndex - offsetWindow.startIndex <= Math.ceil(510 / 29) + 1 + 20);
    assert.deepEqual(virtualRowWindow(0, 0, 510, 29, 10), {
      startIndex: 0, endIndex: 0, topSpacerHeight: 0, bottomSpacerHeight: 0,
    });
    assert.deepEqual(virtualRowWindow(3, 10_000, 510, 29, 10), {
      startIndex: 0, endIndex: 3, topSpacerHeight: 0, bottomSpacerHeight: 0,
    });
  });

  it('projects only resolved edges whose selected endpoints and evidence match exact snapshot hashes', () => {
    const graph = projectProvenRelationshipGraph(index, manifest, new Set(['src/main.ts', 'src/helper.ts']));
    assert.equal(graph.snapshotId, 'snap-1');
    assert.deepEqual(graph.nodes.map((node) => node.symbol.symbolId), ['helper', 'main']);
    assert.deepEqual(graph.edges.map((edge) => edge.relationship.relationshipId), ['r-good']);
    assert.deepEqual(graph.edges[0].evidence, {
      path: 'src/main.ts', sha256: sha('a'), startLine: 2, endLine: 2, startByte: 1, endByte: 5,
    });
    assert.equal(graph.edges[0].snapshotId, 'snap-1');
    assert.equal(projectProvenRelationshipGraph(index, manifest, new Set(['src/main.ts'])).edges.length, 0);
    const staleEvidenceIndex = {
      ...index,
      relationships: index.relationships.map((relationship) => relationship.relationshipId === 'r-good'
        ? { ...relationship, evidence: anchor('src/main.ts', sha('e')) }
        : relationship),
    };
    assert.equal(projectProvenRelationshipGraph(staleEvidenceIndex, manifest, new Set(['src/main.ts', 'src/helper.ts'])).edges.length, 0);
    assert.equal(projectProvenRelationshipGraph({ ...index, snapshotId: 'snap-other' }, manifest, new Set(['src/main.ts', 'src/helper.ts'])).edges.length, 0);
  });

  it('filters proven relationships by folder scope, type, and case-insensitive search', () => {
    const graph = projectProvenRelationshipGraph(index, manifest, new Set(['src/main.ts', 'src/helper.ts']));
    const options = { folderPath: null, relationshipKind: 'all', query: '' };
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, index.symbols, options), graph.edges);
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, index.symbols, { ...options, folderPath: 'assets' }), []);
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, index.symbols, { ...options, relationshipKind: 'imports' }), []);
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, index.symbols, { ...options, query: 'HELPER' }), graph.edges);
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, index.symbols, { ...options, query: 'not-present' }), []);
    assert.equal(graph.snapshotId, 'snap-1');
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, [], options), []);
  });

  it('combines relationship language filters with the selected path scope', () => {
    const graph = projectProvenRelationshipGraph(index, manifest, new Set(['src/main.ts', 'src/helper.ts']));
    const mixedLanguageSymbols = index.symbols.map((item) => item.symbolId === 'helper'
      ? { ...item, language: 'Rust' }
      : item);
    const base = { folderPath: 'src', relationshipKind: 'all', query: '' };
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, mixedLanguageSymbols, { ...base, language: 'Rust' }), graph.edges);
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, mixedLanguageSymbols, { ...base, language: 'Python' }), []);
    assert.deepEqual(filterProvenRelationshipEdges(graph.edges, mixedLanguageSymbols, { ...base, folderPath: 'tests', language: 'Rust' }), []);
  });

  it('reports current per-file indexing state from the selected snapshot index', () => {
    const rows = buildInventory(manifest);
    const main = rows.find((row) => row.path === 'src/main.ts');
    assert.equal(main.indexEligibility, 'pending');
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, null, 'loading')), 'indexing');
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, null, 'pending')), 'index pending');
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, null, 'error')), 'index unavailable');
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, index, 'ready')), 'indexed');

    const partial = {
      ...index,
      files: index.files.map((file) => file.path === main.path ? { ...file, status: 'partialParse' } : file),
    };
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, partial, 'ready')), 'partial parse');
    const stale = {
      ...index,
      files: index.files.map((file) => file.path === main.path ? { ...file, sourceSha256: sha('e') } : file),
    };
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, stale, 'ready')), 'stale index');
    const notIndexed = { ...index, files: index.files.filter((file) => file.path !== main.path) };
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(main, notIndexed, 'ready')), 'not indexed');
    const unsupported = rows.find((row) => row.path === 'src/odd.zzz');
    const binary = rows.find((row) => row.path === 'assets/image.bin');
    assert.equal(atlasRowIndexStatusLabel(atlasRowIndexStatus(unsupported, index, 'ready')), 'unsupported');
    assert.equal(atlasRowIndexStatus(binary, index, 'ready'), null);
  });

  it('summarizes exact-file and descendant index counts for Atlas details', () => {
    const files = [
      { path: 'src/main.ts', symbolCount: 3, relationshipCount: 4 },
      { path: 'src/nested/helper.ts', symbolCount: 2, relationshipCount: 1 },
      { path: 'tests/main.test.ts', symbolCount: 1, relationshipCount: 2 },
    ];
    assert.deepEqual(summarizeAtlasPathIndexCounts(files, 'src/main.ts', false), {
      indexedFiles: 1, symbols: 3, relationships: 4,
    });
    assert.deepEqual(summarizeAtlasPathIndexCounts(files, 'src', true), {
      indexedFiles: 2, symbols: 5, relationships: 5,
    });
    assert.deepEqual(summarizeAtlasPathIndexCounts(files, '.', true), {
      indexedFiles: 3, symbols: 6, relationships: 7,
    });
    assert.deepEqual(summarizeAtlasPathIndexCounts(files, 'srcx', true), {
      indexedFiles: 0, symbols: 0, relationships: 0,
    });
  });

  it('normalizes, fits, zooms around a fixed point, and pans the map viewport', () => {
    assert.deepEqual(normalizeAtlasViewport({ x: 3, y: -4, scale: 99 }), { x: 3, y: -4, scale: ATLAS_MAX_ZOOM });
    assert.deepEqual(normalizeAtlasViewport({ x: Number.NaN, y: 8, scale: Number.NaN }), { x: 0, y: 8, scale: 1 });
    assert.deepEqual(fitAtlasViewport(800, 600, 400, 300), { x: 0, y: 0, scale: 0.5 });
    assert.deepEqual(fitAtlasViewport(0, 600, 400, 300), { x: 0, y: 0, scale: 1 });

    const start = { x: 30, y: -12, scale: 1 };
    const center = { x: 140, y: 90 };
    const sourcePoint = { x: (center.x - start.x) / start.scale, y: (center.y - start.y) / start.scale };
    const zoomed = zoomAtlasViewport(start, ATLAS_ZOOM_STEP, center.x, center.y);
    assert.equal(zoomed.x + zoomed.scale * sourcePoint.x, center.x);
    assert.equal(zoomed.y + zoomed.scale * sourcePoint.y, center.y);
    assert.equal(zoomAtlasViewport(zoomed, 20).scale, ATLAS_MAX_ZOOM);
    assert.equal(zoomAtlasViewport(start, -20).scale, ATLAS_MIN_ZOOM);
    assert.deepEqual(panAtlasViewport(start, 25, -7), { x: 55, y: -19, scale: 1 });
  });
});
