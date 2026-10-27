import type { NarrationPrewarmStatus, NarrationSynthesisInput, ProviderCommandResult } from '../lib/commands';
import { applyPronunciationLexicon } from '../lib/narration';

export interface NarrationPrewarmQueue {
  enqueue(inputs: readonly NarrationSynthesisInput[]): number;
  state(input: NarrationSynthesisInput): 'idle' | 'queued' | 'preparing';
  cancelPending(): number;
  invalidateIdentity(): void;
  flush(): Promise<void>;
}

/** Feed the native single-worker queue incrementally instead of overflowing
 * its bounded admission limit when a guide publishes many scenes at once. */
export function createNarrationPrewarmQueue(options: {
  prepare: (input: NarrationSynthesisInput) => Promise<ProviderCommandResult<NarrationPrewarmStatus>>;
  status: (input: NarrationSynthesisInput) => Promise<ProviderCommandResult<NarrationPrewarmStatus>>;
  cancelNativePending?: (input: NarrationSynthesisInput) => Promise<boolean>;
  wait?: (milliseconds: number) => Promise<void>;
  pollMilliseconds?: number;
  maxPending?: number;
  maxConcurrent?: number;
}): NarrationPrewarmQueue {
  const pending: Array<{ input: NarrationSynthesisInput; key: string }> = [];
  const keys = new Set<string>();
  const queuedKeys = new Set<string>();
  const preparingKeys = new Set<string>();
  const active = new Set<Promise<void>>();
  const activeInputs = new Map<string, NarrationSynthesisInput>();
  let identityEpoch = 0;
  const wait = options.wait ?? ((milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds)));
  const pollMilliseconds = options.pollMilliseconds ?? 750;
  // The native worker admits one active request and four waiting requests.
  // Leave one slot free so playback can start the clip the reader selected;
  // prewarming must not fill the native queue or lose excess requests.
  const maxConcurrent = Math.max(1, Math.min(4, options.maxConcurrent ?? 4));
  // The native cache exposes 48 prewarm slots: five Guide slides plus one
  // refill slide, with up to eight scenes each. Keep the renderer's queue to
  // that same bound so a long run cannot accumulate hundreds of stale scripts.
  const maxPending = options.maxPending ?? 48;

  const keyOf = (input: NarrationSynthesisInput) => {
    const spokenText = input.spokenText.trim();
    // Native caching fingerprints the script after applying enabled
    // pronunciations. Key equivalent scripts together so unused pronunciation
    // entries do not consume extra bounded-queue slots.
    const lexicon = input.lexicon.filter((entry) => entry.include)
      .map(({ name, sayAs }) => ({ name: name.trim(), sayAs: sayAs.trim(), include: true }));
    const names = new Set<string>();
    const encoder = new TextEncoder();
    const lexiconIsValid = input.lexicon.length <= 12 && lexicon.every(({ name, sayAs }) => {
      if (!name || encoder.encode(name).byteLength > 120 || /[\s\u0085]/u.test(name)
        || !sayAs || encoder.encode(sayAs).byteLength > 160 || names.has(name)) return false;
      names.add(name);
      return true;
    });

    if (lexiconIsValid) {
      return JSON.stringify([
        identityEpoch,
        'audio',
        applyPronunciationLexicon(spokenText, lexicon),
        input.voice,
        input.style.trim(),
      ]);
    }

    // Invalid settings must reach native validation independently; otherwise
    // an invalid request could coalesce with a valid request for the same text.
    return JSON.stringify([
      identityEpoch,
      'invalid',
      spokenText,
      input.voice,
      input.style.trim(),
      lexicon,
    ]);
  };

  const prepareOne = async (input: NarrationSynthesisInput): Promise<void> => {
    let result = await options.prepare(input);
    while (result.status === 'ok' && result.data.state === 'preparing') {
      await wait(pollMilliseconds);
      result = await options.status(input);
    }
  };

  const dropPending = () => {
    const count = pending.length;
    for (const { key } of pending.splice(0)) {
      queuedKeys.delete(key);
      keys.delete(key);
    }
    return count;
  };

  const pump = () => {
    while (pending.length && active.size < maxConcurrent) {
      const { input, key } = pending.shift()!;
      queuedKeys.delete(key);
      preparingKeys.add(key);
      activeInputs.set(key, input);
      let task: Promise<void>;
      task = prepareOne(input).catch(() => {
        // Narration is optional; a failed clip must not block later clips.
      }).finally(() => {
        active.delete(task);
        preparingKeys.delete(key);
        activeInputs.delete(key);
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
        queuedKeys.add(key);
        pending.push({ input, key });
        accepted += 1;
      }
      if (accepted) pump();
      return accepted;
    },
    state(input) {
      const key = keyOf(input);
      if (preparingKeys.has(key)) return 'preparing';
      return queuedKeys.has(key) ? 'queued' : 'idle';
    },
    cancelPending() {
      // These jobs have not crossed into the native worker yet. When the saved
      // voice switches to cloud, discard them without cancelling submitted
      // local synthesis or its reusable cache entries.
      return dropPending();
    },
    invalidateIdentity() {
      // Native audio fingerprints bind the runtime/model. Advance the
      // client-only dedup epoch when the managed runtime may have changed so
      // an active old request cannot suppress a request for the new runtime.
      identityEpoch += 1;
      // Do not spend the native queue on audio keyed to the replaced runtime.
      dropPending();
      if (options.cancelNativePending) {
        for (const input of activeInputs.values()) {
          try {
            void options.cancelNativePending(input).catch(() => {});
          } catch {
            // Cancellation is best-effort; active synthesis keeps its cache key.
          }
        }
      }
    },
    async flush() {
      while (active.size || pending.length) {
        if (!active.size) { pump(); continue; }
        await Promise.all([...active]);
      }
    },
  };
}
