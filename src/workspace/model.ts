/** Flat, snapshot-scoped conversation state. Generated text lives in native revisions. */
export type View = 'guide' | 'flow' | 'history';
export interface Context { path: string; sha256?: string; from?: number; to?: number }
export interface ReadingPosition { revisionId: string; sectionId: string }
export interface PublishedSection {
  runId: string; key: string; order: number; generationId: string; revisionId: string;
  supersedes: string | null; contextElapsedMs: number; runElapsedMs: number | null;
}
export interface AnswerSummary {
  runId: string; projectId: string; snapshotId: string; question: string;
  outcome: 'working' | 'complete' | 'partial' | 'stopped' | 'failed' | 'interrupted';
  sections: PublishedSection[]; incompleteTopics: string[];
}
export interface RepositoryGuide {
  revision: number;
  id: string; projectId: string; snapshotId: string; question: string; title: string;
  status: 'planning' | 'building' | 'ready' | 'partial' | 'stopped' | 'interrupted' | 'failed';
  openingRunId?: string | null;
  openingRunIds?: string[];
  canContinue?: boolean;
  continuationOperations?: number;
  units: { key: string; question: string; prerequisites: string[]; status: string; runIds: string[] }[];
  answers: AnswerSummary[]; omissions: string[]; capturedFiles: number; startingFiles: number;
}
export interface Turn {
  id: string; question: string; context: Context[];
  status: 'queued' | 'working' | 'ready' | 'partial' | 'failed' | 'stopped' | 'interrupted';
  revisionId?: string; generationId?: string; error?: string;
  runIds?: string[]; publishedSections?: PublishedSection[];
  notes?: string[];
  guideId?: string;
  guide?: Omit<RepositoryGuide, 'answers'>;
  reading?: Partial<Record<'guide' | 'flow', ReadingPosition>>;
  outcomeAt?: number;
  outcomeSequence?: number;
  outcomeNotice?: { read?: string; dismissed?: string };
}
export interface Exploration { id: string; parentId: string | null; originTurnId: string | null; title: string; view: View; turns: Turn[]; focusedTurnId?: string; revealTurnId?: string; file?: string; reveal?: Context; scrollTop?: number; scrollPositions?: Partial<Record<View, number>> }
export interface WorkspaceState { version: 1; projectId: string; snapshotId: string; currentId: string; file: string | null; explorations: Exploration[]; grantId?: string }

/** History describes an available result only when its exact revision loaded.
 * A stopped question and an unavailable revision are still useful history. */
export function historyEntry(turn: Turn, availableRevisions: ReadonlySet<string>): { text: string; action: string } {
  const saved = !!turn.revisionId && availableRevisions.has(turn.revisionId)
    || !!turn.publishedSections?.some((section) => availableRevisions.has(section.revisionId));
  if (saved) {
    const state = turn.status === 'queued' || turn.status === 'working' ? 'Adding more detail.'
      : turn.status === 'partial' ? 'Partially completed.'
      : turn.status === 'stopped' ? 'Stopped.'
      : turn.status === 'interrupted' ? 'Interrupted.'
      : turn.status === 'failed' ? 'Could not finish.' : 'Saved explanation.';
    return { text: `${state}${turn.status === 'ready' ? '' : ' Saved sections are available.'}`, action: 'Open explanation' };
  }
  const text: Record<Turn['status'], string> = {
    queued: 'Waiting to start…', working: 'Working on this question…',
    partial: 'This question is incomplete. No saved section is available.',
    failed: 'This question could not be completed.',
    stopped: 'Stopped before an explanation was saved.',
    interrupted: 'Interrupted before an explanation was saved.',
    ready: 'The saved explanation is unavailable.',
  };
  return { text: text[turn.status], action: 'Open question' };
}

