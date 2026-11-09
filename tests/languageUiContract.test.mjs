import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
const codePane = readFileSync(new URL('../src/design/CodePane.tsx', import.meta.url), 'utf8');

test('the reading UI hides parser and index internals while preserving useful source actions', () => {
  const start = app.indexOf('<aside className="atlas-details-panel"');
  const end = app.indexOf('</aside>', start);
  assert.ok(start >= 0 && end > start, 'the source details panel is present');
  const details = app.slice(start, end);

  assert.doesNotMatch(details, /Language support|Partial structural|Text only|Local index|Indexed files|symbolCount|relationshipCount|SHA-256|parserReadiness|indexState/);
  assert.match(details, /Source is unavailable in this snapshot/);
  assert.match(details, /Open source/);
  assert.match(details, /Possible connection/);

  assert.doesNotMatch(workspace, /languageSupport(Label|Note|Tier)|supportTier|capability=/);
  assert.doesNotMatch(codePane, /languageSupport|capability|Partial structural|Text only/);
  assert.match(codePane, /Select code to ask about it/);
});
