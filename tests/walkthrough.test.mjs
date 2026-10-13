import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  chooseSectionId,
  latestAcceptedRevisions,
  orderedSections,
  parseRememberedSection,
  rememberedSectionRecord,
  resumeStorageKey,
} from '../src/lib/walkthrough.ts';

const section = (sectionId, order) => ({ sectionId, order });
const revision = (revisionId, chapterId, revisionNumber, lifecycle = 'accepted', sections = []) => ({
  revisionId,
  chapterId,
  revisionNumber,
  lifecycle,
  createdAtUnixMs: revisionNumber,
  title: revisionId,
  sections,
});

describe('walkthrough chapter selection', () => {
  it('shows only the newest accepted revision for each chapter', () => {
    const result = latestAcceptedRevisions([
      revision('one-v1', 'one', 1, 'accepted', [section('a', 1)]),
      revision('one-draft', 'one', 9, 'draft', [section('draft', 1)]),
      revision('one-v2', 'one', 2, 'accepted', [section('b', 1)]),
      revision('empty', 'empty', 1, 'accepted'),
      revision('two-v1', 'two', 1, 'accepted', [section('c', 1)]),
    ]);
    assert.deepEqual(result.map((item) => item.revisionId), ['one-v2', 'two-v1']);
  });

  it('orders sections and restores route selection before remembered selection', () => {
    const sections = [section('last', 3), section('first', 1), section('middle', 2)];
    const sorted = orderedSections({ sections });
    assert.deepEqual(sorted.map((item) => item.sectionId), ['first', 'middle', 'last']);
    assert.equal(chooseSectionId(sorted, 'last', 'middle'), 'last');
    assert.equal(chooseSectionId(sorted, 'deleted', 'middle'), 'middle');
    assert.equal(chooseSectionId(sorted, 'deleted', 'also-deleted'), 'first');
    assert.equal(chooseSectionId([], null, null), null);
  });
});

describe('walkthrough resume record', () => {
  it('scopes progress to a project, chapter, and exact accepted revision', () => {
    const serialized = rememberedSectionRecord('project/one', 'chapter', 'revision-2', 'section-4');
    assert.match(resumeStorageKey('project/one', 'chapter', 'revision-2'), /project%2Fone/);
    assert.equal(parseRememberedSection(serialized, 'project/one', 'chapter', 'revision-2'), 'section-4');
    assert.equal(parseRememberedSection(serialized, 'project/one', 'chapter', 'revision-3'), null);
    assert.equal(parseRememberedSection(serialized, 'another-project', 'chapter', 'revision-2'), null);
  });

  it('ignores missing, malformed, or incomplete progress', () => {
    assert.equal(parseRememberedSection(null, 'p', 'c', 'r'), null);
    assert.equal(parseRememberedSection('{oops', 'p', 'c', 'r'), null);
    assert.equal(parseRememberedSection('{"version":1,"projectId":"p","chapterId":"c","revisionId":"r"}', 'p', 'c', 'r'), null);
  });
});
