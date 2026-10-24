import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';

const matches=(source,pattern)=>assert.ok(pattern.test(source),String(pattern));
const workspace=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
const css=readFileSync(new URL('../src/workspace/workspace.css',import.meta.url),'utf8');

test('opening and closing code always reveals its corresponding narrow pane',()=>{
  matches(workspace,/const setCodeOpen = \(open:boolean\) => \{\s*setNarrowPane\(open\?'source':'explanation'\);/);
  matches(workspace,/aria-label=\{narrowPane==='source'\?'Show explanation':'Show source'\} onClick=\{\(\)=>\{if\(narrowPane==='source'\)setNarrowPane\('explanation'\);else setCodeOpen\(true\);\}\}/);
});

test('view and composer shortcuts reveal their targets before focusing',()=>{
  matches(workspace,/const setView = \(view:View\) => \{setNarrowPane\('explanation'\);/);
  matches(workspace,/function focusComposer\(\)\{setNarrowPane\('explanation'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>composer.current\?\.focus\(\)\);\}/);
  matches(workspace,/if\(mod&&k==='l'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);focusComposer\(\);\}/);
  matches(workspace,/function attach\(context:Context\)[^\n]*focusComposer\(\);\}/);
});

test('search commands reveal the sidebar and narrow navigation has a direct return',()=>{
  matches(workspace,/const find=\(\)=>\{setActivity\('search'\);setNarrowPane\('explorer'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>search.current\?\.focus\(\)\);\};/);
  matches(workspace,/if\(mod&&\(k==='p'\|\|k==='k'\|\|\(event.shiftKey&&k==='f'\)\)\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);find\(\);\}/);
  matches(workspace,/const returnToGuide = \(pane:'explorer'\|'explanation'\) => \{[\s\S]*?setNarrowPane\(pane\);\s*onWorkspace\('guide-plan'\);\s*\};/);
  matches(workspace,/className="dw-sidebar-back"[^\n]*aria-label="Back to explanation"[^\n]*onClick=\{\(\)=>returnToGuide\('explanation'\)\}/);
  matches(css,/\.developer-workspace \.dw-sidebar-back\s*\{display:none;\}/);
  matches(css,/@media\(max-width:680px\)\{\.developer-workspace \.dw-sidebar-back\{display:inline-flex;\}/);
});

test('enlarged headers keep the provider name instead of truncating two labels',()=>{
  const header=readFileSync(new URL('../src/ConnectionHeader.tsx',import.meta.url),'utf8');
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(header,/<span className="connection-header-label">Connections<\/span><span className="connection-header-value">\{label\}<\/span>/);
  matches(shell,/@media\(max-width:800px\)\{\s*\.topbar \.connection-header \{flex-shrink:0;\}/);
  matches(shell,/\.topbar \.connection-header-label \{display:none;\}/);
  matches(shell,/\.topbar \.connection-header-value \{padding-left:0;border-left:0;\}/);
});

test('native high zoom reflows the project and account header without hiding core controls',()=>{
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(shell,/html\[data-app-zoom-compact="true"\] \.app-window > \.topbar\s*\{[^}]*display:grid;[^}]*grid-template-columns:minmax\(0,1fr\);[^}]*grid-template-rows:auto auto;[^}]*flex:0 0 auto;/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.project-controls\s*\{[^}]*grid-row:1;[^}]*flex-wrap:wrap;/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.topbar-actions\s*\{[^}]*grid-row:2;[^}]*flex-wrap:wrap;/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.connection-header-value \{padding-left:0;border-left:0;\}/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.search-control > span,[\s\S]*?\.topbar \.search-control kbd \{display:none;\}/);
});
