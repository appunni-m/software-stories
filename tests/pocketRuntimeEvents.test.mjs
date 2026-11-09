import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { it } from 'node:test';
import { listenForPocketTtsRuntimeReady, notifyPocketTtsRuntimeReady } from '../src/workspace/pocketRuntimeEvents.ts';

it('refreshes local audio status after setup without starting synthesis', () => {
  const target = new EventTarget();
  let refreshCount = 0;
  const stop = listenForPocketTtsRuntimeReady(target, () => { refreshCount += 1; });
  notifyPocketTtsRuntimeReady(target);
  stop();
  assert.equal(refreshCount, 1);

  const player = readFileSync(new URL('../src/NarrationPlayer.tsx', import.meta.url), 'utf8');
  const listenerStart = player.indexOf('listenForPocketTtsRuntimeReady(window, () => {');
  assert.notEqual(listenerStart, -1);
  const listenerEnd = player.indexOf('})', listenerStart);
  const listenerBody = player.slice(listenerStart, listenerEnd);
  assert.match(listenerBody, /void refresh\(\)/u);
  assert.doesNotMatch(listenerBody, /enqueue|retry|synthesizeReviewedNarrationAudio/u);
});
