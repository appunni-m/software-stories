import assert from 'node:assert/strict';
import test from 'node:test';
import {
  acquirePlannedChapterBatchRun, exactSavedPlannedChapter, matchingPlannedChapterPendingPreview,
  matchesPlannedChapterPreview, plannedChapterBatchBoundary, sameReviewedPlannedChapterRequest,
} from '../src/lib/plannedChapterBatch.ts';

const digest = (character) => character.repeat(64);
const target = { revisionId: 'route-revision-1', recordSha256: digest('a'), stepId: 'step-one' };
const scope = {
  schemaVersion: 1,
  target,
  snapshotId: 'snapshot-1',
  primaryPath: 'src/lib.rs',
  selectedPaths: ['src/lib.rs'],
  planQuestion: 'How does this route handle a request?',
  question: 'How does this function validate a request?',
  prerequisites: [],
  planContext: {
    schemaVersion: 1,
    question: 'How does this route handle a request?',
    inventorySha256: digest('b'),
    contextSha256: digest('c'),
    planningContractSha256: digest('d'),
    captureStatus: 'complete',
    coverage: {
      selectedFileCount: 1,
      eligibleCapturedFileCount: 2,
      unselectedEligibleFileCount: 1,
      excludedCapturedPathCount: 0,
      skippedPathCount: 0,
    },
    indexStatus: 'complete',
  },
};

test('stop after current save preserves the in-flight result and blocks the next step', () => {
  assert.deepEqual(plannedChapterBatchBoundary({ alive: true, cancelled: false, stopAfterCurrentSave: false }), {
    keepCurrent: true, continueBatch: true, stopped: false,
  });
  assert.deepEqual(plannedChapterBatchBoundary({ alive: true, cancelled: false, stopAfterCurrentSave: true }), {
    keepCurrent: true, continueBatch: false, stopped: true,
  });
  assert.deepEqual(plannedChapterBatchBoundary({ alive: true, cancelled: true, stopAfterCurrentSave: false }), {
    keepCurrent: false, continueBatch: false, stopped: true,
  });
  assert.deepEqual(plannedChapterBatchBoundary({ alive: false, cancelled: false, stopAfterCurrentSave: true }), {
    keepCurrent: false, continueBatch: false, stopped: true,
  });
});

