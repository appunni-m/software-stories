import test from 'node:test';
import assert from 'node:assert/strict';
import {rebaseChapterDraft} from '../src/lib/chapterRebase.ts';
import {baseRevision} from './helpers/revision-history-fixture.mjs';

function versions(){
  const base=baseRevision(),latest=structuredClone(base),draft=structuredClone(base);
  latest.revisionId='revision:latest';latest.revisionNumber=2;
  draft.revisionId='revision:local';
  return {base,latest,draft};
}

test('rebasing carries independent title and narration edits over the newest content',()=>{
  const {base,latest,draft}=versions();
  draft.title='My updated chapter title';
  latest.sections[0].prose='The saved version explains what the handler receives.';
  draft.sections[0].spokenText='My narration explains why the request enters here.';
  const result=rebaseChapterDraft(base,latest,draft);
  assert.equal(result.conflicts.length,0);assert.deepEqual(result.unresolved,[]);
  assert.equal(result.revision.title,draft.title);
  assert.equal(result.revision.sections[0].prose,latest.sections[0].prose);
  assert.equal(result.revision.sections[0].spokenText,draft.sections[0].spokenText);
  assert.equal(result.revision.revisionId,latest.revisionId);
  assert.equal(result.revision.basedOnRevisionId,latest.basedOnRevisionId);
});

test('overlapping prose edits require a choice and default to the saved version until resolved',()=>{
  const {base,latest,draft}=versions();
  latest.sections[0].prose='Saved prose changed while the draft was open.';
  draft.sections[0].prose='My unsaved prose explains the same section.';
  const conflictId='section:request:prose';
  const pending=rebaseChapterDraft(base,latest,draft);
  assert.equal(pending.conflicts.length,1);assert.deepEqual(pending.unresolved,[conflictId]);
  assert.equal(pending.revision.sections[0].prose,latest.sections[0].prose);
  const resolved=rebaseChapterDraft(base,latest,draft,{[conflictId]:'draft'});
  assert.deepEqual(resolved.unresolved,[]);
  assert.equal(resolved.revision.sections[0].prose,draft.sections[0].prose);
});

test('linked claims stay atomic when both versions edit them and neither input is changed',()=>{
  const {base,latest,draft}=versions();
  latest.sections[0].claims[0].prose='The saved claim describes dispatch.';
  draft.sections[0].claims[0].prose='My claim explains the route behavior.';
  const before=JSON.stringify({base,latest,draft});
  const conflictId='section:request:claims';
  const result=rebaseChapterDraft(base,latest,draft,{[conflictId]:'draft'});
  assert.equal(result.conflicts[0].id,conflictId);
  assert.equal(result.revision.sections[0].claims[0].prose,draft.sections[0].claims[0].prose);
  assert.equal(JSON.stringify({base,latest,draft}),before,'rebase does not mutate its inputs');
});

test('same-section delete and edit conflict explicitly; unchanged latest content accepts the draft deletion',()=>{
  const {base,latest,draft}=versions();
  draft.sections=[];
  let result=rebaseChapterDraft(base,latest,draft);
  assert.deepEqual(result.revision.sections,[]);assert.equal(result.conflicts.length,0);
  latest.sections[0].prose='A concurrent edit changed this section.';
  result=rebaseChapterDraft(base,latest,draft);
  assert.equal(result.conflicts[0].id,'section:request:presence');
  assert.deepEqual(result.unresolved,['section:request:presence']);
  assert.equal(result.revision.sections[0].prose,latest.sections[0].prose);
  result=rebaseChapterDraft(base,latest,draft,{'section:request:presence':'draft'});
  assert.deepEqual(result.revision.sections,[]);
});

test('rebase refuses revisions from different snapshots or targets',()=>{
  const {base,latest,draft}=versions();latest.snapshotId='snapshot:other';
  assert.throws(()=>rebaseChapterDraft(base,latest,draft),/same project, snapshot and target/);
});
