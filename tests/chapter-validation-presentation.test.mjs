import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInThisContext } from 'node:vm';
import ts from 'typescript';
import * as jsxRuntime from 'react/jsx-runtime';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { baseRevision } from './helpers/revision-history-fixture.mjs';
import { chapterIssueFeedback, presentChapterIssues } from '../src/lib/chapterValidationPresentation.ts';

const issue = (overrides = {}) => ({ code: 'rangeMismatch', message: 'The source range is outside the captured file.', path: 'src/routes.ts', sectionId: 'section:request', claimId: 'claim:entry', anchorId: 'anchor:route', hotspotId: null, visualItemId: null, ...overrides });

test('findings name the displayed section, claim and exact source without changing native data', () => {
  const revision = baseRevision(), finding = issue(), before = structuredClone({ revision, finding });
  const [view] = presentChapterIssues(revision, [finding]);
  assert.match(view.location, /Section 1 · Request entry point \/ Claim 1 · The route dispatches/);
  assert.equal(view.source, 'src/routes.ts:2–4');
  assert.deepEqual(view.target, { sectionId: 'section:request', claimId: 'claim:entry' });
  assert.equal(view.repairAnchor, revision.sections[0].claims[0].anchors[0]);
  assert.equal(view.issue, finding);
  assert.deepEqual({ revision, finding }, before);
});

test('locations follow current unsaved section order and claim text', () => {
  const revision = baseRevision(), first = structuredClone(revision.sections[0]);
  first.sectionId = 'section:other'; first.claims = []; first.hotspots = []; first.visual = null;
  revision.sections.unshift(first); revision.sections[1].claims[0].prose = 'An unsaved claim';
  const [view] = presentChapterIssues(revision, [issue({ claimId: null })]);
  assert.match(view.location, /Section 2 .*Claim 1 · An unsaved claim/);
  assert.equal(view.target.claimId, 'claim:entry');
});

test('missing, duplicate, contradictory and cross-kind identities never select an arbitrary repair target', () => {
  const cases = [];
  for (const field of ['sectionId', 'claimId', 'anchorId']) cases.push([baseRevision(), issue({ [field]: 'missing:identity' })]);
  for (const kind of ['section', 'claim', 'anchor', 'crossKind']) {
    const revision = baseRevision(), section = revision.sections[0];
    if (kind === 'section') revision.sections.push(structuredClone(section));
    if (kind === 'claim') section.claims.push(structuredClone(section.claims[0]));
    if (kind === 'anchor') section.claims[0].anchors.push(structuredClone(section.claims[0].anchors[0]));
    if (kind === 'crossKind') section.visual.nodes[0].nodeId = 'anchor:route';
    cases.push([revision, issue()]);
  }
  const foreign = baseRevision(), other = structuredClone(foreign.sections[0]);
  other.sectionId = 'section:other'; other.claims = []; other.hotspots = []; other.visual = null; foreign.sections.push(other);
  cases.push([foreign, issue({ sectionId: other.sectionId })], [baseRevision(), issue({ path: 'src/wrong.ts' })]);
  const claims = baseRevision(), second = structuredClone(claims.sections[0].claims[0]);
  second.claimId = 'claim:other'; second.anchors = []; claims.sections[0].claims.push(second);
  cases.push([claims, issue({ claimId: second.claimId })]);
  const repeatedParent = baseRevision(), duplicateClaim = structuredClone(repeatedParent.sections[0].claims[0]);
  duplicateClaim.anchors = []; repeatedParent.sections[0].claims.push(duplicateClaim);
  cases.push([repeatedParent, issue({ claimId: null })]);
  const repeatedSection = baseRevision(), duplicateSection = structuredClone(repeatedSection.sections[0]);
  duplicateSection.claims = []; duplicateSection.hotspots = []; duplicateSection.visual = null; repeatedSection.sections.push(duplicateSection);
  cases.push([repeatedSection, issue({ sectionId: null, claimId: null })]);
  for (const [revision, finding] of cases) {
    const [view] = presentChapterIssues(revision, [finding]);
    assert.equal(view.target, null); assert.equal(view.repairAnchor, null); assert.match(view.location, /location needs review/);
    assert.ok(!view.source?.includes(':2–4'));
  }
});

test('invalid line numbers can still be repaired but are never shown as a verified range', () => {
  for (const [startLine, endLine] of [[0, 4], [4, 2], [2, Number.MAX_SAFE_INTEGER + 1]]) {
    const revision = baseRevision(); Object.assign(revision.sections[0].claims[0].anchors[0], { startLine, endLine });
    const [view] = presentChapterIssues(revision, [issue()]);
    assert.equal(view.source, 'src/routes.ts'); assert.ok(view.repairAnchor);
  }
});

