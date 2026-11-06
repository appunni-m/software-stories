import assert from 'node:assert/strict';
import { it } from 'node:test';
import { listenForPocketTtsRuntimeReady, notifyPocketTtsRuntimeReady } from '../src/workspace/pocketRuntimeEvents.ts';
import { createNarrationPrewarmQueue } from '../src/workspace/narrationQueue.ts';

it('retries the active local clip after Pocket becomes ready', async () => {
  const target = new EventTarget();
  const input = { spokenText: 'A local synthetic narration check.', voice: 'alba', style: '', lexicon: [] };
  let attempts = 0;
  const queue = createNarrationPrewarmQueue({
    prepare: async () => {
      attempts += 1;
      return attempts === 1
        ? { status: 'error', error: { message: 'Pocket runtime is not ready.' } }
        : { status: 'ok', data: { state: 'ready', cachedAudioAvailable: true } };
    },
    status: async () => assert.fail('ready audio should not be polled'),
  });

  queue.enqueue([input]);
  await queue.flush();
  assert.equal(queue.enqueue([input]), 0, 'the failed attempt is coalesced until the runtime identity changes');

  queue.invalidateIdentity();
  const stop = listenForPocketTtsRuntimeReady(target, () => { queue.enqueue([input]); });
  notifyPocketTtsRuntimeReady(target);
  await queue.flush();

  assert.equal(attempts, 2);
  assert.equal(queue.state(input), 'idle');
  stop();
});
