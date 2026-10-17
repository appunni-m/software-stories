import assert from 'node:assert/strict';
import { it } from 'node:test';
import { renderOfflineScene } from '../src/lib/offlineScene.ts';
import { validScenePresentation } from '../src/lib/scenePresentation.ts';

const anchor = { anchorId: 'anchor:one', excerpt: 'run()' };
const visual = { nodes: [{ label: 'Example <input>', evidenceAnchorIds: ['anchor:one'] }] };
const claims = [{ anchors: [anchor] }];
const scene = (kind, value, parentKind = 'row') => ({ claimIndex: 0, title: 'An illustrative representation', mode: 'composition', elements: [
  { kind: parentKind, parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
  { kind, parentIndex: 0, nodeIndex: 0, tone: 'accent', value, valueOrigin: 'illustration' },
] });

it('offline composition preserves every supported data and layout primitive', () => {
  const values = { card: 'A card', token: 'PFR0', code: 'run()', metric: '42', bytes: '50 46 52 30', matrix: '01\n10', table: 'input|output\nA|B', text: 'A useful cue' };
  for (const [kind, value] of Object.entries(values)) {
    for (const group of ['row', 'column', 'grid', 'panel', 'compare', 'stack']) {
      const authored = scene(kind, value, group);
      assert.ok(validScenePresentation({ schemaVersion: 1, scenes: [authored] }, visual.nodes, claims));
      const markup = renderOfflineScene(authored, visual, () => '#evidence-1-1');
      assert.ok(markup.includes(`scene-${kind}`));
      assert.ok(markup.includes(`scene-${group}`));
      assert.match(markup, /Illustrative example/);
      assert.match(markup, /href="#evidence-1-1"/);
      assert.doesNotMatch(markup, /<script|onClick|onclick|<input>/);
      if (kind === 'matrix') {
        assert.match(markup, /2 by 2 cell representation/);
        assert.equal((markup.match(/class="filled"/g) ?? []).length, 2);
      }
      if (kind === 'table') assert.match(markup, /<th scope="col">input<\/th>/);
      if (kind === 'bytes') assert.match(markup, /<b>30<\/b><small>3<\/small>/);
    }
  }
});

it('payload markup stays escaped and only generated package destinations become links', () => {
  const authored = scene('code', '<script>bad()</script><img src=x onerror="bad()">');
  assert.ok(validScenePresentation({ schemaVersion: 1, scenes: [authored] }, visual.nodes, claims));
  for (const destination of ['javascript:bad()', 'file:///private/source.rs', 'https://example.com', '#evidence" onclick="bad()']) {
    const markup = renderOfflineScene(authored, visual, () => destination);
    assert.doesNotMatch(markup, /<script|<img|href=|onclick=/);
    assert.match(markup, /&lt;script&gt;/);
    assert.match(markup, /Supporting source omitted/);
  }
});
