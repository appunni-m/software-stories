/** Flat, snapshot-scoped conversation state. Generated text lives in native revisions. */
export type View = 'guide' | 'flow' | 'history';
export interface Context { path: string; sha256?: string; from?: number; to?: number }
export interface ReadingPosition { revisionId: string; sectionId: string }
export interface Turn {
  id: string; question: string; context: Context[];
  status: 'queued' | 'working' | 'ready' | 'failed' | 'stopped' | 'interrupted';
  revisionId?: string; generationId?: string; error?: string;
  reading?: Partial<Record<'guide' | 'flow', ReadingPosition>>;
}
export interface Exploration { id: string; parentId: string | null; originTurnId: string | null; title: string; view: View; turns: Turn[]; file?: string; reveal?: Context; scrollTop?: number; scrollPositions?: Partial<Record<View, number>> }
export interface WorkspaceState { version: 1; projectId: string; snapshotId: string; currentId: string; file: string | null; explorations: Exploration[]; grantId?: string }
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
      for (const t of e.turns) {
        if (typeof t.id !== 'string' || typeof t.question !== 'string' || !Array.isArray(t.context)
          || !['queued','working','ready','failed','stopped','interrupted'].includes(t.status)
          || t.context.some((c) => typeof c.path !== 'string' || c.path.startsWith('/') || c.path.split('/').includes('..'))) return fallback();
        const reading = t.reading;
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
  if (!clean || !turn || turn.revisionId !== clean.revisionId || (turn.reading?.[view]?.revisionId === clean.revisionId && turn.reading[view]?.sectionId === clean.sectionId)) return state;
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
  return {...state,currentId,file:branch?turn.context[0]?.path??state.file:state.file,explorations:explorations.map((e)=>e.id===currentId?{...e,view,turns:[...e.turns,turn]}:e)};
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
