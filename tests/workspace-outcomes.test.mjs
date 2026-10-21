import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { addTurn, createWorkspace, parseWorkspace } from '../src/workspace/model.ts';
import { createQuestionOutcomeSelector, featuredGuideTurnId, groupUnavailablePublishedSections, historyQuestionPreview, lastAvailableAnswerItemIndex, markQuestionOutcome, openWorkspaceExploration, openWorkspaceQuestion, questionOutcomeKey, questionOutcomes, recordQuestionOutcomes, visibleConversationTurns, withSavedAnswerAvailability } from '../src/workspace/outcomes.ts';

const turn = (id, status = 'failed', extra = {}) => ({ id, question: `Question ${id}`, context: [], status, ...extra });
const stateWith = (...turns) => ({ ...createWorkspace('project', 'snapshot'), explorations: [{ ...createWorkspace('project', 'snapshot').explorations[0], turns }] });
const patch = (state, id, values, now = 100) => recordQuestionOutcomes(state, { ...state, explorations: state.explorations.map((e) => ({ ...e, turns: e.turns.map((t) => t.id === id ? { ...t, ...values } : t) })) }, now);

test('History shows a compact question preview and preserves the exact question in a collapsed disclosure', () => {
  const question = '  Explain how\nFont::memory_face selects a format. ';
  assert.equal(historyQuestionPreview(question), 'Explain how Font::memory_face selects a format.');
  const long = `🧭${'a'.repeat(120)}`;
  const preview = historyQuestionPreview(long);
  assert.equal(Array.from(preview).length, 64);
  assert.ok(preview.endsWith('…'));
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.match(source, /<details className="dw-history-question"><summary><strong><InlineCodeText text=\{historyQuestionPreview\(turn\.question\)\}\/><\/strong><\/summary><p><InlineCodeText text=\{turn\.question\}\/><\/p><\/details>/);
});

test('Guide hides old empty and unavailable answers while retaining saved, active and selected questions', () => {
  const state = stateWith(...['failed', 'stopped', 'interrupted', 'partial'].map((status) => turn(status, status)),
    turn('saved', 'ready', { revisionId: 'revision:one' }), turn('unavailable', 'failed', { revisionId: 'revision:missing' }),
    turn('saved-partial', 'partial', { revisionId: 'revision:old', publishedSections: [{ revisionId: 'revision:two' }, { revisionId: 'revision:missing-section' }] }),
    turn('missing-sections', 'partial', { revisionId: 'revision:one', publishedSections: [{ revisionId: 'revision:missing-section' }] }),
    turn('working', 'working'), turn('queued', 'queued'));
  const available = new Set(['revision:one', 'revision:two']);
  const expected = ['saved', 'saved-partial', 'working', 'queued'];
  assert.deepEqual(visibleConversationTurns(state, state.explorations[0], available).map((t) => t.id), expected);
  assert.equal(visibleConversationTurns(state, state.explorations[0], available, true).includes(state.explorations[0].turns.find((t) => t.id === 'missing-sections')), true,
    'keep answer cards stable while saved revisions are loading');
  const opened = openWorkspaceQuestion(state, 'root', 'failed');
  assert.deepEqual(visibleConversationTurns(opened, opened.explorations[0], available).map((t) => t.id), ['failed', ...expected]);
  const openedUnavailable = openWorkspaceQuestion(state, 'root', 'unavailable');
  assert.deepEqual(visibleConversationTurns(openedUnavailable, openedUnavailable.explorations[0], available).map((t) => t.id),
    ['saved', 'unavailable', 'saved-partial', 'working', 'queued'], 'an explicitly opened unavailable answer remains explainable');
  const openedMissingSections = openWorkspaceQuestion(state, 'root', 'missing-sections');
  assert.ok(visibleConversationTurns(openedMissingSections, openedMissingSections.explorations[0], available)
    .some((t) => t.id === 'missing-sections'), 'a selected progressive answer remains explainable even if its old base revision exists');
  assert.equal(state.explorations[0].turns.length, 10);
});

