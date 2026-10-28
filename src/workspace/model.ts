import { cleanReaderProgress, cleanStoryProgress, cleanDiagramProgress, sameSection, withReaderProgress, type StoryProgress, type DiagramProgress, type ReaderProgress } from '../lib/readerProgress.ts';
import { cleanAnswerVersions, ownsAnswerRevision, type AnswerVersion } from './answerVersions.ts';
import type { ChapterRevisionLifecycle } from '../lib/commands.ts';
import { isDocumentationPath } from '../lib/inventory.ts';

// Match workspace_conversation::MAX_BYTES before either parsing or persisting.
const MAX_WORKSPACE_STATE_BYTES = 8 * 1024 * 1024;

function exceedsUtf8ByteLimit(value: string, limit: number): boolean {
  if (value.length > limit) return true;
  let bytes = 0;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code <= 0x7f) bytes += 1;
    else if (code <= 0x7ff) bytes += 2;
    else if (code >= 0xd800 && code <= 0xdbff && index + 1 < value.length) {
      const next = value.charCodeAt(index + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        bytes += 4;
        index += 1;
      } else bytes += 3;
    } else bytes += 3;
    if (bytes > limit) return true;
  }
  return false;
}

/** Flat, snapshot-scoped conversation state. Generated text lives in native revisions. */
export type View = 'guide' | 'flow' | 'history';
export interface Context { path: string; sha256?: string; from?: number; to?: number }
export interface ComposerDraft { question: string; context: Context[] }
export interface ReadingPosition { revisionId: string; sectionId: string }
export interface PublishedSection {
  runId: string; key: string; order: number; generationId: string; revisionId: string;
  supersedes: string | null; contextElapsedMs: number; runElapsedMs: number | null;
}
export type ExplanationFailureCode = 'stopped' | 'invalidToolArguments' | 'toolEvent' | 'contextTimeout'
  | 'contextPeerClosed' | 'contextTransport' | 'contextDelivery' | 'runTimeout' | 'runtimeIdentity'
  | 'finalManifestTooLarge' | 'invalidFinalManifest' | 'unsupportedManifestVersion'
  | 'noPublishedSections' | 'invalidIncompleteTopics' | 'unknown';
export interface AnswerSummary {
  runId: string; projectId: string; snapshotId: string; question: string;
  outcome: 'working' | 'complete' | 'partial' | 'stopped' | 'failed' | 'interrupted';
  failureCode?: ExplanationFailureCode;
  sections: PublishedSection[]; incompleteTopics: string[];
}
export interface RepositoryGuide {
  revision: number;
  id: string; projectId: string; snapshotId: string; question: string; title: string;
  status: 'planning' | 'building' | 'waitingForPlayback' | 'ready' | 'partial' | 'stopped' | 'interrupted' | 'failed';
  openingRunId: string | null;
  openingRunIds: string[];
  canContinue: boolean;
  canExtend: boolean;
  continuationOperations: number;
  consumedSlideRevisions?: string[];
  units: { key: string; question: string; prerequisites: string[]; status: string; canRetry: boolean; runIds: string[] }[];
  answers: AnswerSummary[]; omissions: string[]; capturedFiles: number; startingFiles: number;
}
export interface RepositoryGuideList { guides: RepositoryGuide[]; unavailableCount: number }
export interface Turn {
  id: string; question: string; context: Context[];
  status: 'queued' | 'working' | 'waiting' | 'ready' | 'partial' | 'failed' | 'stopped' | 'interrupted';
  revisionId?: string; generationId?: string; error?: string;
  failureCode?: ExplanationFailureCode;
  runIds?: string[]; publishedSections?: PublishedSection[];
  notes?: string[];
  sourceDiscoveryNotes?: string[];
  guideId?: string;
  guide?: Omit<RepositoryGuide, 'answers'>;
  reading?: Partial<Record<'guide' | 'flow', ReadingPosition>>;
  presentation?: ReaderProgress;
  answerVersions?: AnswerVersion[];
  outcomeAt?: number;
  outcomeSequence?: number;
  outcomeNotice?: { read?: string; dismissed?: string };
}
export interface Exploration { id: string; parentId: string | null; originTurnId: string | null; title: string; view: View; turns: Turn[]; focusedTurnId?: string; revealTurnId?: string; file?: string; reveal?: Context; codeOpen?: boolean; fileFlow?: {path:string;showAll:boolean}; scrollPositions?: Partial<Record<View, number>>; draft?: ComposerDraft }
export interface WorkspaceState { version: 1; projectId: string; snapshotId: string; currentId: string; file: string | null; explorations: Exploration[]; grantId?: string }

/** Distinguish questions at each level from child exploration branches. */
export function explorationSummaryCounts(explorations: readonly Exploration[]): Map<string, { questions: number; children: number }> {
  const counts = new Map(explorations.map((item) => [item.id, { questions: item.turns.length, children: 0 }]));
  for (const item of explorations) {
    if (!item.parentId) continue;
    const parent = counts.get(item.parentId);
    if (parent) parent.children += 1;
  }
  return counts;
}

/** Keep publication bookkeeping out of the reader; only a rejected result needs a visible warning. */
export function workspaceAnswerLifecycleBadge(lifecycle: ChapterRevisionLifecycle): 'Rejected' | null {
  return lifecycle === 'rejected' ? 'Rejected' : null;
}

/** History describes an available result only when its exact revision loaded.
 * A stopped question and an unavailable revision are still useful history. */
