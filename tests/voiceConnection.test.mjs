import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { applyVoiceConnection, DEFAULT_VOICE_CONNECTION, parseVoiceConnection } from '../src/lib/voiceConnection.ts';
import { narrationPreferencesMatch, parseNarrationPreferences } from '../src/lib/narration.ts';

describe('one voice connection for every answer', () => {
  it('defaults only when no connection is saved and rejects malformed or unknown connections', () => {
    assert.deepEqual(parseVoiceConnection(null), DEFAULT_VOICE_CONNECTION);
    assert.deepEqual(parseVoiceConnection('{"backend":"pocket-tts"}'), DEFAULT_VOICE_CONNECTION);
    for (const input of ['{bad', '[]', 'null']) assert.throws(() => parseVoiceConnection(input), /malformed/);
    assert.throws(() => parseVoiceConnection('{"backend":"future"}'), /not available/);
    assert.throws(() => parseVoiceConnection('{"backend":"gemini"}'), /not available/);
    assert.throws(() => parseVoiceConnection('{"backend":"pocket-tts","googleVoice":"Kore","googleStyle":""}'), /unsupported format/);
  });
  it('uses Connections delivery while preserving current script and pronunciation edits', () => {
    const section = parseNarrationPreferences(JSON.stringify({ backend: 'pocket-tts', spokenText: 'A saved script.', voice: 'alba', style: '', lexicon: [{ name: 'parseRoutes', sayAs: 'parse routes', include: true }] }), 'Original');
    assert.equal(section.backend, 'pocket-tts');
    const effective = applyVoiceConnection(section, DEFAULT_VOICE_CONNECTION);
    assert.equal(effective.backend, 'pocket-tts');
    assert.equal(effective.voice, 'alba');
    assert.equal(effective.style, '');
    assert.equal(effective.spokenText, 'A saved script.');
    assert.deepEqual(effective.lexicon, section.lexicon);
    assert.equal(section.voice, 'alba');
    assert.throws(() => parseNarrationPreferences(JSON.stringify({ googleVoice: 'Puck', googleStyle: 'Warm', spokenText: 'A saved script.', voice: 'Puck', style: 'Warm', lexicon: [] }), 'Original'), /unsupported format/);
  });
  it('rejects old Gemini narration preferences without translating them to Pocket', () => {
    const saved = '{"backend":"gemini","spokenText":"Explain this.","voice":"Charon","style":"Calm","lexicon":[]}';
    assert.throws(() => parseNarrationPreferences(saved, 'Fallback'), /unsupported voice engine/);
    assert.equal(saved.includes('"backend":"gemini"'), true);
    assert.equal(narrationPreferencesMatch(
      parseNarrationPreferences('{"backend":"pocket-tts","spokenText":"Explain this.","voice":"alba","style":"","lexicon":[]}', 'Fallback'),
      applyVoiceConnection(parseNarrationPreferences('{"backend":"pocket-tts","spokenText":"Explain this.","voice":"alba","style":"","lexicon":[]}', 'Fallback'), DEFAULT_VOICE_CONNECTION),
    ), true);
  });
});
