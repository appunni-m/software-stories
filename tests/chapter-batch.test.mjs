import assert from 'node:assert/strict';
import { it } from 'node:test';
import { chapterForStep, runChapterBatch } from '../src/lib/chapterBatch.ts';

function fixture() {
  const calls = [];
  const saved = [];
  const steps = [1, 2].map((id) => ({
    scope: { target: { revisionId: 'plan', recordSha256: 'record', stepId: `step${id}` }, snapshotId: 'snapshot',
      primaryPath: `src/${id}.ts`, question: `How does stage ${id} work?` },
    review: { reviewId: `old-${id}`, projectId: 'project', snapshotId: 'snapshot', fingerprint: `fp${id}`, requestBodySha256: `body${id}`, modelId: 'gpt-6-luna' },
  }));
  const revision = (scope) => ({ revisionId: `chapter-${scope.target.stepId}`, projectId: 'project', snapshotId: scope.snapshotId, lifecycle: 'draft',
    generationOrigin: { projectId: 'project', snapshotId: scope.snapshotId, targetPath: scope.primaryPath, plannedChapter: scope } });
  let cancelled = false;
  const commands = {
    build: async (scope) => { calls.push(`build:${scope.target.stepId}`); return { ...steps.find((step) => step.scope.target.stepId === scope.target.stepId).review, reviewId: `fresh-${scope.target.stepId}` }; },
    matches: () => true,
    send: async (review) => { calls.push(`send:${review.reviewId}`); const scope = steps.find((step) => step.review.fingerprint === review.fingerprint).scope;
      return { generationId: scope.target.stepId, revision: revision(scope), validation: { status: 'valid' } }; },
    save: async (id) => { calls.push(`save:${id}`); const result = revision(steps.find((step) => step.scope.target.stepId === id).scope); saved.push(result); return result; },
    list: async () => saved,
    discardReview: async (id) => { calls.push(`release:${id}`); },
    discardDraft: async (id) => { calls.push(`discard:${id}`); },
  };
  const options = { cancelled: () => cancelled, active: () => {}, progress: () => {} };
  return { steps, commands, options, calls, saved, revision, cancel: () => { cancelled = true; } };
}

it('builds route chapters sequentially with fresh requests and independent draft saves', async () => {
  const f = fixture();
  const result = await runChapterBatch(f.steps, f.commands, f.options);
  assert.equal(result.length, 2);
  assert.deepEqual(f.calls, ['build:step1', 'send:fresh-step1', 'save:step1', 'release:fresh-step1', 'build:step2', 'send:fresh-step2', 'save:step2', 'release:fresh-step2']);
  assert.ok(result.every((chapter) => chapter.lifecycle === 'draft'));
});

it('resumes from exact saved step lineage and does not skip rejected or unrelated chapters', async () => {
  const f = fixture();
  const first = f.revision(f.steps[0].scope);
  assert.equal(chapterForStep([{ ...first, lifecycle: 'rejected' }], f.steps[0].scope, 'project'), undefined);
  assert.equal(chapterForStep([first], f.steps[0].scope, 'another-project'), undefined);
  assert.equal(chapterForStep([first], { ...f.steps[0].scope, target: { ...f.steps[0].scope.target, recordSha256: 'changed' } }, 'project'), undefined);
  f.saved.push(first);
  await runChapterBatch(f.steps, f.commands, f.options);
  assert.ok(!f.calls.includes('build:step1'));
  assert.ok(f.calls.includes('save:step2'));
});

it('keeps two questions for the same file separate by accepted step identity', async () => {
  const f = fixture();
  const sameFileSteps = f.steps;
  sameFileSteps[1].scope.primaryPath = sameFileSteps[0].scope.primaryPath;
  sameFileSteps[1].scope.question = 'How does the second operation use the same file?';

  const result = await runChapterBatch(sameFileSteps, f.commands, f.options);

  assert.deepEqual(result.map((chapter) => chapter.generationOrigin.plannedChapter.target.stepId), ['step1', 'step2']);
  assert.equal(result[0].generationOrigin.targetPath, result[1].generationOrigin.targetPath);
  assert.notEqual(result[0].generationOrigin.plannedChapter.question, result[1].generationOrigin.plannedChapter.question);
  assert.equal(chapterForStep([result[0]], sameFileSteps[1].scope, 'project'), undefined);
  assert.equal(chapterForStep([result[1]], sameFileSteps[0].scope, 'project'), undefined);
  assert.deepEqual(f.calls.filter((call) => call.startsWith('send:')), ['send:fresh-step1', 'send:fresh-step2']);
});

it('stops before transmission when reviewed account, source, settings or prompt change', async () => {
  for (const field of ['fingerprint', 'requestBodySha256', 'modelId', 'snapshotId', 'projectId']) {
    const f = fixture();
    const build = f.commands.build;
    f.commands.build = async (scope) => ({ ...await build(scope), [field]: 'changed' });
    await assert.rejects(runChapterBatch(f.steps, f.commands, f.options), /changed/);
    assert.ok(!f.calls.some((call) => call.startsWith('send:')));
    assert.ok(f.calls.includes('release:fresh-step1'));
  }
});

it('stops on provider failure without retrying or discarding previously saved chapters', async () => {
  const f = fixture();
  const send = f.commands.send;
  f.commands.send = async (review) => { if (review.fingerprint === 'fp2') throw Error('quota exhausted'); return send(review); };
  await assert.rejects(runChapterBatch(f.steps, f.commands, f.options), /quota/);
  assert.equal(f.saved.length, 1);
  assert.equal(f.calls.filter((call) => call === 'build:step2').length, 1);
});

it('cancels the queue after a completed save and discards a late cancelled response', async () => {
  const f = fixture();
  f.options.progress = (_index, phase) => { if (phase === 'saved') f.cancel(); };
  await runChapterBatch(f.steps, f.commands, f.options);
  assert.equal(f.saved.length, 1);
  assert.ok(!f.calls.includes('build:step2'));
  const late = fixture();
  const send = late.commands.send;
  late.commands.send = async (review) => { const response = await send(review); late.cancel(); return response; };
  await runChapterBatch(late.steps, late.commands, late.options);
  assert.equal(late.saved.length, 0);
  assert.ok(late.calls.includes('discard:step1'));
});

it('reconciles a lost save reply by exact revision ID without repeating inference', async () => {
  const f = fixture();
  const save = f.commands.save;
  f.commands.save = async (id) => { await save(id); throw Error('reply lost'); };
  const result = await runChapterBatch(f.steps, f.commands, f.options);
  assert.equal(result.length, 2);
  assert.equal(f.calls.filter((call) => call.startsWith('send:')).length, 2);
  assert.ok(f.calls.includes('discard:step1'));
});

it('rejects invalid results and invalid batch shapes without accepting content', async () => {
  const f = fixture();
  const send = f.commands.send;
  f.commands.send = async (review) => ({ ...await send(review), validation: { status: 'invalid' } });
  await assert.rejects(runChapterBatch(f.steps, f.commands, f.options), /validated reading step/);
  assert.equal(f.saved.length, 0);
  await assert.rejects(runChapterBatch([], f.commands, f.options), /valid route/);
  await assert.rejects(runChapterBatch([f.steps[0], f.steps[0]], f.commands, f.options), /valid route/);
});