export function historyEntry(
  turn: Turn,
  availableRevisions: ReadonlySet<string>,
  historyLoading = false,
): { text: string; action: string } {
  const answerRevisionIds = [turn.revisionId,
    ...(turn.publishedSections ?? []).map((section) => section.revisionId),
    ...(turn.answerVersions ?? []).map((version) => version.revisionId)];
  const saved = answerRevisionIds.some((revisionId) => revisionId && availableRevisions.has(revisionId));
  if (!saved && historyLoading && (answerRevisionIds.some(Boolean) || turn.status === 'ready')) {
    return { text: 'Checking saved explanation…', action: 'Open question' };
  }
  if (saved) {
    const guideCoverage = turn.guideId ? repositoryGuideCoverageLabel(turn.guide, saved) : null;
    const state = turn.status === 'queued' ? 'Waiting for the current question to finish.'
      : turn.status === 'working' && turn.guideId
        ? turn.publishedSections?.length ? 'Building the next slide as you read or listen.' : 'Preparing the first slides.'
      : turn.status === 'working' ? 'Adding more detail.'
      : turn.status === 'waiting' ? 'Building the next slide as you read or listen.'
      : guideCoverage === 'Partial coverage' ? 'Guide partly covered.'
      : turn.status === 'partial' ? 'Partially completed.'
      : turn.status === 'stopped' ? 'Stopped.'
      : turn.status === 'interrupted' ? 'Interrupted.'
      : turn.status === 'failed' ? 'Could not finish.'
      : 'Saved explanation.';
    const readyNote = guideCoverage === 'Partial coverage'
      ? ' Some planned topics remain uncovered.'
      : turn.status === 'ready' ? '' : ' Saved sections are available.';
    return { text: `${state}${readyNote}`, action: 'Open explanation' };
  }
  const text: Record<Turn['status'], string> = {
    queued: 'Waiting to start…', working: turn.guideId
      ? (turn.publishedSections?.length ? 'Preparing the next guide slide…' : 'Preparing the first guide slides…')
      : 'Working on this question…',
    waiting: 'More slides will be ready as you read or listen.',
    partial: 'This question is incomplete. No saved section is available.',
    failed: 'This question could not be completed.',
    stopped: 'Stopped before an explanation was saved.',
    interrupted: 'Interrupted before an explanation was saved.',
    ready: 'The saved explanation is unavailable.',
  };
  return { text: text[turn.status], action: 'Open question' };
}

export function historySearchResultSummary(questionGroupCount: number, savedExplanationCount: number): string | null {
  const matches: string[] = [];
  if (questionGroupCount > 0) {
    matches.push(`${questionGroupCount} matching question${questionGroupCount === 1 ? '' : 's'}`);
  }
  if (savedExplanationCount > 0) {
    matches.push(`${savedExplanationCount} saved explanation${savedExplanationCount === 1 ? '' : 's'}`);
  }
  return matches.length > 0 ? matches.join(' · ') : null;
}

/** Keep queued questions visibly distinct from the one active model turn. */
export function questionProgressText(turn: Pick<Turn, 'status' | 'guideId' | 'guide' | 'publishedSections'>): string | null {
  if (turn.status === 'queued') return 'Waiting for the current question to finish…';
  if (turn.status !== 'working') return null;
  if (turn.guideId) {
    if (!turn.guide || turn.guide.status === 'planning') return 'Your guide is coming together…';
    const ready = turn.publishedSections?.length ?? 0;
    return ready ? `${ready} slide${ready === 1 ? '' : 's'} ready · more on the way` : 'Your first slide is taking shape…';
  }
  const ready = turn.publishedSections?.length ?? 0;
  return ready ? `${ready} section${ready === 1 ? '' : 's'} ready · more on the way` : 'Your first section is taking shape…';
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
  return decodeWorkspace(raw,projectId,snapshotId) ?? createWorkspace(projectId,snapshotId);
}
/** Keep unsent intent and its selected source scope local to one exploration. */
export function cleanComposerDraft(value: unknown): ComposerDraft | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const draft=value as {question?:unknown;context?:unknown};
  if (typeof draft.question!=='string' || !Array.isArray(draft.context)) return undefined;
  const question=draft.question.slice(0,1000);
  const context:Context[]=[];
  const seen=new Set<string>();
  for (const value of draft.context.slice(0,32)) {
    if (!value || typeof value!=='object' || Array.isArray(value)) continue;
    const candidate=value as Context;
    const path=candidate.path;
    const validPath=typeof path==='string'&&path.length>0&&!path.startsWith('/')&&!path.includes('\\')
      &&!path.split('/').some(part=>!part||part==='.'||part==='..')&&!/[\u0000-\u001f\u007f]/.test(path)
      &&new TextEncoder().encode(path).length<=4096;
    const hasLines=candidate.from!==undefined||candidate.to!==undefined;
    const validLines=!hasLines||(Number.isSafeInteger(candidate.from)&&Number.isSafeInteger(candidate.to)
      &&(candidate.from as number)>=1&&(candidate.to as number)>=(candidate.from as number));
    const validHash=candidate.sha256===undefined||typeof candidate.sha256==='string'&&/^[a-f0-9]{64}$/.test(candidate.sha256);
    if (!validPath||!validLines||!validHash) continue;
    const identity=`${path}\u0000${candidate.from??''}\u0000${candidate.to??''}`;
    if (seen.has(identity)) continue;
    seen.add(identity);
    context.push({path,...(candidate.sha256===undefined?{}:{sha256:candidate.sha256}),
      ...(candidate.from===undefined?{}:{from:candidate.from}),...(candidate.to===undefined?{}:{to:candidate.to})});
  }
  return question||context.length?{question,context}:undefined;
}
const WORKSPACE_STATE_KEYS = ['version','projectId','snapshotId','currentId','file','explorations','grantId'] as const;
const EXPLORATION_KEYS = ['id','parentId','originTurnId','title','view','turns','focusedTurnId','revealTurnId','file','reveal','codeOpen','fileFlow','scrollPositions','draft'] as const;
const TURN_KEYS = ['id','question','context','status','revisionId','generationId','error','failureCode','runIds','publishedSections','notes','sourceDiscoveryNotes','guideId','reading','presentation','answerVersions','outcomeAt','outcomeSequence','outcomeNotice'] as const;
function hasOnlyKeys(value: unknown, allowed: readonly string[]): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value)
    && Object.keys(value).every((key) => allowed.includes(key));
}

