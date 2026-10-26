import { getLocalNarrationPreparationStatus, prewarmLocalNarration, type NarrationSynthesisInput } from '../lib/commands';
import { createNarrationPrewarmQueue } from './narrationQueue';

/** One process-wide queue keeps generation-time Pocket work visible to every reader player. */
export const localNarrationPrewarmer = createNarrationPrewarmQueue({
  prepare: prewarmLocalNarration,
  status: getLocalNarrationPreparationStatus,
});

export function localNarrationPrewarmState(input: NarrationSynthesisInput): 'idle' | 'queued' | 'preparing' {
  return localNarrationPrewarmer.state(input);
}
