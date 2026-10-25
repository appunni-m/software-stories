import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import {
  applyPronunciationLexicon,
  defaultNarrationText,
  DEFAULT_NARRATION_STYLE,
  DEFAULT_NARRATION_VOICE,
  localNarrationAction,
  narrationAvailabilityLabel,
  narrationDurationLabel,
  narrationPreferencesMatch,
  parseNarrationPreferences,
} from '../src/lib/narration.ts';

const preferences = (overrides = {}) => ({
  backend: 'gemini',
  spokenText: 'Explain the cacheKey.',
  voice: 'Kore',
  style: 'Warm',
  lexicon: [{ name: 'cacheKey', sayAs: 'cache key', include: true }],
  ...overrides,
});

describe('default code-name pronunciation', () => {
  it('keeps every sentence and numeric condition while expanding code names', () => {
    const script = 'The `Font::memory_face` method calls parseBDF. If face_index exceeds 3, InvalidFont is returned. Otherwise the HTTPClient keeps the original error.';
    assert.equal(defaultNarrationText(script), 'The Font memory face method calls parse BDF. If face index exceeds 3, Invalid Font is returned. Otherwise the HTTP Client keeps the original error.');
  });

  it('leaves paths, URLs, expressions and embedded names intact', () => {
    const script = 'Read src/file_name.rs or FileName.swift and https://example.com/file_name. Keep `x != 0` and éface_index unchanged. memory_face_extra is a separate name.';
    assert.equal(defaultNarrationText(script), 'Read src/file_name.rs or FileName.swift and https://example.com/file_name. Keep `x != 0` and éface_index unchanged. memory face extra is a separate name.');
  });

  it('never rewrites saved user scripts or explicit pronunciation choices', () => {
    const fallback = defaultNarrationText('memory_face rejects face_index.');
    const saved = parseNarrationPreferences(JSON.stringify({ backend: 'pocket-tts', spokenText: 'My edited memory_face script.', voice: 'alba', style: '', lexicon: [{ name: 'memory_face', sayAs: 'memory face function', include: true }] }), fallback);
    assert.equal(saved.spokenText, 'My edited memory_face script.');
    assert.equal(applyPronunciationLexicon(saved.spokenText, saved.lexicon), 'My edited memory face function script.');
    assert.equal(parseNarrationPreferences(null, fallback).spokenText, 'memory face rejects face index.');
  });

  it('preserves all reviewed Fontdone validation and fallback sentences', () => {
    const artifact = JSON.parse(readFileSync(new URL('../docs/acceptance/artifacts/2026-10-17-fontdone-progressive-partial.json', import.meta.url)));
    const section = artifact.answer.sections[1].preview.revision.sections[0];
    const complete = section.prose;
    const spoken = defaultNarrationText(complete);
    assert.ok(complete.length > 1_000);
    for (const phrase of ['low 16 bits', 'bits 16 through 30', 'at least 17 bytes', 'at least 118 bytes', 'shorter inputs', 'four-byte offset per face', '12-byte offset table', 'zero-table directory']) assert.ok(spoken.includes(phrase), phrase);
    assert.ok(spoken.includes('preserve the original True Type error'));
    assert.ok(!spoken.includes('resolve_face_index'));
  });
});

describe('narration preparation status', () => {
  it('uses plain-language states for cached and on-demand audio', () => {
    assert.equal(narrationAvailabilityLabel('checking'), 'Checking audio');
    assert.equal(narrationAvailabilityLabel('idle'), 'Audio not prepared');
    assert.equal(narrationAvailabilityLabel('preparing'), 'Preparing audio');
    assert.equal(narrationAvailabilityLabel('ready'), 'Ready to play');
    assert.equal(narrationAvailabilityLabel('unavailable'), 'Audio unavailable');
  });

  it('does not show unloaded audio metadata as a zero-second recording', () => {
    assert.equal(narrationDurationLabel(0), '—:—');
    assert.equal(narrationDurationLabel(Number.NaN), '—:—');
    assert.equal(narrationDurationLabel(Number.POSITIVE_INFINITY), '—:—');
    assert.equal(narrationDurationLabel(13.8), '00:13');
    assert.equal(narrationDurationLabel(61), '01:01');
  });
});

