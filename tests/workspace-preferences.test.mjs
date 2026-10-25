import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { readFileLocation, rememberFileLocation } from '../src/lib/workspacePreferences.ts';

describe('local file location', () => {
  it('restores the source version and file independently for each project', () => {
    const previous = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
    const saved = new Map();
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: {
      getItem: (key) => saved.get(key) ?? null, setItem: (key, value) => saved.set(key, value),
    } });
    try {
      assert.deepEqual(readFileLocation('a'), { status: 'missing' });
      assert.equal(rememberFileLocation('a', { snapshotId: 'snapshot-1', path: 'src/main.rs' }), true);
      assert.equal(rememberFileLocation('b', { snapshotId: 'snapshot-2' }), true);
      assert.deepEqual(readFileLocation('a'), { status: 'available', location: { snapshotId: 'snapshot-1', path: 'src/main.rs' } });
      assert.deepEqual(readFileLocation('b'), { status: 'available', location: { snapshotId: 'snapshot-2' } });
      saved.set('code-atlas:last-file:a', '{bad');
      assert.deepEqual(readFileLocation('a'), { status: 'unavailable' });
      assert.equal(rememberFileLocation('a', { snapshotId: 'snapshot-3' }), false);
      assert.equal(saved.get('code-atlas:last-file:a'), '{bad');
      assert.equal(rememberFileLocation('a', { snapshotId: 'snapshot-3' }, true), true);
      assert.deepEqual(readFileLocation('a'), { status: 'available', location: { snapshotId: 'snapshot-3' } });
      saved.set('code-atlas:last-file:a', '{"snapshotId":12}');
      assert.deepEqual(readFileLocation('a'), { status: 'unavailable' });
      saved.set('code-atlas:last-file:a', '{"snapshotId":"snapshot-1","path":"../secret"}');
      assert.deepEqual(readFileLocation('a'), { status: 'unavailable' });
      saved.set('code-atlas:last-file:a', ' '.repeat(8193));
      assert.deepEqual(readFileLocation('a'), { status: 'unavailable' });
    } finally {
      if (previous) Object.defineProperty(globalThis, 'localStorage', previous);
      else delete globalThis.localStorage;
    }
  });
});