test('chapter, visual and link issues have readable labels and retain every original finding', () => {
  const revision = baseRevision(), findings = [issue({ sectionId: null, claimId: null, anchorId: null, path: null }),
    issue({ claimId: null, anchorId: null, path: null, visualItemId: 'node:request' }),
    issue({ claimId: null, anchorId: null, path: null, hotspotId: 'hotspot:route' })];
  const views = presentChapterIssues(revision, findings);
  assert.equal(views[0].location, 'Chapter'); assert.equal(views[0].target, null);
  assert.match(views[1].location, /Visual · Request/); assert.match(views[2].location, /Link · Open route code/);
  assert.deepEqual(views.map(view => view.issue), findings);
});

test('opaque identities are replaced only as whole tokens; declared source identifiers remain readable', () => {
  const message = "Claim 'claim:entry' in section 'section:request' does not name 'record_pipeline_operation_path'. Longer claim:entry-extra and src/claim:entry/file.ts remain intact.";
  const [view] = presentChapterIssues(baseRevision(), [issue({ message })]);
  assert.match(view.message, /Claim 'the affected claim' in section 'the affected section'/);
  assert.match(view.message, /record_pipeline_operation_path/);
  assert.match(view.message, /src\/claim:entry\/file.ts/);
  assert.equal(view.issue.message, message);
});

test('singular and plural feedback use the correct verb', () => {
  assert.equal(chapterIssueFeedback(1), '1 issue needs review before acceptance.');
  assert.equal(chapterIssueFeedback(2), '2 issues need review before acceptance.');
});

const componentSource = readFileSync(new URL('../src/ChapterValidationIssues.tsx', import.meta.url), 'utf8');
const compiled = ts.transpileModule(componentSource, { compilerOptions: { jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
const exports = {};
runInThisContext(`(function(exports,require){${compiled}\n})`)(exports, name => {
  if (name === 'react/jsx-runtime') return jsxRuntime;
  if (name === './lib/chapterValidationPresentation') return { presentChapterIssues };
  if (name === './InlineCodeText') return { InlineCodeText: ({ text }) => jsxRuntime.jsx('span', { children: text }) };
  if (name === './design/AppButton') return { AppButton: 'button' };
  throw new Error(`Unexpected component dependency ${name}`);
});
function buttons(element) {
  if (!element || typeof element !== 'object') return [];
  return [...(element.type === 'button' ? [element] : []), ...[element.props?.children].flat(Infinity).flatMap(buttons)];
}
test('real issue component collapses exact technical data and escapes untrusted labels/messages', () => {
  const revision = baseRevision(); revision.sections[0].title = '<script>private()</script>';
  const finding = issue({ message: '<img src=x onerror=private()> section:request' });
  const html = renderToStaticMarkup(createElement(exports.ChapterValidationIssues, { revision, issues: [finding], busy: false, canRepair: true, onReview() {}, onRepair() {} }));
  const visible = html.split('<details>')[0];
  assert.ok(!visible.includes('section:request')); assert.ok(!visible.includes('anchor:route'));
  assert.ok(!html.includes('<script>')); assert.ok(!html.includes('<img ')); assert.match(html, /&lt;script&gt;/);
  assert.match(html, /<details><summary>Technical details<\/summary>/); assert.ok(!html.includes('<details open'));
  assert.match(html, /section:request/); assert.match(html, /anchor:route/); assert.match(html, /src\/routes.ts:2–4/);
});
test('real review and repair buttons call only their uniquely resolved target and obey busy/history gates', () => {
  const revision = baseRevision(), calls = [];
  const props = { revision, issues: [issue()], busy: false, canRepair: true, onReview: (...args) => calls.push(['review', ...args]), onRepair: anchor => calls.push(['repair', anchor]) };
  const controls = buttons(exports.ChapterValidationIssues(props));
  for (const button of controls) button.props.onClick();
  assert.deepEqual(calls, [['review', 'section:request', 'claim:entry'], ['repair', revision.sections[0].claims[0].anchors[0]]]);
  assert.ok(buttons(exports.ChapterValidationIssues({ ...props, busy: true })).every(button => button.props.disabled));
  assert.equal(buttons(exports.ChapterValidationIssues({ ...props, canRepair: false })).length, 1);
  assert.equal(buttons(exports.ChapterValidationIssues({ ...props, issues: [issue({ anchorId: 'missing' })] })).length, 0);
  assert.equal(exports.ChapterValidationIssues({ ...props, issues: [] }), null);
});
