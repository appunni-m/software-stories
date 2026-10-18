import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { readDirectApiSelection, readGenerationBackend, rememberDirectApiSelection, rememberGenerationBackend } from '../src/lib/generationPreferences.ts';

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

  it('remembers non-secret direct API provider and model defaults', () => {
    const saved = new Map();
    withStorage({ getItem: (key) => saved.get(key), setItem: (key, value) => saved.set(key, value) }, () => {
      assert.deepEqual(readDirectApiSelection(), { providerId: '', modelId: '' });
      rememberDirectApiSelection({ providerId: 'gemini', modelId: 'gemini-2.5-pro' });
      assert.deepEqual(readDirectApiSelection(), { providerId: 'gemini', modelId: 'gemini-2.5-pro' });
      assert.deepEqual([...saved.keys()], ['code-atlas:direct-api-selection']);
      assert.doesNotMatch([...saved.values()].join(' '), /api.?key|credential|secret/i);
    });
  });

  it('reads the same saved preference at file, planning, and PR entry points', async () => {
    for (const file of ['App.tsx', 'RepositoryPlanningDialog.tsx', 'PrGuideStudio.tsx']) {
      const source = await readFile(new URL(`../src/${file}`, import.meta.url), 'utf8');
      assert.match(source, /readGenerationBackend\(/);
    }
  });

  it('opens settings for the selected header backend and keeps PR Guide on that choice', async () => {
    const [app, header, connections, prGuide] = await Promise.all([
      readFile(new URL('../src/App.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/ConnectionHeader.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/ConnectionsDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/PrGuideStudio.tsx', import.meta.url), 'utf8'),
    ]);
    assert.match(app, /setProviderDialog\(\{ kind: 'settings' \}\)/);
    assert.match(app, /backend === 'codex'\) return <ConnectionsDialog/);
    assert.match(connections, /Change AI connection/);
    assert.match(header, /code-atlas:connection-changed/);
    assert.match(header, /backend === 'api' \? providerName/);
    assert.match(prGuide, /window\.addEventListener\('code-atlas:connection-changed'/);
    assert.match(header, /code-atlas:direct-api-selection-changed/);
    assert.match(app, /rememberDirectApiSelection\(\{ providerId, modelId: nextModelId \}\)/);
    assert.match(prGuide, /rememberDirectApiSelection\(\{ providerId, modelId: nextModelId \}\)/);
    assert.match(prGuide, /backend === 'handoff' &&/);
    assert.doesNotMatch(prGuide, /Generate with\s*<select/);
  });

  it('prepares a local review automatically while retaining explicit consent before send', async () => {
    const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
    const automatic = source.slice(source.indexOf('// Prepare an AI-selected request'), source.indexOf('  return (\n    <div className="provider-overlay">'));
    assert.match(automatic, /automaticReviewKey.current === key/);
    assert.match(automatic, /void buildReview\(\)/);
    assert.match(automatic, /mode.kind === 'overview' && discoveryResult/);
    assert.doesNotMatch(automatic, /sendCodex|setConsent\(true\)/);
    assert.match(source, /setConsent\(false\)/);
    assert.match(source, /!consent/);
  });
});
