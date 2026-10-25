import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src/PrGuideStudio.tsx', import.meta.url), 'utf8');
const changesSource = readFileSync(new URL('../src/GitChanges.tsx', import.meta.url), 'utf8');

test('changing the global AI connection abandons an in-flight PR request review', () => {
  assert.ok(source.includes('const refreshBackend = () => {\n      abandonRequestReview();\n      setBackend(readPrGuideBackend());'));
  assert.ok(source.includes('const refreshDirectApiSelection = () => {\n      abandonRequestReview();'));
  assert.ok(source.includes('setProviderId(saved.providerId);\n      setModelId(saved.modelId);'));

  const abandonStart = source.indexOf('function abandonRequestReview()');
  const abandonEnd = source.indexOf('\n  function toggleChange', abandonStart);
  const abandon = source.slice(abandonStart, abandonEnd);
  for (const required of [
    'reviewLifecycle.current += 1',
    'setBuilding(false)',
    'discardProviderRequestReview(requestReviewRef.current.reviewId)',
    'requestReviewRef.current = null',
    'setRequestReview(null)',
  ]) assert.ok(abandon.includes(required), 'missing lifecycle cleanup: ' + required);
});

test('late PR request-review results are discarded without updating the abandoned UI', () => {
  const buildStart = source.indexOf('async function handleBuildReview()');
  const buildEnd = source.indexOf('\n  async function handleSend()', buildStart);
  const build = source.slice(buildStart, buildEnd);
  assert.ok(build.indexOf('clearReview();') < build.indexOf('const lifecycle = reviewLifecycle.current'));
  for (const required of [
    'if (lifecycle !== reviewLifecycle.current)',
    'void discardProviderRequestReview(result.data.reviewId)',
    'if (lifecycle === reviewLifecycle.current) setRequestError',
    'if (lifecycle === reviewLifecycle.current) setBuilding(false)',
  ]) assert.ok(build.includes(required), 'missing abandoned-result guard: ' + required);
});

test('comparison identity remounts PR Guide state and decisions stay scoped to that pair', () => {
  assert.ok(changesSource.includes('key={JSON.stringify([projectId, baseRef, headRef, comparison.comparisonId])}'));

  const acceptStart = source.indexOf('async function handleAccept()');
  const acceptEnd = source.indexOf('\n  async function handleReject()', acceptStart);
  const accept = source.slice(acceptStart, acceptEnd);
  assert.ok(accept.includes('comparisonId: comparison.comparisonId'));
  assert.ok(accept.includes('revisionId: currentDraft.revisionId'));

  const rejectStart = source.indexOf('async function handleReject()');
  const rejectEnd = source.indexOf('\n  function openDraft', rejectStart);
  const reject = source.slice(rejectStart, rejectEnd);
  assert.ok(reject.includes('comparisonId: comparison.comparisonId'));
  assert.ok(reject.includes('revisionId: currentDraft.revisionId'));

  const historyEffectStart = source.indexOf('useEffect(() => {\n    if (comparison.status !== \'ready\' || !comparison.comparisonId)');
  const historyEffectEnd = source.indexOf('\n  function clearReview()', historyEffectStart);
  const historyEffect = source.slice(historyEffectStart, historyEffectEnd);
  assert.ok(historyEffect.includes('detail?.projectId === projectId && detail.comparisonId === comparisonId'));
  assert.ok(historyEffect.includes('historyLookup.current += 1'));
});

test('an empty PR Guide shows why every changed file was excluded', () => {
  assert.ok(source.includes('open={accountOnly ? eligibleChanges.length === 0 : true}'));
  assert.ok(source.includes('open={eligibleChanges.length === 0}>'));
  assert.ok(source.includes('Review the exclusion reasons below, then open Diff to inspect each change.'));
});
