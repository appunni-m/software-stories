import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src/PrGuideStudio.tsx', import.meta.url), 'utf8');

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
