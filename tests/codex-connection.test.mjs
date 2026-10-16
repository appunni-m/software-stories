import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { restoreCodexConnection } from '../src/lib/codexConnection.ts';

const ready = {
  executablePath: '/installed/codex', runtimeRunning: true, authenticated: true,
  accountEmail: 'synthetic@example.com', planType: 'test', pendingLoginId: null,
  activeRunId: null, capabilityChecked: true, message: 'Ready',
};
const catalog = [{ modelId: 'gpt-6-luna', displayName: 'GPT-6 Luna',
  supportedReasoningEfforts: ['max'], defaultReasoningEffort: 'max', isDefault: true }];

function runtime(initial = ready, resumed = ready) {
  const calls = [];
  return {
    calls,
    commands: {
      status: async () => { calls.push('status'); return initial; },
      resume: async () => { calls.push('resume'); return resumed; },
      models: async () => { calls.push('models'); return catalog; },
    },
  };
}

describe('one-time Codex connection', () => {
  it('reuses the running connection every time a request opens', async () => {
    const { commands, calls } = runtime();
    for (let request = 0; request < 3; request++) {
      assert.deepEqual(await restoreCodexConnection(commands), { status: ready, models: catalog });
    }
    assert.deepEqual(calls, ['status', 'models', 'status', 'models', 'status', 'models']);
  });

  it('silently restores an existing account after app restart', async () => {
    const { commands, calls } = runtime({ ...ready, runtimeRunning: false, authenticated: false });
    assert.deepEqual(await restoreCodexConnection(commands), { status: ready, models: catalog });
    assert.deepEqual(calls, ['status', 'resume', 'models']);
  });

  it('leaves explicit disconnect and missing or expired login for one deliberate connect action', async () => {
    for (const unavailable of [
      { ...ready, authenticated: false, runtimeRunning: false, message: 'Disconnected' },
      { ...ready, authenticated: false, executablePath: null },
      { ...ready, authenticated: false, message: 'Sign in' },
    ]) {
      const { commands, calls } = runtime(unavailable, unavailable);
      assert.deepEqual(await restoreCodexConnection(commands), { status: unavailable, models: [] });
      assert.deepEqual(calls, ['status', 'resume']);
    }
  });

  it('does not reconnect or disturb another active generation', async () => {
    const active = { ...ready, activeRunId: 'active-run' };
    const { commands, calls } = runtime(active);
    assert.deepEqual(await restoreCodexConnection(commands), { status: active, models: [] });
    assert.deepEqual(calls, ['status']);
  });

  it('retains a known connected account when catalog loading fails', async () => {
    const { commands, calls } = runtime();
    let observed;
    commands.onStatus = (value) => { observed = value; };
    commands.models = async () => { calls.push('models'); throw new Error('offline'); };
    await assert.rejects(restoreCodexConnection(commands), /offline/);
    assert.equal(observed, ready);
    assert.deepEqual(calls, ['status', 'models']);
  });

  it('surfaces connection failure without starting sign-in or retry loops', async () => {
    const { commands, calls } = runtime({ ...ready, authenticated: false });
    commands.resume = async () => { calls.push('resume'); throw new Error('runtime unavailable'); };
    await assert.rejects(restoreCodexConnection(commands), /runtime unavailable/);
    assert.deepEqual(calls, ['status', 'resume']);
  });

  it('keeps setup hidden while checking and never starts login from dialog initialization', async () => {
    const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
    const initialization = source.slice(source.indexOf('    const epoch = ++lifetime.current;'), source.indexOf('  const refresh = async'));
    assert.match(initialization, /ensureCodexConnection/);
    assert.doesNotMatch(initialization, /startCodexLogin|setConsent\(true\)/);
    assert.match(source, /checkingConnection \? <p/);
    assert.match(source, /open=\{mode === 'settings' \|\| !status\?\.authenticated\}/);
    assert.match(source, /if \(!value.length\) return previous/);
    assert.match(source, /\{!status\?\.authenticated && <button className="primary-button"/);
    assert.doesNotMatch(source, /hidden=\{Boolean\(status\?\.authenticated\)\}/);
    assert.match(source, /Source review and consent apply separately/);
  });
});
