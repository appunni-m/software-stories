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

test('explaining selected code submits one source-bound question at the current exploration level',()=>{
  matches(workspace,/function openFile\(context:Context\) \{[\s\S]*?update\(\(s\)=>rememberSourceContext\(s,s\.currentId,context\)\);[\s\S]*?setReveal\(\{\.\.\.context\}\);[\s\S]*?setCodeOpen\(true\);setNarrowPane\('source'\);/);
  const explainStart=workspace.indexOf('function askAboutSelectedCode(context:Context){');
  const explainEnd=workspace.indexOf('function explainSafely',explainStart);
  const explainBlock=workspace.slice(explainStart,explainEnd);
  matches(explainBlock,/const selected=verifiedCodeContext\(context,snapshotId\);[\s\S]*?const question=`Explain what \$\{label\(selected\)\} does and how it works\.`;[\s\S]*?ask\(question,current\.view==='flow'\?'flow':'guide',\[selected\]\);/);
  assert.doesNotMatch(workspace,/pendingCodeExplanation|setPendingCodeExplanation|Review your code question|dw-code-review-dialog/);
  matches(workspace,/if\(!requireCodexConnection\(\)\)\{\s*updateWorkspaceDraft\(projectId,snapshotId,current\.id,\{question:clean,context\}\);\s*return false;\s*\}/);
  matches(workspace,/if\(state\.grantId\)\s*void runQuestion\(projectId,snapshotId,id,state\.grantId\);\s*else\s*\{setConsentError\(null\);setConsentTurn\(id\);\}/);
  matches(workspace,/function verifiedDisplayedContext\(context:Context,expectedSnapshotId:string\):Context\|null \{[\s\S]*?bindSelectionToDisplayedSource\(context,source\)[\s\S]*?source\.path!==selected\.path[\s\S]*?source\.snapshotId!==snapshotId[\s\S]*?selected\.sha256===source\.sha256/);
  matches(workspace,/function verifiedCodeContext\(context:Context,expectedSnapshotId:string\):Context\|null \{[\s\S]*?verifiedDisplayedContext\(context,expectedSnapshotId\)[\s\S]*?selected\.from>=1[\s\S]*?selected\.to<=lineCount[\s\S]*?return rangeValid\?selected:null/);
  matches(workspace,/function explainSafely\(context:Context\)\s*\{\s*try\s*\{askAboutSelectedCode\(context\);\}\s*catch\(failure\)\s*\{[\s\S]*?setDraftNotice\(`The selected code question could not be started\./);
  matches(workspace,/useEffect\(\(\)=>\{setReveal\(current\.reveal\?\?null\);\},\[current\.id,current\.reveal\]\);/);
});

test('view and composer shortcuts reveal their targets before focusing',()=>{
  matches(workspace,/const setView = \(view:View\) => \{[\s\S]*?setNarrowPane\('explanation'\);update\(/);
  matches(workspace,/function focusComposer\(\)\{setNarrowPane\('explanation'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>composer.current\?\.focus\(\)\);\}/);
  matches(workspace,/if\(mod&&k==='l'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);focusComposer\(\);\}/);
  matches(workspace,/function attach\(context:Context\) \{[\s\S]*?setAttachments\(\(all\)=>\[\.\.\.all,context\]\);focusComposer\(\);/);
});

test('search commands reveal their distinct tools and narrow navigation has a direct return',()=>{
  matches(workspace,/const findInRepository=\(\)=>\{setActivity\('search'\);[\s\S]*?setNarrowPane\('explorer'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>\{search\.current\?\.focus\(\);search\.current\?\.select\(\);\}\);\};/);
  matches(workspace,/const quickOpen=\(\)=>\{setActivity\('files'\);[\s\S]*?setNarrowPane\('explorer'\);onWorkspace\('guide-plan'\);requestAnimationFrame\(\(\)=>\{search\.current\?\.focus\(\);search\.current\?\.select\(\);\}\);\};/);
  matches(workspace,/if\(mod&&event.shiftKey&&k==='f'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);findInRepository\(\);\}[\s\S]*?else if\(mod&&k==='p'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);quickOpen\(\);\}[\s\S]*?else if\(mod&&k==='k'\)\{event.preventDefault\(\);event.stopImmediatePropagation\(\);setActivity\('files'\);[\s\S]*?showView\('flow'\);/);
  matches(workspace,/const returnToGuide = \(pane:'explorer'\|'explanation'\) => \{[\s\S]*?setNarrowPane\(pane\);\s*onWorkspace\('guide-plan'\);\s*\};/);
  matches(workspace,/className="dw-sidebar-back"[^\n]*aria-label="Back to explanation"[^\n]*onClick=\{\(\)=>returnToGuide\('explanation'\)\}/);
  matches(css,/\.developer-workspace \.dw-sidebar-back\s*\{display:none;\}/);
  matches(css,/@media\(max-width:680px\)\{\.developer-workspace \.dw-sidebar-back\{display:inline-flex;\}/);
});

test('search results stay inside the resizable repository sidebar',()=>{
  matches(css,/\.dw-search-results > button:not\(\.rt-Button\):not\(\.dw-search-file-match\)\s*\{display:flex;/);
  matches(css,/\.dw-sidebar\s*\{[^}]*min-width:0;/);
  matches(css,/\.dw-search-results\s*\{[^}]*width:100%;[^}]*max-width:100%;[^}]*min-width:0;/);
  matches(css,/\.dw-search-file-heading\s*\{[^}]*width:100%;[^}]*max-width:100%;[^}]*min-width:0;/);
  matches(css,/\.dw-search-results \.dw-search-file-match\s*\{[^}]*display:grid;[^}]*grid-template-columns:minmax\(0,1fr\);[^}]*width:100%;[^}]*max-width:100%;[^}]*min-width:0;[^}]*overflow:hidden;/);
  matches(css,/\.dw-search-file-match > strong\s*\{[^}]*min-width:0;[^}]*overflow:hidden;[^}]*text-overflow:ellipsis;/);
  matches(css,/\.dw-search-file-match > span\s*\{[^}]*overflow:hidden;[^}]*text-overflow:ellipsis;[^}]*white-space:nowrap;/);
});

test('connection header shows the selected provider and model without settings detail',()=>{
  const header=readFileSync(new URL('../src/ConnectionHeader.tsx',import.meta.url),'utf8');
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(header,/<span className="connection-header-value">\{label\}<\/span>/);
  matches(header,/ready \? `Codex · \$\{codexModel\}`/);
  matches(shell,/\.topbar \.connection-header \{width:100%;max-width:min\(42cqi,220px\);min-width:0;padding-inline:6px;\}/);
  matches(shell,/\.topbar \.connection-header-value \{flex:1;\}/);
});

test('native header keeps desktop controls together and reflows compact controls in normal rows',()=>{
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(shell,/\.app-window\s*\{[^}]*container-type:\s*inline-size;[^}]*container-name:\s*app-window;/);
  matches(shell,/\.topbar\s*\{[^}]*display:\s*grid;[^}]*grid-template-columns:\s*minmax\(0,max-content\) minmax\(0,1fr\) minmax\(0,max-content\);[^}]*grid-template-areas:\s*"brand project actions";[^}]*position:\s*relative;/);
  matches(shell,/\.topbar-actions\s*\{[^}]*display:\s*flex;[^}]*flex:\s*0 1 auto;[^}]*flex-wrap:\s*wrap;[^}]*min-width:\s*0;[^}]*max-width:\s*100%;[^}]*margin-left:\s*0;/);
  matches(shell,/\.topbar \.project-controls\s*\{[^}]*flex:\s*0 1 auto;/);
  matches(shell,/\.topbar \.connection-header\s*\{[^}]*flex:\s*0 1 300px;/);
  matches(shell,/@container app-window \(max-width: 1280px\)\s*\{\s*\.topbar\s*\{[^}]*grid-template-columns:auto minmax\(0,1fr\);[^}]*grid-template-areas:"brand actions" "project project";/);
  matches(shell,/@container app-window \(max-width: 680px\)\s*\{\s*\.topbar\s*\{[^}]*grid-template-columns:minmax\(0,1fr\);[^}]*grid-template-areas:"brand" "actions" "project";/);
  matches(shell,/\.project-controls\s*\{[^}]*flex-wrap:\s*wrap/u);
  assert.doesNotMatch(shell,/\.topbar \.topbar-actions\s*\{[^}]*flex-wrap:nowrap;[^}]*width:max-content;/u);
  matches(shell,/\.topbar \.topbar-actions\s*\{[^}]*padding-right:8px;/u);
  matches(shell,/html\[data-app-zoom-header-compact="true"\] \.topbar \.topbar-actions\s*\{[^}]*padding:0 8px 0 0;/u);
  matches(shell,/@container app-window \(max-width: 1280px\)\s*\{[\s\S]*?\.topbar \.topbar-actions\s*\{grid-area:actions;display:flex;flex:0 1 auto;flex-flow:row wrap;width:100%;justify-content:flex-end;gap:8px;margin-left:0;padding:0 8px 0 0;/u);
  matches(shell,/\.topbar \.topbar-actions > \.notification-trigger\s*\{[^}]*margin-left:\s*4px;/u);
  matches(shell,/\.topbar \.topbar-actions > \.help-button\s*\{[^}]*margin-left:\s*12px;/u);
  matches(shell,/@container app-window \(max-width: 600px\)\s*\{[\s\S]*?\.topbar \.brand-name,[\s\S]*?\.topbar \.connection-header \{width:100%;max-width:min\(42cqi,220px\);min-width:0;padding-inline:6px;\}/u);
  matches(shell,/@container app-window \(max-width: 420px\)\s*\{[\s\S]*?\.topbar \.connection-header \{flex:1 1 100%;width:100%;max-width:min\(100%,230px\);\}/u);
  matches(shell,/\.topbar \.theme-control\s*\{[^}]*flex:\s*0 0 auto;/u);
  matches(shell,/\.topbar \.theme-control-trigger\s*\{[^}]*flex:\s*0 0 auto;/u);
});

test('first-run header avoids a duplicate repository action and hides an empty snapshot picker',()=>{
  const app=readFileSync(new URL('../src/App.tsx',import.meta.url),'utf8');
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(app,/const hasProjectControls = !!currentProject \|\| projectOpening \|\| projectMetadataBusy \|\| recentsPending \|\| recentProjects\.length > 0;/);
  matches(app,/className=\{`topbar\$\{hasProjectControls \? '' : ' topbar--no-project-controls'\}`\}/);
  matches(app,/\{hasProjectControls && <div className="project-controls">/);
  matches(app,/recentProjects\.length > 0 \? 'Recent repositories'/);
  matches(app,/\{currentProject && currentSnapshots\.length > 0 && <>\s*<span className="header-divider" \/>/);
  assert.doesNotMatch(app,/: 'No snapshot'<\/span>/);
  matches(shell,/\.topbar\.topbar--no-project-controls\s*\{[^}]*grid-template-areas:\s*"brand actions";/);
  matches(shell,/@container app-window \(max-width: 680px\)\s*\{[\s\S]*?\.topbar\.topbar--no-project-controls\s*\{grid-template-columns:minmax\(0,1fr\);grid-template-areas:"brand" "actions";/);
});

test('repository onboarding focuses on exploring code and progressively discloses technical details',()=>{
  const app=readFileSync(new URL('../src/App.tsx',import.meta.url),'utf8');
  const shell=readFileSync(new URL('../src/App.css',import.meta.url),'utf8');
  matches(app,/<h2>Understand how your codebase works\.<\/h2>/);
  matches(app,/<p>Open a repository to explore its code, see how the parts connect, and trace explanations back to source\.<\/p>/);
  matches(app,/<h2>Ready to explore \{project\.displayName\}\?<\/h2>/);
  matches(app,/busy \? 'Starting…' : 'Start exploring'/);
  matches(app,/<details className="project-facts-details">\s*<summary>Repository details<\/summary>/);
  assert.doesNotMatch(app,/className="empty-steps"|className="preflight-next-step"/);
  assert.doesNotMatch(shell,/\.empty-steps\s*\{|\.empty-step\s*\{|\.preflight-next-step\s*\{/);
});

test('scene navigation stays in layout flow, remains reachable while reading, and wraps within its track',()=>{
  const story=readFileSync(new URL('../src/visual-story.css',import.meta.url),'utf8');
  matches(story,/\.story-sticky-controls\s*\{[^}]*position:\s*sticky;[^}]*top:\s*0;[^}]*z-index:\s*\d+;/);
  matches(story,/\.story-toolbar\s*\{[^}]*position:\s*static;/);
  matches(story,/\.story-scene-progress\s*\{[^}]*position:\s*static;/);
  matches(story,/\.story-scene-dots\s*\{[^}]*flex-wrap:\s*wrap;[^}]*overflow:\s*visible;/);
});

test('Guide answer panels and stacked turns stay within the reader width',()=>{
  matches(css,/\.dw-answer-scroll > \[role="tabpanel"\]\s*\{display:block;width:100%;min-width:0;max-width:100%;box-sizing:border-box;\}/);
  matches(css,/\.dw-guide-attempt-group\s*\{width:100%;min-width:0;max-width:100%;box-sizing:border-box;\}/);
  matches(css,/\.dw-progressive-answer\s*\{\s*display:\s*grid;\s*width:\s*100%;\s*min-width:\s*0;\s*max-width:\s*100%;\s*box-sizing:\s*border-box;/);
});

test('native high zoom shows one workspace pane at a time with a return path',()=>{
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-panels > #repository-sidebar,[\s\S]*?\.dw-panels > \.dw-separator \{display:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-panels > #reading-workspace \{flex:1!important;min-width:0!important;max-width:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-narrow-explorer \.dw-panels > #repository-sidebar \{display:flex!important;flex:1 1 100%!important;min-width:0!important;max-width:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.dw-narrow-explorer \.dw-panels > #reading-workspace \{display:none!important;\}/);
  matches(css,/html\[data-app-zoom-compact="true"\] \.developer-workspace \.dw-sidebar-back \{display:inline-flex;\}/);
});

test('exploration breadcrumb buttons cancel Radix ghost margins so names are not clipped',()=>{
  matches(css,/\.dw-path-root button:not\(\.rt-IconButton\),\.dw-path-segments button:not\(\.rt-IconButton\)\s*\{[^}]*margin:0;[^}]*padding:0;/);
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
