import { test } from 'node:test';
import assert from 'node:assert/strict';
import { run } from '../src/server/util.mjs';
test('a timed-out CLI is rejected even when it gracefully exits with zero', async () => {
  await assert.rejects(run(process.execPath, ['-e', 'process.on("SIGTERM",()=>process.exit(0));setInterval(()=>{},1000)'], { timeout: 200 }), /did not finish/);
});
