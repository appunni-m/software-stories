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
}): NarrationPrewarmQueue {
  const pending: NarrationSynthesisInput[] = [];
  const keys = new Set<string>();
  const wait = options.wait ?? ((milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds)));
  const pollMilliseconds = options.pollMilliseconds ?? 750;
  const maxPending = options.maxPending ?? 512;
  let active: Promise<void> | null = null;

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

  const drain = (): Promise<void> => {
    if (active) return active;
    active = (async () => {
      while (pending.length) {
        const input = pending.shift()!;
        const key = keyOf(input);
        try { await prepareOne(input); }
        catch { /* Narration is optional; a failed clip must not block later clips. */ }
        finally { keys.delete(key); }
      }
    })().finally(() => {
      active = null;
      if (pending.length) void drain().catch(() => {});
    });
    return active;
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
      if (accepted) void drain().catch(() => {});
      return accepted;
    },
    async flush() {
      while (active || pending.length) await (active ?? drain());
    },
  };
}
