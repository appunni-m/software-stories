import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { applyVoiceConnection, DEFAULT_VOICE_CONNECTION, parseVoiceConnection } from '../src/lib/voiceConnection.ts';
import { narrationPreferencesMatch, parseNarrationPreferences } from '../src/lib/narration.ts';

describe('one voice connection for every answer', () => {
  it('defaults only when no connection is saved and rejects malformed or unknown connections', () => {
    assert.deepEqual(parseVoiceConnection(null), DEFAULT_VOICE_CONNECTION);
    for (const input of ['{bad', '[]', 'null']) assert.throws(() => parseVoiceConnection(input), /malformed/);
    assert.throws(() => parseVoiceConnection('{"backend":"future","googleVoice":"Kore","googleStyle":""}'), /not available/);
    assert.throws(() => parseVoiceConnection('{"backend":"gemini","googleVoice":"Kore"}'), /unsupported format/);
  });
  it('uses Connections delivery while preserving current script and pronunciation edits', () => {
    const section = parseNarrationPreferences(JSON.stringify({ backend: 'gemini', spokenText: 'A saved script.', voice: 'Puck', style: 'Warm', lexicon: [{ name: 'parseRoutes', sayAs: 'parse routes', include: true }] }), 'Original');
    assert.equal(section.backend, 'gemini');
    const effective = applyVoiceConnection(section, DEFAULT_VOICE_CONNECTION);
    assert.equal(effective.backend, 'pocket-tts');
    assert.equal(effective.voice, 'alba');
    assert.equal(effective.style, '');
    assert.equal(effective.spokenText, 'A saved script.');
    assert.deepEqual(effective.lexicon, section.lexicon);
    assert.equal(section.voice, 'Puck');
    assert.throws(() => parseNarrationPreferences(JSON.stringify({ googleVoice: 'Puck', googleStyle: 'Warm', spokenText: 'A saved script.', voice: 'Puck', style: 'Warm', lexicon: [] }), 'Original'), /unsupported format/);
  });
  it('keeps Pocket TTS as the only global voice engine, including for a saved cloud preference', () => {
    const savedCloud = parseVoiceConnection('{"backend":"gemini","googleVoice":"Charon","googleStyle":"Calm"}');
    assert.equal(savedCloud.backend, 'pocket-tts');
    assert.deepEqual(parseVoiceConnection(JSON.stringify(savedCloud)), savedCloud);
    const original = parseNarrationPreferences(JSON.stringify({ backend: 'gemini', spokenText: 'Explain this.', voice: 'Charon', style: 'Calm', lexicon: [] }), 'Explain this.');
    const effective = applyVoiceConnection(original, savedCloud);
    assert.equal(effective.backend, 'pocket-tts');
    assert.equal(effective.voice, 'alba');
    assert.equal(effective.style, '');
    assert.equal(effective.spokenText, 'Explain this.');
    assert.equal(narrationPreferencesMatch(effective, applyVoiceConnection(original, DEFAULT_VOICE_CONNECTION)), true);
  });
  it('bounds stored style by UTF-8 bytes and rejects malformed voices', () => {
    assert.throws(() => parseVoiceConnection(JSON.stringify({ backend: 'gemini', googleVoice: '../invalid', googleStyle: '' })), /malformed/);
    assert.throws(() => parseVoiceConnection(JSON.stringify({ backend: 'gemini', googleVoice: 'Kore', googleStyle: '好'.repeat(54) })), /malformed/);
  });
});
