import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';

const matches=(source,pattern)=>assert.ok(pattern.test(source),String(pattern));
const workspace=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
const css=readFileSync(new URL('../src/workspace/workspace.css',import.meta.url),'utf8');

test('opening and closing code always reveals its corresponding narrow pane',()=>{
  matches(workspace,/const setCodeOpen = \(open:boolean\) => \{\s*setNarrowPane\(open\?'source':'explanation'\);/);
  matches(workspace,/aria-label=\{narrowPane==='source'\?'Show explanation':'Show source'\} onClick=\{\(\)=>\{if\(narrowPane==='source'\)setNarrowPane\('explanation'\);else\{setCodeOpen\(true\);setNarrowPane\('source'\);\}\}\}/);
});

test('explaining selected code saves its exact source reveal for this exploration',()=>{
  matches(workspace,/function openFile\(context:Context\) \{ update\(\(s\)=>rememberSourceContext\(s,s\.currentId,context\)\);setReveal\(context\);/);
  matches(workspace,/function explain\(context:Context\)\{[\s\S]*?const selected=bindSelectionToDisplayedSource\(context,source\);[\s\S]*?setCodeRequestReview\(\{context:selected,question,snapshotId,sourceSha256:source\.sha256,[\s\S]*?sourceExcerpt,excerptTruncated\}\);/);
  matches(workspace,/function acceptCodeRequestReview\(\)\{[\s\S]*?if\(!isCurrentCodeRequestReview\(review\)\)/);
  matches(workspace,/openFile\(review\.context\);\s*ask\(review\.question,current\.view==='flow'\?'flow':'guide',\[review\.context\]\);/);
  matches(workspace,/useEffect\(\(\)=>\{setReveal\(current\.reveal\?\?null\);\},\[current\.id,current\.reveal\]\);/);
});

test('view and composer shortcuts reveal their targets before focusing',()=>{
  matches(workspace,/const setView = \(view:View\) => \{setNarrowPane\('explanation'\);update\(/);
  matches(workspace,/function focusComposer\(\)\{setNarrowPane\('explanation'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>composer.current\?\.focus\(\)\);\}/);
  matches(workspace,/if\(mod&&k==='l'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);focusComposer\(\);\}/);
  matches(workspace,/function attach\(context:Context\) \{[\s\S]*?setAttachments\(\(all\)=>\[\.\.\.all,context\]\);focusComposer\(\);/);
});

test('search commands reveal their distinct tools and narrow navigation has a direct return',()=>{
  matches(workspace,/const findInRepository=\(\)=>\{setActivity\('search'\);[\s\S]*?setNarrowPane\('explorer'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>\{search\.current\?\.focus\(\);search\.current\?\.select\(\);\}\);\};/);
  matches(workspace,/const quickOpen=\(\)=>\{setActivity\('files'\);[\s\S]*?setNarrowPane\('explorer'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>\{search\.current\?\.focus\(\);search\.current\?\.select\(\);\}\);\};/);
  matches(workspace,/if\(mod&&event.shiftKey&&k==='f'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);findInRepository\(\);\}[\s\S]*?else if\(mod&&k==='p'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);quickOpen\(\);\}[\s\S]*?else if\(mod&&k==='k'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);onWorkspace\('atlas'\);\}/);
  matches(workspace,/const returnToGuide = \(pane:'explorer'\|'explanation'\) => \{[\s\S]*?setNarrowPane\(pane\);\s*onWorkspace\('guide-plan'\);\s*\};/);
  matches(workspace,/className="dw-sidebar-back"[^\n]*aria-label="Back to explanation"[^\n]*onClick=\{\(\)=>returnToGuide\('explanation'\)\}/);
  matches(css,/\.developer-workspace \.dw-sidebar-back\s*\{display:none;\}/);
  matches(css,/@media\(max-width:680px\)\{\.developer-workspace \.dw-sidebar-back\{display:inline-flex;\}/);
});

test('connection header shows the selected provider model and reasoning in one compact label',()=>{
  const header=readFileSync(new URL('../src/ConnectionHeader.tsx',import.meta.url),'utf8');
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(header,/<span className="connection-header-value">\{label\}<\/span>/);
  matches(header,/ready \? `Codex · \$\{codexModel\} · \$\{connection\.selection\.effort\}`/);
  matches(shell,/\.topbar \.connection-header \{width:100%;max-width:min\(42cqi,220px\);min-width:0;padding-inline:6px;\}/);
  matches(shell,/\.topbar \.connection-header-value \{flex:1;\}/);
});

test('native header keeps desktop controls together and reflows compact controls in normal rows',()=>{
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(shell,/\.app-window\s*\{[^}]*container-type:\s*inline-size;[^}]*container-name:\s*app-window;/);
  matches(shell,/\.topbar\s*\{[^}]*display:\s*flex;[^}]*flex-flow:\s*row wrap;[^}]*position:\s*relative;/);
  matches(shell,/\.topbar-actions\s*\{[^}]*display:\s*flex;[^}]*flex:\s*0 0 auto;[^}]*margin-left:\s*auto;/);
  matches(shell,/\.topbar \.project-controls\s*\{[^}]*flex:\s*0 1 auto;/);
  matches(shell,/\.topbar \.connection-header\s*\{[^}]*flex:\s*0 1 300px;/);
  matches(shell,/@container app-window \(max-width: 1320px\)\s*\{\s*\.topbar\s*\{[^}]*flex-flow:column nowrap;/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar\s*\{[^}]*display:flex;[^}]*flex-flow:row wrap;/);
  matches(shell,/\.project-controls\s*\{[^}]*flex-wrap:\s*wrap/u);
  assert.doesNotMatch(shell,/\.topbar \.topbar-actions\s*\{[^}]*flex-wrap:nowrap;[^}]*width:max-content;/u);
  matches(shell,/@container app-window \(max-width: 1320px\)\s*\{[\s\S]*?\.topbar \.topbar-actions\s*\{display:flex;flex:0 0 auto;flex-flow:row wrap;[^}]*justify-content:flex-start;[^}]*margin-left:0;[^}]*padding-inline:12px;/u);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.topbar-actions\s*\{display:flex;flex:0 0 auto;flex-flow:row wrap;[^}]*justify-content:flex-start;[^}]*margin-left:0;[^}]*padding-inline:12px;/u);
  matches(shell,/\.topbar \.theme-control\s*\{[^}]*min-width:0/u);
  matches(shell,/\.topbar \.theme-control\s*\{[^}]*flex:\s*0 0 auto;/u);
  matches(shell,/\.topbar \.theme-control-trigger\s*\{[^}]*flex:\s*0 0 auto;/u);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.connection-header-value \{flex:1;\}/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.snapshot-refresh-button \{width:34px;min-width:34px;padding-inline:0;\}/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.snapshot-refresh-button > span \{display:none;\}/);
});

