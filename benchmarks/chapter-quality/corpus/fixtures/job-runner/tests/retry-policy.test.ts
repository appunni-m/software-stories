import assert from "node:assert/strict";
import test from "node:test";
import { isRetryable } from "../src/retry-policy";

test("allows failed jobs below the attempt limit", () => {
  assert.equal(isRetryable({ id: "job-1", state: "failed", attempts: 2 }), true);
});

test("blocks jobs at the limit and jobs that are not failed", () => {
  assert.equal(isRetryable({ id: "job-2", state: "failed", attempts: 3 }), false);
  assert.equal(isRetryable({ id: "job-3", state: "done", attempts: 1 }), false);
});
