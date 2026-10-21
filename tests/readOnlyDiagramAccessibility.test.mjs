import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readOnlyDiagramAriaLabelConfig } from '../src/lib/readOnlyDiagramAccessibility.ts';

test('read-only diagram descriptions do not promise selection, movement, or deletion', () => {
  const descriptions = Object.values(readOnlyDiagramAriaLabelConfig);
  assert.match(readOnlyDiagramAriaLabelConfig['node.a11yDescription.default'], /read-only/i);
  assert.match(readOnlyDiagramAriaLabelConfig['node.a11yDescription.keyboardDisabled'], /read-only/i);
  assert.match(readOnlyDiagramAriaLabelConfig['edge.a11yDescription.default'], /read-only/i);
  assert.ok(descriptions.every((description) => !/press enter|press space|press delete|arrow keys/i.test(description)));
});

test('Guide and Flow label their diagrams and disable React Flow node and edge keyboard editing', () => {
  const guide = readFileSync(new URL('../src/VisualStory.tsx', import.meta.url), 'utf8');
  const workspace = readFileSync(new URL('../src/workspace/Workspace.tsx', import.meta.url), 'utf8');
  for (const source of [guide, workspace]) {
    assert.match(source, /ariaLabelConfig=\{readOnlyDiagramAriaLabelConfig\}/);
    assert.match(source, /disableKeyboardA11y/);
  }
  assert.match(guide, /aria-label=\{`Read-only visual explanation:/);
  assert.match(workspace, /aria-label=\{`Read-only code relationship diagram:/);
});
