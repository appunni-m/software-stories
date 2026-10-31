import type { ChapterRevision, NarrationSynthesisInput } from '../lib/commands';
import { sectionNarrationScopes, type ReadNarrationStorage } from '../lib/narrationScope.ts';
import { applyVoiceConnection, type VoiceConnection } from '../lib/voiceConnection.ts';

/** Build only local Pocket requests for the exact section/scene scripts the player uses. */
export function localNarrationPrewarmInputs(
  revision: ChapterRevision,
  connection: VoiceConnection,
  read: ReadNarrationStorage,
): NarrationSynthesisInput[] {
  if (connection.backend !== 'pocket-tts') return [];
  const requests: NarrationSynthesisInput[] = [];
  for (const section of revision.sections) {
    try {
      for (const scope of sectionNarrationScopes(revision, section, read, () => {
        // An unreadable scene cannot identify its cached audio; keep warming
        // valid sibling scenes instead of dropping this whole section.
      })) {
        const preferences = applyVoiceConnection(scope.preferences, connection);
        if (preferences.backend !== 'pocket-tts') continue;
        if (!preferences.spokenText.trim()) continue;
        requests.push({
          spokenText: preferences.spokenText,
          voice: preferences.voice,
          style: preferences.style,
          // Disabled entries do not affect the canonical script or audio
          // fingerprint. Drop them before client queue admission so they
          // cannot consume a separate deduplication slot.
          lexicon: preferences.lexicon.filter((entry) => entry.include),
        });
      }
    } catch {
      // An unreadable preference cannot safely identify audio; skip this scope.
    }
  }
  return requests;
}
