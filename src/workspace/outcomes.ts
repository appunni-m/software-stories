import type { Exploration, PublishedSection, Turn, WorkspaceState } from './model';

const active = (turn: Turn) => turn.status === 'queued' || turn.status === 'working';
const hasAnswer = (turn: Turn) => Boolean(turn.revisionId || turn.publishedSections?.length);

/** A missing saved revision is unavailable only after the snapshot history read finishes. */
export function savedAnswerAvailability(historyLoading: boolean, revisionAvailable: boolean): 'loading' | 'available' | 'unavailable' {
  if (historyLoading) return 'loading';
  return revisionAvailable ? 'available' : 'unavailable';
}

/** Empty past attempts stay in History. Preserve the selected question and
 * branch owners, including branches whose original investigation failed. */
export function visibleConversationTurns(state: WorkspaceState, exploration: Exploration): Turn[] {
  const branchOwners = new Set(state.explorations.filter((e) => e.parentId === exploration.id).map((e) => e.originTurnId));
  return exploration.turns.filter((turn) => active(turn) || hasAnswer(turn)
    || turn.id === exploration.focusedTurnId || branchOwners.has(turn.id));
}

export type PublishedAnswerItem =
  | { kind: 'section'; section: PublishedSection }
  | { kind: 'unavailable'; sections: PublishedSection[] };

/** Collapse adjacent missing parts from one question without changing the
 * order of available parts. While the saved chapter list is loading, retain
 * the per-part loading state instead of guessing that references are missing. */
export function groupUnavailablePublishedSections(
  sections: readonly PublishedSection[],
  availableRevisionIds: ReadonlySet<string>,
  historyLoading: boolean,
): PublishedAnswerItem[] {
  const items: PublishedAnswerItem[] = [];
  for (const section of sections) {
    if (historyLoading || availableRevisionIds.has(section.revisionId)) {
      items.push({ kind: 'section', section });
      continue;
    }
    const previous = items.at(-1);
    if (previous?.kind === 'unavailable') previous.sections.push(section);
    else items.push({ kind: 'unavailable', sections: [section] });
  }
  return items;
}

/** A bounded UI dedupe token, not a source/evidence identity. Include every
 * publication so a revised earlier section also produces a new outcome. */
export function questionOutcomeKey(turn: Turn): string | null {
  if (active(turn)) return null;
  const value = JSON.stringify([turn.status, turn.outcomeSequence ?? 0, turn.runIds ?? [], turn.revisionId ?? null,
    turn.publishedSections?.map((s) => [s.runId, s.generationId, s.revisionId]) ?? [], turn.error ?? null, turn.notes ?? []]);
  let hash = 0xcbf29ce484222325n;
  for (let i = 0; i < value.length; i++) hash = BigInt.asUintN(64, (hash ^ BigInt(value.charCodeAt(i))) * 0x100000001b3n);
  return hash.toString(16).padStart(16, '0');
}

export interface QuestionOutcome {
  explorationId: string; turnId: string; key: string; question: string;
  answerRevisionIds: string[];
  title: string; read: boolean; tone: 'info' | 'warning' | 'error';
}

/** The latest 50 terminal questions per snapshot. Older questions and their
 * receipts remain in History; dismissing an outcome never deletes work. */
export function questionOutcomes(state: WorkspaceState): QuestionOutcome[] {
  return state.explorations.flatMap((e) => e.turns.map((turn) => ({ explorationId: e.id, turn })))
    .map((entry, order) => ({ ...entry, order, key: questionOutcomeKey(entry.turn) }))
    .filter((entry) => entry.key && entry.turn.outcomeNotice?.dismissed !== entry.key)
    .sort((a, b) => (b.turn.outcomeAt ?? 0) - (a.turn.outcomeAt ?? 0) || b.order - a.order)
    .slice(0, 50).map(({ explorationId, turn, key }) => ({
      explorationId, turnId: turn.id, key: key!, question: turn.question,
      answerRevisionIds: [...new Set([turn.revisionId, ...(turn.publishedSections ?? []).map((section) => section.revisionId),
        ...(turn.answerVersions ?? []).map((version) => version.revisionId)].filter((id): id is string => Boolean(id)))],
      read: turn.outcomeNotice?.read === key,
      title: turn.status === 'ready' && hasAnswer(turn) ? 'Explanation ready'
        : turn.status === 'stopped' ? 'Question stopped'
        : turn.status === 'interrupted' ? 'Question interrupted'
        : turn.status === 'partial' && hasAnswer(turn) ? 'Explanation partly complete' : 'Question needs attention',
      tone: turn.status === 'ready' && hasAnswer(turn) ? 'info' : turn.status === 'failed' ? 'error' : 'warning',
    }));
}

