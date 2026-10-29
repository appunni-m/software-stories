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
  matches(shell,/\.topbar \.connection-header \{max-width:min\(42vw,220px\);min-width:0;padding-inline:6px;flex:0 1 auto;\}/);
  matches(shell,/\.topbar \.connection-header-value \{flex:1;\}/);
});

test('native high zoom reflows the project and account header without hiding core controls',()=>{
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(shell,/\.topbar\s*\{[^}]*flex:\s*0 0 auto;[^}]*display:\s*grid;[^}]*grid-template-areas:\s*"brand project actions";/);
  matches(shell,/\.topbar\s*\{[^}]*grid-template-columns:\s*max-content minmax\(0,1\.15fr\) minmax\(0,\.85fr\)/);
  matches(shell,/@media\s*\(max-width:\s*1180px\)\s*\{\s*\.topbar\s*\{[^}]*grid-template-areas:"project" "actions";/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar\s*\{[^}]*grid-template-areas:"project" "actions";[^}]*grid-template-rows:auto auto;/);
  matches(shell,/\.topbar-actions\s*\{[^}]*grid-area:\s*actions;/);
  matches(shell,/\.project-controls\s*\{[^}]*grid-area:\s*project;/);
  matches(shell,/\.project-controls\s*\{[^}]*flex-wrap:\s*wrap/u);
  matches(shell,/\.topbar-actions\s*\{[^}]*display:\s*grid;[^}]*grid-template-columns:\s*minmax\(0,1fr\) auto auto auto/u);
  matches(shell,/\.topbar \.topbar-actions\s*\{[^}]*grid-template-columns:minmax\(0,min\(42vw,260px\)\) auto auto auto;[^}]*justify-content:start/u);
  matches(shell,/\.topbar \.theme-control\s*\{[^}]*min-width:0/u);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.connection-header-value \{flex:1;\}/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.snapshot-refresh-button \{width:34px;min-width:34px;padding-inline:0;\}/);
  matches(shell,/html\[data-app-zoom-compact="true"\] \.topbar \.snapshot-refresh-button > span \{display:none;\}/);
});

test('native high zoom shows one workspace pane at a time with a return path',()=>{
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-panels > #repository-sidebar,[\s\S]*?\.dw-panels > \.dw-separator \{display:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-panels > #reading-workspace \{flex:1!important;min-width:0!important;max-width:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-narrow-explorer \.dw-panels > #repository-sidebar \{display:flex!important;flex:1 1 100%!important;min-width:0!important;max-width:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-narrow-explorer \.dw-panels > #reading-workspace \{display:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.developer-workspace \.dw-sidebar-back \{display:inline-flex;\}/);
});

test('source-linked relationship previews wrap labels in wide and narrow Guide panes',()=>{
  matches(css,/\.dw-answer-visual-preview-graph li \{[^}]*min-width:0;[^}]*grid-template-columns:minmax\(0,1fr\) minmax\(0,1\.2fr\) minmax\(0,1fr\)/);
  matches(css,/\.dw-answer-visual-preview-graph button \{[^}]*width:100%;[^}]*white-space:normal;/);
  matches(css,/\.dw-answer-visual-edge \{[^}]*min-width:0;[^}]*flex-wrap:wrap;/);
  matches(css,/@container guide-pane \(max-width: 700px\) \{[\s\S]*?\.dw-answer-visual-preview-graph li > \.dw-answer-visual-edge \{grid-column:1 \/ -1;grid-row:2;\}/);
  matches(css,/@container guide-pane \(max-width: 420px\) \{[\s\S]*?\.dw-answer-visual-preview-graph li > \.dw-answer-visual-edge svg \{transform:rotate\(90deg\);\}/);
});
