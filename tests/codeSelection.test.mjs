import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { bindSelectionToDisplayedSource, codeSelectionContext } from '../src/lib/codeSelection.ts';

describe('Code Lens selection source identity', () => {
  it('captures the displayed source hash with selected lines', () => {
    assert.deepEqual(codeSelectionContext('src/font.rs', 'a'.repeat(64), 8, 12), {
      path: 'src/font.rs', sha256: 'a'.repeat(64), from: 8, to: 12,
    });
  });

  it('preserves the selection hash if the displayed file has since changed', () => {
    const selected = codeSelectionContext('src/font.rs', 'a'.repeat(64), 8, 12);
    assert.equal(bindSelectionToDisplayedSource(selected, {
      path: 'src/font.rs', sha256: 'b'.repeat(64),
    }).sha256, 'a'.repeat(64));
  });

  it('fills only a missing hash from the same displayed path', () => {
    const selected = codeSelectionContext('src/font.rs', undefined, 8, 12);
    assert.equal(bindSelectionToDisplayedSource(selected, {
      path: 'src/font.rs', sha256: 'c'.repeat(64),
    }).sha256, 'c'.repeat(64));
    assert.equal(bindSelectionToDisplayedSource(selected, {
      path: 'src/other.rs', sha256: 'd'.repeat(64),
    }).sha256, undefined);
  });
});
