import { afterEach, describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  parseGitComparisonPreference,
  readGitComparisonPreference,
  rememberGitComparisonPreference,
  restoreGitComparisonPreference,
} from '../src/lib/gitComparisonPreferences.ts';

const originalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');

afterEach(() => {
  if (originalStorage) Object.defineProperty(globalThis, 'localStorage', originalStorage);
  else delete globalThis.localStorage;
});

function installStorage({ failRead = false, failWrite = false } = {}) {
  const values = new Map();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem(key) {
        if (failRead) throw new Error('storage unavailable');
        return values.get(key) ?? null;
      },
      setItem(key, value) {
        if (failWrite) throw new Error('storage full');
        values.set(key, value);
      },
    },
  });
  return values;
}

describe('Git comparison preferences', () => {
  it('remembers a normalized pair separately for each repository', () => {
    installStorage();
    assert.equal(rememberGitComparisonPreference('repo one', ' v2.14.3 ', 'main'), true);
    assert.deepEqual(readGitComparisonPreference('repo one'), { baseRef: 'v2.14.3', headRef: 'main' });
    assert.equal(readGitComparisonPreference('repo two'), null);
  });

  it('rejects incomplete, malformed, control-containing, or oversized refs', () => {
    assert.equal(parseGitComparisonPreference({ baseRef: 'main', headRef: '' }), null);
    assert.equal(parseGitComparisonPreference({ baseRef: 'main\n--upload-pack=evil', headRef: 'HEAD' }), null);
    assert.equal(parseGitComparisonPreference({ baseRef: 'x'.repeat(1025), headRef: 'HEAD' }), null);
    assert.equal(parseGitComparisonPreference('{"baseRef":"main","headRef":"HEAD"}'), null);
  });

  it('keeps the workspace usable when local storage is unavailable', () => {
    installStorage({ failRead: true, failWrite: true });
    assert.equal(readGitComparisonPreference('repo'), null);
    assert.equal(rememberGitComparisonPreference('repo', 'main', 'HEAD'), false);
  });

  it('restores the saved pair only when opening Changes without explicit refs', () => {
    const preference = { baseRef: 'v2.14.3', headRef: 'main' };
    const changes = { workspace: 'changes', projectId: 'repo', selectedChangePath: 'README.md' };
    assert.deepEqual(restoreGitComparisonPreference(changes, preference), {
      ...changes,
      ...preference,
    });
    assert.equal(restoreGitComparisonPreference({ ...changes, baseRef: 'custom-base' }, preference).baseRef, 'custom-base');
    const explorer = { ...changes, workspace: 'atlas' };
    assert.equal(restoreGitComparisonPreference(explorer, preference), explorer);
    assert.equal(restoreGitComparisonPreference(changes, null), changes);
  });
});