function isWorkspaceContext(value: unknown): value is Context {
  if (!hasOnlyKeys(value, ['path','sha256','from','to'])) return false;
  const path = value.path;
  const validPath = typeof path === 'string' && path.length > 0 && !path.startsWith('/') && !path.includes('\\')
    && !path.split('/').some((part) => !part || part === '.' || part === '..')
    && !/[\u0000-\u001f\u007f]/u.test(path) && new TextEncoder().encode(path).byteLength <= 4096;
  const hasFrom = Object.hasOwn(value, 'from'), hasTo = Object.hasOwn(value, 'to');
  const validLines = !hasFrom && !hasTo || hasFrom && hasTo
    && Number.isSafeInteger(value.from) && Number.isSafeInteger(value.to)
    && Number(value.from) >= 1 && Number(value.to) >= Number(value.from);
  const validHash = !Object.hasOwn(value, 'sha256') || typeof value.sha256 === 'string' && /^[a-f0-9]{64}$/u.test(value.sha256);
  return validPath && validLines && validHash;
}
const validRecordIdentifier = (value: unknown): value is string =>
  typeof value === 'string' && /^[A-Za-z0-9_.:-]{1,256}$/u.test(value);
const validRecordText = (value: unknown, maxUtf8Bytes: number): value is string =>
  typeof value === 'string' && new TextEncoder().encode(value).byteLength <= maxUtf8Bytes
    && !/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/u.test(value);
function cleanOwnedReaderProgress(value: unknown, ownsRevision: (revisionId: string) => boolean): ReaderProgress {
  const clean = cleanReaderProgress(value);
  return {
    stories: clean.stories.filter((item) => ownsRevision(item.revisionId)),
    diagrams: clean.diagrams.filter((item) => ownsRevision(item.revisionId)),
  };
}
/** The browser preview starts with an empty tree when its saved state is unavailable. */
export function decodeWorkspace(raw: string | null, projectId: string, snapshotId: string): WorkspaceState | null {
  const fallback = () => null;
  try {
    if (raw !== null && exceedsUtf8ByteLimit(raw, MAX_WORKSPACE_STATE_BYTES)) return fallback();
    const parsed: unknown = JSON.parse(raw ?? 'null');
    if (!hasOnlyKeys(parsed, WORKSPACE_STATE_KEYS)) return fallback();
    const value = parsed as unknown as WorkspaceState;
    if (!value || value.version !== 1 || value.projectId !== projectId || value.snapshotId !== snapshotId || !Array.isArray(value.explorations) || !value.explorations.length || value.explorations.length > 2000) return fallback();
    if (value.file !== null && !isWorkspaceContext({ path: value.file })) value.file = null;
    if (value.grantId !== undefined && !validRunId(value.grantId)) return fallback();
    const ids = new Set(value.explorations.map((e) => e.id));
    if (ids.size !== value.explorations.length || !ids.has('root') || !ids.has(value.currentId)) return fallback();
    const turnOwners = new Map<string,string>();
    for (const e of value.explorations) {
      if (!hasOnlyKeys(e, EXPLORATION_KEYS)) return fallback();
      if (typeof e.id !== 'string' || !validRecordText(e.title, 65536) || !['guide', 'flow', 'history'].includes(e.view) || !Array.isArray(e.turns)
        || (e.id === 'root' ? e.parentId !== null : !e.parentId || !ids.has(e.parentId))) return fallback();
      if ((e.id === 'root' && e.originTurnId !== null)
        || (e.id !== 'root' && (!validRunId(e.id) || !(e.parentId === 'root' || validRunId(e.parentId)) || !validRunId(e.originTurnId)))) return fallback();
      if (e.file !== undefined && !isWorkspaceContext({ path: e.file })) delete e.file;
      if (e.reveal !== undefined && !isWorkspaceContext(e.reveal)) delete e.reveal;
      if (e.scrollPositions !== undefined) {
        const scrollPositions = cleanScrollPositions(e.scrollPositions);
        if (scrollPositions) e.scrollPositions = scrollPositions; else delete e.scrollPositions;
      }
      if (e.fileFlow !== undefined) {
        const flow = e.fileFlow;
        if (!hasOnlyKeys(flow, ['path','showAll']) || typeof flow.path !== 'string' || flow.path.startsWith('/') || flow.path.includes('\\')
          || flow.path.split('/').some((part) => !part || part === '.' || part === '..') || /[\u0000-\u001f\u007f]/.test(flow.path)
          || new TextEncoder().encode(flow.path).length > 4096 || typeof flow.showAll !== 'boolean') delete e.fileFlow;
      }
      if (e.draft !== undefined) {
        const draft = cleanComposerDraft(e.draft);
        if (draft) e.draft = draft; else delete e.draft;
      }
      delete e.revealTurnId; // A reload restores reading, never moves keyboard focus.
      if (e.codeOpen !== undefined && typeof e.codeOpen !== 'boolean') delete e.codeOpen;
      if (e.focusedTurnId !== undefined && !e.turns.some((t) => t.id === e.focusedTurnId)) delete e.focusedTurnId;
      for (const t of e.turns) {
        if (!hasOnlyKeys(t, TURN_KEYS)) return fallback();
        if (!validRunId(t.id) || turnOwners.has(t.id) || !validRecordText(t.question, 65536) || !Array.isArray(t.context) || t.context.length > 32
          || !['queued','working','waiting','ready','partial','failed','stopped','interrupted'].includes(t.status)
          || !t.context.every(isWorkspaceContext)) return fallback();
        turnOwners.set(t.id,e.id);
        if ((t.revisionId !== undefined && !validRecordIdentifier(t.revisionId))
          || (t.generationId !== undefined && !validGenerationId(t.generationId))
          || (t.error !== undefined && !validRecordText(t.error, 4096))) return fallback();
        if (t.reading !== undefined) {
          const reading: Partial<Record<'guide'|'flow',ReadingPosition>> = {};
          if (t.reading && typeof t.reading === 'object' && !Array.isArray(t.reading)) {
            for (const view of ['guide','flow'] as const) {
              const position = cleanReadingPosition(t.reading[view]);
              if (position && ownsAnswerRevision(t, position.revisionId)) reading[view] = position;
            }
          }
          if (Object.keys(reading).length > 0) t.reading = reading; else delete t.reading;
        }
        if (t.outcomeNotice !== undefined && (!hasOnlyKeys(t.outcomeNotice, ['read','dismissed'])
          || Object.keys(t.outcomeNotice).length === 0
          || Object.values(t.outcomeNotice).some((id) => typeof id !== 'string' || !/^[a-f0-9]{16}$/u.test(id)))) delete t.outcomeNotice;
        const savedRunIds: unknown = t.runIds;
        if (savedRunIds !== undefined && (!Array.isArray(savedRunIds) || savedRunIds.length > 32 || savedRunIds.some((id) => !validRunId(id)))) return fallback();
        const runIds = (savedRunIds === undefined ? [] : savedRunIds) as string[];
        if (new Set(runIds).size !== runIds.length
          || (t.guideId !== undefined && !validRunId(t.guideId))
          || (t.failureCode !== undefined && !cleanExplanationFailureCode(t.failureCode))) return fallback();
        const rawSections: unknown = t.publishedSections;
        const sectionValues = rawSections === undefined ? [] : rawSections;
        if (!Array.isArray(sectionValues) || sectionValues.length > 256
          || sectionValues.some((section) => !hasOnlyKeys(section, ['runId','key','order','generationId','revisionId','supersedes','contextElapsedMs','runElapsedMs']))) return fallback();
        const publishedSections = cleanPublishedSections(sectionValues);
        if (publishedSections.length !== sectionValues.length || publishedSections.some((section) => !runIds.includes(section.runId))) return fallback();
        t.runIds ??= runIds;
        const rawNotes: unknown = t.notes;
        const noteValues = rawNotes === undefined ? [] : rawNotes;
        if (!Array.isArray(noteValues) || noteValues.length > 8) return fallback();
        const notes = cleanAnswerNotes(noteValues);
        if (notes.length !== noteValues.length || notes.some((note, index) => note !== noteValues[index])) return fallback();
        const rawDiscoveryNotes: unknown = t.sourceDiscoveryNotes;
        const discoveryNoteValues = rawDiscoveryNotes === undefined ? [] : rawDiscoveryNotes;
        if (!Array.isArray(discoveryNoteValues) || discoveryNoteValues.length > 14) return fallback();
        const discoveryNotes = cleanSourceDiscoveryNotes(discoveryNoteValues);
        if (discoveryNotes.length !== discoveryNoteValues.length || discoveryNotes.some((note, index) => note !== discoveryNoteValues[index])) return fallback();
        if (t.outcomeAt !== undefined && (!Number.isSafeInteger(t.outcomeAt) || Number(t.outcomeAt) < 0)) delete t.outcomeAt;
        if (t.outcomeSequence !== undefined && (!Number.isSafeInteger(t.outcomeSequence) || Number(t.outcomeSequence) < 0)) delete t.outcomeSequence;
        if (t.answerVersions !== undefined) {
          const cleanVersions = cleanAnswerVersions(t.answerVersions, t);
          if (cleanVersions.length > 0) t.answerVersions = cleanVersions; else delete t.answerVersions;
        }
        if (t.presentation !== undefined) {
          const presentation = cleanOwnedReaderProgress(t.presentation, (revisionId) => ownsAnswerRevision(t, revisionId));
          if (presentation.stories.length + presentation.diagrams.length > 0) t.presentation = presentation;
          else delete t.presentation;
        }
        // A reload never silently resubmits inference. Native saved previews remain recoverable.
        if (t.status === 'working' || t.status === 'queued') {
          t.status = 'interrupted';
          t.error ??= 'This question was interrupted. Saved explanations are preserved.';
          t.runIds ??= [];
        }
      }
      if (ancestors(value, e.id)[0]?.id !== 'root') return fallback();
    }
    for (const e of value.explorations) {
      if (e.id !== 'root' && (!e.parentId || !e.originTurnId || turnOwners.get(e.originTurnId) !== e.parentId)) return fallback();
    }
    return value;
  } catch { return fallback(); }
}

