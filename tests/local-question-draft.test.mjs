import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { parseLocalQuestionDraft, serializeLocalQuestionDraft } from '../src/lib/localQuestionDraft.ts';

describe('saved unsent learning question', () => {
  it('reads only the current exact record while allowing an incomplete in-progress question', () => {
    const serialized = serializeLocalQuestionDraft('How does this work?');
    assert.equal(parseLocalQuestionDraft(null).issue, null);
    assert.deepEqual(parseLocalQuestionDraft(serialized), { question: 'How does this work?', issue: null });
    assert.deepEqual(parseLocalQuestionDraft(serializeLocalQuestionDraft('How')), { question: 'How', issue: null });
  });

  it('preserves unsupported, malformed, oversized and invalid records for the UI to report', () => {
    assert.equal(parseLocalQuestionDraft('{bad').issue, 'unreadable');
    assert.equal(parseLocalQuestionDraft('{"schemaVersion":2,"question":"How?"}').issue, 'unsupported');
    assert.equal(parseLocalQuestionDraft('{"schemaVersion":1,"question":"x","extra":true}').issue, 'invalid');
    assert.equal(parseLocalQuestionDraft(JSON.stringify({ schemaVersion: 1, question: 'x'.repeat(301) })).issue, 'invalid');
    assert.equal(parseLocalQuestionDraft('{"schemaVersion":1,"question":"bad\\nquestion"}').issue, 'invalid');
    assert.equal(parseLocalQuestionDraft(' '.repeat(2_049)).issue, 'invalid');
    assert.equal(serializeLocalQuestionDraft('bad\nquestion'), null);
    assert.equal(serializeLocalQuestionDraft('x'.repeat(301)), null);
  });
});
