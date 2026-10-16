import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { readGenerationBackend, rememberGenerationBackend } from '../src/lib/generationPreferences.ts';

describe('generation backend preference', () => {
  const withStorage = (storage, run) => {
    const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
    try { run(); } finally {
      if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
      else delete globalThis.localStorage;
    }
  };

  it('defaults to Codex without a backend chooser', () => {
    withStorage({ getItem: () => null }, () => assert.equal(readGenerationBackend(), 'codex'));
  });

  it('remembers each supported backend without storing credentials or consent', () => {
    const saved = new Map();
    withStorage({ getItem: (key) => saved.get(key), setItem: (key, value) => saved.set(key, value) }, () => {
      for (const backend of ['codex', 'api', 'claude-setup', 'handoff']) {
        rememberGenerationBackend(backend);
        assert.equal(readGenerationBackend(), backend);
      }
      assert.deepEqual([...saved.keys()], ['code-atlas:generation-backend']);
    });
  });

  it('handles unavailable storage and unknown preferences', () => {
    withStorage({ getItem: () => 'unknown' }, () => assert.equal(readGenerationBackend(), 'codex'));
    withStorage({ getItem() { throw Error('blocked'); }, setItem() { throw Error('blocked'); } }, () => {
      assert.equal(readGenerationBackend(), 'codex');
      assert.doesNotThrow(() => rememberGenerationBackend('api'));
    });
  });

  it('uses the same preference at file, planning, and PR entry points', async () => {
    for (const file of ['App.tsx', 'RepositoryPlanningDialog.tsx', 'PrGuideStudio.tsx']) {
      const source = await readFile(new URL(`../src/${file}`, import.meta.url), 'utf8');
      assert.match(source, /readGenerationBackend\(/);
      assert.match(source, /rememberGenerationBackend\(/);
    }
  });

  it('prepares a local review automatically while retaining explicit consent before send', async () => {
    const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
    const automatic = source.slice(source.indexOf('// Prepare this one-file'), source.indexOf('  return (\n    <div className="provider-overlay">'));
    assert.match(automatic, /automaticReviewKey.current === key/);
    assert.match(automatic, /void buildReview\(\)/);
    assert.doesNotMatch(automatic, /sendCodex|setConsent\(true\)/);
    assert.match(source, /setConsent\(false\)/);
    assert.match(source, /!consent/);
  });
});
