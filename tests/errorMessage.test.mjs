import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { errorMessageOr, explanationErrorMessage } from '../src/lib/errorMessage.ts';

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

it('explains claim validation failures without internal section IDs, including persisted failures', () => {
  for (const message of [
    "Claim in section 'section:4cf1f1eb-3dda-4b65-8af4-d3218b8c09c1' does not name its declared identifier 'record_pipeline_operation_path' in the explanation.",
    "The AI explanation does not name its declared source identifier 'prices'. No draft was saved.",
    "The AI draft still did not connect its explanation to the cited code after one correction. No draft was saved.",
  ]) {
    const result = explanationErrorMessage(message);
    assert.match(result, /cited code/);
    assert.doesNotMatch(result, /section:|record_pipeline_operation_path|prices/);
  }
  assert.equal(explanationErrorMessage('Codex account access expired.'), 'Codex account access expired.');
});
