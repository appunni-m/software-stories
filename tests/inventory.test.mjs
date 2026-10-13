import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildInventory, emptyInventoryFilters, filterInventory, paginateInventory } from '../src/lib/inventory.ts';

const manifest = {
  directories: [
    { path: '.', kind: 'directory', sizeBytes: null },
    { path: 'src', kind: 'directory', sizeBytes: null },
  ],
  files: [
    { path: 'src/main.rs', kind: 'file', sizeBytes: 40, sha256: 'a'.repeat(64), contentKind: 'text', secretLike: false },
    { path: 'src/utility.py', kind: 'file', sizeBytes: 12, sha256: 'e'.repeat(64), contentKind: 'text', secretLike: false },
    { path: 'assets/logo.bin', kind: 'file', sizeBytes: 2, sha256: 'b'.repeat(64), contentKind: 'binary', secretLike: false },
    { path: 'src/.env', kind: 'file', sizeBytes: 14, sha256: 'c'.repeat(64), contentKind: 'text', secretLike: true },
    { path: 'src/unknown.zzz', kind: 'file', sizeBytes: 4, sha256: 'd'.repeat(64), contentKind: 'text', secretLike: false },
  ],
  skipped: [
    { path: 'vendor', kind: 'directory', sizeBytes: null, reason: 'excludedByPolicy', details: 'generated dependency folder' },
    { path: 'large.dat', kind: 'file', sizeBytes: 300, reason: 'fileByteLimit', details: 'file exceeds configured bound' },
    { path: 'linked.rs', kind: 'symlink', sizeBytes: null, reason: 'symlink', details: 'symlinks are excluded' },
  ],
};

describe('snapshot inventory', () => {
  it('accounts for captured files, traversed folders, and every recorded skip exactly once', () => {
    const rows = buildInventory(manifest);
    assert.equal(rows.length, manifest.directories.length + manifest.files.length + manifest.skipped.length);
    assert.equal(rows.filter((row) => row.disposition === 'captured').length, manifest.files.length);
    assert.equal(rows.filter((row) => row.disposition !== 'captured').length, manifest.directories.length + manifest.skipped.length);
    assert.deepEqual(rows.map((row) => row.path), [...rows.map((row) => row.path)].sort());
  });

  it('keeps binary, secret-like, parser-pending, unsupported, and skipped reasons distinct', () => {
    const rows = buildInventory(manifest);
    const byPath = new Map(rows.map((row) => [row.path, row]));
    assert.equal(byPath.get('assets/logo.bin').contentKind, 'binary');
    assert.equal(byPath.get('src/.env').categories.includes('secret'), true);
    assert.equal(byPath.get('src/main.rs').parserReadiness, 'notReady');
    assert.equal(byPath.get('src/unknown.zzz').parserReadiness, 'unsupported');
    assert.equal(byPath.get('src/utility.py').language, 'Python');
    assert.equal(byPath.get('src/utility.py').parserReadiness, 'unsupported');
    assert.equal(byPath.get('large.dat').disposition, 'skipped');
    assert.equal(byPath.get('vendor').disposition, 'excluded');
    assert.equal(byPath.get('linked.rs').kind, 'symlink');
  });

  it('filters without changing the underlying inventory count or capture state', () => {
    const rows = buildInventory(manifest);
    const filters = { ...emptyInventoryFilters, query: 'fileByteLimit', disposition: 'skipped' };
    const filtered = filterInventory(rows, filters);
    assert.deepEqual(filtered.map((row) => row.path), ['large.dat']);
    assert.equal(rows.length, manifest.directories.length + manifest.files.length + manifest.skipped.length);
  });

  it('filters by parser readiness, content, category, and capture outcome independently', () => {
    const rows = buildInventory(manifest);
    assert.deepEqual(filterInventory(rows, { ...emptyInventoryFilters, parser: 'unsupported' }).map((row) => row.path), ['src/.env', 'src/unknown.zzz', 'src/utility.py']);
    assert.deepEqual(filterInventory(rows, { ...emptyInventoryFilters, content: 'binary' }).map((row) => row.path), ['assets/logo.bin']);
    assert.deepEqual(filterInventory(rows, { ...emptyInventoryFilters, category: 'secret' }).map((row) => row.path), ['src/.env']);
    assert.deepEqual(filterInventory(rows, { ...emptyInventoryFilters, disposition: 'excluded' }).map((row) => row.path), ['.', 'linked.rs', 'src', 'vendor']);
    assert.deepEqual(filterInventory(rows, { ...emptyInventoryFilters, language: 'Rust' }).map((row) => row.path), ['linked.rs', 'src/main.rs']);
    assert.equal(rows.find((row) => row.path === 'src/unknown.zzz').disposition, 'captured');
  });

  it('keeps page totals and visible ranges stable, including an empty result set', () => {
    const rows = Array.from({ length: 251 }, (_, index) => ({ ...buildInventory(manifest)[0], path: `path-${String(index).padStart(3, '0')}` }));
    const page2 = paginateInventory(rows, 1, 100);
    assert.equal(page2.total, 251);
    assert.equal(page2.firstVisible, 101);
    assert.equal(page2.lastVisible, 200);
    assert.equal(page2.rows.length, 100);
    const clamped = paginateInventory(rows, 99, 100);
    assert.equal(clamped.page, 2);
    assert.equal(clamped.firstVisible, 201);
    assert.equal(clamped.lastVisible, 251);
    assert.deepEqual(paginateInventory([], 0, 100), {
      rows: [], total: 0, page: 0, pageSize: 100, pageCount: 1, firstVisible: 0, lastVisible: 0,
    });
  });
});
