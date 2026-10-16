import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { it } from 'node:test';
import { requestScopeKey } from '../src/lib/requestScopeKey.ts';

it('keeps request lifetime across equivalent parent scope copies', () => {
  const original = { kind: 'repository-plan', selectedPaths: ['src/cart.ts'], question: 'How is a total calculated?', projectId: 'project', snapshotId: 'snapshot' };
  const copied = { snapshotId: 'snapshot', projectId: 'project', question: original.question, selectedPaths: [...original.selectedPaths], kind: 'repository-plan' };
  assert.equal(requestScopeKey(original), requestScopeKey(copied));
  for (const [key, value] of Object.entries({ question: 'How are errors handled?', selectedPaths: ['src/tax.ts'], snapshotId: 'new-snapshot', projectId: 'another-project' })) {
    assert.notEqual(requestScopeKey(original), requestScopeKey({ ...original, [key]: value }));
  }
  const planned = { target: { stepId: 'a', revisionId: 'plan', recordSha256: 'sha' } };
  assert.notEqual(requestScopeKey(planned), requestScopeKey({ target: { ...planned.target, stepId: 'b' } }));
});

it('Codex initialization depends on scope value rather than renderer object identity', async () => {
  const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
  const lifetime = source.slice(source.indexOf('    const epoch = ++lifetime.current;'), source.indexOf('  const refresh = async'));
  assert.match(lifetime, /\[native, scopeIdentity\]/);
  assert.doesNotMatch(lifetime, /\}, \[native, projectId/);
  assert.match(lifetime, /setPlanResult\(null\)/);
});
