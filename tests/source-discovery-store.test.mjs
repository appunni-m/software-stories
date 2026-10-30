import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import {
  readSourceDiscoveryResult, saveSourceDiscoveryResult,
} from '../src/lib/sourceDiscoveryStore.ts';

const scope = { projectId: '/repo/example', snapshotId: 'snapshot-1', question: 'How does parsing work?' };
const result = {
  selectedFiles: [
    { path: 'src/parser.ts', reason: 'Contains the parser entry point.' },
    { path: 'src/token.ts', reason: 'Defines the input tokens.' },
  ],
  limitation: 'The test folder was outside this discovery result.',
};

function withStorage(storage, run) {
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
  try { run(); } finally {
    if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
    else delete globalThis.localStorage;
  }
}

describe('saved source discovery scope', () => {
  it('restores bounded path reasons only for the exact question and snapshot', () => {
    const values = new Map();
    withStorage({ getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }, () => {
      assert.equal(saveSourceDiscoveryResult(scope, result), true);
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'available', result });
      const collection = JSON.parse([...values.values()][0]);
      assert.equal(collection.schemaVersion, 2);
      assert.equal(collection.projectId, scope.projectId);
      assert.equal(collection.snapshotId, scope.snapshotId);
      assert.equal(collection.records.length, 1);
      const record = collection.records[0];
      assert.equal(record.schemaVersion, 1);
      assert.deepEqual(Object.keys(record).sort(), [
        'limitation', 'projectId', 'question', 'savedAtUnixMs', 'schemaVersion', 'selectedFiles', 'snapshotId', 'targetPath',
      ]);
      assert.doesNotMatch(JSON.stringify(collection), /reviewId|fingerprint|consent|sourceContents/i);
    });
  });

  it('rejects a changed question, snapshot, target, or path absent from the current manifest', () => {
    const values = new Map();
    withStorage({ getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }, () => {
      saveSourceDiscoveryResult(scope, result);
      assert.deepEqual(readSourceDiscoveryResult({ ...scope, question: 'A different question?' }, ['src/parser.ts', 'src/token.ts']), { status: 'missing' });
      assert.deepEqual(readSourceDiscoveryResult({ ...scope, snapshotId: 'snapshot-2' }, ['src/parser.ts', 'src/token.ts']), { status: 'missing' });
      assert.deepEqual(readSourceDiscoveryResult({ ...scope, targetPath: 'src/other.ts' }, ['src/parser.ts', 'src/token.ts']), { status: 'missing' });
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts']), { status: 'unavailable', issue: 'stale-path' });
    });
  });

  it('rejects malformed, duplicate-path, traversal, extra-field, and oversized records', () => {
    const values = new Map();
    withStorage({ getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }, () => {
      assert.equal(saveSourceDiscoveryResult(scope, { ...result, selectedFiles: [result.selectedFiles[0], result.selectedFiles[0]] }), false);
      assert.equal(saveSourceDiscoveryResult(scope, { ...result, selectedFiles: [{ path: '../secret', reason: 'Traversal.' }] }), false);
      assert.equal(saveSourceDiscoveryResult(scope, { ...result, extra: 'must not persist' }), false);
      assert.equal(saveSourceDiscoveryResult(scope, { selectedFiles: [{ path: 'src/parser.ts', reason: 'x'.repeat(33 * 1024) }], limitation: '' }), false);
      saveSourceDiscoveryResult(scope, result);
      const key = [...values.keys()][0];
      const collection = JSON.parse(values.get(key));
      collection.extra = 'unknown schema';
      values.set(key, JSON.stringify(collection));
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'unavailable', issue: 'invalid' });
      const unsupported = JSON.stringify({ schemaVersion: 99, projectId: scope.projectId, snapshotId: scope.snapshotId, records: [] });
      values.set(key, unsupported);
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'unavailable', issue: 'unsupported' });
      assert.equal(saveSourceDiscoveryResult(scope, result), false);
      assert.equal(values.get(key), unsupported);
      values.set(key, '{');
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'unavailable', issue: 'unreadable' });
    });
  });

  it('treats unavailable local storage as an optional cache', () => {
    withStorage({ getItem() { throw Error('blocked'); }, setItem() { throw Error('full'); } }, () => {
      assert.equal(saveSourceDiscoveryResult(scope, result), false);
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'unavailable', issue: 'storage' });
    });
  });

  it('rejects malformed UTF-16 scope identifiers without throwing', () => {
    const malformed = { ...scope, projectId: '/repo/\ud800' };
    withStorage({ getItem: () => null, setItem() {} }, () => {
      assert.equal(saveSourceDiscoveryResult(malformed, result), false);
      assert.deepEqual(readSourceDiscoveryResult(malformed, ['src/parser.ts', 'src/token.ts']), { status: 'missing' });
    });
  });

  it('counts the 300-character question limit in Unicode code points', () => {
    const values = new Map();
    const storage = { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
    const accepted = { ...scope, question: '🧭'.repeat(300) };
    withStorage(storage, () => {
      assert.equal(saveSourceDiscoveryResult(accepted, result), true);
      assert.deepEqual(readSourceDiscoveryResult(accepted, ['src/parser.ts', 'src/token.ts']), { status: 'available', result });
      assert.equal(saveSourceDiscoveryResult({ ...scope, question: '🧭'.repeat(301) }, result), false);
    });
  });

  it('persists only successful planner discovery and keeps ordinary file discovery session-only', async () => {
    const [discovery, planning, context] = await Promise.all([
      readFile(new URL('../src/SourceDiscoveryDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/RepositoryPlanningDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/CodexContextDialog.tsx', import.meta.url), 'utf8'),
    ]);
    const successPath = discovery.slice(discovery.indexOf('const result = await sendSourceDiscovery'), discovery.indexOf('active.current = null;', discovery.indexOf('const result = await sendSourceDiscovery')));
    assert.match(discovery, /const cacheableResult = result\.rangeTargets\?\.length \? null : \{/);
    assert.match(successPath, /if \(persistSuccessfulResult && cacheableResult && saveSourceDiscoveryResult\(scope, cacheableResult\)/);
    assert.match(planning, /persistSuccessfulResult autoDiscover ignoreSavedResult=\{ignoreSavedDiscovery\}/);
    assert.match(context, /persistSuccessfulResult=\{false\} autoDiscover/);
  });

  it('reuses an exact saved scope and offers an explicit rediscovery action', async () => {
    const [discovery, planning, dialog, app] = await Promise.all([
      readFile(new URL('../src/SourceDiscoveryDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/RepositoryPlanningDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/App.tsx', import.meta.url), 'utf8'),
    ]);
    assert.match(discovery, /readSourceDiscoveryResult\(scope,availablePaths\)/);
    assert.match(discovery, /persistSuccessfulResult && !ignoreSavedResult/);
    assert.match(planning, /useState<SourceDiscoveryResult \| null>\(\(\) => scope\.discoveryResult \?\? null\)/);
    assert.match(planning, /onFindCodeAgain=\{\(\) => \{ setDiscovery\(null\); setIgnoreSavedDiscovery\(true\); \}\}/);
    assert.match(dialog, /discoveryResult && onFindCodeAgain && <AppButton[^>]*>Find code again<\/AppButton>/);
    assert.match(planning, /discoveryResult=\{discovery \?\? undefined\}/);
    assert.match(app, /planningDialog\.discoveryResult \?\? null/);
  });
});
