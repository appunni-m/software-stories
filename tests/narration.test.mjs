import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { applyPronunciationLexicon, narrationPreferencesMatch } from '../src/lib/narration.ts';

const preferences = (overrides = {}) => ({
  spokenText: 'Explain the cacheKey.',
  voice: 'Kore',
  style: 'Warm',
  lexicon: [{ name: 'cacheKey', sayAs: 'cache key', include: true }],
  ...overrides,
});

describe('narration cache request snapshots', () => {
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
