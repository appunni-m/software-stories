import type { ChapterRevision, ChapterSection } from './commands';
import { defaultNarrationText, parseNarrationPreferences, type NarrationPreferencesSnapshot } from './narration.ts';
import { storyScenes } from './visualStory.ts';

export type ReadNarrationStorage = (key: string) => string | null;

export function narrationStorageKey(projectId: string, chapterId: string, revisionId: string, sectionId: string): string {
  return `code-atlas:narration:${projectId}:${chapterId}:${revisionId}:${sectionId}`;
}

/** Player and export must inherit delivery settings without substituting a
 * whole-section script for a scene. Storage failures are handled by the caller.
 */
export function scopedNarrationPreferences(read: ReadNarrationStorage, key: string, fallback: string, parentKey?: string): NarrationPreferencesSnapshot {
  const saved = read(key);
  if (saved) return parseNarrationPreferences(saved, fallback);
  const parent = parentKey ? parseNarrationPreferences(read(parentKey), fallback) : null;
  return parent ? { ...parent, spokenText: fallback } : parseNarrationPreferences(null, fallback);
}

export interface NarrationScope {
  sceneId?: string;
  label: string;
  preferences: NarrationPreferencesSnapshot;
}

/** Match the scopes actually used by VisualStory/NarrationPlayer. A custom
 * whole-section script or legacy summary stays one track, with no guessed cuts.
 */
export function sectionNarrationScopes(revision: ChapterRevision, section: ChapterSection, read: ReadNarrationStorage): NarrationScope[] {
  const baseKey = narrationStorageKey(revision.projectId, revision.chapterId, revision.revisionId, section.sectionId);
  const fallback = defaultNarrationText(section.spokenText);
  const parent = scopedNarrationPreferences(read, baseKey, fallback);
  const scenes = storyScenes(section, undefined, parent.spokenText !== fallback ? parent.spokenText : undefined);
  if (scenes.length === 1) return [{ label: section.title, preferences: parent }];
  return scenes.map((scene, index) => ({
    sceneId: scene.id,
    label: `Scene ${index + 1}`,
    preferences: scopedNarrationPreferences(read, `${baseKey}:scene:${scene.id}`, defaultNarrationText(scene.narration), baseKey),
  }));
}
