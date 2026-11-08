import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { cancelPendingAutoplay, takePendingAutoplay } from '../src/lib/narrationPlayback.ts';

describe('deferred narration autoplay', () => {
  it('plays at most once when the media element becomes ready', () => {
    const intent = { current: true };

    assert.equal(takePendingAutoplay(intent), true);
    assert.equal(takePendingAutoplay(intent), false);
    assert.equal(intent.current, false);
  });

  it('does not start later when Stop or a deliberate seek cancels canplay autoplay', () => {
    const intent = { current: true };

    cancelPendingAutoplay(intent);

    assert.equal(takePendingAutoplay(intent), false);
    assert.equal(intent.current, false);
  });
});
