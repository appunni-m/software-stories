import test from 'node:test';
import assert from 'node:assert/strict';
import { addTurn, createWorkspace, parseWorkspace } from '../src/workspace/model.ts';
import { createQuestionOutcomeSelector, groupUnavailablePublishedSections, markQuestionOutcome, openWorkspaceQuestion, questionOutcomeKey, questionOutcomes, recordQuestionOutcomes, visibleConversationTurns, withSavedAnswerAvailability } from '../src/workspace/outcomes.ts';

const turn = (id, status = 'failed', extra = {}) => ({ id, question: `Question ${id}`, context: [], status, ...extra });
const stateWith = (...turns) => ({ ...createWorkspace('project', 'snapshot'), explorations: [{ ...createWorkspace('project', 'snapshot').explorations[0], turns }] });
const patch = (state, id, values, now = 100) => recordQuestionOutcomes(state, { ...state, explorations: state.explorations.map((e) => ({ ...e, turns: e.turns.map((t) => t.id === id ? { ...t, ...values } : t) })) }, now);

test('Guide hides old empty outcomes while retaining all saved, active and selected questions', () => {
  const state = stateWith(...['failed', 'stopped', 'interrupted', 'partial'].map((status) => turn(status, status)),
    turn('saved', 'ready', { revisionId: 'revision:one' }), turn('unavailable', 'failed', { revisionId: 'revision:missing' }),
    turn('saved-partial', 'partial', { publishedSections: [{ revisionId: 'revision:two' }] }), turn('working', 'working'), turn('queued', 'queued'));
  const expected = ['saved', 'unavailable', 'saved-partial', 'working', 'queued'];
  assert.deepEqual(visibleConversationTurns(state, state.explorations[0]).map((t) => t.id), expected);
  const opened = openWorkspaceQuestion(state, 'root', 'failed');
  assert.deepEqual(visibleConversationTurns(opened, opened.explorations[0]).map((t) => t.id), ['failed', ...expected]);
  assert.equal(state.explorations[0].turns.length, 9);
});

test('adjacent unavailable answer parts collapse together without changing available-part order', () => {
  const sections = ['missing-1', 'missing-2', 'available', 'missing-3'].map((revisionId, order) => ({
    runId: 'run', key: `part-${order}`, order, generationId: `generation-${order}`, revisionId,
    supersedes: null, contextElapsedMs: 0, runElapsedMs: null,
  }));
  const items = groupUnavailablePublishedSections(sections, new Set(['available']), false);
  assert.deepEqual(items.map((item) => item.kind), ['unavailable', 'section', 'unavailable']);
  assert.deepEqual(items[0].sections.map((section) => section.revisionId), ['missing-1', 'missing-2']);
  assert.equal(items[1].section.revisionId, 'available');
  assert.deepEqual(items[2].sections.map((section) => section.revisionId), ['missing-3']);
});

test('answer parts remain in loading state until the saved chapter list is ready', () => {
  const section = { runId: 'run', key: 'part', order: 0, generationId: 'generation', revisionId: 'pending',
    supersedes: null, contextElapsedMs: 0, runElapsedMs: null };
  assert.deepEqual(groupUnavailablePublishedSections([section], new Set(), true), [{ kind: 'section', section }]);
});

test('a branch remains reachable even when its original investigation saved nothing', () => {
  let state = stateWith(turn('origin'), turn('unrelated'));
  state = addTurn(state, turn('child', 'ready', { revisionId: 'revision:child' }), 'flow', { id: 'child', originTurnId: 'origin' });
  assert.deepEqual(visibleConversationTurns(state, state.explorations[0]).map((t) => t.id), ['origin']);
});

test('History opens the exact question at its existing depth without changing answers or reading positions', () => {
  let state = stateWith(turn('parent', 'ready', { revisionId: 'revision:one', reading: { guide: { revisionId: 'revision:one', sectionId: 'second' } } }));
  state = addTurn(state, turn('child', 'stopped'), 'flow', { id: 'child-level', originTurnId: 'parent' });
  const opened = openWorkspaceQuestion(state, 'root', 'parent');
  assert.equal(opened.currentId, 'root');
  assert.equal(opened.explorations[0].view, 'guide');
  assert.equal(opened.explorations[0].focusedTurnId, 'parent');
  assert.equal(opened.explorations[0].revealTurnId, 'parent');
  assert.deepEqual(opened.explorations[0].turns[0].reading, state.explorations[0].turns[0].reading);
  assert.equal(opened.explorations[1], state.explorations[1]);
  assert.equal(opened.explorations.flatMap((e) => e.turns).length, 2);
  assert.equal(openWorkspaceQuestion(state, 'root', 'missing'), state);
  assert.equal(openWorkspaceQuestion(state, 'missing', 'child'), state);
});

