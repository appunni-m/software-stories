import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  chooseSectionId,
  latestAcceptedRevisions,
  readerSelection,
  orderedSections,
  parseRememberedProgress,
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
    const serialized = rememberedSectionRecord('project/one', 'chapter', 'revision-2', 'section-4', 18.75, 420);
    assert.match(resumeStorageKey('project/one', 'chapter', 'revision-2'), /project%2Fone/);
    assert.equal(parseRememberedSection(serialized, 'project/one', 'chapter', 'revision-2'), 'section-4');
    assert.deepEqual(parseRememberedProgress(serialized, 'project/one', 'chapter', 'revision-2'), {
      sectionId: 'section-4', playbackPosition: 18.75, transcriptPosition: 420,
    });
    assert.equal(parseRememberedSection(serialized, 'project/one', 'chapter', 'revision-3'), null);
    assert.equal(parseRememberedSection(serialized, 'another-project', 'chapter', 'revision-2'), null);
  });

  it('ignores missing, malformed, or incomplete progress', () => {
    assert.equal(parseRememberedSection(null, 'p', 'c', 'r'), null);
    assert.equal(parseRememberedSection('{oops', 'p', 'c', 'r'), null);
    assert.equal(parseRememberedSection('{"version":1,"projectId":"p","chapterId":"c","revisionId":"r"}', 'p', 'c', 'r'), null);
    assert.equal(parseRememberedProgress('{"version":2,"projectId":"p","chapterId":"c","revisionId":"r","sectionId":"s","playbackPosition":-1,"transcriptPosition":0}', 'p', 'c', 'r'), null);
    assert.deepEqual(parseRememberedProgress('{"version":1,"projectId":"p","chapterId":"c","revisionId":"r","sectionId":"s"}', 'p', 'c', 'r'), {
      sectionId: 's', playbackPosition: 0, transcriptPosition: 0,
    });
  });
});

it('explicit draft playback preserves exact identity and never enters accepted exports',()=>{
  const accepted=revision('accepted','one',1,'accepted',[section('a',1)]);
  const draft=revision('draft','one',2,'draft',[section('b',1)]);
  const rejected=revision('rejected','two',1,'rejected',[section('c',1)]);
  const all=[accepted,draft,rejected];
  const preview=readerSelection(all,'one','draft');
  assert.equal(preview.selected,draft);
  assert.deepEqual(preview.library,[draft]);
  assert.deepEqual(preview.exportable,[accepted]);
  assert.equal(readerSelection(all,'one','missing').selected,null);
  assert.equal(readerSelection(all,'two','rejected').selected,null);
  assert.equal(readerSelection(all,'one').selected,accepted);
  assert.equal(readerSelection([draft],'one','draft').selected,draft);
});
