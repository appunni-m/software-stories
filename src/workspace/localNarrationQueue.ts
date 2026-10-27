import { getLocalNarrationPreparationStatus, prewarmLocalNarration, type NarrationSynthesisInput } from '../lib/commands';
import { VOICE_CONNECTION_CHANGED_EVENT } from '../lib/voiceConnectionStore';
import { createNarrationPrewarmQueue } from './narrationQueue';

/** One process-wide queue keeps generation-time Pocket work visible to every reader player. */
export const localNarrationPrewarmer = createNarrationPrewarmQueue({
  prepare: prewarmLocalNarration,
  status: getLocalNarrationPreparationStatus,
});

// The global saved voice applies to every answer. Discard not-yet-started jobs
// after any setting change so Pocket-to-Pocket edits do not synthesize stale
// voice or script settings. Submitted native work remains fingerprinted/cacheable.
if (typeof window !== 'undefined') {
  window.addEventListener(VOICE_CONNECTION_CHANGED_EVENT, () => {
    localNarrationPrewarmer.cancelPending();
  });
}

export function localNarrationPrewarmState(input: NarrationSynthesisInput): 'idle' | 'queued' | 'preparing' {
  return localNarrationPrewarmer.state(input);
}

/** Separate pending client deduplication from a replaced managed runtime. */
export function invalidateLocalNarrationPrewarmIdentity() {
  localNarrationPrewarmer.invalidateIdentity();
}
