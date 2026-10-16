import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { readFileLocation, rememberFileLocation } from '../src/lib/workspacePreferences.ts';

describe('local file location', () => {
  it('restores the source version and file independently for each project', () => {
    const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
    const saved = new Map();
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: {
      getItem: (key) => saved.get(key), setItem: (key, value) => saved.set(key, value),
    } });
    try {
      assert.equal(readFileLocation('a'), null);
      rememberFileLocation('a', { snapshotId: 'snapshot-1', path: 'src/main.rs' });
      rememberFileLocation('b', { snapshotId: 'snapshot-2' });
      assert.deepEqual(readFileLocation('a'), { snapshotId: 'snapshot-1', path: 'src/main.rs' });
      assert.deepEqual(readFileLocation('b'), { snapshotId: 'snapshot-2', path: undefined });
      saved.set('code-atlas:last-file:a', '{bad');
      assert.equal(readFileLocation('a'), null);
      saved.set('code-atlas:last-file:a', '{"snapshotId":12}');
      assert.equal(readFileLocation('a'), null);
    } finally {
      if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
      else delete globalThis.localStorage;
    }
  });
});