describe('narration cache request snapshots', () => {
  it('uses one Play action for local synthesis and reuses cached audio', () => {
    assert.equal(localNarrationAction(false), 'synthesize');
    assert.equal(localNarrationAction(true), 'play-cache');
  });

  it('accepts one explicit preference contract and rejects malformed or unsupported records', () => {
    assert.deepEqual(parseNarrationPreferences('{"backend":"gemini","spokenText":"Saved script","voice":"Puck","style":"Gentle","lexicon":[{"name":"foo","sayAs":"f oo","include":true}]}', 'Chapter script'), {
      backend: 'gemini',
      spokenText: 'Saved script',
      voice: 'Puck',
      style: 'Gentle',
      lexicon: [{ name: 'foo', sayAs: 'f oo', include: true }],
    });
    assert.throws(() => parseNarrationPreferences('{broken', 'Chapter script'), /malformed/);
    assert.throws(() => parseNarrationPreferences('{"spokenText":"Old script","voice":"Puck","style":"Gentle","lexicon":[]}', 'Chapter script'), /unsupported format/);
    assert.throws(() => parseNarrationPreferences('{"googleVoice":"Puck","googleStyle":"Gentle","spokenText":"Old script","voice":"Puck","style":"Gentle","lexicon":[]}', 'Chapter script'), /unsupported format/);
    assert.throws(() => parseNarrationPreferences('{"backend":"future-backend","spokenText":"Script","voice":"alba","style":"","lexicon":[]}', 'Chapter script'), /unsupported voice engine/);
    assert.throws(() => parseNarrationPreferences('{"backend":"pocket-tts","spokenText":"Script","voice":"alba","style":"","lexicon":[null]}', 'Chapter script'), /malformed/);
    assert.deepEqual(parseNarrationPreferences(null, 'Chapter script'), {
      backend: 'pocket-tts',
      spokenText: 'Chapter script',
      voice: DEFAULT_NARRATION_VOICE,
      style: DEFAULT_NARRATION_STYLE,
      lexicon: [],
    });
  });

  it('accepts cached audio only while the transcript and speech settings still match', () => {
    const requested = preferences();
    assert.equal(narrationPreferencesMatch(requested, preferences()), true);
    assert.equal(narrationPreferencesMatch(requested, preferences({ spokenText: 'Edited script.' })), false);
    assert.equal(narrationPreferencesMatch(requested, preferences({ voice: 'Puck' })), false);
    assert.equal(narrationPreferencesMatch(requested, preferences({ style: 'Gentle' })), false);
    assert.equal(narrationPreferencesMatch(requested, preferences({
      lexicon: [{ name: 'cacheKey', sayAs: 'cash key', include: true }],
    })), false);
  });

  it('matches cached audio by the exact spoken payload, not inactive lexicon metadata', () => {
    const requested = preferences({
      spokenText: 'The cacheKey calls parseRoutes.',
      lexicon: [
        { name: 'cacheKey', sayAs: 'cache key', include: true },
        { name: 'unusedName', sayAs: 'unused name', include: false },
      ],
    });
    assert.equal(narrationPreferencesMatch(requested, preferences({
      spokenText: '  The cacheKey calls parseRoutes.  ',
      lexicon: [
        { name: 'parseRoutes', sayAs: 'parse routes', include: false },
        { name: 'cacheKey', sayAs: 'cache key', include: true },
      ],
    })), true);
    assert.equal(narrationPreferencesMatch(requested, preferences({
      spokenText: 'The cacheKey calls parseRoutes.',
      lexicon: [{ name: 'cacheKey', sayAs: 'cash key', include: true }],
    })), false);
  });

  it('replaces the longest whole identifier match and leaves embedded names alone', () => {
    assert.equal(applyPronunciationLexicon('parseRoutes parseRoutesFast myparseRoutes', [
      { name: 'parseRoutes', sayAs: 'parse routes', include: true },
      { name: 'parseRoutesFast', sayAs: 'parse routes fast', include: true },
    ]), 'parse routes parse routes fast myparseRoutes');
  });
});