test('duplicate Build clicks admit only one batch worker before rerender', async () => {
  const run = { current: false };
  assert.equal(acquirePlannedChapterBatchRun(run, false), false);
  assert.equal(run.current, false);

  assert.equal(acquirePlannedChapterBatchRun(run, true), true);
  assert.equal(acquirePlannedChapterBatchRun(run, true), false);
  assert.equal(run.current, true);

  const { readFileSync } = await import('node:fs');
  const source = readFileSync(new URL('../src/RepositoryPlanBatchDialog.tsx', import.meta.url), 'utf8');
  assert.match(source, /const buildRunStarted = useRef\(false\)/);
  assert.match(source, /acquirePlannedChapterBatchRun\(\s*buildRunStarted/);
});

test('the batch stop-after-save control leaves the active Codex request running', async () => {
  const { readFileSync } = await import('node:fs');
  const source = readFileSync(new URL('../src/RepositoryPlanBatchDialog.tsx', import.meta.url), 'utf8');
  const control = source.match(/\{phase === 'running' && <AppButton intent="quiet" onClick=\{\(\) => \{ stopAfterCurrentSave\.current = true;[^}]+\}\}/)?.[0];
  assert.ok(control, 'running batch exposes the stop-after-save action');
  assert.doesNotMatch(control, /cancelCodexChapterRequest/);
  assert.match(source, /plannedChapterBatchBoundary\([\s\S]+?\)\.keepCurrent/);
  assert.match(source, /plannedChapterBatchBoundary\([\s\S]+?\)\.continueBatch/);
});

function review() {
  return {
    reviewId: 'request-1',
    projectId: 'project-1',
    snapshotId: 'snapshot-1',
    purpose: 'fileChapter',
    targetPath: 'src/lib.rs',
    providerId: 'codex',
    modelId: 'gpt-6-luna',
    selectedFiles: [{ path: 'src/lib.rs', sha256: digest('e'), sizeBytes: 15, contents: 'fn validate() {}' }],
    selectedSymbols: [],
    selectedRelationships: [],
    indexEvidenceTruncated: false,
    prGuideContext: null,
    sectionUpdateContext: null,
    codeLensContext: null,
    prChangeContext: null,
    systemInstruction: 'Explain the selected route step.',
    userPrompt: 'How does this function validate a request?',
    generationConfig: {
      contractVersion: 'codex-planned-chapter-v1',
      plannedChapter: structuredClone(scope),
      effort: 'max',
      approvalPolicy: 'never',
    },
    sourceBytes: 15,
    estimatedInputTokens: 100,
    requestBodyBytes: 300,
    requestBodySha256: digest('f'),
    requestBodyPreview: {},
    fingerprint: digest('1'),
    expiresAtUnixMs: Date.now() + 60_000,
  };
}

function savedRevision(step = target.stepId, lifecycle = 'draft') {
  const plannedChapter = structuredClone(scope);
  plannedChapter.target.stepId = step;
  return {
    projectId: 'project-1',
    snapshotId: 'snapshot-1',
    lifecycle,
    generationOrigin: {
      schemaVersion: 6,
      projectId: 'project-1',
      snapshotId: 'snapshot-1',
      targetPath: 'src/lib.rs',
      purpose: 'fileChapter',
      requestId: 'request-1',
      requestFingerprint: digest('1'),
      requestBodySha256: digest('f'),
      contractVersion: 'codex-planned-chapter-v1',
      settings: { backend: 'codex', modelId: 'gpt-6-luna', effort: 'max', approvalPolicy: 'never' },
      sourceFiles: [{ path: 'src/lib.rs', sha256: digest('e'), sizeBytes: 15 }],
      sectionUpdate: null,
      plannedChapter,
      contentSha256: digest('2'),
    },
  };
}

test('batch skip requires exact route-step lineage, not a matching file or rejected result', () => {
  assert.equal(exactSavedPlannedChapter(savedRevision(), scope), true);
  assert.equal(exactSavedPlannedChapter(savedRevision('another-step'), scope), false);
  assert.equal(exactSavedPlannedChapter(savedRevision(target.stepId, 'rejected'), scope), false);
});

test('a rebuilt request must match the reviewed fingerprint, source bytes, model, and effort', () => {
  const reviewed = review();
  assert.equal(sameReviewedPlannedChapterRequest(reviewed, structuredClone(reviewed), scope, {
    modelId: 'gpt-6-luna', effort: 'max',
  }), true);

  const changedSource = structuredClone(reviewed);
  changedSource.selectedFiles[0].contents = 'fn validate() { changed }';
  assert.equal(sameReviewedPlannedChapterRequest(reviewed, changedSource, scope, {
    modelId: 'gpt-6-luna', effort: 'max',
  }), false);

  const changedRequest = structuredClone(reviewed);
  changedRequest.requestBodySha256 = digest('3');
  assert.equal(sameReviewedPlannedChapterRequest(reviewed, changedRequest, scope, {
    modelId: 'gpt-6-luna', effort: 'max',
  }), false);
});

test('recovery blocks only for a pending preview from the exact project, snapshot, and route step', () => {
  const candidate = savedRevision();
  const unrelated = {
    generationId: 'other-generation', projectId: 'other-project', snapshotId: 'snapshot-1',
    title: 'Other result', generationOrigin: candidate.generationOrigin,
  };
  const exact = {
    generationId: 'generation-1', projectId: 'project-1', snapshotId: 'snapshot-1',
    title: 'Validated request', generationOrigin: candidate.generationOrigin,
    modelId: 'gpt-6-luna', targetPath: 'src/lib.rs', unavailableReason: null, originContentMatches: true,
  };
  const otherStep = {
    ...exact,
    generationId: 'generation-2',
    generationOrigin: { ...candidate.generationOrigin, plannedChapter: {
      ...candidate.generationOrigin.plannedChapter,
      target: { ...target, stepId: 'another-step' },
    } },
  };
  assert.equal(matchingPlannedChapterPendingPreview([unrelated, otherStep, exact], 'project-1', 'snapshot-1', [scope])?.generationId, 'generation-1');
  assert.equal(matchingPlannedChapterPendingPreview([otherStep], 'project-1', 'snapshot-1', [scope]), null);
});

test('a chapter preview must preserve the reviewed request and every selected source identity', () => {
  const request = review();
  const draft = {
    generationId: 'generation-1',
    modelId: request.modelId,
    modelVersion: null,
    inputTokens: null,
    outputTokens: null,
    updateNotes: [],
    validation: { status: 'valid' },
    revision: { revisionId: 'revision-1', ...savedRevision() },
  };
  assert.equal(matchesPlannedChapterPreview(draft, request, scope), true);

  const changedSource = structuredClone(draft);
  changedSource.revision.generationOrigin.sourceFiles[0].sha256 = digest('9');
  assert.equal(matchesPlannedChapterPreview(changedSource, request, scope), false);
});
