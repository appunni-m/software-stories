import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { applyVoiceConnection, DEFAULT_VOICE_CONNECTION, parseVoiceConnection } from '../src/lib/voiceConnection.ts';
import { narrationPreferencesMatch, parseNarrationPreferences } from '../src/lib/narration.ts';

describe('one voice connection for every answer', () => {
  it('defaults malformed or unknown connections to local speech', () => {
    for (const input of [null, '{bad', '[]', 'null', '{"backend":"future"}']) {
      assert.deepEqual(parseVoiceConnection(input), DEFAULT_VOICE_CONNECTION);
    }
  });
  it('uses Connections delivery while preserving legacy script and pronunciation edits', () => {
    const section = parseNarrationPreferences(JSON.stringify({ spokenText: 'A saved script.', voice: 'Puck', style: 'Warm', lexicon: [{ name: 'parseRoutes', sayAs: 'parse routes', include: true }] }), 'Original');
    assert.equal(section.backend, 'gemini');
    const effective = applyVoiceConnection(section, DEFAULT_VOICE_CONNECTION);
    assert.equal(effective.backend, 'pocket-tts');
    assert.equal(effective.voice, 'alba');
    assert.equal(effective.style, '');
    assert.equal(effective.spokenText, 'A saved script.');
    assert.deepEqual(effective.lexicon, section.lexicon);
    assert.equal(section.voice, 'Puck'); // Do not rewrite saved legacy preferences.
  });
  it('round trips the global choice and keeps cloud delivery when switching back from local', () => {
    const google = parseVoiceConnection('{"backend":"gemini","googleVoice":"Charon","googleStyle":"Calm"}');
    assert.deepEqual(parseVoiceConnection(JSON.stringify(google)), google);
    const local = { ...google, backend: 'pocket-tts' };
    assert.equal(parseVoiceConnection(JSON.stringify(local)).googleVoice, 'Charon');
    const original = parseNarrationPreferences(null, 'Explain this.');
    const cloud = applyVoiceConnection(original, google);
    assert.equal(cloud.voice, 'Charon');
    assert.equal(narrationPreferencesMatch(cloud, applyVoiceConnection(original, local)), false);
    assert.equal(narrationPreferencesMatch(cloud, applyVoiceConnection(original, google)), true);
  });
  it('bounds stored style by UTF-8 bytes and rejects malformed voices', () => {
    const parsed = parseVoiceConnection(JSON.stringify({ backend: 'gemini', googleVoice: '../invalid', googleStyle: '好'.repeat(54) }));
    assert.equal(parsed.googleVoice, 'Kore');
    assert.equal(parsed.googleStyle, '');
  });
});
