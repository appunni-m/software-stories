import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { runInThisContext } from 'node:vm';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import ts from 'typescript';
import * as tree from '../src/workspace/explorerTree.ts';
import * as atlas from '../src/lib/atlas.ts';

const paths = ['README.md', 'src/main.rs', 'src/io/read.rs', 'src/io/write.rs', 'tests/read.rs'];
const expanded = () => new Set(['src', 'src/io', 'tests']);
const rows = () => tree.explorerRows(tree.buildExplorerTree(paths), expanded(), null);

test('captured explorer synthesizes folders once, deduplicates paths and sorts folders before files', () => {
  const built = tree.buildExplorerTree([...paths, ...paths]);
  assert.equal(built.byPath.size, 8);
  assert.deepEqual(built.roots.map(row => row.path), ['src', 'tests', 'README.md']);
  assert.deepEqual(tree.explorerRows(built, expanded(), null).map(row => row.path), [
    'src', 'src/io', 'src/io/read.rs', 'src/io/write.rs', 'src/main.rs', 'tests', 'tests/read.rs', 'README.md',
  ]);
  assert.deepEqual(tree.explorerAncestors('src/io/read.rs'), ['src', 'src/io']);
  assert.deepEqual(tree.explorerAncestors('README.md'), []);
});

test('filter keeps exact matching files and ancestors without changing ordinary expansion', () => {
  const built = tree.buildExplorerTree(paths), ordinary = new Set(['tests']);
  const matches = tree.matchingExplorerPaths(built, 'READ.RS');
  const filtered = tree.explorerRows(built, ordinary, matches);
  assert.deepEqual(filtered.map(row => row.path), ['src', 'src/io', 'src/io/read.rs', 'tests', 'tests/read.rs']);
  assert.deepEqual([...ordinary], ['tests']);
  assert.deepEqual(filtered.map(row => [row.depth, row.positionInSet, row.setSize]), [[0, 1, 2], [1, 1, 1], [2, 1, 1], [0, 2, 2], [1, 1, 1]]);
  assert.deepEqual(tree.explorerRows(built, ordinary, matches, new Set(['src'])).map(row => row.path), ['src', 'tests', 'tests/read.rs']);
  assert.deepEqual(tree.explorerRows(built, ordinary, tree.matchingExplorerPaths(built, 'nothing')), []);
  assert.deepEqual(tree.explorerRows(built, ordinary, null).map(row => row.path), ['src', 'tests', 'tests/read.rs', 'README.md']);
});

test('arrow navigation shares the existing tree contract including parent, child, Home and End', () => {
  const visible = rows();
  const key = (index, value) => atlas.atlasTreeKeyboardAction(visible, index, value);
  assert.deepEqual(key(1, 'ArrowRight'), { kind: 'focus', index: 2 });
  assert.deepEqual(key(2, 'ArrowLeft'), { kind: 'focus', index: 1 });
  assert.deepEqual(key(1, 'ArrowLeft'), { kind: 'collapse', path: 'src/io' });
  assert.deepEqual(key(3, 'Home'), { kind: 'focus', index: 0 });
  assert.deepEqual(key(3, 'End'), { kind: 'focus', index: 7 });
  assert.deepEqual(key(7, 'ArrowDown'), { kind: 'focus', index: 7 });
  assert.deepEqual(key(0, 'ArrowUp'), { kind: 'focus', index: 0 });
  assert.deepEqual(key(2, 'Enter'), { kind: 'activate' });
  assert.deepEqual(key(2, ' '), { kind: 'select' });
  assert.deepEqual(key(2, 'Tab'), { kind: 'none' });
  assert.deepEqual(key(-1, 'Enter'), { kind: 'none' });
});

test('collapsed and filtered targets retain the closest visible parent rather than another file', () => {
  const built = tree.buildExplorerTree(paths);
  const collapsed = tree.explorerRows(built, new Set(['src']), null);
  assert.equal(tree.explorerActivePath(collapsed, 'src/io/read.rs', 'README.md'), 'src/io');
  assert.equal(tree.explorerActivePath(collapsed, 'missing.rs', 'README.md'), 'README.md');
  assert.equal(tree.explorerActivePath(collapsed, null, null), 'src');
  assert.equal(tree.explorerActivePath([], 'src/io/read.rs', 'README.md'), null);
});