test('read and dismissal persist across restart without deleting answers or changing generation ownership', () => {
  let state = stateWith(turn('one', 'partial', { revisionId: 'revision:one' }));
  const key = questionOutcomeKey(state.explorations[0].turns[0]);
  state = openWorkspaceQuestion(state, 'root', 'one');
  assert.equal(questionOutcomes(state)[0].read, true);
  state = markQuestionOutcome(state, 'root', 'one', key, 'dismissed');
  const restored = parseWorkspace(JSON.stringify(state), 'project', 'snapshot');
  assert.deepEqual(questionOutcomes(restored), []);
  assert.equal(restored.explorations[0].turns[0].revisionId, 'revision:one');
  assert.equal(restored.explorations[0].turns[0].status, 'partial');
  assert.equal(restored.explorations[0].focusedTurnId, 'one');
  assert.equal(restored.explorations[0].revealTurnId, undefined);
  assert.equal(restored.explorations[0].turns[0].runIds.length, 0);
});

test('repeated reconciliation and reading never resurrect a dismissed outcome', () => {
  let state = stateWith(turn('one', 'failed', { error: 'Connection unavailable' }));
  const key = questionOutcomeKey(state.explorations[0].turns[0]);
  state = markQuestionOutcome(state, 'root', 'one', key, 'dismissed');
  const reconciled = patch(state, 'one', { status: 'failed', error: 'Connection unavailable' });
  assert.deepEqual(questionOutcomes(reconciled), []);
  assert.equal(reconciled.explorations[0].turns[0].outcomeAt, undefined);
  assert.equal(recordQuestionOutcomes(state, state, 1000), state);
});

test('a new attempt is unread even when it fails before a native run can be allocated', () => {
  let state = stateWith(turn('one', 'failed', { error: 'Connection unavailable' }));
  const oldKey = questionOutcomeKey(state.explorations[0].turns[0]);
  state = markQuestionOutcome(state, 'root', 'one', oldKey, 'dismissed');
  state = patch(state, 'one', { status: 'queued' });
  assert.equal(questionOutcomes(state).length, 0);
  state = patch(state, 'one', { status: 'working' });
  assert.equal(state.explorations[0].turns[0].outcomeSequence, 1);
  state = patch(state, 'one', { status: 'failed' }, 200);
  assert.equal(questionOutcomes(state).length, 1);
  assert.equal(questionOutcomes(state)[0].read, false);
  assert.notEqual(questionOutcomes(state)[0].key, oldKey);
  assert.equal(markQuestionOutcome(state, 'root', 'one', oldKey, 'dismissed'), state);
  assert.equal(state.explorations[0].turns[0].outcomeAt, 200);
});

test('updating an earlier published section changes the outcome even if section counts and the final section stay the same', () => {
  const original = turn('one', 'partial', { publishedSections: [{ runId: 'run', generationId: 'first', revisionId: 'rev-one' }, { runId: 'run', generationId: 'last', revisionId: 'rev-last' }] });
  const changed = { ...original, publishedSections: [{ ...original.publishedSections[0], generationId: 'updated' }, original.publishedSections[1]] };
  assert.notEqual(questionOutcomeKey(changed), questionOutcomeKey(original));
});

test('native guide metadata refresh alone does not create a duplicate outcome', () => {
  const original = turn('one', 'partial', { guide: { revision: 5 } });
  assert.equal(questionOutcomeKey(original), questionOutcomeKey({ ...original, guide: { revision: 6 } }));
});

test('notifications are snapshot scoped, bounded and ordered by terminal outcome time', () => {
  const state = stateWith(...Array.from({ length: 60 }, (_, i) => turn(String(i), 'failed', { outcomeAt: i })));
  const notifications = questionOutcomes(state);
  assert.equal(notifications.length, 50);
  assert.equal(notifications[0].turnId, '59');
  assert.equal(notifications.at(-1).turnId, '10');
  assert.equal(state.explorations[0].turns.length, 60);
  assert.equal(questionOutcomes(createWorkspace('other', 'snapshot')).length, 0);
});