test('Guide keeps the selected answer expanded and otherwise prefers the newest available answer', () => {
  const turns = [
    turn('old', 'ready', { revisionId: 'revision:old' }),
    turn('missing', 'partial', { publishedSections: [{ revisionId: 'revision:missing' }] }),
    turn('latest', 'ready', { publishedSections: [{ revisionId: 'revision:latest' }] }),
    turn('newer-missing', 'failed', { revisionId: 'revision:gone' }),
  ];
  const available = new Set(['revision:old', 'revision:latest']);
  assert.equal(featuredGuideTurnId(turns, undefined, available), 'latest');
  assert.equal(featuredGuideTurnId(turns, 'old', available), 'old');
  assert.equal(featuredGuideTurnId([...turns, turn('working', 'working')], undefined, available), 'working');
  assert.equal(featuredGuideTurnId([turn('unavailable', 'failed')], undefined, available), 'unavailable');
  const source = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.match(source, /compact=\{turn\.id!==featuredTurnId\}/);
});

test('reopening an exploration focuses its latest question without changing its view or reading state', () => {
  let state = stateWith(turn('origin', 'ready', { revisionId: 'revision:origin' }));
  state = addTurn(state, turn('first', 'failed'), 'flow', { id: 'child', originTurnId: 'origin' });
  state = addTurn(state, turn('latest', 'failed'), 'history');
  state = {
    ...state,
    currentId: 'root',
    explorations: state.explorations.map((exploration) => exploration.id === 'child'
      ? { ...exploration, focusedTurnId: undefined, file: 'src/outline.rs', scrollPositions: { history: 180 } }
      : exploration),
  };

  const opened = openWorkspaceExploration(state, 'child');
  const child = opened.explorations.find((exploration) => exploration.id === 'child');
  assert.equal(opened.currentId, 'child');
  assert.equal(opened.file, 'src/outline.rs');
  assert.equal(child.view, 'history');
  assert.equal(child.focusedTurnId, 'latest');
  assert.deepEqual(child.scrollPositions, { history: 180 });
  assert.deepEqual(visibleConversationTurns(opened, child).map((question) => question.id), ['latest']);
  assert.equal(openWorkspaceExploration(state, 'missing'), state);

  const explicitlyFocused = openWorkspaceExploration({
    ...state,
    explorations: state.explorations.map((exploration) => exploration.id === 'child'
      ? { ...exploration, focusedTurnId: 'first' }
      : exploration),
  }, 'child');
  assert.equal(explicitlyFocused.explorations.find((exploration) => exploration.id === 'child').focusedTurnId, 'first');
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

test('question-owned branches stay attached to the last available part before unavailable newer parts', () => {
  const sections = ['missing-1', 'available-1', 'missing-2', 'available-2', 'missing-3'].map((revisionId, order) => ({
    runId: 'run', key: `part-${order}`, order, generationId: `generation-${order}`, revisionId,
    supersedes: null, contextElapsedMs: 0, runElapsedMs: null,
  }));
  const items = groupUnavailablePublishedSections(sections, new Set(['available-1', 'available-2']), false);
  assert.equal(lastAvailableAnswerItemIndex(items), 3);
  assert.equal(lastAvailableAnswerItemIndex(groupUnavailablePublishedSections(sections, new Set(), false)), -1);
});

test('a branch remains reachable even when its original investigation saved nothing', () => {
  let state = stateWith(turn('origin'), turn('unrelated'));
  state = addTurn(state, turn('child', 'ready', { revisionId: 'revision:child' }), 'flow', { id: 'child', originTurnId: 'origin' });
  assert.deepEqual(visibleConversationTurns(state, state.explorations[0], new Set()).map((t) => t.id), ['origin']);
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

test('History preserves the reader position when reopening its already focused question', () => {
  const state = stateWith(turn('parent', 'ready', {
    revisionId: 'revision:one',
    reading: { guide: { revisionId: 'revision:one', sectionId: 'second' } },
  }));
  state.explorations[0].view = 'history';
  state.explorations[0].focusedTurnId = 'parent';
  state.explorations[0].scrollPositions = { guide: 1_240, history: 80 };

  const opened = openWorkspaceQuestion(state, 'root', 'parent');
  const root = opened.explorations[0];
  assert.equal(root.view, 'guide');
  assert.equal(root.focusedTurnId, 'parent');
  assert.equal(root.revealTurnId, undefined, 'do not jump to the top of a question the reader was already following');
  assert.deepEqual(root.scrollPositions, { guide: 1_240, history: 80 });
  assert.deepEqual(root.turns[0].reading, state.explorations[0].turns[0].reading);
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