test('typeahead continues a word, cycles a repeated letter, wraps and resets after the timeout', () => {
  const visible = rows(), empty = { text: '', at: -Infinity };
  const first = tree.explorerTypeahead(visible, 'src', 'r', empty, 1);
  assert.equal(first.path, 'src/io/read.rs');
  assert.equal(tree.explorerTypeahead(visible, first.path, 'e', first.state, 2).path, 'src/io/read.rs');
  const repeated = tree.explorerTypeahead(visible, first.path, 'r', first.state, 2);
  assert.equal(repeated.path, 'tests/read.rs');
  assert.equal(tree.explorerTypeahead(visible, repeated.path, 'r', repeated.state, 3).path, 'README.md');
  assert.equal(tree.explorerTypeahead(visible, first.path, 'w', first.state, 900).path, 'src/io/write.rs');
  assert.equal(tree.explorerTypeahead(visible, first.path, 'z', empty, 1).path, null);
  assert.equal(tree.explorerTypeahead([], null, 'r', empty, 1).path, null);
});

const file = new URL('../src/workspace/WorkspaceExplorer.tsx', import.meta.url);
const compiled = ts.transpileModule(readFileSync(file, 'utf8'), { fileName: file.pathname, compilerOptions: {
  module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX,
} }).outputText;
const require = createRequire(import.meta.url);
function loadComponent(react) {
  const module = { exports: {} };
  runInThisContext(`(function(require,module,exports){${compiled}\n})`, { filename: file.pathname })(name => {
    if (name === './explorerTree') return tree;
    if (name === '../lib/atlas') return atlas;
    if (name === 'react' && react) return react;
    assert.ok(['react', 'react/jsx-runtime', 'lucide-react'].includes(name), `unexpected Explorer dependency ${name}`);
    return require(name);
  }, module, module.exports);
  return module.exports.WorkspaceExplorer;
}
const WorkspaceExplorer = loadComponent();

test('the real component renders a bounded accessible window for 100,000 files', () => {
  const large = Array.from({ length: 100_000 }, (_, i) => `src/group-${Math.floor(i / 100)}/file-${i}.rs`);
  const all = new Set(['src', ...Array.from({ length: 1_000 }, (_, i) => `src/group-${i}`)]);
  const html = renderToStaticMarkup(createElement(WorkspaceExplorer, { paths: large, query: '', selectedPath: large.at(-1), expanded: all,
    onExpandedChange() {}, onOpen() {}, treeRef: { current: null } }));
  assert.match(html, /role="tree" aria-label="Captured files" tabindex="0"/);
  const count = [...html.matchAll(/role="treeitem"/g)].length;
  assert.equal(count, 22, '15 visible rows, six overscan and one offscreen active descendant');
  const active = /aria-activedescendant="([^"]+)"/.exec(html)?.[1];
  assert.ok(active);
  assert.ok(html.includes(`id="${active}"`), 'the active descendant exists even outside the rendered scroll window');
  assert.match(html, /aria-level="3" aria-posinset="100" aria-setsize="100" aria-selected="true"/);
  assert.equal([...html.matchAll(/tabindex="0"/g)].length, 1, 'the complete tree has one Tab stop');
  assert.ok(html.length < 35_000, 'rendered markup stays bounded by viewport, not file count');
});

test('untrusted filenames are escaped and empty filters have an explicit result', () => {
  const props = { paths: ['src/<script>unsafe().rs'], query: '', selectedPath: null, expanded: new Set(['src']), onExpandedChange() {}, onOpen() {}, treeRef: { current: null } };
  const html = renderToStaticMarkup(createElement(WorkspaceExplorer, props));
  assert.ok(!html.includes('<script>'));
  assert.match(html, /&lt;script&gt;unsafe\(\)\.rs/);
  const empty = renderToStaticMarkup(createElement(WorkspaceExplorer, { ...props, query: 'missing' }));
  assert.match(empty, /role="status">No matching files\./);
  assert.ok(!empty.includes('aria-activedescendant='));
});

/** Exercise the real component's event handlers with hook storage and a scroll
 * boundary. Native focus/speech and ResizeObserver still require app review. */
