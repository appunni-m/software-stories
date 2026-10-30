import test from 'node:test';
import assert from 'node:assert/strict';
import { filterExplorations } from '../src/workspace/explorationFilter.ts';

const explorations = [
  { id: 'root', parentId: null, title: 'Repository', file: undefined },
  { id: 'loading', parentId: 'root', title: 'How glyph loading works', file: 'src/glyph.rs' },
  { id: 'flags', parentId: 'loading', title: 'Normalize variation flags', file: 'src/glyph.rs' },
  { id: 'errors', parentId: 'root', title: 'How errors are reported', file: 'src/error.rs' },
];

test('exploration search keeps matching descendants and the ancestor path visible', () => {
  const result = filterExplorations(explorations, 'variation flags');
  assert.deepEqual([...result.matchedIds], ['flags']);
  assert.deepEqual([...result.visibleIds], ['flags', 'loading', 'root']);
  assert.deepEqual([...result.expandedIds], ['loading', 'root']);
});

test('exploration search matches file context and the repository label', () => {
  const byFile = filterExplorations(explorations, 'error.rs');
  assert.deepEqual([...byFile.matchedIds], ['errors']);
  assert.ok(byFile.visibleIds.has('root'));
  const byRepository = filterExplorations(explorations, 'Fontdone', 'Fontdone');
  assert.deepEqual([...byRepository.matchedIds], ['root']);
});

test('empty and unmatched exploration searches have stable tree results', () => {
  const empty = filterExplorations(explorations, '  ');
  assert.equal(empty.matchedIds.size, 0);
  assert.deepEqual([...empty.visibleIds], explorations.map(({ id }) => id));
  assert.equal(empty.expandedIds.size, 0);
  const missing = filterExplorations(explorations, 'not present');
  assert.equal(missing.matchedIds.size, 0);
  assert.equal(missing.visibleIds.size, 0);
});