/** Keep browser-preview writes within the same byte limit used during restore. */
export function serializeWorkspace(state: WorkspaceState): string | null {
  const serialized = JSON.stringify(state);
  return typeof serialized === 'string' && !exceedsUtf8ByteLimit(serialized, MAX_WORKSPACE_STATE_BYTES)
    ? serialized : null;
}

/** A guide is one conversation card, even across independent native runs and
 * a lost local-storage write. Never change the reader's view or section. */
export function mergeRepositoryGuide(state: WorkspaceState, guide: RepositoryGuide): WorkspaceState {
  if (guide.projectId !== state.projectId || guide.snapshotId !== state.snapshotId || !validRunId(guide.id)) return state;
  const runIds = [...new Set([...guide.openingRunIds, ...guide.units.flatMap((unit) => unit.runIds)].filter(validRunId))];
  const owner = state.explorations.find((e) => e.turns.some((t) => t.guideId === guide.id));
  const previous = owner?.turns.find((t) => t.guideId === guide.id)?.guide;
  if (previous && previous.revision > guide.revision) return state;
  const status: Turn['status'] = guide.status === 'planning' || guide.status === 'building' ? 'working'
    : guide.status === 'waitingForPlayback' ? 'waiting' : guide.status;
  const { answers: _, ...metadata } = guide;
  const incomingSections = guide.answers.flatMap((answer) => answer.sections);
  const merge = (turn: Turn): Turn => {
    const hasPublishedSections = Boolean(turn.publishedSections?.length
      || incomingSections.some((section) => runIds.includes(section.runId)));
    const partialCoverage = repositoryGuideCoverageLabel(metadata, hasPublishedSections) === 'Partial coverage';
    return mergePublishedSections({ ...turn, guideId: guide.id, guide: metadata,
      runIds, status, error: undefined,
      notes: partialCoverage ? ['Guide partly covered. Some planned topics remain uncovered.'] : [] }, incomingSections);
  };
  return { ...state, explorations: state.explorations.map((e) => owner ? e.id !== owner.id ? e : {
    ...e, turns: e.turns.map((t) => t.guideId === guide.id ? merge(t) : t),
  } : e.id !== 'root' ? e : { ...e, turns: [...e.turns, merge({ id: guide.id, question: guide.question, context: [], status: 'interrupted' })] }) };
}