test('scene navigation stays in flow and scrolls long scene indexes within their track',()=>{
  const story=readFileSync(new URL('../src/visual-story.css',import.meta.url),'utf8');
  matches(story,/\.story-sticky-controls\s*\{[^}]*position:\s*static;/);
  matches(story,/\.story-toolbar\s*\{[^}]*position:\s*static;/);
  matches(story,/\.story-scene-progress\s*\{[^}]*position:\s*static;/);
  matches(story,/\.story-scene-dots\s*\{[^}]*flex-wrap:\s*nowrap;[^}]*overflow-x:\s*auto;/);
});

test('native high zoom shows one workspace pane at a time with a return path',()=>{
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-panels > #repository-sidebar,[\s\S]*?\.dw-panels > \.dw-separator \{display:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-panels > #reading-workspace \{flex:1!important;min-width:0!important;max-width:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-narrow-explorer \.dw-panels > #repository-sidebar \{display:flex!important;flex:1 1 100%!important;min-width:0!important;max-width:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-narrow-explorer \.dw-panels > #reading-workspace \{display:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.developer-workspace \.dw-sidebar-back \{display:inline-flex;\}/);
});

test('source-linked relationship previews wrap labels in wide and narrow Guide panes',()=>{
  matches(css,/\.dw-answer-visual-preview-graph li \{display:grid;min-width:0;grid-template-columns:minmax\(0,1fr\) minmax\(0,1fr\)/);
  matches(css,/@container answer-visual \(min-width: 720px\) \{[\s\S]*?\.dw-answer-visual-preview-graph > ul > li \{grid-template-columns:minmax\(0,1fr\) minmax\(0,1\.2fr\) minmax\(0,1fr\)/);
  matches(css,/\.dw-answer-visual-preview-graph button \{[^}]*width:100%;[^}]*white-space:normal;/);
  matches(css,/\.dw-answer-visual-edge \{[^}]*min-width:0;[^}]*flex-wrap:wrap;/);
  matches(css,/@container guide-pane \(max-width: 820px\) \{[\s\S]*?\.dw-answer-visual-preview-graph li > \.dw-answer-visual-edge \{grid-column:1 \/ -1;grid-row:2;flex-direction:column;\}/);
  matches(css,/@container guide-pane \(max-width: 420px\) \{[\s\S]*?\.dw-answer-visual-preview-graph li > \.dw-answer-visual-edge svg \{transform:rotate\(90deg\);\}/);
});

test('Flow cards stack their title, meaning, and citation in the narrow reader pane',()=>{
  matches(css,/\.dw-flow-panel \.rt-Button\.dw-linear-flow-step,\.dw-flow-panel \.rt-Button\.dw-unlinked-flow-point\s*\{display:flex;flex-direction:column;align-items:stretch;justify-content:flex-start;text-align:left;white-space:normal;\}/u);
});
