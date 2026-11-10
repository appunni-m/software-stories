import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { readGenerationBackend, rememberGenerationBackend } from '../src/lib/generationPreferences.ts';

describe('first-release generation connection', () => {
  const withStorage = (storage, run) => {
    const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
    try { run(); } finally {
      if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
      else delete globalThis.localStorage;
    }
  };

  it('defaults to Codex when no backend is saved', () => {
    withStorage({ getItem: () => null }, () => assert.equal(readGenerationBackend(), 'codex'));
  });

  it('rejects deferred backend preferences without rewriting saved state', () => {
    for (const unsupported of ['api', 'claude-setup', 'handoff', 'unknown-backend']) {
      const saved = new Map([['code-atlas:generation-backend', unsupported]]);
      withStorage({
        getItem: (key) => saved.get(key),
        setItem: (key, value) => saved.set(key, value),
      }, () => {
        assert.equal(readGenerationBackend(), null);
        rememberGenerationBackend(unsupported);
        assert.equal(readGenerationBackend(), null);
        assert.equal(saved.get('code-atlas:generation-backend'), unsupported);
      });
    }
  });

  it('persists the connected Codex choice', () => {
    const saved = new Map();
    withStorage({
      getItem: (key) => saved.get(key) ?? null,
      setItem: (key, value) => saved.set(key, value),
    }, () => {
      rememberGenerationBackend('codex');
      assert.equal(readGenerationBackend(), 'codex');
      assert.deepEqual([...saved.keys()], ['code-atlas:generation-backend']);
    });
  });

  it('keeps the current Codex session usable when preference storage fails', () => {
    withStorage({
      getItem: () => null,
      setItem() { throw Error('storage unavailable'); },
    }, () => {
      rememberGenerationBackend('codex');
      assert.equal(readGenerationBackend(), 'codex');
    });
  });

  it('uses the saved Codex choice at each current generation entry point', async () => {
    for (const file of ['ConnectionHeader.tsx', 'RepositoryPlanningDialog.tsx', 'workspace/Workspace.tsx']) {
      const source = await readFile(new URL(`../src/${file}`, import.meta.url), 'utf8');
      assert.match(source, /readGenerationBackend\(/);
    }
  });

  it('prepares local reviews and dispatches snapshot questions only under the saved grant', async () => {
    const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
    const start = source.indexOf('// Prepare an AI-selected request');
    const end = source.indexOf('return (\n', start);
    const automatic = source.slice(start, end);
    const routes = source.slice(source.indexOf('  const automaticReviewRequest ='), source.indexOf('  const [recoveryReady'));
    assert.ok(start >= 0 && end > start, 'the automatic review preparation flow is present');
    assert.match(automatic, /automaticReviewKey.current === key/);
    assert.match(automatic, /void buildReview\(\)/);
    assert.match(routes, /mode.kind === 'overview' && Boolean\(discoveryResult\)/);
    assert.match(routes, /\|\| automaticSectionRepair/u);
    assert.match(source, /const requiresFreshConsent = !automaticSectionRepair && !retrievalLimits/u);
    assert.match(source, /const automaticSnapshotRequest = native && automaticReviewRequest && snapshotRetrievalRoute\s*&& Boolean\(retrievalLimits\) && !chapterRecovery/u);
    assert.match(source, /if \(!automaticSnapshotRequest \|\| !review \|\| !retrievalLimits/u);
    assert.match(source, /\(requiresFreshConsent && !consent\)/u);
    assert.doesNotMatch(automatic, /sendCodex|setConsent\(true\)/);
    assert.match(source, /setConsent\(false\)/);
    assert.match(source, /!consent/);
  });
});