/** Native lifetime accounting includes an unfinished opening. */
export function guideContinuationOperations(guide: Omit<RepositoryGuide, 'answers'> | undefined): number {
  if (!guide) return 0;
  return Number.isInteger(guide.continuationOperations) && guide.continuationOperations >= 0 && guide.continuationOperations <= 7
    ? guide.continuationOperations : 0;
}

export function canContinueRepositoryGuide(guide: Omit<RepositoryGuide, 'answers'> | undefined): boolean {
  return guide?.canContinue === true;
}

/** Make terminal guide coverage visible without labeling an active outline early. */
export function repositoryGuideCoverageLabel(guide: Omit<RepositoryGuide, 'answers'> | undefined, hasPublishedSections = false): 'Planned topics complete' | 'Partial coverage' | 'Incomplete' | null {
  if (!guide || guide.status === 'planning' || guide.status === 'building' || guide.status === 'waitingForPlayback') return null;
  if (guide.status !== 'ready') return hasPublishedSections ? 'Partial coverage' : 'Incomplete';
  if (!guide.units.length) return 'Incomplete';
  return guide.omissions.length > 0 || guide.units.some((unit) => unit.status !== 'complete')
    ? 'Partial coverage' : 'Planned topics complete';
}

/** Return a small, safe display/request list from a guide's omission labels. */
export function repositoryGuideOmissionTopics(omissions: readonly string[]): string[] {
  return [...new Set(omissions
    .filter((topic): topic is string => typeof topic === 'string')
    .map((topic) => topic.replace(/[\r\n\t]+/g, ' ').trim())
    .filter(Boolean))].slice(0, 8).map((topic) => topic.slice(0, 180));
}

/** Include both planner omissions and planned units that never completed. */
export function repositoryGuideUncoveredTopics(guide: Pick<RepositoryGuide, 'omissions' | 'units'> | undefined): string[] {
  if (!guide) return [];
  return [...guide.omissions, ...guide.units
    .filter((unit) => unit.status !== 'complete')
    .map((unit) => unit.question)];
}

/** Keep the visible coverage note concise while naming the major omissions. */
export function repositoryGuideOmissionSummary(omissions: readonly string[]): string | null {
  const topics = repositoryGuideOmissionTopics(omissions);
  if (!topics.length) return null;
  const visible = topics.slice(0, 3).map((topic) => {
    const label = topic.replace(/^(?:not yet covered|not covered|uncovered|omitted)\s*:\s*/i, '').trim();
    return label.length > 84 ? `${label.slice(0, 81).trimEnd()}…` : label;
  }).join(' · ');
  const remaining = topics.length - 3;
  return remaining > 0 ? `${visible} · and ${remaining} more` : visible;
}

/** Canceling a review removes only its undispatched, empty UI card. Native
 * jobs, saved output and branches are never discarded by closing a dialog. */
export function discardUnstartedGuide(state: WorkspaceState, turnId: string): WorkspaceState {
  const turn = state.explorations.flatMap((e) => e.turns).find((t) => t.id === turnId);
  if (!turn?.guideId || turn.status !== 'queued' || turn.guide || turn.runIds?.length
    || turn.publishedSections?.length || state.explorations.some((e) => e.originTurnId === turnId)) return state;
  return { ...state, explorations: state.explorations.map((e) => ({ ...e, turns: e.turns.filter((t) => t.id !== turnId) })) };
}

/** An empty contextual action keeps attached/nested intent; the root starts a
 * repository guide directly so readers do not have to discover a second CTA. */
export function contextualGuideIntent(query: string, context: readonly Context[], nested: boolean): string {
  if (query.trim()) return query;
  if (context.length) return 'Guide me through this selected code, its decisions and consequences.';
  if (nested) return 'Guide me through this topic, its decisions and consequences.';
  return 'How does this project work, from its entry points through its main behavior and failure paths?';
}

export type ContextualGuideAction =
  | { kind: 'repository-guide'; intent: string }
  | { kind: 'ask'; intent: string };

/** Guide me is an exploration action. At the root it starts the durable
 * incremental repository guide; scoped contexts remain topic-sized. */
export function contextualGuideAction(query: string, context: readonly Context[], nested: boolean): ContextualGuideAction {
  // At the repository root, choosing Guide me is an explicit request for the
  // durable outline-and-slides experience. Do not let the wording in the chat
  // box accidentally send that request through the ordinary one-shot path.
  // Selected code and nested explorations remain scoped to the current topic.
  if (context.length === 0 && !nested) {
    return { kind: 'repository-guide', intent: contextualGuideIntent(query, context, nested) };
  }
  return { kind: 'ask', intent: contextualGuideIntent(query, context, nested) };
}

/** Route repository-wide asks through the durable slide planner instead of a
 * one-shot answer. Topic-scoped code questions remain ordinary investigations,
 * with a user-controlled run policy, regardless of the current view. */
