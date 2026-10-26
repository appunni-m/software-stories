import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildInventory } from '../src/lib/inventory.ts';
import {
  addCoverageGroup,
  buildCoverageRows,
  coverageStorageKey,
  emptyCoveragePlan,
  excludeCoveragePath,
  filterCoverageRows,
  loadCoveragePlan,
  paginateCoverage,
  readCoveragePlan,
  removeCoverageGroup,
  restoreCoveragePath,
  saveCoveragePlan,
  summarizeCoverage,
} from '../src/lib/coverage.ts';

const manifest = {
  directories: [
    { path: '.', kind: 'directory', sizeBytes: null },
    { path: 'src', kind: 'directory', sizeBytes: null },
  ],
  files: [
    { path: 'src/main.ts', kind: 'file', sizeBytes: 40, sha256: 'a'.repeat(64), contentKind: 'text', secretLike: false },
    { path: 'src/view.tsx', kind: 'file', sizeBytes: 50, sha256: 'b'.repeat(64), contentKind: 'text', secretLike: false },
    { path: 'src/readme.weird', kind: 'file', sizeBytes: 10, sha256: 'c'.repeat(64), contentKind: 'text', secretLike: false },
    { path: 'assets/logo.bin', kind: 'file', sizeBytes: 2, sha256: 'd'.repeat(64), contentKind: 'binary', secretLike: false },
  ],
  skipped: [
    { path: 'vendor', kind: 'directory', sizeBytes: null, reason: 'excludedByPolicy', details: 'dependency folder' },
    { path: 'too-large.ts', kind: 'file', sizeBytes: 300, reason: 'fileByteLimit', details: 'file exceeds configured bound' },
  ],
};

const inventory = buildInventory(manifest);
const projectId = 'project-a';
const snapshotId = 'snapshot-1';

function revision({
  targetId,
  scope,
  revisionId,
  revisionNumber = 1,
  lifecycle = 'draft',
  project = projectId,
  snapshot = snapshotId,
  createdAtUnixMs = revisionNumber,
}) {
  return {
    projectId: project,
    snapshotId: snapshot,
    target: { targetId, scope },
    revisionId,
    revisionNumber,
    lifecycle,
    createdAtUnixMs,
  };
}