/** Keep the notification honest when conversation metadata points at a
 * revision that is no longer present in the snapshot's saved chapter list. */
export function withSavedAnswerAvailability(outcomes: QuestionOutcome[], availableRevisionIds: ReadonlySet<string>): QuestionOutcome[] {
  return outcomes.map((outcome) => outcome.answerRevisionIds.length > 0
    && !outcome.answerRevisionIds.some((revisionId) => availableRevisionIds.has(revisionId))
    ? { ...outcome, title: 'Saved explanation unavailable', tone: 'warning' }
    : outcome);
}

/** Stable snapshots for React's external-store subscription. Source/scroll
 * changes do not recompute outcomes or rerender their consumers. */
export function createQuestionOutcomeSelector(): (state?: WorkspaceState) => QuestionOutcome[] {
  let previous: WorkspaceState | undefined;
  let items: QuestionOutcome[] = [];
  return (state) => {
    if (previous === state) return items;
    const sameTurns = previous && state && previous.projectId === state.projectId && previous.snapshotId === state.snapshotId
      && previous.explorations.length === state.explorations.length
      && previous.explorations.every((e, i) => e.id === state.explorations[i].id && e.turns === state.explorations[i].turns);
    if (!sameTurns) {
      const next = state ? questionOutcomes(state) : [];
      if (JSON.stringify(items) !== JSON.stringify(next)) items = next;
    }
    previous = state;
    return items;
  };
}

/** Only a changed terminal outcome gets a new time. Reading, scrolling and
 * repeated native reconciliation cannot resurrect an already dismissed item. */
export function recordQuestionOutcomes(previous: WorkspaceState, next: WorkspaceState, now: number): WorkspaceState {
  if (previous === next) return next;
  const before = new Map(previous.explorations.flatMap((e) => e.turns.map((turn) => [turn.id, turn] as const)));
  let changed = false;
  const explorations = next.explorations.map((e) => {
    let ownerChanged = false;
    const turns = e.turns.map((turn) => {
      const old = before.get(turn.id);
      if (old === turn) return turn;
      let nextTurn = turn;
      if (old && active(turn) && !active(old)) {
        changed = ownerChanged = true;
        nextTurn = { ...turn, outcomeSequence: (old.outcomeSequence ?? 0) < Number.MAX_SAFE_INTEGER ? (old.outcomeSequence ?? 0) + 1 : 0 };
      }
      const key = questionOutcomeKey(nextTurn);
      if (!key || old && questionOutcomeKey(old) === key) return nextTurn;
      changed = ownerChanged = true;
      return { ...nextTurn, outcomeAt: now };
    });
    return ownerChanged ? { ...e, turns } : e;
  });
  return changed ? { ...next, explorations } : next;
}

export function markQuestionOutcome(state: WorkspaceState, explorationId: string, turnId: string, key: string, action: 'read' | 'dismissed'): WorkspaceState {
  const owner = state.explorations.find((e) => e.id === explorationId);
  const turn = owner?.turns.find((t) => t.id === turnId);
  if (!turn || questionOutcomeKey(turn) !== key || turn.outcomeNotice?.[action] === key) return state;
  return { ...state, explorations: state.explorations.map((e) => e !== owner ? e : {
    ...e, turns: e.turns.map((t) => t !== turn ? t : { ...t, outcomeNotice: { ...t.outcomeNotice, [action]: key } }),
  }) };
}

/** Reopen the exact existing question without retrying inference. The one-shot
 * reveal is consumed by the mounted reader and discarded during reload. */
export function openWorkspaceQuestion(state: WorkspaceState, explorationId: string, turnId: string): WorkspaceState {
  const owner = state.explorations.find((e) => e.id === explorationId);
  const turn = owner?.turns.find((t) => t.id === turnId);
  if (!owner || !turn) return state;
  const key = questionOutcomeKey(turn);
  const read = key ? markQuestionOutcome(state, explorationId, turnId, key, 'read') : state;
  return { ...read, currentId: owner.id, file: owner.file ?? state.file,
    explorations: read.explorations.map((e) => e.id !== owner.id ? e : {
      ...e, view: 'guide', focusedTurnId: turnId, revealTurnId: turnId,
    }) };
}