export function isRepositoryGuideIntent(question: string): boolean {
  const query = question.trim().toLowerCase();
  if (!query) return false;
  const repository = String.raw`(?:project|repo(?:sitory)?|code\s*base|codebase|application|app|software|system)`;
  // A scope word inside a negation is not evidence that the user asked for a
  // repository-wide guide. This matters for focused prompts such as “one new
  // Guide section, not a repository overview”: the old substring heuristic
  // saw “repository” + “overview” and silently widened the request.
  const negatedBroadScope = new RegExp(
    String.raw`\b(?:not|no|without|rather\s+than|instead\s+of)\s+(?:(?:a|an|the|any)\s+)?(?:(?:whole|entire|full|complete|comprehensive)\s+)?${repository}(?:[\s-]?wide)?\s+(?:guide|tour|overview|walk(?:\s*through)?|architecture|big\s+picture)\b`,
    'gi',
  );
  const affirmativeQuery = query.replace(negatedBroadScope, ' ');
  const hasRepository = new RegExp(String.raw`\b${repository}\b`, 'i').test(affirmativeQuery);
  if (!hasRepository) return false;
  const oneUnit = /\b(?:exactly\s+one|one|single|only|just)\s+(?:(?:new|fresh)\s+)?(?:guide\s+)?(?:section|slide|scene|file|function|method|branch|symbol)\b/i;
  const completeScope = /\b(?:whole|entire|full|all|every|each|complete|comprehensive|end[ -]to[ -]end)\b/i;
  if (oneUnit.test(query) && !completeScope.test(affirmativeQuery)) return false;
  // Treat completeness language as a guide request even when the user does not
  // say "guide". This preserves the outline-and-slide experience for natural
  // requests such as "generate it for the Fontdone project entirely".
  const explicitGuide = new RegExp(String.raw`\b(?:guide|walk(?:\s+me)?\s+through|tour|overview|architecture|big\s+picture|whole|entire(?:ly)?|full|all|every|each|complete|comprehensive|overall|end[ -]to[ -]end)\b`, 'i');
  // Explicitly asking for a project guide always uses the durable, incremental
  // guide pipeline, even from a nested exploration. Otherwise the same broad
  // request could fall back to a single section without the guide outline.
  if (explicitGuide.test(affirmativeQuery)) return true;
  const directProjectAsk = new RegExp(String.raw`^(?:explain|describe|show|understand)\s+(?:(?:this|the)\s+)?${repository}\b`, 'i');
  const projectBehavior = /\bhow\s+(?:does|do)\s+(?:this\s+|the\s+)?(?:project|repo(?:sitory)?|code\s*base|application|system)\s+(?:work|fit\s+together)\b/i;
  const explainProjectBehavior = new RegExp(String.raw`\b(?:explain|describe|show(?:\s+me)?|help\s+me\s+understand)\s+(?:how\s+)?(?:this|the)\s+${repository}\s+(?:works?|fits?\s+together|is\s+(?:structured|organized|built|wired))\b`, 'i');
  const asksProjectPurpose = new RegExp(String.raw`\b(?:what|which)\s+(?:does|do)\s+(?:(?:this|the)\s+)?${repository}\s+(?:do|provide|offer|contain|include|cover)\b`, 'i');
  const explainsProjectPurpose = new RegExp(String.raw`\b(?:explain|describe|show(?:\s+me)?|tell(?:\s+me)?|help\s+me\s+understand)\s+what\s+(?:(?:this|the)\s+)?${repository}\s+(?:does|provides?|offers?|contains?|includes?|covers?)\b`, 'i');
  if (directProjectAsk.test(affirmativeQuery) || projectBehavior.test(affirmativeQuery) || explainProjectBehavior.test(affirmativeQuery)
    || asksProjectPurpose.test(affirmativeQuery) || explainsProjectPurpose.test(affirmativeQuery)) return true;
  return false;
}

/** Keep partial-answer scope visible after navigation and restart. */
export function cleanAnswerNotes(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [...new Set(value.filter((note): note is string => typeof note === 'string'
    && note.trim().length > 0 && note.length <= 1_000 && !/[\u0000-\u001f\u007f-\u009f]/.test(note))
    .map((note) => note.trim()))].slice(0, 8);
}

/** Keep the reviewed source-selection rationale with the answer after restart. */
export function cleanSourceDiscoveryNotes(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [...new Set(value.filter((note): note is string => typeof note === 'string'
    && note.trim().length > 0 && note.length <= 1_500 && !/[\u0000-\u001f\u007f-\u009f]/.test(note))
    .map((note) => note.trim()))].slice(0, 14);
}

