import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { errorMessageOr } from '../src/lib/errorMessage.ts';

describe('native and frontend error messages', () => {
  it('preserves string rejections returned by Rust commands', () => {
    assert.equal(
      errorMessageOr('Captured source failed its size or SHA-256 integrity check.', 'Fallback'),
      'Captured source failed its size or SHA-256 integrity check.',
    );
  });

  it('supports Error and structured message values', () => {
    assert.equal(errorMessageOr(new Error('webview failure'), 'Fallback'), 'webview failure');
    assert.equal(errorMessageOr({ message: 'structured failure' }, 'Fallback'), 'structured failure');
  });

  it('uses the fallback for empty or unrecognized rejections', () => {
    assert.equal(errorMessageOr('', 'Fallback'), 'Fallback');
    assert.equal(errorMessageOr({ code: 'E_FAIL' }, 'Fallback'), 'Fallback');
    assert.equal(errorMessageOr(null, 'Fallback'), 'Fallback');
  });
});