test('reloading an active question reports interruption without automatically submitting it', () => {
  const state = stateWith(turn('one', 'working'));
  const restored = parseWorkspace(JSON.stringify(state), 'project', 'snapshot');
  assert.equal(questionOutcomes(restored)[0].title, 'Question interrupted');
  assert.equal(restored.explorations[0].turns[0].status, 'interrupted');
  assert.deepEqual(restored.explorations[0].turns[0].runIds, []);
});

test('invalid focus and notification metadata are discarded during restore', () => {
  const state = stateWith(turn('one', 'failed', { outcomeAt: -1, outcomeSequence: Infinity, outcomeNotice: { read: 'x'.repeat(5000), dismissed: ['invalid'] } }));
  state.explorations[0].focusedTurnId = 'someone-elses-question';
  state.explorations[0].revealTurnId = 'one';
  const restored = parseWorkspace(JSON.stringify(state), 'project', 'snapshot');
  assert.equal(restored.explorations[0].focusedTurnId, undefined);
  assert.equal(restored.explorations[0].revealTurnId, undefined);
  assert.equal(restored.explorations[0].turns[0].outcomeNotice, undefined);
  assert.equal(restored.explorations[0].turns[0].outcomeAt, undefined);
  assert.equal(restored.explorations[0].turns[0].outcomeSequence, undefined);
});

test('scrolling, source selection and repeated reconciliation reuse the exact notification snapshot', () => {
  const select = createQuestionOutcomeSelector();
  let state = stateWith(turn('one', 'failed'));
  const items = select(state);
  assert.equal(select(state), items);
  state = { ...state, file: 'src/another.rs', explorations: state.explorations.map((e) => ({ ...e, scrollPositions: { guide: 1200 } })) };
  assert.equal(select(state), items);
  state = patch(state, 'one', { status: 'failed' });
  assert.equal(select(state), items);
  const dismissed = markQuestionOutcome(state, 'root', 'one', items[0].key, 'dismissed');
  assert.notEqual(select(dismissed), items);
  assert.deepEqual(select(dismissed), []);
});

test('the outcome selector clears entries when switching snapshots or closing a project', () => {
  const select = createQuestionOutcomeSelector();
  const state = stateWith(turn('one', 'failed'));
  assert.equal(select(state).length, 1);
  assert.equal(select(createWorkspace('project', 'other-snapshot')).length, 0);
  assert.equal(select(state).length, 1);
  const empty = select(undefined);
  assert.deepEqual(empty, []);
  assert.equal(select(undefined), empty);
});

test('an incomplete question with no saved reference is not announced as a completed explanation', () => {
  for (const status of ['ready', 'partial', 'failed']) {
    const [outcome] = questionOutcomes(stateWith(turn('one', status)));
    assert.equal(outcome.title, 'Question needs attention');
  }
});

test('notifications report a missing saved revision instead of claiming an explanation is ready', () => {
  const state = stateWith(turn('one', 'ready', { revisionId: 'revision:missing' }));
  const [outcome] = questionOutcomes(state);
  const unavailable = withSavedAnswerAvailability([outcome], new Set(['revision:other']));
  assert.equal(unavailable[0].title, 'Saved explanation unavailable');
  assert.equal(unavailable[0].tone, 'warning');
  assert.equal(unavailable[0].turnId, 'one');

  const available = withSavedAnswerAvailability([outcome], new Set(['revision:missing']));
  assert.equal(available[0].title, 'Explanation ready');
  assert.equal(available[0].tone, 'info');
});

test('background completion changes neither the current branch, focused question, source nor reading positions', () => {
  let state = stateWith(turn('parent', 'working'));
  state = addTurn(state, turn('child', 'ready', { revisionId: 'revision:child' }), 'flow', { id: 'branch', originTurnId: 'parent' });
  const previousBranch = state.explorations[1];
  const completed = patch(state, 'parent', { status: 'ready', revisionId: 'revision:parent' });
  assert.equal(completed.currentId, state.currentId);
  assert.equal(completed.file, state.file);
  assert.deepEqual(completed.explorations[1], previousBranch);
  assert.equal(questionOutcomes(completed)[0].turnId, 'parent');
});
