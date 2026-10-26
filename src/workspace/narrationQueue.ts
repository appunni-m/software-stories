import type { NarrationPrewarmStatus, NarrationSynthesisInput, ProviderCommandResult } from '../lib/commands';

export interface NarrationPrewarmQueue {
  enqueue(inputs: readonly NarrationSynthesisInput[]): number;
  flush(): Promise<void>;
}

/** Feed the native single-worker queue incrementally instead of overflowing
 * its bounded admission limit when a guide publishes many scenes at once. */
export function createNarrationPrewarmQueue(options: {
  prepare: (input: NarrationSynthesisInput) => Promise<ProviderCommandResult<NarrationPrewarmStatus>>;
  status: (input: NarrationSynthesisInput) => Promise<ProviderCommandResult<NarrationPrewarmStatus>>;
  wait?: (milliseconds: number) => Promise<void>;
  pollMilliseconds?: number;
  maxPending?: number;
  maxConcurrent?: number;
}): NarrationPrewarmQueue {
  const pending: NarrationSynthesisInput[] = [];
  const keys = new Set<string>();
  const active = new Set<Promise<void>>();
  const wait = options.wait ?? ((milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds)));
  const pollMilliseconds = options.pollMilliseconds ?? 750;
  const maxPending = options.maxPending ?? 512;
  // Native synthesis is single-worker, but it can hold a small queue and let
  // playback promote the clip the reader just selected. Keep that runway
  // populated without consuming the native queue's full capacity.
  const maxConcurrent = Math.max(1, Math.min(8, options.maxConcurrent ?? 8));

  const keyOf = (input: NarrationSynthesisInput) => JSON.stringify([
    input.spokenText, input.voice, input.style, input.lexicon,
  ]);

  const prepareOne = async (input: NarrationSynthesisInput): Promise<void> => {
    let result = await options.prepare(input);
    while (result.status === 'ok' && result.data.state === 'preparing') {
      await wait(pollMilliseconds);
      result = await options.status(input);
    }
  };

  const pump = () => {
    while (pending.length && active.size < maxConcurrent) {
      const input = pending.shift()!;
      const key = keyOf(input);
      let task: Promise<void>;
      task = prepareOne(input).catch(() => {
        // Narration is optional; a failed clip must not block later clips.
      }).finally(() => {
        active.delete(task);
        keys.delete(key);
        pump();
      });
      active.add(task);
    }
  };

  return {
    enqueue(inputs) {
      let accepted = 0;
      for (const input of inputs) {
        const key = keyOf(input);
        if (keys.has(key)) continue;
        if (keys.size >= maxPending) break;
        keys.add(key);
        pending.push(input);
        accepted += 1;
      }
      if (accepted) pump();
      return accepted;
    },
    async flush() {
      while (active.size || pending.length) {
        if (!active.size) { pump(); continue; }
        await Promise.all([...active]);
      }
    },
  };
}
