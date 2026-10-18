import type { ChapterRevision } from '../lib/commands.ts';
import type { ReadingPosition, Turn, WorkspaceState } from './model.ts';

/** Reader preferences only. The original generation receipt is never replaced. */
export interface AnswerVersion {
  originalRevisionId: string;
  revisionId: string;
  selected: boolean;
  reading?: Partial<Record<'guide' | 'flow', string>>;
}
const identifier = (v: unknown): v is string => typeof v === 'string' && /^[A-Za-z0-9_.:-]{1,256}$/.test(v);
const originalIds = (turn: Turn) => new Set([turn.revisionId, ...(turn.publishedSections ?? []).map(s => s.revisionId)].filter((id): id is string => !!id));

export function cleanAnswerVersions(value: unknown, turn: Turn): AnswerVersion[] {
  if (!Array.isArray(value) || value.length > 256) return [];
  const originals = originalIds(turn), pairs = new Set<string>(), selected = new Set<string>();
  const result: AnswerVersion[] = [];
  for (const item of value) {
    if (!item || typeof item !== 'object' || !identifier(item.originalRevisionId) || !originals.has(item.originalRevisionId)
      || !identifier(item.revisionId) || typeof item.selected !== 'boolean') continue;
    const key = `${item.originalRevisionId}\n${item.revisionId}`;
    if (pairs.has(key) || (item.selected && selected.has(item.originalRevisionId))) continue;
    pairs.add(key); if (item.selected) selected.add(item.originalRevisionId);
    const reading: AnswerVersion['reading'] = {};
    for (const view of ['guide', 'flow'] as const) if (identifier(item.reading?.[view])) reading[view] = item.reading[view];
    result.push({originalRevisionId:item.originalRevisionId, revisionId:item.revisionId, selected:item.selected, reading});
  }
  return result;
}

export function ownsAnswerRevision(turn: Turn, revisionId: string): boolean {
  return originalIds(turn).has(revisionId) || cleanAnswerVersions(turn.answerVersions, turn).some(v => v.revisionId === revisionId);
}

/** Only a complete, increasing parent chain in this exact chapter and snapshot
 * connects an accepted edit to an answer. File names and timestamps prove nothing. */
export function answerRevisionOptions(original: ChapterRevision, chapters: readonly ChapterRevision[]): ChapterRevision[] {
  const sameScope = (scope: ChapterRevision['target']['scope']) => {
    const expected = original.target.scope;
    if (scope.kind !== expected.kind) return false;
    if (scope.kind === 'file' && expected.kind === 'file') return scope.path === expected.path;
    if (scope.kind === 'group' && expected.kind === 'group') return scope.paths.length === expected.paths.length && scope.paths.every((p,i) => p === expected.paths[i]);
    return scope.kind === 'overview';
  };
  const family = chapters.filter(r => r.projectId === original.projectId && r.snapshotId === original.snapshotId
    && r.chapterId === original.chapterId && r.target.targetId === original.target.targetId && sameScope(r.target.scope));
  const byId = new Map(family.map(r => [r.revisionId, r]));
  byId.set(original.revisionId, original);
  const descends = (candidate: ChapterRevision): boolean => {
    const seen = new Set<string>(); let current = candidate;
    while (current.revisionId !== original.revisionId) {
      if (seen.has(current.revisionId) || !current.basedOnRevisionId) return false;
      seen.add(current.revisionId);
      const parent = byId.get(current.basedOnRevisionId);
      if (!parent || parent.revisionNumber >= current.revisionNumber) return false;
      current = parent;
    }
    return true;
  };
  return [original, ...family.filter(r => r.revisionId !== original.revisionId && r.lifecycle === 'accepted'
    && r.validationStatus === 'valid' && descends(r)).sort((a,b) => a.revisionNumber - b.revisionNumber || a.revisionId.localeCompare(b.revisionId))];
}

export function resolveAnswerRevision(original: ChapterRevision, turn: Turn, chapters: readonly ChapterRevision[]): ChapterRevision {
  const selected = cleanAnswerVersions(turn.answerVersions, turn).find(v => v.originalRevisionId === original.revisionId && v.selected);
  return answerRevisionOptions(original, chapters).find(r => r.revisionId === selected?.revisionId) ?? original;
}

export function answerReadingPosition(turn: Turn, view: 'guide' | 'flow', revisionId: string): ReadingPosition | undefined {
  const saved = turn.reading?.[view];
  if (saved?.revisionId === revisionId) return saved;
  const sectionId = cleanAnswerVersions(turn.answerVersions, turn).find(v => v.revisionId === revisionId)?.reading?.[view];
  return sectionId ? {revisionId, sectionId} : undefined;
}

export function rememberAnswerVersion(state: WorkspaceState, explorationId: string, turnId: string, originalRevisionId: string, revisionId: string, chapters: readonly ChapterRevision[]): WorkspaceState {
  const turn = state.explorations.find(e => e.id === explorationId)?.turns.find(t => t.id === turnId);
  const original = chapters.find(r => r.revisionId === originalRevisionId && r.projectId === state.projectId && r.snapshotId === state.snapshotId);
  if (!turn || !original || !originalIds(turn).has(originalRevisionId) || !answerRevisionOptions(original, chapters).some(r => r.revisionId === revisionId)) return state;
  const previous = cleanAnswerVersions(turn.answerVersions, turn);
  const versions = previous.map(v => ({...v, reading:{...v.reading}, selected:v.originalRevisionId === originalRevisionId ? false : v.selected}));
  const current = resolveAnswerRevision(original, turn, chapters);
  for (const id of [current.revisionId, revisionId]) {
    let entry = versions.find(v => v.originalRevisionId === originalRevisionId && v.revisionId === id);
    if (!entry) {entry = {originalRevisionId, revisionId:id, selected:false, reading:{}};versions.push(entry);}
    for (const view of ['guide', 'flow'] as const) if (turn.reading?.[view]?.revisionId === id) entry.reading[view] = turn.reading[view]!.sectionId;
    if (id === revisionId) entry.selected = true;
  }
  // Prefer current choices when bounded reader history fills up.
  while (versions.length > 256) {
    const oldest = versions.findIndex(v => !v.selected); if (oldest < 0) return state;
    versions.splice(oldest,1);
  }
  const reading = {...turn.reading};
  const entry = versions.find(v => v.originalRevisionId === originalRevisionId && v.revisionId === revisionId)!;
  const selected = chapters.find(r => r.revisionId === revisionId)!;
  for (const view of ['guide', 'flow'] as const) {
    if (reading[view]?.revisionId === current.revisionId || (!reading[view] && view === 'guide')) {
      const sectionId = entry.reading[view] ?? selected.sections[0]?.sectionId;
      if (sectionId) reading[view] = {revisionId, sectionId};
    }
  }
  if (JSON.stringify(previous) === JSON.stringify(versions) && JSON.stringify(reading) === JSON.stringify(turn.reading)) return state;
  return {...state, explorations:state.explorations.map(e => e.id !== explorationId ? e : {...e, turns:e.turns.map(t => t.id !== turnId ? t : {...t, answerVersions:versions, reading})})};
}