describe('Guide Plan coverage', () => {
  it('keeps one status row per inventory path and reconciles all status totals', () => {
    const rows = buildCoverageRows(inventory, emptyCoveragePlan(projectId, snapshotId));
    const totals = summarizeCoverage(rows);
    assert.equal(rows.length, inventory.length);
    assert.deepEqual(rows.map((row) => row.inventory.path), inventory.map((row) => row.path));
    assert.equal(totals.total, inventory.length);
    assert.equal(Object.entries(totals).filter(([key]) => key !== 'total').reduce((sum, [, count]) => sum + count, 0), inventory.length);
    assert.equal(rows.find((row) => row.inventory.path === 'src/main.ts').status, 'missing');
    assert.equal(rows.find((row) => row.inventory.path === 'src/main.ts').targetId, null);
    assert.equal(rows.find((row) => row.inventory.path === 'src/main.ts').canSelectForGroup, true);
    assert.equal(rows.find((row) => row.inventory.path === 'src/readme.weird').status, 'unsupported');
    assert.equal(rows.find((row) => row.inventory.path === 'src/readme.weird').canOpenSource, true);
    assert.equal(rows.find((row) => row.inventory.path === 'assets/logo.bin').status, 'unavailable');
    assert.equal(rows.find((row) => row.inventory.path === 'assets/logo.bin').canOpenSource, false);
    assert.equal(rows.find((row) => row.inventory.path === 'vendor').status, 'unavailable');
    assert.match(rows.find((row) => row.inventory.path === 'too-large.ts').reason, /no captured source bytes/i);
  });

  it('creates groups only from explicitly selected independent files, keeps members searchable, and restores standalone defaults when removed', () => {
    const plan = emptyCoveragePlan(projectId, snapshotId);
    assert.throws(() => addCoverageGroup(plan, {
      groupId: 'group-one', name: 'runtime', rationale: 'Shared runtime path', paths: ['src/main.ts'], createdAtUnixMs: 1,
    }, inventory), /at least two/);
    const grouped = addCoverageGroup(plan, {
      groupId: 'group-one', name: 'Runtime flow', rationale: 'These modules hand a request from entry to view.',
      paths: ['src/view.tsx', 'src/main.ts', 'src/main.ts'], createdAtUnixMs: 2,
    }, inventory);
    const rows = buildCoverageRows(inventory, grouped);
    assert.equal(rows.length, inventory.length);
    for (const path of ['src/main.ts', 'src/view.tsx']) {
      const row = rows.find((item) => item.inventory.path === path);
      assert.equal(row.status, 'missing');
      assert.equal(row.group.groupId, 'group-one');
      assert.equal(row.revisionId, null);
      assert.match(row.group.rationale, /hand a request/);
    }
    assert.deepEqual(filterCoverageRows(rows, 'hand a request').map((row) => row.inventory.path), ['src/main.ts', 'src/view.tsx']);
    const independent = buildCoverageRows(inventory, removeCoverageGroup(grouped, 'group-one'));
    assert.equal(independent.find((row) => row.inventory.path === 'src/main.ts').status, 'missing');
    assert.equal(independent.find((row) => row.inventory.path === 'src/main.ts').targetId, null);
    assert.equal(independent.length, inventory.length);
  });

  it('requires an exclusion reason, keeps capture exclusions unavailable, and reverses a user exclusion', () => {
    const plan = emptyCoveragePlan(projectId, snapshotId);
    const row = buildCoverageRows(inventory, plan).find((item) => item.inventory.path === 'src/main.ts');
    assert.throws(() => excludeCoveragePath(plan, row, '   ', 10), /reason is required/);
    const excluded = excludeCoveragePath(plan, row, 'Generated by a tool; not useful in the guide.', 11);
    const excludedRows = buildCoverageRows(inventory, excluded);
    assert.equal(excludedRows.find((item) => item.inventory.path === 'src/main.ts').status, 'excluded');
    assert.equal(excludedRows.find((item) => item.inventory.path === 'src/main.ts').reason, 'Generated by a tool; not useful in the guide.');
    assert.equal(excludedRows.find((item) => item.inventory.path === 'too-large.ts').status, 'unavailable');
    assert.throws(() => excludeCoveragePath(plan, excludedRows.find((item) => item.inventory.path === 'too-large.ts'), 'Skip it'), /cannot be excluded/);
    const restored = buildCoverageRows(inventory, restoreCoveragePath(excluded, 'src/main.ts'));
    assert.equal(restored.find((item) => item.inventory.path === 'src/main.ts').status, 'missing');
  });

  it('matches draft and accepted revisions only to their exact project, snapshot, and file target', () => {
    const plan = emptyCoveragePlan(projectId, snapshotId);
    const revisions = [
      revision({ targetId: 'file:src/main.ts', scope: { kind: 'file', path: 'src/main.ts' }, revisionId: 'rev-draft', lifecycle: 'draft' }),
      revision({ targetId: 'file:src/view.tsx', scope: { kind: 'file', path: 'src/view.tsx' }, revisionId: 'rev-accepted', lifecycle: 'accepted' }),
      revision({ targetId: 'file:src/main.ts', scope: { kind: 'file', path: 'src/main.ts' }, revisionId: 'other-snapshot', snapshot: 'snapshot-2', lifecycle: 'accepted' }),
      revision({ targetId: 'file:src/main.ts', scope: { kind: 'file', path: 'src/main.ts' }, revisionId: 'other-project', project: 'project-b', lifecycle: 'accepted' }),
      revision({ targetId: 'target:file:other', scope: { kind: 'file', path: 'src/other.ts' }, revisionId: 'wrong-scope', lifecycle: 'accepted' }),
    ];
    const rows = buildCoverageRows(inventory, plan, revisions);
    assert.deepEqual(
      ['src/main.ts', 'src/view.tsx'].map((path) => {
        const row = rows.find((item) => item.inventory.path === path);
        return [row.status, row.revisionId, row.revisionNumber];
      }),
      [['draft', 'rev-draft', 1], ['accepted', 'rev-accepted', 1]],
    );
  });

  it('keeps every file coverage row unchanged when the optional overview is added or omitted', () => {
    const plan = emptyCoveragePlan(projectId, snapshotId);
    const acceptedFile = revision({
      targetId: 'target:file:src/main.ts',
      scope: { kind: 'file', path: 'src/main.ts' },
      revisionId: 'accepted-main',
      lifecycle: 'accepted',
      revisionNumber: 3,
    });
    const withoutOverview = buildCoverageRows(inventory, plan, [acceptedFile]);
    const withOverview = buildCoverageRows(inventory, plan, [
      acceptedFile,
      revision({
        targetId: 'target:overview',
        scope: { kind: 'overview' },
        revisionId: 'overview-draft',
        lifecycle: 'draft',
        revisionNumber: 1,
      }),
      revision({
        targetId: 'target:overview',
        scope: { kind: 'overview' },
        revisionId: 'overview-accepted',
        lifecycle: 'accepted',
        revisionNumber: 2,
      }),
    ]);

    assert.deepEqual(withOverview, withoutOverview);
    assert.equal(withOverview.find((row) => row.inventory.path === 'src/main.ts').status, 'accepted');
    assert.equal(withOverview.find((row) => row.inventory.path === 'src/main.ts').revisionId, 'accepted-main');
  });

  it('shows the latest current revision and reports rejected or older-snapshot chapters distinctly', () => {
    const plan = emptyCoveragePlan(projectId, snapshotId);
    const revisions = [
      revision({ targetId: 'file:src/main.ts', scope: { kind: 'file', path: 'src/main.ts' }, revisionId: 'old-main', snapshot: 'snapshot-0', lifecycle: 'accepted', revisionNumber: 8 }),
      revision({ targetId: 'file:src/main.ts', scope: { kind: 'file', path: 'src/main.ts' }, revisionId: 'rejected-main', lifecycle: 'rejected', revisionNumber: 9 }),
      revision({ targetId: 'file:src/view.tsx', scope: { kind: 'file', path: 'src/view.tsx' }, revisionId: 'old-view', snapshot: 'snapshot-0', lifecycle: 'accepted', revisionNumber: 2 }),
    ];
    const rows = buildCoverageRows(inventory, plan, revisions);
    assert.deepEqual(
      ['src/main.ts', 'src/view.tsx'].map((path) => {
        const row = rows.find((item) => item.inventory.path === path);
        return [row.status, row.revisionId];
      }),
      [['rejected', 'rejected-main'], ['stale', 'old-view']],
    );
  });

  it('keeps group revisions separate from path-specific file coverage', () => {
    const plan = addCoverageGroup(emptyCoveragePlan(projectId, snapshotId), {
      groupId: 'group-one', name: 'Runtime', rationale: 'One path through the app.',
      paths: ['src/main.ts', 'src/view.tsx'], createdAtUnixMs: 1,
    }, inventory);
    const accepted = revision({
      targetId: 'target:group:group-one',
      scope: { kind: 'group', paths: ['src/view.tsx', 'src/main.ts'] },
      revisionId: 'group-revision', lifecycle: 'accepted', revisionNumber: 4,
    });
    const rows = buildCoverageRows(inventory, plan, [accepted]);
    for (const path of ['src/main.ts', 'src/view.tsx']) {
      const row = rows.find((item) => item.inventory.path === path);
      assert.equal(row.status, 'missing');
      assert.equal(row.targetId, null);
      assert.equal(row.revisionId, null);
      assert.equal(row.group.rationale, 'One path through the app.');
    }
    const staleMembers = revision({
      targetId: 'target:group:group-one',
      scope: { kind: 'group', paths: ['src/main.ts', 'assets/logo.bin'] },
      revisionId: 'stale-group-revision', lifecycle: 'accepted',
    });
    assert.equal(buildCoverageRows(inventory, plan, [staleMembers]).find((row) => row.inventory.path === 'src/main.ts').status, 'missing');
  });

  it('filters searchable status details, clamps pages, and scopes persisted plans to project plus snapshot', () => {
    const rows = buildCoverageRows(inventory, emptyCoveragePlan(projectId, snapshotId));
    assert.equal(filterCoverageRows(rows, 'src/main', 'missing').length, 1);
    assert.equal(filterCoverageRows(rows, 'dependency folder', 'unavailable').length, 1);
    const page = paginateCoverage(rows, 999, 2);
    assert.equal(page.total, rows.length);
    assert.equal(page.page, Math.ceil(rows.length / 2) - 1);
    assert.equal(page.lastVisible, rows.length);
    const store = new Map();
    const fakeStorage = {
      getItem: (key) => store.get(key) ?? null,
      setItem: (key, value) => store.set(key, value),
    };
    const saved = {
      ...emptyCoveragePlan(projectId, snapshotId),
      exclusions: [{ path: 'src/main.ts', reason: 'Temporarily out of scope', excludedAtUnixMs: 12 }],
    };
    assert.equal(saveCoveragePlan(saved, fakeStorage), true);
    assert.equal(coverageStorageKey(projectId, snapshotId).includes(encodeURIComponent(snapshotId)), true);
    assert.deepEqual(loadCoveragePlan(projectId, snapshotId, fakeStorage), saved);
    assert.deepEqual(loadCoveragePlan(projectId, 'snapshot-2', fakeStorage), emptyCoveragePlan(projectId, 'snapshot-2'));
  });

  it('preserves an oversized saved plan and refuses to overwrite it', () => {
    const projectId = 'project-storage';
    const snapshotId = 'snapshot-storage';
    const key = coverageStorageKey(projectId, snapshotId);
    const stored = ' '.repeat(4 * 1024 * 1024 + 1);
    const values = new Map([[key, stored]]);
    const storage = {
      getItem: (storageKey) => values.get(storageKey) ?? null,
      setItem: (storageKey, value) => values.set(storageKey, value),
    };

    assert.equal(readCoveragePlan(projectId, snapshotId, storage).status, 'unavailable');
    assert.equal(saveCoveragePlan(emptyCoveragePlan(projectId, snapshotId), storage), false);
    assert.equal(values.get(key), stored);
  });
});
