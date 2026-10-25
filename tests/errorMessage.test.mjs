import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { errorMessageOr, explanationErrorMessage, explanationFailureMessage } from '../src/lib/errorMessage.ts';

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

it('explains current claim validation failures without source identifiers', () => {
  for (const message of [
    "The AI explanation does not name its declared source identifier 'prices'. No draft was saved.",
    "The AI explanation does not connect source identifier 'subtotal' to its cited code. No draft was saved.",
  ]) {
    const result = explanationErrorMessage(message);
    assert.match(result, /cited code/);
    assert.doesNotMatch(result, /section:|record_pipeline_operation_path|prices/);
  }
  assert.equal(explanationErrorMessage('Codex account access expired.'), 'Codex account access expired.');
});

it('turns a rejected empty answer into a useful, private retry hint', () => {
  const message = explanationFailureMessage('noPublishedSections');
  assert.match(message, /couldn’t verify an explanation/);
  assert.match(message, /one behavior at a time/);
  assert.match(message, /caller or helper/);
  assert.doesNotMatch(message, /namedIdentifierEvidence|proposalCorrectionLimit|noPublishedSections/);
  assert.equal(explanationErrorMessage('The answer did not publish any sections.'), message);
});

it('surfaces the actionable local-storage recovery when Codex context staging fails', () => {
  for (const operation of ['created', 'saved']) {
    const error = `The private Codex file could not be ${operation}. Local storage is full. Free disk space, then try again.`;
    const message = explanationErrorMessage(error);
    assert.match(message, /prepare the selected code/);
    assert.match(message, /Free some space, then try again/);
    assert.doesNotMatch(message, /private Codex file|Codex file could not be/);
  }
  assert.equal(
    explanationErrorMessage('The private Codex file could not be created. Check permissions.'),
    'The private Codex file could not be created. Check permissions.',
  );
});