export function createWorkspace(projectId: string, snapshotId: string): WorkspaceState {
  return { version: 1, projectId, snapshotId, currentId: 'root', file: null,
    explorations: [{ id: 'root', parentId: null, originTurnId: null, title: 'Repository', view: 'guide', turns: [] }] };
}
export function ancestors(state: WorkspaceState, id = state.currentId): Exploration[] {
  const result: Exploration[] = []; const seen = new Set<string>();
  let node = state.explorations.find((e) => e.id === id);
  while (node && !seen.has(node.id)) { seen.add(node.id); result.unshift(node); node = state.explorations.find((e) => e.id === node!.parentId); }
  return result;
}
export function parseWorkspace(raw: string | null, projectId: string, snapshotId: string): WorkspaceState {
  const fallback = () => createWorkspace(projectId, snapshotId);
  try {
    const value = JSON.parse(raw ?? 'null') as WorkspaceState;
    if (!value || value.version !== 1 || value.projectId !== projectId || value.snapshotId !== snapshotId || !Array.isArray(value.explorations) || !value.explorations.length || value.explorations.length > 2000) return fallback();
    const ids = new Set(value.explorations.map((e) => e.id));
    if (ids.size !== value.explorations.length || !ids.has('root') || !ids.has(value.currentId)) return fallback();
    for (const e of value.explorations) {
      if (typeof e.id !== 'string' || typeof e.title !== 'string' || !['guide', 'flow', 'history'].includes(e.view) || !Array.isArray(e.turns)
        || (e.id === 'root' ? e.parentId !== null : !e.parentId || !ids.has(e.parentId))) return fallback();
      e.scrollPositions = cleanScrollPositions(e.scrollPositions);
      if (validScroll(e.scrollTop) && e.scrollPositions[e.view] === undefined) e.scrollPositions[e.view] = e.scrollTop;
      delete e.scrollTop;
      delete e.revealTurnId; // A reload restores reading, never moves keyboard focus.
      if (!e.turns.some((t) => t.id === e.focusedTurnId)) delete e.focusedTurnId;
      for (const t of e.turns) {
        if (typeof t.id !== 'string' || typeof t.question !== 'string' || !Array.isArray(t.context)
          || !['queued','working','ready','partial','failed','stopped','interrupted'].includes(t.status)
          || t.context.some((c) => typeof c.path !== 'string' || c.path.startsWith('/') || c.path.split('/').includes('..'))) return fallback();
        const reading = t.reading;
        if (!validRunId(t.guideId)) { delete t.guideId; delete t.guide; }
        else delete t.guide; // Native job state is authoritative after reopening.
        t.runIds = Array.isArray(t.runIds) ? [...new Set(t.runIds.filter(validRunId))].slice(0, 32) : [];
        t.publishedSections = cleanPublishedSections(t.publishedSections).filter((s) => t.runIds!.includes(s.runId));
        t.notes = cleanAnswerNotes(t.notes);
        if (typeof t.outcomeAt !== 'number' || !Number.isSafeInteger(t.outcomeAt) || t.outcomeAt < 0) delete t.outcomeAt;
        if (typeof t.outcomeSequence !== 'number' || !Number.isSafeInteger(t.outcomeSequence) || t.outcomeSequence < 0) delete t.outcomeSequence;
        const notice = t.outcomeNotice;
        delete t.outcomeNotice;
        if (notice && typeof notice === 'object' && !Array.isArray(notice)) {
          const clean: NonNullable<Turn['outcomeNotice']> = {};
          for (const field of ['read', 'dismissed'] as const) if (typeof notice[field] === 'string' && /^[a-f0-9]{16}$/.test(notice[field]!)) clean[field] = notice[field];
          if (Object.keys(clean).length) t.outcomeNotice = clean;
        }
        t.reading = {};
        if (reading && typeof reading === 'object' && !Array.isArray(reading)) {
          for (const view of ['guide', 'flow'] as const) {
            const position = cleanReadingPosition(reading[view]);
            if (position) t.reading[view] = position;
          }
        }
        // A reload never silently resubmits inference. Native saved previews remain recoverable.
        if (t.status === 'working' || t.status === 'queued') { t.status = 'interrupted'; t.error = 'This question was interrupted. Saved explanations are preserved.'; }
      }
      if (ancestors(value, e.id)[0]?.id !== 'root') return fallback();
    }
    return value;
  } catch { return fallback(); }
}

