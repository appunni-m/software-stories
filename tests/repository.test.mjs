import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { captureRepository, loadSnapshot, searchSources } from '../src/server/repository.mjs';
import { run } from '../src/server/util.mjs';
test('snapshots retain earlier source evidence after code changes and skip outside symlinks', async () => {
  const root = await mkdtemp(join(tmpdir(), 'stories-source-'));
  const outside = await mkdtemp(join(tmpdir(), 'stories-outside-'));
  try {
    await writeFile(join(root, 'cache.js'), 'export const value = 1;');
    await writeFile(join(outside, 'outside.js'), 'outside');
    await symlink(join(outside, 'outside.js'), join(root, 'linked.js'));
    const before = await captureRepository(root);
    await writeFile(join(root, 'cache.js'), 'export const value = 2;');
    const after = await captureRepository(root);
    assert.notEqual(before.id, after.id);
    assert.equal((await loadSnapshot(before.id)).files[0].text, 'export const value = 1;');
    assert.equal(before.files.length, 1);
    assert.equal(searchSources(after, 'cache')[0].path, 'cache.js');
  } finally { await rm(root, { recursive: true, force: true }); await rm(outside, { recursive: true, force: true }); }
});
test('change explanations capture current source, earlier code, and an actual Git diff', async () => {
  const root = await mkdtemp(join(tmpdir(), 'stories-git-'));
  const git = args => run('git', args, { cwd: root });
  try {
    await git(['init', '-q']); await git(['config', 'user.email', 'fixture@example.invalid']); await git(['config', 'user.name', 'Fixture']);
    await writeFile(join(root, 'cache.js'), 'export const value = 1;\n'); await git(['add', 'cache.js']); await git(['commit', '-qm', 'Fixture']);
    await writeFile(join(root, 'cache.js'), 'export const value = 2;\n');
    const snapshot = await captureRepository(root, { baseRef: 'HEAD' });
    assert.ok(snapshot.diff.includes('+export const value = 2;'));
    assert.equal(snapshot.files.find(file => file.path === 'before:cache.js').text, 'export const value = 1;\n');
    assert.equal(snapshot.files.find(file => file.path === 'cache.js').text, 'export const value = 2;\n');
    assert.equal(snapshot.files.find(file => file.path === 'before:cache.js').revision, snapshot.baseCommit);
    await assert.rejects(captureRepository(root, { baseRef: '--output=/tmp/fake' }), /Invalid Git base/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