const validRunId = (id: unknown): id is string => typeof id === 'string' && /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(id);
const explanationFailureCodes = new Set<ExplanationFailureCode>(['stopped','invalidToolArguments','toolEvent','contextTimeout','contextPeerClosed','contextTransport','contextDelivery','runTimeout','runtimeIdentity','finalManifestTooLarge','invalidFinalManifest','unsupportedManifestVersion','noPublishedSections','invalidIncompleteTopics','unknown']);
export function cleanExplanationFailureCode(value: unknown): ExplanationFailureCode | undefined {
  return typeof value === 'string' && explanationFailureCodes.has(value as ExplanationFailureCode) ? value as ExplanationFailureCode : undefined;
}
const validGenerationId = (id: unknown): id is string => typeof id === 'string' && id.startsWith('generation:') && validRunId(id.slice('generation:'.length));
export function cleanPublishedSections(value: unknown): PublishedSection[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  return value.filter((s): s is PublishedSection => {
    if (!s || typeof s !== 'object' || !validRunId(s.runId) || !validGenerationId(s.generationId)
      || !validRecordIdentifier(s.revisionId)
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
  return validRecordIdentifier(p.revisionId) && validRecordIdentifier(p.sectionId)
    ? { revisionId: p.revisionId, sectionId: p.sectionId } : undefined;
}
const validScroll = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= 10_000_000;
function cleanScrollPositions(value: unknown): Partial<Record<View,number>> | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const source = value as Partial<Record<View,unknown>>;
  const clean: Partial<Record<View,number>> = {};
  for (const view of ['guide','flow','history'] as const) if (validScroll(source[view])) clean[view] = source[view];
  return Object.keys(clean).length > 0 ? clean : undefined;
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
  if (!clean || !turn || !ownsAnswerRevision(turn,clean.revisionId)) return state;

  // Progressive guides publish each slide as a separate revision. Keep that
  // slide's section in the existing bounded reader map; `reading` remains the
  // active slide cursor.
  const perRevisionReading = Boolean(turn.guideId || (turn.publishedSections?.length ?? 0) > 1);
  let answerVersions: AnswerVersion[] | undefined;
  if (turn.answerVersions !== undefined || perRevisionReading) {
    const versions = cleanAnswerVersions(turn.answerVersions, turn).map((version) => ({
      ...version,
      reading: { ...version.reading },
    }));
    let entry = versions.find((version) => version.revisionId === clean.revisionId);
    if (!entry && perRevisionReading && (turn.revisionId === clean.revisionId
      || (turn.publishedSections ?? []).some((section) => section.revisionId === clean.revisionId))) {
      entry = {
        originalRevisionId: clean.revisionId,
        revisionId: clean.revisionId,
        selected: false,
        reading: {},
      };
      versions.push(entry);
    }
    if (entry) entry.reading = { ...entry.reading, [view]: clean.sectionId };
    while (versions.length > 256) {
      const oldestUnselected = versions.findIndex((version) => !version.selected);
      if (oldestUnselected < 0) return state;
      versions.splice(oldestUnselected, 1);
    }
    answerVersions = versions;
  }

  const readingUnchanged = turn.reading?.[view]?.revisionId === clean.revisionId
    && turn.reading?.[view]?.sectionId === clean.sectionId;
  if (readingUnchanged && (answerVersions === undefined || JSON.stringify(answerVersions) === JSON.stringify(turn.answerVersions))) return state;
  return { ...state, explorations: state.explorations.map((e) => e.id !== explorationId ? e : { ...e, turns: e.turns.map((t) => t.id !== turnId ? t : {
    ...t,
    reading: { ...t.reading, [view]: clean },
    ...(answerVersions === undefined ? {} : { answerVersions }),
  }) }) };
}
/** Opening a compact answer card must restore the exact section the reader chose,
 * even when the same question has several published revisions. */
export function focusAnswerSection(state: WorkspaceState, explorationId: string, turnId: string, position: ReadingPosition): WorkspaceState {
  const clean = cleanReadingPosition(position);
  const owner = state.explorations.find((e) => e.id === explorationId);
  const turn = owner?.turns.find((t) => t.id === turnId);
  if (!clean || !owner || !turn || !ownsAnswerRevision(turn, clean.revisionId)) return state;
  const positioned = rememberSection(state, explorationId, turnId, 'guide', clean);
  if (owner.focusedTurnId === turnId && positioned === state) return state;
  const focusChanged = owner.focusedTurnId !== turnId;
  return { ...positioned, explorations: positioned.explorations.map((e) => e.id !== explorationId ? e : {
    ...e,
    focusedTurnId: turnId,
    ...(focusChanged ? { revealTurnId: turnId } : {}),
  }) };
}
export function rememberScroll(state: WorkspaceState, explorationId: string, view: View, top: number): WorkspaceState {
  if (!validScroll(top) || !['guide', 'flow', 'history'].includes(view)) return state;
  const owner = state.explorations.find((e) => e.id === explorationId);
  if (!owner || owner.scrollPositions?.[view] === top) return state;
  return { ...state, explorations: state.explorations.map((e) => e.id !== explorationId ? e : { ...e, scrollPositions: { ...e.scrollPositions, [view]: top } }) };
}
/** Prefer the saved viewport; use the answer anchor only for an explicit reveal or an uninitialized Guide. */
export function shouldRestoreFocusedAnswerAnchor(view: View, revealTurnId?: string, savedGuideScrollTop?: number): boolean {
  return view === 'guide' && (Boolean(revealTurnId) || !validScroll(savedGuideScrollTop));
}
export function rememberCodePane(state:WorkspaceState,explorationId:string,codeOpen:boolean):WorkspaceState {
  if(typeof codeOpen!=='boolean'||!state.explorations.some(e=>e.id===explorationId&&e.codeOpen!==codeOpen))return state;
  return {...state,explorations:state.explorations.map(e=>e.id===explorationId?{...e,codeOpen}:e)};
}
export function rememberComposerDraft(state:WorkspaceState,explorationId:string,draft:ComposerDraft|null):WorkspaceState {
  const owner=state.explorations.find(e=>e.id===explorationId);
  if(!owner)return state;
  const clean=draft?cleanComposerDraft(draft):undefined;
  if(JSON.stringify(owner.draft??null)===JSON.stringify(clean??null))return state;
  return {...state,explorations:state.explorations.map(e=>{
    if(e.id!==explorationId)return e;
    const next={...e};
    if(clean)next.draft=clean;else delete next.draft;
    return next;
  })};
}
/** Keep the active source path and exact selected/cited lines with this exploration. */
export function rememberSourceContext(state:WorkspaceState,explorationId:string,context:Context):WorkspaceState {
  const path=context?.path;
  const validPath=typeof path==='string'&&path.length>0&&!path.startsWith('/')&&!path.includes('\\')
    &&!path.split('/').some(part=>!part||part==='.'||part==='..')&&!/[\u0000-\u001f\u007f]/.test(path)
    &&new TextEncoder().encode(path).length<=4096;
  const hasLines=context?.from!==undefined||context?.to!==undefined;
  const validLines=!hasLines||(Number.isSafeInteger(context.from)&&Number.isSafeInteger(context.to)
    &&(context.from as number)>=1&&(context.to as number)>=(context.from as number));
  const validHash=context?.sha256===undefined||typeof context.sha256==='string'&&/^[a-f0-9]{64}$/.test(context.sha256);
  const owner=state.explorations.find(e=>e.id===explorationId);
  if(!owner||!validPath||!validLines||!validHash)return state;
  const reveal={path,...(context.sha256===undefined?{}:{sha256:context.sha256}),
    ...(context.from===undefined?{}:{from:context.from}),...(context.to===undefined?{}:{to:context.to})};
  if(state.file===path&&owner.file===path&&JSON.stringify(owner.reveal)===JSON.stringify(reveal))return state;
  return {...state,file:path,explorations:state.explorations.map(e=>e.id===explorationId?{...e,file:path,reveal}:e)};
}
export function rememberFileFlow(state:WorkspaceState,explorationId:string,path:string,showAll:boolean):WorkspaceState {
  const owner=state.explorations.find(e=>e.id===explorationId);
  if(!owner||typeof showAll!=='boolean'||typeof path!=='string'||!path||path.startsWith('/')||path.includes('\\')||path.split('/').some(p=>!p||p==='.'||p==='..')||/[\u0000-\u001f\u007f]/.test(path)||new TextEncoder().encode(path).length>4096||(owner.fileFlow?.path===path&&owner.fileFlow.showAll===showAll))return state;
  return {...state,explorations:state.explorations.map(e=>e.id===explorationId?{...e,fileFlow:{path,showAll}}:e)};
}
export function rememberPresentation(state:WorkspaceState,explorationId:string,turnId:string,kind:'stories'|'diagrams',value:StoryProgress|DiagramProgress):WorkspaceState {
  if(kind!=='stories'&&kind!=='diagrams')return state;
  const next=kind==='stories'?cleanStoryProgress(value):cleanDiagramProgress(value);
  const turn=state.explorations.find(e=>e.id===explorationId)?.turns.find(t=>t.id===turnId);
  if(!next||!turn||!ownsAnswerRevision(turn,next.revisionId))return state;
  const previous=cleanReaderProgress(turn.presentation);
  if(JSON.stringify(previous[kind].find(p=>sameSection(p,next)))===JSON.stringify(next))return state;
  const presentation:ReaderProgress=kind==='stories'?{...previous,stories:withReaderProgress(previous.stories,next as StoryProgress)}:{...previous,diagrams:withReaderProgress(previous.diagrams,next as DiagramProgress)};
  return {...state,explorations:state.explorations.map(e=>e.id!==explorationId?e:{...e,turns:e.turns.map(t=>t.id!==turnId?t:{...t,presentation})})};
}
/** A player opened from a card belongs to that focused question. Reopening an
 * unrelated saved chapter cannot write into another question's metadata. */
export function presentationOwner(exploration:Exploration,revisionId:string):Turn|undefined {
  const owns=(t:Turn)=>ownsAnswerRevision(t,revisionId);
  return exploration.turns.find(t=>t.id===exploration.focusedTurnId&&owns(t))??exploration.turns.slice().reverse().find(owns);
}
const commonEntryPointNames = new Set([
  'main', 'lib', 'index', 'app', 'server', 'program', '__main__', 'entrypoint', 'cli', 'run',
]);
const commonEntryPointExtensions = new Set([
  'rs', 'ts', 'tsx', 'mts', 'cts', 'js', 'jsx', 'mjs', 'cjs', 'py', 'pyw', 'pyi', 'go', 'swift',
  'c', 'h', 'cc', 'cpp', 'cxx', 'c++', 'hpp', 'hh', 'hxx', 'h++', 'ipp', 'inl', 'ixx', 'cppm',
  'ccm', 'cxxm', 'cs', 'csx', 'java', 'kt', 'kts', 'm', 'mm', 'rb', 'php', 'scala', 'sc', 'ex',
  'exs', 'erl', 'hrl', 'hs', 'lhs', 'ml', 'mli', 'fs', 'fsx', 'clj', 'cljs', 'cljc', 'lua', 'zig',
  'dart', 'vue', 'svelte', 'nim', 'd', 'f', 'f90', 'f95', 'f03', 'cob', 'cbl', 'jl', 'pl', 'pm', 'r',
  'sol', 'purs', 'res', 'resi', 'coffee', 'odin', 'gleam', 'elm', 'sh', 'bash', 'zsh', 'ps1', 'psm1',
  'psd1', 'bat', 'cmd',
]);

export function rankSourcePaths(paths: string[]): string[] {
  const score = (path: string) => {
    const p = path.toLowerCase();
    const fileName = p.split('/').at(-1) ?? '';
    const extensionStart = fileName.lastIndexOf('.');
    const name = extensionStart > 0 ? fileName.slice(0, extensionStart) : '';
    const extension = extensionStart > 0 ? fileName.slice(extensionStart + 1) : '';
    return (commonEntryPointNames.has(name) && commonEntryPointExtensions.has(extension) ? 100 : 0)
      + (/^(src|app|lib|sources|cmd|bin)\//.test(p) ? 40 : 0)
      + (/(^|\/)src\/bin\/[^/]+\.rs$/.test(p) ? 60 : 0)
      - (/(^|\/)(\.[^/]+|tests?|docs?|examples?|skills|agents|deprecated)(\/|$)/.test(p) ? 90 : 0)
      - (/\.(json|ya?ml|toml|md|txt|lock)$/.test(p) ? 50 : 0);
  };
  return paths.map((path) => ({ path, score: score(path) }))
    .sort((a, b) => b.score - a.score || (a.path < b.path ? -1 : a.path > b.path ? 1 : 0))
    .map(({ path }) => path);
}
/** Only an explicit branch action creates an exploration; ordinary messages stay here. */
export function addTurn(state: WorkspaceState, turn: Turn, view: View, branch?: { originTurnId: string; id: string }): WorkspaceState {
  let currentId = state.currentId; let explorations = state.explorations;
  if (branch) {
    if (explorations.some((e)=>e.id===branch.id) || !explorations.find((e)=>e.id===currentId)?.turns.some((t)=>t.id===branch.originTurnId)) return state;
    explorations = explorations.map((e)=>e.id===currentId?{...e,draft:undefined}:e);
    currentId = branch.id;
    explorations = [...explorations, {id:currentId,parentId:state.currentId,originTurnId:branch.originTurnId,title:turn.question,view,turns:[],file:turn.context[0]?.path??state.file??undefined,reveal:turn.context[0]}];
  }
  return {...state,currentId,file:branch?turn.context[0]?.path??state.file:state.file,explorations:explorations.map((e)=>e.id===currentId?{...e,draft:undefined,view,focusedTurnId:turn.id,turns:[...e.turns,turn]}:e)};
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
  const isCode = (path: string) => !isDocumentationPath(path);
  const explicit = context.find((item) => selectedPaths.includes(item.path) && isCode(item.path));
  return explicit?.path ?? (selectedPaths.length === 1 && isCode(selectedPaths[0]) ? selectedPaths[0] : null);
}