/** A guide is one conversation card, even across independent native runs and
 * a lost local-storage write. Never change the reader's view or section. */
export function mergeRepositoryGuide(state: WorkspaceState, guide: RepositoryGuide): WorkspaceState {
  if (guide.projectId !== state.projectId || guide.snapshotId !== state.snapshotId || !validRunId(guide.id)) return state;
  const runIds = [...new Set([...(guide.openingRunIds ?? [guide.openingRunId]), ...guide.units.flatMap((unit) => unit.runIds)].filter(validRunId))];
  const owner = state.explorations.find((e) => e.turns.some((t) => t.guideId === guide.id));
  const previous = owner?.turns.find((t) => t.guideId === guide.id)?.guide;
  if (previous && previous.revision > guide.revision) return state;
  const status: Turn['status'] = guide.status === 'planning' || guide.status === 'building' ? 'working' : guide.status;
  const { answers: _, ...metadata } = guide;
  const merge = (turn: Turn): Turn => mergePublishedSections({ ...turn, guideId: guide.id, guide: metadata,
    runIds, status, error: undefined, notes: [] }, guide.answers.flatMap((answer) => answer.sections));
  return { ...state, explorations: state.explorations.map((e) => owner ? e.id !== owner.id ? e : {
    ...e, turns: e.turns.map((t) => t.guideId === guide.id ? merge(t) : t),
  } : e.id !== 'root' ? e : { ...e, turns: [...e.turns, merge({ id: guide.id, question: guide.question, context: [], status: 'interrupted' })] }) };
}

/** Native lifetime accounting includes an unfinished opening. Older summaries
 * keep their previous unit-only behavior until native state is refreshed. */
export function guideContinuationOperations(guide: Omit<RepositoryGuide, 'answers'> | undefined): number {
  if (!guide) return 0;
  if (Number.isInteger(guide.continuationOperations) && guide.continuationOperations! >= 0 && guide.continuationOperations! <= 7) return guide.continuationOperations!;
  return guide.units.filter((unit) => unit.status !== 'complete' && unit.runIds.length < 2).length;
}

export function canContinueRepositoryGuide(guide: Omit<RepositoryGuide, 'answers'> | undefined): boolean {
  return guide?.canContinue ?? guideContinuationOperations(guide) > 0;
}

/** Canceling a review removes only its undispatched, empty UI card. Native
 * jobs, saved output and branches are never discarded by closing a dialog. */
export function discardUnstartedGuide(state: WorkspaceState, turnId: string): WorkspaceState {
  const turn = state.explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (!turn?.guideId || turn.status !== 'queued' || turn.guide || turn.runIds?.length
    || turn.publishedSections?.length || state.explorations.some((e) => e.originTurnId === turnId)) return state;
  return { ...state, explorations: state.explorations.map((e) => ({ ...e, turns: e.turns.filter((t) => t.id !== turnId) })) };
}

/** An empty contextual action keeps attached/nested intent. Only the root's
 * empty action starts a repository-wide guide. */
export function contextualGuideIntent(query: string, context: readonly Context[], nested: boolean): string | null {
  if (query.trim()) return query;
  if (context.length) return 'Guide me through this selected code, its decisions and consequences.';
  if (nested) return 'Guide me through this topic, its decisions and consequences.';
  return null;
}

/** Keep partial-answer scope visible after navigation and restart. */
export function cleanAnswerNotes(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [...new Set(value.filter((note): note is string => typeof note === 'string'
    && note.trim().length > 0 && note.length <= 1_000 && !/[\u0000-\u001f\u007f]/.test(note))
    .map((note) => note.trim()))].slice(0, 8);
}

