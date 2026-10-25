import assert from 'node:assert/strict';
import test from 'node:test';
import { PrGuideOperationFence } from '../src/lib/prGuideLifecycle.ts';

const comparison = {
  projectId: 'fontdone',
  baseRef: 'main',
  headRef: 'feature',
  comparisonId: 'cmp-main-feature',
};

test('a comparison has at most one in-flight action of each kind', () => {
  const fence = new PrGuideOperationFence(comparison);
  const send = fence.begin('send');

  assert.ok(send);
  assert.equal(fence.begin('send'), null);
  const cancel = fence.begin('cancel');
  const decision = fence.begin('decision');
  assert.ok(cancel);
  assert.ok(decision);
  assert.equal(fence.begin('decision'), null);
  assert.equal(fence.isCurrent(send), true);
  assert.equal(fence.isCurrent(cancel), true);
  assert.equal(fence.isCurrent(decision), true);
});

test('unmount invalidates late callbacks and reopening starts a distinct operation', () => {
  const fence = new PrGuideOperationFence(comparison);
  const send = fence.begin('send');
  assert.ok(send);

  fence.unmount();
  assert.equal(fence.isCurrent(send), false);
  fence.mount(comparison);
  const reopenedSend = fence.begin('send');

  assert.ok(reopenedSend);
  assert.notEqual(reopenedSend.id, send.id);
  assert.equal(fence.isCurrent(send), false);
  assert.equal(fence.isCurrent(reopenedSend), true);
  fence.finish(send);
  assert.equal(fence.isCurrent(reopenedSend), true);
});

test('changing any immutable comparison identity invalidates pending callbacks', () => {
  const fence = new PrGuideOperationFence(comparison);
  const decision = fence.begin('decision');
  assert.ok(decision);

  fence.setScope({ ...comparison, headRef: 'another-feature', comparisonId: 'cmp-main-another-feature' });

  assert.equal(fence.isCurrent(decision), false);
  assert.equal(fence.isActive('decision'), false);
  const nextDecision = fence.begin('decision');
  assert.ok(nextDecision);
  assert.equal(nextDecision.scopeKey, JSON.stringify(['fontdone', 'main', 'another-feature', 'cmp-main-another-feature']));
});

test('finishing an older operation cannot release its replacement', () => {
  const fence = new PrGuideOperationFence(comparison);
  const first = fence.begin('decision');
  assert.ok(first);
  fence.finish(first);
  const second = fence.begin('decision');
  assert.ok(second);

  fence.finish(first);

  assert.equal(fence.isCurrent(first), false);
  assert.equal(fence.isCurrent(second), true);
});
