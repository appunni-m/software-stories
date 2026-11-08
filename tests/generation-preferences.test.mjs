import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { readDirectApiSelection, readDirectApiSelectionIssue, readGenerationBackend, rememberDirectApiSelection, rememberGenerationBackend, repositoryPlanningBackend } from '../src/lib/generationPreferences.ts';

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

  it('only remembers Codex and leaves legacy backend preferences for an explicit choice', () => {
    for (const legacy of ['api', 'claude-setup', 'handoff']) {
      const legacyStorage = new Map([['code-atlas:generation-backend', legacy]]);
      withStorage({ getItem: (key) => legacyStorage.get(key), setItem: (key, value) => legacyStorage.set(key, value) }, () => {
        assert.equal(readGenerationBackend(), null, `${legacy} must require an explicit Codex choice`);
        rememberGenerationBackend(legacy);
        assert.equal(readGenerationBackend(), null, `${legacy} cannot be selected by current UI`);
        assert.equal(legacyStorage.get('code-atlas:generation-backend'), legacy, 'legacy preference is not silently rewritten');
      });
    }

    const saved = new Map();
    withStorage({ getItem: (key) => saved.get(key), setItem: (key, value) => saved.set(key, value) }, () => {
      rememberGenerationBackend('codex');
      assert.equal(readGenerationBackend(), 'codex');
      assert.deepEqual([...saved.keys()], ['code-atlas:generation-backend']);
    });
  });

  it('keeps the newly selected backend for this session when storage rejects the write', () => {
    const saved = new Map([['code-atlas:generation-backend', 'api']]);
    withStorage({
      getItem: (key) => saved.get(key) ?? null,
      setItem() { throw Error('storage unavailable'); },
    }, () => {
      rememberGenerationBackend('codex');
      assert.equal(readGenerationBackend(), 'codex', 'the header and request entry points must use the explicit Codex selection');
    });

    // Leave the module's session fallback in its normal persisted state for
    // other tests that share the same process.
    const persisted = new Map();
    withStorage({ getItem: (key) => persisted.get(key) ?? null, setItem: (key, value) => persisted.set(key, value) }, () => {
      rememberGenerationBackend('codex');
    });

    const finalPersisted = new Map();
    withStorage({ getItem: (key) => finalPersisted.get(key) ?? null, setItem: (key, value) => finalPersisted.set(key, value) }, () => {
      rememberGenerationBackend('codex');
    });
  });

  it('defaults to Codex only when unset and preserves a known session choice if storage becomes unreadable', () => {
    withStorage({ getItem: () => null }, () => assert.equal(readGenerationBackend(), 'codex'));
    withStorage({ getItem: () => 'unknown' }, () => assert.equal(readGenerationBackend(), null));
    withStorage({ getItem() { throw Error('blocked'); }, setItem() { throw Error('blocked'); } }, () => {
      assert.equal(readGenerationBackend(), 'codex');
      assert.doesNotThrow(() => rememberGenerationBackend('handoff'));
      assert.equal(readGenerationBackend(), 'codex', 'unsupported choices cannot replace the session Codex selection');
    });

    const persisted = new Map();
    withStorage({ getItem: (key) => persisted.get(key) ?? null, setItem: (key, value) => persisted.set(key, value) }, () => {
      rememberGenerationBackend('codex');
    });
  });

  it('uses the saved supported agent for repository plans without substituting another backend', () => {
    assert.equal(repositoryPlanningBackend('codex'), 'codex');
    assert.equal(repositoryPlanningBackend('claude-setup'), 'claude-agent');
    assert.equal(repositoryPlanningBackend('claude-agent'), 'claude-agent');
    assert.equal(repositoryPlanningBackend('api'), null);
    assert.equal(repositoryPlanningBackend('handoff'), null);
    assert.equal(repositoryPlanningBackend(undefined), null);
  });

  it('remembers non-secret direct API provider and model defaults', () => {
    const saved = new Map();
    withStorage({ getItem: (key) => saved.get(key), setItem: (key, value) => saved.set(key, value) }, () => {
      assert.deepEqual(readDirectApiSelection(), { providerId: '', modelId: '' });
      rememberDirectApiSelection({ providerId: 'gemini', modelId: 'gemini-2.5-pro' });
      assert.deepEqual(readDirectApiSelection(), { providerId: 'gemini', modelId: 'gemini-2.5-pro' });
      assert.equal(readDirectApiSelectionIssue(), null);
      assert.deepEqual([...saved.keys()], ['code-atlas:direct-api-selection']);
      assert.doesNotMatch([...saved.values()].join(' '), /api.?key|credential|secret/i);
    });
  });

  it('keeps malformed direct API settings visible and leaves them until an explicit replacement', () => {
    const saved = new Map([['code-atlas:direct-api-selection', '{']]);
    withStorage({ getItem: (key) => saved.get(key) ?? null, setItem: (key, value) => saved.set(key, value) }, () => {
      assert.deepEqual(readDirectApiSelection(), { providerId: '', modelId: '' });
      assert.equal(readDirectApiSelectionIssue(), 'invalid');
      assert.equal(saved.get('code-atlas:direct-api-selection'), '{');
      rememberDirectApiSelection({ providerId: 'gemini', modelId: '' });
      assert.deepEqual(readDirectApiSelection(), { providerId: 'gemini', modelId: '' });
      assert.equal(readDirectApiSelectionIssue(), null);
    });
  });

  it('bounds and classifies direct API preference reads before parsing', () => {
    const oversized = ' '.repeat(8_193);
    withStorage({ getItem: () => oversized }, () => {
      assert.deepEqual(readDirectApiSelection(), { providerId: '', modelId: '' });
      assert.equal(readDirectApiSelectionIssue(), 'oversized');
    });
    withStorage({ getItem() { throw Error('blocked'); } }, () => {
      assert.deepEqual(readDirectApiSelection(), { providerId: '', modelId: '' });
      assert.equal(readDirectApiSelectionIssue(), 'unavailable');
    });
  });

  it('reads the same saved preference at file, planning, and PR entry points', async () => {
    for (const file of ['App.tsx', 'RepositoryPlanningDialog.tsx', 'PrGuideStudio.tsx']) {
      const source = await readFile(new URL(`../src/${file}`, import.meta.url), 'utf8');
      assert.match(source, /readGenerationBackend\(/);
    }
  });

  it('opens settings for the selected header backend and keeps PR Guide on that choice', async () => {
    const [app, header, connections, prGuide, codexDialog] = await Promise.all([
      readFile(new URL('../src/App.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/ConnectionHeader.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/ConnectionsDialog.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/PrGuideStudio.tsx', import.meta.url), 'utf8'),
      readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8'),
    ]);
    assert.match(app, /setProviderDialog\(\{ kind: 'settings' \}\)/);
    assert.match(app, /backend === 'codex'\) return <ConnectionsDialog/);
    assert.doesNotMatch(connections, /Change AI connection/);
    assert.doesNotMatch(app, /More AI options|provider-alternatives/);
    assert.match(app, /Connect with Codex/);
    assert.match(app, /Use Codex/);
    assert.doesNotMatch(codexDialog, /Change backend|onChooseBackend/);
    assert.match(header, /code-atlas:connection-changed/);
    assert.match(header, /const codexNeedsAttention = codexSelected && Boolean\(connection\.error\)/);
    assert.match(header, /codexNeedsAttention \? 'Retry Codex'/);
    assert.match(header, /Could not refresh Codex right now\. Open Connections to retry; your saved account and model choice are unchanged\./);
    assert.match(header, /backend === null \? 'Connect Codex'/);
    assert.match(header, /Connect your Codex account once in Connections\./);
    assert.doesNotMatch(header, /Direct API|Claude Agent|handoff|Anthropic|Gemini|OpenAI/);
    assert.match(app, /The saved direct API selection is \{directApiSelectionIssue === 'invalid' \? 'invalid' : directApiSelectionIssue === 'oversized' \? 'too large'/);
    assert.match(prGuide, /The saved direct API selection is \{directApiSelectionIssue === 'invalid' \? 'invalid' : directApiSelectionIssue === 'oversized' \? 'too large'/);
    assert.match(prGuide, /window\.addEventListener\('code-atlas:connection-changed'/);
    assert.doesNotMatch(header, /code-atlas:direct-api-selection-changed|code-atlas:handoff-agent-changed/);
    assert.match(app, /rememberDirectApiSelection\(\{ providerId, modelId: nextModelId \}\)/);
    assert.match(prGuide, /onClick=\{onOpenProviderSettings\}/);
    assert.doesNotMatch(prGuide, /<select value=\{providerId\}|<select value=\{modelId\}/);
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