const validRunId = (id: unknown): id is string => typeof id === 'string' && /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(id);
const validGenerationId = (id: unknown): id is string => typeof id === 'string' && id.startsWith('generation:') && validRunId(id.slice('generation:'.length));
export function cleanPublishedSections(value: unknown): PublishedSection[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  return value.filter((s): s is PublishedSection => {
    if (!s || typeof s !== 'object' || !validRunId(s.runId) || !validGenerationId(s.generationId)
      || typeof s.revisionId !== 'string' || !s.revisionId || s.revisionId.length > 256
      || !/^[a-z0-9-]{1,80}$/.test(s.key) || !Number.isInteger(s.order) || s.order < 0 || s.order > 5
      || !(s.supersedes === null || validGenerationId(s.supersedes))
      || !Number.isFinite(s.contextElapsedMs) || s.contextElapsedMs < 0
      || !(s.runElapsedMs === null || Number.isFinite(s.runElapsedMs) && s.runElapsedMs >= 0)) return false;
    const id = `${s.runId}:${s.generationId}`;
    if (seen.has(id)) return false;
    seen.add(id); return true;
  }).slice(0, 256).map((s) => ({ runId:s.runId,key:s.key,order:s.order,generationId:s.generationId,revisionId:s.revisionId,
    supersedes:s.supersedes,contextElapsedMs:s.contextElapsedMs,runElapsedMs:s.runElapsedMs }));
}

/** Append immutable cards; a later section never replaces a reading position. */
export function mergePublishedSections(turn: Turn, incoming: PublishedSection[]): Turn {
  const allowed = incoming.filter((s) => turn.runIds?.includes(s.runId));
  const sections = cleanPublishedSections([...(turn.publishedSections ?? []), ...allowed]);
  return { ...turn, publishedSections: sections, revisionId: turn.revisionId ?? sections[0]?.revisionId };
}

/** Start Flow with this answer's diagram, preserving any explicit selection. */
export function chooseFlowRevision<T extends { revisionId: string }>(revisions: readonly T[], savedRevisionId: string | undefined, hasDiagram: (revision: T) => boolean): T | undefined {
  return revisions.find((revision) => revision.revisionId === savedRevisionId)
    ?? revisions.find(hasDiagram) ?? revisions[0];
}