function interactiveExplorer() {
  const slots = []; let cursor = 0;
  const react = { useId: () => 'explorer-test', useMemo: fn => fn(), useLayoutEffect() {},
    useState(initial) { const i = cursor++; if (!(i in slots)) slots[i] = typeof initial === 'function' ? initial() : initial;
      return [slots[i], next => { slots[i] = typeof next === 'function' ? next(slots[i]) : next; }]; },
    useRef(initial) { const i = cursor++; if (!(i in slots)) slots[i] = { current: initial }; return slots[i]; },
  };
  const Component = loadComponent(react), opened = [], scroll = { scrollTop: 0, clientHeight: 84, focus() {} };
  const props = { paths, query: '', selectedPath: 'src/main.rs', expanded: expanded(), onExpandedChange(next) { props.expanded = next; }, onOpen(path) { opened.push(path); }, treeRef: { current: scroll } };
  const render = () => { cursor = 0; return Component(props); };
  const children = element => [element.props?.children].flat(Infinity).filter(Boolean);
  const entries = element => children(element).flatMap(child => child.props?.role === 'treeitem' ? [child] : children(child));
  const active = element => entries(element).find(child => child.props?.id === element.props['aria-activedescendant']);
  const key = (value, extra = {}) => { let prevented = false; render().props.onKeyDown({ key: value, nativeEvent: { isComposing: false }, preventDefault() { prevented = true; }, ...extra }); return prevented; };
  return { props, render, entries, active, key, opened, scroll };
}

test('real Explorer handlers navigate and scroll without opening files until activation', () => {
  const h = interactiveExplorer();
  assert.equal(h.active(h.render()).props.title, 'src/main.rs');
  assert.equal(h.key('Home'), true);
  assert.equal(h.active(h.render()).props.title, 'src');
  h.key('ArrowRight'); h.key('ArrowRight');
  assert.equal(h.active(h.render()).props.title, 'src/io/read.rs');
  assert.deepEqual(h.opened, []);
  h.key('Enter');
  assert.deepEqual(h.opened, ['src/io/read.rs']);
  h.key('End');
  assert.equal(h.active(h.render()).props.title, 'README.md');
  assert.equal(h.scroll.scrollTop, 140);
  h.key(' ');
  assert.deepEqual(h.opened, ['src/io/read.rs', 'README.md']);
  assert.equal(h.key('Tab'), false);
  assert.equal(h.key('a', { metaKey: true }), false);
  assert.equal(h.key('a', { nativeEvent: { isComposing: true } }), false);
});

test('real folder handlers support collapse, parent/child traversal and filtered collapse', () => {
  const h = interactiveExplorer();
  h.key('Home'); h.key('ArrowRight');
  assert.equal(h.active(h.render()).props.title, 'src/io');
  h.key('ArrowLeft');
  assert.equal(h.props.expanded.has('src/io'), false);
  assert.equal(h.active(h.render()).props['aria-expanded'], false);
  h.key('ArrowRight'); h.key('ArrowRight'); h.key('ArrowLeft');
  assert.equal(h.active(h.render()).props.title, 'src/io');
  h.props.query = 'read.rs'; h.key('Home'); h.key('ArrowLeft');
  assert.equal(h.active(h.render()).props.title, 'src');
  assert.equal(h.active(h.render()).props['aria-expanded'], false);
  h.key('ArrowRight'); h.key('ArrowRight');
  assert.equal(h.active(h.render()).props.title, 'src/io');
});

test('real file drag retains the scoped chat payload and folders cannot be dragged as source', () => {
  const h = interactiveExplorer(), payload = new Map(), dataTransfer = { setData(key, value) { payload.set(key, value); } };
  const file = h.entries(h.render()).find(row => row.props.title === 'src/main.rs');
  file.props.onDragStart({ dataTransfer });
  assert.deepEqual(JSON.parse(payload.get('application/x-code-atlas-context')), { path: 'src/main.rs' });
  assert.equal(dataTransfer.effectAllowed, 'copy');
  let prevented = false;
  const folder = h.entries(h.render()).find(row => row.props.title === 'src');
  folder.props.onDragStart({ preventDefault() { prevented = true; }, dataTransfer });
  assert.equal(prevented, true);
});
