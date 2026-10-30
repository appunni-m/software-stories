import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { sectionPosition } from '../src/workspace/model.ts';

test('Guide keeps a multi-section answer focused on one stable section', () => {
  const sections = [{ sectionId: 'first' }, { sectionId: 'second' }, { sectionId: 'third' }];
  assert.equal(sectionPosition('revision-1', sections, { revisionId: 'revision-1', sectionId: 'second' }), 1);
  assert.equal(sectionPosition('revision-2', sections, { revisionId: 'revision-1', sectionId: 'second' }), 0);
  assert.equal(sectionPosition('revision-1', sections, { revisionId: 'revision-1', sectionId: 'removed' }), 0);

  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.match(workspace, /const position=revision\?sectionPosition\(revision\.revisionId,revision\.sections,answerReadingPosition\(turn,'guide',revision\.revisionId\)\):0/);
  assert.match(workspace, /const selectSection=\(index:number\)=>/);
  assert.match(workspace, /onSection\(revision\.revisionId,next\.sectionId\)/);
  assert.match(workspace, /revision\.sections\.length>1&&<div className="dw-section-controls">[\s\S]*?aria-label="Previous section"[\s\S]*?Section \{position\+1\} of \{revision\.sections\.length\}[\s\S]*?Next section/);
  assert.match(workspace, /disabled=\{position===0\}[\s\S]*?disabled=\{position>=revision\.sections\.length-1\}/);

  const story = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
  assert.match(story, /Audio completion, not guessed[\s\S]*?Sections always advance manually/);
  assert.match(story, /if \(index < scenes\.length - 1\) select\(index \+ 1, true\); else if \(!sectionCompletionSent\.current\)/);
});

test('single-section answers do not render section navigation controls', () => {
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  assert.match(workspace, /revision\.sections\.length>1&&<div className="dw-section-controls"/);
});
