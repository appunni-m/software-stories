import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import {
  readSourceDiscoveryResult, saveSourceDiscoveryResult,
} from '../src/lib/sourceDiscoveryStore.ts';
import { sourceDiscoveryQuestion } from '../src/lib/sourceDiscovery.ts';

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
  it('uses documentation paths as bounded hints and keeps implementation code as proof', () => {
    const question = sourceDiscoveryQuestion('docs/architecture.md');
    assert.match(question, /^Use documentation path docs\/architecture\.md only as an unverified hint\./);
    assert.match(question, /Find current implementation code on its topic and use code as evidence\./);
    assert.match(sourceDiscoveryQuestion('src/font.rs'), /How does src\/font\.rs work/);
    assert.ok(sourceDiscoveryQuestion(`docs/${'very-long/'.repeat(80)}architecture.md`).length <= 300);
  });

  it('restores bounded path reasons only for the exact question and snapshot', () => {
    const values = new Map();
    withStorage({ getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }, () => {
      assert.equal(saveSourceDiscoveryResult(scope, result), true);
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'available', result });
      const collection = JSON.parse([...values.values()][0]);
      assert.equal(collection.schemaVersion, 3);
      assert.equal(collection.projectId, scope.projectId);
      assert.equal(collection.snapshotId, scope.snapshotId);
      assert.equal(collection.records.length, 1);
      const record = collection.records[0];
      assert.equal(record.schemaVersion, 3);
      assert.deepEqual(Object.keys(record).sort(), [
        'discoveryNotes', 'limitation', 'projectId', 'question', 'rangeTargets', 'savedAtUnixMs', 'schemaVersion', 'selectedFiles', 'snapshotId', 'targetPath',
      ]);
      assert.doesNotMatch(JSON.stringify(collection), /reviewId|fingerprint|consent|sourceContents/i);
    });
  });

  it('restores a bounded large-file target and its local coverage notes after restart', () => {
    const values = new Map();
    const storage = { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
    const rangeResult = {
      selectedFiles: [{ path: 'src/api.rs', reason: 'Contains the caller.' }],
      rangeTargets: [{ path: 'src/font.rs', reason: 'Contains the large font implementation.' }],
      limitation: 'Some source did not fit the first review.',
    };
    const discoveryNotes = ['3 candidate paths did not fit the bounded review.', 'Discovery note: Some source did not fit the first review.'];
    withStorage(storage, () => {
      assert.equal(saveSourceDiscoveryResult(scope, rangeResult, discoveryNotes), true);
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/api.rs', 'src/font.rs']), {
        status: 'available', result: rangeResult, discoveryNotes,
      });
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/api.rs']), { status: 'unavailable', issue: 'stale-path' });
      assert.equal(saveSourceDiscoveryResult(scope, {
        ...rangeResult,
        rangeTargets: [rangeResult.rangeTargets[0], { path: 'src/other.rs', reason: 'A second large target.' }],
      }), false);
      assert.equal(saveSourceDiscoveryResult(scope, {
        ...rangeResult,
        selectedFiles: [...rangeResult.selectedFiles, { path: 'src/font.rs', reason: 'Duplicate target path.' }],
      }), false);
      assert.equal(saveSourceDiscoveryResult(scope, {
        ...rangeResult,
        selectedFiles: Array.from({ length: 12 }, (_, index) => ({ path: `src/file-${index}.rs`, reason: 'Supporting file.' })),
      }), false);
      const collection = JSON.parse([...values.values()][0]);
      assert.equal(collection.records[0].schemaVersion, 3);
      assert.deepEqual(collection.records[0].rangeTargets, rangeResult.rangeTargets);
      assert.deepEqual(collection.records[0].discoveryNotes, discoveryNotes);
      assert.doesNotMatch(JSON.stringify(collection), /reviewId|fingerprint|consent|sourceContents/i);
    });
  });

  it('starts a new discovery cache without reading or migrating old scope records', () => {
    const values = new Map();
    const storage = { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
    const rangeScope = { ...scope, question: 'How does the large font parser work?' };
    const rangeResult = {
      selectedFiles: [],
      rangeTargets: [{ path: 'src/font.rs', reason: 'Contains the parser.' }],
      limitation: '',
    };
    const oldKey = `code-atlas:source-discovery:v2:${encodeURIComponent(scope.projectId)}:${encodeURIComponent(scope.snapshotId)}`;
    values.set(oldKey, JSON.stringify({
      schemaVersion: 2,
      projectId: scope.projectId,
      snapshotId: scope.snapshotId,
      records: [{
        schemaVersion: 1,
        projectId: scope.projectId,
        snapshotId: scope.snapshotId,
        question: scope.question,
        targetPath: null,
        selectedFiles: result.selectedFiles,
        limitation: result.limitation,
        savedAtUnixMs: Date.now(),
      }],
    }));
    withStorage(storage, () => {
      assert.deepEqual(readSourceDiscoveryResult(scope, ['src/parser.ts', 'src/token.ts']), { status: 'missing' });
      assert.equal(saveSourceDiscoveryResult(rangeScope, rangeResult), true);
      assert.deepEqual(readSourceDiscoveryResult(rangeScope, ['src/font.rs']), { status: 'available', result: rangeResult });
      assert.equal(values.has(oldKey), true, 'old derived data remains untouched but is never read or merged');
      const currentKey = [...values.keys()].find((key) => key.includes(':source-discovery:v3:'));
      assert.ok(currentKey);
      const current = JSON.parse(values.get(currentKey));
      assert.equal(current.schemaVersion, 3);
      assert.equal(current.records[0].schemaVersion, 3);
      assert.equal(current.records.length, 1);
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

  it('persists planner range targets while keeping Code Lens discovery session-only', async () => {
    const [discovery, planning, context] = await Promise.all([
      readFile(new URL('../src/SourceDiscoveryDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/RepositoryPlanningDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/CodexContextDialog.tsx', import.meta.url), 'utf8'),
    ]);
    const successPath = discovery.slice(discovery.indexOf('const result = await sendSourceDiscovery'), discovery.indexOf('active.current = null;', discovery.indexOf('const result = await sendSourceDiscovery')));
    assert.match(successPath, /sourceDiscoveryLimitNotes\(result,\s*selectedReview\)/);
    assert.match(successPath, /saveSourceDiscoveryResult\(scope,\s*result,\s*discoveryNotes\)/);
    assert.match(discovery, /onReadyRef\.current\(saved\.result,\s*undefined,\s*saved\.discoveryNotes\)/);
    assert.match(planning, /persistSuccessfulResult autoDiscover ignoreSavedResult=\{ignoreSavedDiscovery\}/);
    assert.match(planning, /savedDiscoveryNotes\s*\?\?\s*sourceDiscoveryLimitNotes\(result,\s*review\)/);
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

  it('routes documentation targets through code discovery instead of a documentation chapter', async () => {
    const [context, dialog] = await Promise.all([
      readFile(new URL('../src/CodexContextDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8'),
    ]);
    assert.match(context, /documentationTargetPath[\s\S]*?targetPath: documentationTargetPath \? null : mode\.targetPath/);
    assert.match(context, /documentationTargetPath[\s\S]*?kind: 'overview'[\s\S]*?question: sourceDiscoveryQuestion\(documentationTargetPath\)/);
    assert.match(dialog, /const chapterQuestion = mode !== 'settings' && \(mode\.kind === 'review' \|\| mode\.kind === 'overview'\) \? mode\.question : undefined/);
    assert.match(dialog, /question: chapterQuestion,/);
  });

  it('offers a manual source picker after automatic discovery fails and preserves its question', async () => {
    const [context, discovery] = await Promise.all([
      readFile(new URL('../src/CodexContextDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/SourceDiscoveryDialog.tsx', import.meta.url), 'utf8'),
    ]);
    assert.match(discovery, /onChooseFilesManually\?: \(\) => void/);
    assert.match(discovery, /error && autoDiscover && onChooseFilesManually && <AppButton intent="quiet" onClick=\{onChooseFilesManually\}>Choose files manually<\/AppButton>/);
    assert.match(context, /setManualFallbackScope\(discoveryScopeKey\)/);
    assert.match(context, /const chapterMode = useManualFallback && discoveryScope/);
    assert.match(context, /\{ \.\.\.mode, question: discoveryScope\.question \}/);
    assert.match(context, /kind: 'overview' as const, projectId: mode\.projectId, snapshotId: mode\.snapshotId, question: discoveryScope\.question/);
    assert.match(context, /discovery\?\.scopeKey === discoveryScopeKey/);
  });
});