function cleanReadingPosition(value: unknown): ReadingPosition | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const p = value as ReadingPosition;
  const valid = (id: unknown): id is string => typeof id === 'string' && id.length > 0 && id.length <= 256 && !/[\u0000-\u001f\u007f]/.test(id);
  return valid(p.revisionId) && valid(p.sectionId) ? { revisionId: p.revisionId, sectionId: p.sectionId } : undefined;
}
const validScroll = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= 10_000_000;
function cleanScrollPositions(value: unknown): Partial<Record<View, number>> {
  const result: Partial<Record<View, number>> = {};
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    for (const view of ['guide', 'flow', 'history'] as const) {
      const top = (value as Partial<Record<View, number>>)[view];
      if (validScroll(top)) result[view] = top;
    }
  }
  return result;
}
/** Stable section IDs survive sorting; positions never carry across revisions. */
export function sectionPosition(revisionId: string, sections: readonly { sectionId: string }[], saved?: ReadingPosition): number {
  if (saved?.revisionId !== revisionId) return 0;
  return Math.max(0, sections.findIndex((section) => section.sectionId === saved.sectionId));
}
export function rememberSection(state: WorkspaceState, explorationId: string, turnId: string, view: 'guide' | 'flow', position: ReadingPosition): WorkspaceState {
  if (!['guide', 'flow'].includes(view)) return state;
  const clean = cleanReadingPosition(position);
  const owner = state.explorations.find((e) => e.id === explorationId);
  const turn = owner?.turns.find((t) => t.id === turnId);
  if (!clean || !turn || (turn.revisionId !== clean.revisionId && !turn.publishedSections?.some((s) => s.revisionId === clean.revisionId)) || (turn.reading?.[view]?.revisionId === clean.revisionId && turn.reading[view]?.sectionId === clean.sectionId)) return state;
  return { ...state, explorations: state.explorations.map((e) => e.id !== explorationId ? e : { ...e, turns: e.turns.map((t) => t.id !== turnId ? t : { ...t, reading: { ...t.reading, [view]: clean } }) }) };
}
export function rememberScroll(state: WorkspaceState, explorationId: string, view: View, top: number): WorkspaceState {
  if (!validScroll(top) || !['guide', 'flow', 'history'].includes(view)) return state;
  const owner = state.explorations.find((e) => e.id === explorationId);
  if (!owner || owner.scrollPositions?.[view] === top) return state;
  return { ...state, explorations: state.explorations.map((e) => e.id !== explorationId ? e : { ...e, scrollPositions: { ...e.scrollPositions, [view]: top } }) };
}
export function rankSourcePaths(paths: string[]): string[] {
  const score = (p: string) => (/(^|\/)(main|lib|index|app|server)\.(rs|tsx?|jsx?|py|go|swift)$/i.test(p) ? 100 : 0)
    + (/^(src|app|lib|Sources)\//.test(p) ? 40 : 0)
    - (/(^|\/)(\.[^/]+|tests?|docs?|examples?|skills|agents|deprecated)(\/|$)/.test(p) ? 90 : 0)
    - (/\.(json|ya?ml|toml|md|txt|lock)$/.test(p) ? 50 : 0);
  return [...paths].sort((a,b) => score(b) - score(a) || a.localeCompare(b));
}
/** Only an explicit branch action creates an exploration; ordinary messages stay here. */
export function addTurn(state: WorkspaceState, turn: Turn, view: View, branch?: { originTurnId: string; id: string }): WorkspaceState {
  let currentId = state.currentId; let explorations = state.explorations;
  if (branch) {
    if (explorations.some((e)=>e.id===branch.id) || !explorations.find((e)=>e.id===currentId)?.turns.some((t)=>t.id===branch.originTurnId)) return state;
    currentId = branch.id;
    explorations = [...explorations, {id:currentId,parentId:state.currentId,originTurnId:branch.originTurnId,title:turn.question,view,turns:[],file:turn.context[0]?.path??state.file??undefined,reveal:turn.context[0]}];
  }
  return {...state,currentId,file:branch?turn.context[0]?.path??state.file:state.file,explorations:explorations.map((e)=>e.id===currentId?{...e,view,focusedTurnId:turn.id,turns:[...e.turns,turn]}:e)};
}

/** Keep recent intent with a follow-up; code evidence is selected and validated separately. */
export function learningQuestion(state: WorkspaceState, explorationId: string, turnId: string, limit: number): string {
  const owner = state.explorations.find((e) => e.id === explorationId);
  const position = owner?.turns.findIndex((t) => t.id === turnId) ?? -1;
  const turn = owner?.turns[position];
  if (!turn) return '';
  const earlier = [...new Set([
    ...ancestors(state, explorationId).slice(1).map((e) => e.title),
    ...owner!.turns.slice(Math.max(0, position - 2), position).map((t) => t.question),
  ])].filter((question) => question !== turn.question);
  return `${earlier.length ? `Earlier questions: ${earlier.join(' → ')}. ` : ''}Current question: ${turn.question}`.replace(/\s+/g, ' ').trim().slice(-limit).trim();
}

/** Route an AI-selected single code file through the file explanation contract. */
export function chapterTarget(context: Context[], selectedPaths: string[]): string | null {
  const isCode = (path: string) => !/\.(md|mdx|rst|txt|adoc)$/i.test(path);
  const explicit = context.find((item) => selectedPaths.includes(item.path) && isCode(item.path));
  return explicit?.path ?? (selectedPaths.length === 1 && isCode(selectedPaths[0]) ? selectedPaths[0] : null);
}
