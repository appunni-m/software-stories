import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { verifyMacExecutableIntegrity } from '../scripts/macos-executable-integrity.mjs';

const macOnly = { skip: process.platform !== 'darwin' };

test('macOS integrity accepts an existing signed executable without executing it', macOnly, () => {
  assert.doesNotThrow(() => verifyMacExecutableIntegrity('/bin/echo'));
});

test('macOS integrity rejects a truncated Mach-O with a surviving header', macOnly, async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atlas-macho-integrity-'));
  try {
    const original = await readFile('/bin/echo');
    const truncated = join(directory, 'truncated');
    await writeFile(truncated, original.subarray(0, Math.floor(original.length / 2)));
    assert.throws(() => verifyMacExecutableIntegrity(truncated), /integrity validation failed/);
  } finally {
    await rm(directory, { recursive: true });
  }
});

test('macOS integrity rejects a missing executable', macOnly, () => {
  assert.throws(() => verifyMacExecutableIntegrity('/no-such-atlas-executable'), /integrity validation failed/);
});
