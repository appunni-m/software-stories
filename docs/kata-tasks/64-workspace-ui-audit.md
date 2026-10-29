---
kind: task
key: workspace-ui-audit
parent: design-parity
title: Replace inconsistent pages with the conversational developer workspace
priority: 1
labels: design, ux, product
---

## Outcome
Replace the installed Guide/Files workflow with the conversational developer workspace shown in workspace-preview.html. Explorer, Search and Changes live in a narrow VS Code style activity bar; source and explanation receive the remaining space. Stacked conversations and nested explorations must be visible without discovering hidden controls.

## Audited inconsistencies and required fixes
1. Files: the original source-level heading issue is resolved. `src/App.css` clips the direct `.workspace-heading` child while retaining its heading in the accessibility tree, and the captured workspace hides the obsolete toolbar. The old claim that `.has-snapshot` and `.view-atlas` were applied to the same element was incorrect. Keep visual/package acceptance open; the source rule alone does not prove the installed layout.
2. Guide: two competing starting points (question form and tour dashboard), progress tables and metadata dominate the reading journey. Replace with a single pinned composer and real answer cards.
3. Files/source: three simultaneous empty instructions, assistant configuration sorted ahead of application code, expanded folders, no sensible initial source. Show one quiet prompt, collapse folders and choose a likely implementation entry point without hiding searchable files.
4. Navigation: Guide/Files/Changes across the header compete with local Explanation/Flow/Player tabs. Use the left activity bar for repository tools; Guide/Flow/Chat history belong to each exploration.
5. Conversation: preview follow-ups are below a large unrelated visual and branches appear only in breadcrumbs. Render question/answer pairs as a stack, real branch counts attached to originating turns, a hierarchical exploration list and parent/root controls.
6. Shared controls: legacy 8–11px typography, inconsistent button sizes, blue paragraph text, different pane headers, dense card nesting, duplicate status labels. Use shared Radix controls and neutral semantic surfaces; keep readable text and a consistent spacing scale.
7. Generation: source discovery and chapter generation own separate modals and discard intent. Give one question-specific background operation a lifetime independent of navigation, an explicit stop action, persisted result references and native evidence validation. A revocable snapshot permission replaces repeated source-review steps for the new workspace only; old explicit review/recovery remains available.
8. Saved work/edit/player: do not remove revision editing, source citations, independent section changes or Pocket TTS. Expose them on the answer that owns them.
9. Changes: comparisons require typed refs and show internal IDs. Provide a real bounded local commit list/ancestry with one-click comparison; retain exact diff/PR explanation functionality.
10. Search: current header search opens another workflow. Search captured paths/code beside the composer, with Explore with AI even for no results; do not run inference on typing.
11. Notifications/settings: keep background outcomes in the existing bell, one connection in the header, settings only on demand. Remove duplicate navigation and unused controls from the normal path.
12. First-run, capture/error, connection/recovery, chapter editor, player and Git details: review shared typography/spacing/focus; preserve actionable errors and existing records. Do not claim these flows passed merely because a detached preview looks correct.

## Acceptance
Build, run frontend and native regression checks, and inspect the packaged app. Record screenshots of real source + Guide, Flow, stacked follow-ups, at least two branch depths, Search, Changes/commit history, settings and first-run where feasible. Verify evidence remains snapshot-bound, cancellation preserves prior drafts, navigation does not cancel generation, re-opening restores branches and last view, selection explains at the current level, and ordinary typing never sends source. Native permissions must bind snapshot/account/runtime and be checked again at dispatch, including after revocation. Use the existing single port 4173. Install the verified replacement bundle and open it; clearly record any incomplete qualification instead of closing unrelated gates.

## Latest local changes — 28 October 2026

The native workspace header now offers **Load latest changes** when a repository snapshot is selected. It captures the current local working tree as a new immutable snapshot and switches the workspace to it after a complete or partial capture. Earlier snapshots and their explanations remain available. The duplicate “Capture current code” action inside snapshot details was removed. Capture failure or cancellation leaves the selected snapshot unchanged. This is a source change only: no build, test or packaged-app review ran.

When a file is open, its path stays selected in the new snapshot so the updated source is immediately visible. The previous line range is cleared because its source hash belongs to the older snapshot.

Packaged acceptance must verify committed and uncommitted edits appear in the new snapshot; the currently open file stays selected and shows the new bytes; prior source, answers and citations remain available from the earlier snapshot; partial captures show skipped paths; failure/cancellation keeps the prior selection; changing repositories during capture never redirects the open workspace; and the header control remains usable with keyboard, VoiceOver and narrow/high-zoom layouts.

## October 16 migration checkpoint
The production workspace replaces normal Guide/Files routes and reuses native source, chapters, permission validation and generation. Automatic local indexing is shared between reading and questions so there is no extra Build index step in the normal path. Short natural follow-ups are accepted and retain recent question intent. Shared source controls correctly identify file languages; JavaScript/TSX variants use the existing syntax highlighter and selection shortcuts use inclusive end lines. Captured-source search and real local commit history are implemented. Browser proof images show stacked follow-ups, originating-turn child links and branch counts; they remain labelled sample design evidence. Frontend 127 tests / native 198 tests pass. The Mac was initially locked and has now been unlocked; packaged acceptance is in progress, not considered passed. Root-commit comparison, complete reading-position recovery and durable scheduling remain follow-up work in the workspace task set.

Draft playback and visual destination integration were corrected before the final package: an explicitly selected draft has a labelled player preview; rejected/missing revision IDs never fall through to a different accepted answer. Export stays accepted-only. File/symbol/relationship hotspots resolve to captured evidence in the new shell.

## Native walkthrough corrections
The replacement was installed and opened after unlocking the Mac. A synthetic GPT-6 Luna/max question generated a three-section native draft, and player citations returned to exact captured lines. The walkthrough caught a control-character separator in contextual follow-ups (rejected by native discovery), the Radix root theme's viewport minimum clipping the workspace footer/tree, an unintended desktop source-toggle control, stale shortcut help, a draft labelled accepted in the player footer, and an empty Flow despite available indexed links. Source fixes normalize contextual questions, outrank the theme minimum, hide narrow-only controls at desktop width, correct lifecycle/shortcut copy, and fall back to verified static links. Changes now starts from commit history with optional manual comparison; initial commits are visibly labelled and disabled rather than compared with themselves. Native requalification of these final fixes remains required.

A second follow-up reached Codex but was rejected for an unsupported item. The adapter now supplies an explanation-only base instruction set, checked against the installed published schema, and exposes only the bounded event type for diagnostics. Tool arguments/source are never included in this diagnostic; command/web/write/delegation access remains disabled. Question dispatch is serialized. The embedded PR view reuses Codex, preselects eligible changed text files up to the 12-file bound, and folds file overrides instead of repeating a backend picker. The last chosen recent project is restored on launch without inference. These final refinements require a new live walkthrough before claiming completion.

## Live generation and visual corrections
A real child answer saved; its diagram exposed a native/frontend wire mismatch: enum variant fields serialized as `anchor_id` while the interactive viewer expects `anchorId`. Normalize hotspot, inference-label and dependency fields at serialization; continue reading legacy snake_case fields. Qualify a stored legacy diagram and newly generated diagram, including node-to-source navigation, without editing stored revisions.

The root follow-up also exposed the overview duplicate-copy contract. AI-selected single code files now use file chapters automatically. Workspace overviews use a fingerprinted anchored-claim presentation recipe; original model prose/narration is discarded, evidence and visuals are checked first, inference labels survive, and the independent storage copy invariant still applies. Require both single-file and multi-file native questions before closing this correction.

Restart qualification exposed origin-v1 content hashes coupled to the old enum serialization. Preserve an exact legacy digest projection for old origins; create origin-v2 records for current wire bytes. Require an independent saved synthetic v1 fixture to survive decode/current-wire round trip and reject prose/narration tampering. Never overwrite a saved digest to make an upgrade pass. Reopen the installed app and inspect saved diagrams again before calling migration complete.


## Remove voice configuration from answers
The header Connections window owns Explanations and Voice. Engine/account/model/voice/style/download/repair belong there. Each answer offers Listen and ordinary playback; no per-section engine dropdown or runtime card. Reuse a persisted global delivery choice for all repositories, answer sections and export cache matching. Keep genuine cloud data-use consent at send time, hide hashes/segments behind optional details, and preserve transcript editing. Native acceptance must cover the actual installed window, two answer sections, restart persistence, and correct recovery after setup/key changes.


Historical installed observation: origin-v1 answers/diagrams loaded; clicking Validation selected captured lines 2–4. A new explicit empty-list retry saved an origin-v2 file chapter and diagram; subtotal selected captured line 12. Shared Connections and local Listen were inspected in that older app. The source gaps noted in that run have since been addressed: diagrams use read-only accessibility copy and labeled controls; the captured workspace landmark now says “Explore this codebase.” instead of the former file-selection copy; prose uses inert `InlineCodeText`; exploration rows count questions rather than completed answers. The installed executable revision for those later source fixes was not verified. Keep packaged visual, keyboard, focus and VoiceOver acceptance open.

## Diagram semantics — source correction, 28 October 2026

Current source uses explicit read-only React Flow descriptions and labeled native
controls inside visual/flow nodes; HTML-scene nodes also expose button semantics
and Enter/Space activation. The shared screen-reader hint now describes the
actual code-detail controls instead of referring to a nonexistent “source
button.” This corrects a copy mismatch and records the source-level semantics;
it does not establish packaged keyboard, VoiceOver, or focus behavior. Keep
those acceptance checks open.

## Fontdone findings — October 17

The [large-repository acceptance run](../acceptance/2026-10-17-fontdone.md) reproduced remaining user-facing inconsistencies:

- [ ] Returning from a child, switching Guide/history or restarting restores the parent answer at section one. Preserve exact reader position under separate [task 68](68-exploration-reading-position.md), Kata `erzc`.
- [x] History labels describe stopped/interrupted/failed questions truthfully and call an explanation saved only when its exact revision is available. The current model is `historyEntry` in `src/workspace/model.ts`; regressions are in `tests/workspace.test.mjs`.
- [x] Exploration summaries distinguish questions and child explorations from answer sections. The current model is `explorationSummaryCounts` in `src/workspace/model.ts`; failed-child and count regressions are in `tests/workspace-outcomes.test.mjs`.
- [x] Supported inline code is rendered as inert `<code>` elements through `InlineCodeText` and `inlineCodeParts`; stored prose and citation identity are unchanged.
- [x] Search results and Explorer filtering have separate state (`repositorySearch` versus `fileFilter` in `src/workspace/Workspace.tsx`; the legacy Atlas tree uses its own `treeQuery`). A search query is not applied to the file tree.

These four fixes are source/model findings, not packaged design acceptance. Keep the parent reader-position issue and full native navigation, first-run, theme, zoom, keyboard, and VoiceOver review open.

The snapshot, source range, question tree, selected view and saved draft did survive restart; Pocket playback also reused its recording. These passes do not close the unresolved UI, reader-position, accessibility or deeper-branch gates.

## Installed progressive reader findings — 17 October

Computer use found valid native section IDs discarded by the reader, leaving a stopped draft hidden despite successful local storage. The reader/native ID contract is fixed and a regression uses real native notices; installed restart restored the draft and its exact source citation without inference. A later manual retry kept prior cards visible and allowed nested navigation while work continued.

Flow initially selected an older draft without a diagram and substituted unrelated file relationships despite a generated question diagram being available. The checkout now prefers that answer's evidenced diagram only when there is no explicit selection, preserves the user's choice as sections arrive, and does not substitute unrelated relationships for an existing explanation. The visual footer also incorrectly called a draft accepted; that claim is removed. [Exact observations and qualification boundaries](../acceptance/2026-10-17-progressive-reader-integration.md#fresh-fontdone-retry-useful-delivery-mechanics-incomplete-content). Retain installed default-selection, keyboard/focus/VoiceOver and complete UI acceptance as open gates.

## Narrow activity return action — 28 October 2026

Source review confirmed that at widths of 680px or less, opening Explorer, Search,
Explorations, or Changes hides the reading workspace. The sidebar's **Back**
action was rendered only for secondary tools, leaving these normal activity
screens without a visible route back to the explanation. It now appears whenever
the activity sidebar replaces the reader; activating it restores the Guide
workspace, and leaving Changes also restores the Files activity selection. This
is a source-level correction; packaged narrow-window navigation and the full
screen-by-screen UI acceptance remain open.

## Remove redundant one-section controls — 28 October 2026

The sequential Guide already owns Previous/Next navigation across published
slides. Its child section renderer also displayed a second `1 / 1` heading and
a disabled Previous/Next bar for every one-section revision. Both inner
indicators are now omitted for a one-section revision; the revision-local
navigator appears only when a revision actually contains multiple sections,
and its counter uses the explicit `Section n of m` label. This leaves the
outer Guide slide sequence as the only progress control for sequential slides.

In the running packaged Fontdone workspace, the accessibility tree showed the
outer `Section 4 of 4`, followed by inner `1 / 1` and disabled section buttons
for the same slide. The installed executable's source revision was not
verified, so this is a concrete observed inconsistency, not acceptance of the
updated source.

No test, build or packaged interaction check ran in this source-only increment.
Native growing-answer, citation-return, narrow-layout and high-zoom checks
remain open under the existing acceptance task.

## Repository sharing menu placement — source change, 28 October 2026

Moved **Repository sharing** from the activity-specific sidebar header to the
current repository menu in the app header. The command is disabled until the
selected snapshot's manifest is loaded; the existing dialog still displays the
grant status and can revoke permission and stop active questions. The sidebar
header now contains only the active tool label and its relevant Back action.
This removes the settings affordance from the code-navigation rail without
changing the snapshot/account grant contract. No test, build, or packaged menu
and revoke check ran; keep `na54` open.

## Load latest repository changes — 28 October 2026

The workspace header now places **Load latest changes** beside the selected
snapshot. It captures the current working tree as a new immutable snapshot,
opens that snapshot, and leaves earlier snapshots available. The currently
open source file is retained when its path exists in the refreshed snapshot.
The control is unavailable while a capture is already running.

The rebuilt packaged app exposes the control in the Explorer header with the
help text “Capture the current repository and open the new copy. Earlier
snapshots stay available.” A disposable repository check changed a source file
after the first capture, refreshed, and confirmed that the new snapshot showed
the updated bytes while the earlier snapshot still showed the original bytes.
The repository itself was unchanged by capture. The user's existing `pillow-rs`
snapshot was left selected; no refresh was run against its source.

On 28 October, the active native Code Atlas window was inspected with `pillow-rs`
open: **Load latest changes** is visible beside the snapshot selector. The
browser preview cannot read a local repository, so this action is available in
the native Mac app.

This qualifies the explicit refresh action and its immutable snapshot behavior.
It does not close the screen-by-screen design, keyboard, accessibility, or
responsive acceptance; keep `na54` open.

## Source-linked preview overflow — source correction, 28 October 2026

The small relationship previews embedded in stacked Guide answers now constrain
source-node and relationship-label widths. Labels wrap inside the relationship
column, and Guide-pane container queries place the relationship between
side-by-side nodes at compact widths and stack the three items in very narrow
panes. The separate empty-preview action retains its natural width.

Evidence: npm run build and git diff --check pass. The reflow assertions were
updated to match the current source-review flow, keyboard mappings, and compact
snapshot-refresh control. The reflow suite was not rerun, and the packaged
workspace was not rebuilt or visually rechecked for this change. Keep x21h,
13yp, and the broader UI acceptance open.

## Cold-open graph scene remains blank — 28 October 2026

On the rebuilt packaged app (binary SHA-256
`ccec56cc498407ac0edad083ef6a1198d344d141268441049cd7df0e6e940433`), the
saved `pillow-rs` Guide for `canvas.rs` initially opened to scene 1, “Image
variants choose their drawing storage,” with only the dotted React Flow canvas
visible even though **Scene details & source** listed four cited image-variant
nodes. Clicking **Fit View** did not reveal them. Scene 2 rendered its three-step
HTML composition correctly; returning to scene 1 made the source-linked graph
appear. After quitting and relaunching the packaged app, scene 1 rendered on its
first view. The blank-canvas symptom is therefore not yet a reproducible cold-
open failure; its cause could not be distinguished from a transient reader/view
state. No inference, capture, or saved-content changes were made. Screenshots
were inspected live but not saved durably. Keep packaged acceptance open and
verify first-entry rendering on multiple saved graph scenes before treating the
symptom as resolved.

## Shared-source relationship previews — 28 October 2026

The live packaged Guide showed four outgoing relationships that repeated the
same **Image input** source node in every row. The source link was clickable,
but the repeated origin obscured the fact that the visual represented one
input branching to several format outcomes. The compact Guide renderer now
shows a shared source once and lays its outcomes out as labeled, source-linked
branches. Single-edge and mixed-source relationship lists retain their prior
layout; narrow Guide panes stack the branch source above its outcomes.

Focused visual and reflow checks pass (63/63), `npm run build` passes, and
`npm run bundle:macos` produced the app and DMG with verified metadata and DMG
checksum. The packaged binary SHA-256 is
`443fbe48ae1c2eaab2f5c2a4c78951b76079d7ab58c45c93d012167a79f43126`. The
existing app process stayed open, so this change has not yet been rendered in
the rebuilt native app. The live screenshot was inspected but not retained as
a durable file; keep `13yp` and `x21h` open.

## Flow edge labels colliding with nodes — 28 October 2026

A live Flow view showed relationship labels crossing node cards in a branching
diagram. Edge-label placement now checks candidate positions against other
labels and the diagram's node bounds, trying additional lanes and choosing the
candidate with the fewest node overlaps if the bounded search cannot find a
clear position. A focused branching-layout assertion covers source and target
card clearance. Flow layout, reader progress, Flow presentation, and native
conversation tests pass (70/70); `npm run build` and the macOS app/DMG bundle
complete successfully. The packaged executable SHA-256 is
`477c707ce17dcf5adc9e1ff13b6139fbe5f4bc13544b06fa7bf326c1b85b20ea`.

Follow-up, 28 October: the stale reader-layout recovery fix was included in a
new local package. The rebuilt executable at
`/Users/lazytrot/work/software-stories/src-tauri/target/release/bundle/macos/Code Atlas.app/Contents/MacOS/code-atlas-desktop` has SHA-256
`34a3cfa25534af1400c3a39e27d4c1ac8906436205935a7b67739c65fb1ebdcd`. Opening
Fontdone `main@5249b72371` now loads its existing Guide and 27-question
conversation instead of reporting that the whole saved conversation is
unavailable. The selected `glyf.rs` answer and source range `554–574` are
visible; several individual older answers still have unavailable or stale
visual states. The Flow edge-label layout itself was not rechecked. No snapshot
refresh, generation, or answer edit was performed. Keep `13yp` and `x21h` open.

## Duplicate source identities in compact Guide previews — 28 October 2026

The compact preview grouped relationships only when they reused the same
internal node ID. Generated visuals can represent the same cited code location
with separate node IDs, leaving duplicate source labels in the preview. It now
groups those nodes only when both the normalized label and exact captured
source location (path, SHA-256, and byte range) match. Different or unknown
source locations remain separate. The focused layout checks pass (4/4), the
frontend build passes, and `npm run bundle:macos` produced verified unsigned
app and DMG bundles. The executable SHA-256 is
`66efce0b7badba4a40872a5943af291428a322549e62e056ccdeb3ff80e9e5df`.

The rebuilt app was reopened and its saved `canvas.rs` Guide showed one
**Image input** node branching to four labeled, clickable outcomes. That is the
full interactive visual rather than the compact stacked-answer preview, so the
new compact duplicate-ID case is covered by the focused check but has not yet
been observed in that exact native layout. No model request, snapshot refresh,
or saved-answer edit occurred. The full screenshot matrix, accessibility and
responsive checks remain open; keep `13yp` and `x21h` open.

## Stale window and relationship-button spacing — 28 October 2026

The currently open native window still rendered an older package: the
**Explain this code** action floated over selected source, and the compact
relationship view repeated its **Image input** source in each row while labels
crowded the endpoint buttons. Current source already places selection actions
in the source-path row and groups shared sources. This follow-up widens the
compact-reader reflow to 820 px, adds spacing between endpoints, and gives the
relationship label its own row with a downward connector at that width.

`npm run bundle:macos` completed the frontend build, license check, unsigned
app/DMG packaging, metadata verification, and DMG checksum verification. The
new executable SHA-256 is
`a193dcf4444e22ab244ab704afa40fb49931cb9d070cba26ef90338faa03877a`.
Packaged visual verification is pending: the current window's safe-close
handshake reports a rejected conversation save, so it was left open and no
replacement was launched over it. The screenshot was inspected live but not
retained. Keep `13yp` and `x21h` open until the fresh process and compact
relationship layout are checked without disturbing the saved workspace.

## Fresh-process packaged Guide confirmation — 28 October 2026

Correction and stronger identity evidence: the first post-build UI observation
came from a process launched before the target executable was rewritten. I
restarted the target app after the bundle's 22:01 local modification; the new
process started at 22:11. Its window screenshot (captured through CUA and
visible in the task history) showed the `66efce0` executable with the saved
`pillow-rs` `main@e843f864fc` workspace, selected `canvas.rs` beside the Guide,
and one **Image input** branching to four labeled outcomes. The player was
idle and **Load latest changes** was visible. No snapshot refresh, generation,
audio playback, or answer mutation occurred. This confirms the full visual in a
fresh process; it still does not qualify the compact stacked-answer preview or
the remaining Guide/Flow/Search/Changes, keyboard, accessibility, and reflow
matrix. Keep `13yp` and `x21h` open.

## Button placement correction — 28 October 2026

Reworked the selected-code actions so **Explain this code** and **Add selection
to chat** live in the source path row instead of floating over selected lines
or the editor footer. The repository/snapshot controls no longer stretch the
refresh action into a visually isolated header group. In both the full Guide
visual fallback and the compact stacked-answer preview, refresh and source
buttons now sit on a separate row below the status text.

Evidence: `npm run bundle:macos` completed, including the frontend TypeScript
build, license metadata check, unsigned app and DMG packaging, and DMG checksum
verification. The rebuilt executable SHA-256 is
`af81e2bf60a0593788eafd4941de0244dee4c1665f5a6d40dcc8230fe2459ea5`. After
restarting that exact bundle, the Fontdone `main@5249b72371` Guide showed the
selection actions in the source path row and the compact preview's status and
actions in separate rows. The live screenshot is visible in task history; it
was not saved because it includes captured source. No test suite was run, no
snapshot refresh or AI request was made, and no saved answer was edited. Keep
the broad packaged, responsive, keyboard, and accessibility acceptance open;
this single Guide state does not close `13yp` or `x21h`.

## Compact relationship-card reflow — 28 October 2026

A live `pillow-rs` Guide showed relationship endpoints overlapping their
connector labels in the stacked-answer preview. The previous responsive rule
measured the whole Guide pane, which can remain wide while a preview is
constrained by its card. The relationship preview now measures its own width:
at compact widths it lays source and destination buttons on separate columns
and moves the connector to its own row; very narrow previews stack all three.

`CARGO_INCREMENTAL=0 npm run bundle:macos` passed the TypeScript/Vite build,
license and metadata checks, unsigned app/DMG packaging, and DMG checksum
verification. The rebuilt executable SHA-256 is
`1308378cd93b1f4d0cf8e65abcd618bca3f4560da4451b995c5837767eff6af9`.
The open `pillow-rs` app window was left untouched because its close-save
handshake is still rejecting an update. Packaged visual confirmation in a
fresh process remains pending; keep `13yp` and `x21h` open.

## Compact preview button placement follow-up — 29 October 2026

A fresh live CUA screenshot of the open `pillow-rs` Guide still showed the
relationship label colliding with its source buttons. The screen also showed
the older floating **Explain this code** action, so the open process is not
evidence for the current source layout. The compact preview now defaults to
two endpoint columns with the relationship label between them; a horizontal
three-column row is only enabled when the preview itself is at least 720 px
wide. Very narrow previews still stack the endpoints and label. This fallback
does not depend on a container query to avoid the observed overlap.

`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run bundle:macos` completed: TypeScript
check, Vite build, license and metadata checks, unsigned app/DMG bundling, and
DMG checksum verification passed. The new executable SHA-256 is
`a39be2640c7a15e739d403ce1814e2d0d3fd8d1d3da918d7783954c6d5300cc7`.
`git diff --check` passed. No test suite, AI request, snapshot refresh, or
saved explanation edit was performed. I did not close the existing app window;
fresh-process visual confirmation remains pending. Keep `13yp` and `x21h` open.

## Compact relationship buttons follow-up — 29 October 2026

The previous compact layout still placed source and destination buttons side
by side, with their relationship label below. This made it hard to tell which
label connected which buttons. Compact relationship rows now read vertically:
source button, relationship label and down arrow, then destination button.
The horizontal three-column layout remains available when the visual card is
at least 720 px wide. Shared-source branches also stack their outcome label,
arrow, and destination at narrower widths.

`npm run build` passed (TypeScript and Vite). No test suite, snapshot refresh,
AI request, or saved explanation edit was performed. The app was not rebuilt or
restarted, so the packaged visual result remains unverified; keep `13yp` and
`x21h` open.

## Guide controls alignment follow-up — 29 October 2026

The live `pillow-rs` Guide showed relationship labels touching or overlapping
their source buttons. The edge label used `width: 100%` plus horizontal padding
without `box-sizing: border-box`, so its rendered width could exceed its grid
column. The edge now includes padding in its assigned width. Added vertical
breathing room around the Guide slide/section navigation so Previous, progress,
and Next sit clearly apart from the fixed Guide/Flow/history tabs.

`npm run build` and `git diff --check` pass. The open packaged app has not been
rebuilt or restarted for visual confirmation; keep `13yp` and `x21h` open.

## Packaged button-position check — 29 October 2026

Rebuilt the unsigned macOS app and DMG with the latest Guide spacing changes.
License checks and metadata verification passed; the DMG checksum was verified.
The executable SHA-256 is
`5edb510afd679ab57f7eb45e5cee02bf6f3f226a25c4a354c0514a0c9a80bb10`.
In the reopened packaged workspace, the Guide slide Previous/progress/Next row
and the answer-section Previous/progress/Next row are visibly separated and
aligned. The selected `pillow-rs` scene rendered a blank React Flow canvas, so
the source-linked relationship-label/button spacing was not visible in this
state. No repository content or saved answer was edited, and no screenshot
containing source was retained. Keep `13yp` and `x21h` open for the remaining
visual, accessibility, and screen-by-screen acceptance.

## Sticky Guide navigation follow-up — 29 October 2026

The reopened Guide exposed a separate position defect: the section navigation
could be restored partly above the answer reader's scrollport, leaving its
Previous and Next controls clipped against the Guide/Flow/history bar. The
navigation now sticks to the top of the answer reader while scrolling, keeping
section controls reachable without changing the saved answer or its scroll
position. Source change is in `src/workspace/workspace.css`.

The scene Previous/Next arrows were also flanking the multi-row narration
player. Their vertical position followed the whole player height, making the
scene controls look detached and crowding playback. Moved scene navigation
beside the scene count/dots in the toolbar; narration now gets a full-width
playback row. Compact and narrow layouts use the same grouping.

Packaged visual confirmation: the rebuilt app reopened at pillow-rs
`main@e843f864fc`. Scrolling the Guide kept Previous / section count / Next
visible below the Guide/Flow/history bar. Scene arrows now sit beside the scene
count and dots, while narration uses its own full-width row. The top-of-section
view also shows the section row in normal flow without overlap. `npm run build`
and `npm run bundle:macos` passed; notices for 148 npm packages and 391 Cargo
crates, bundle metadata, and the DMG checksum verified. The executable SHA-256
is `ee2384349810ec5a1024a11336d6fb40d13c03d18e3b6c94d20e519ed69c1114`.
The broader screen, accessibility, and responsive audit remains open under
`13yp` and `x21h`.

## Stacked Guide and scene navigation — 29 October 2026

A fresh screenshot of the packaged `pillow-rs` Guide showed the section
Previous/progress/Next bar floating over the diagram while the scene title and
scene controls had scrolled away. The reader intentionally kept story controls
in normal flow for chat history, but that same rule was hiding scene navigation
in the sequential Guide. The Guide now stacks its opaque section navigator at
the top of the answer reader and its opaque scene toolbar directly below; other
inline stories remain in normal flow. On compact narration rows, Replay, Play,
and Stop use fixed minimum hit areas and a wider gap so their borders do not
crowd each other.

The rebuilt app was reopened on the existing `pillow-rs` `canvas.rs` Guide. The
section and scene navigation bars were both visible with no overlap; scene
controls stayed grouped beside the scene count/dots, and Replay/Play/Stop were
visibly separated. `npm run build`, license validation, app/DMG bundling,
metadata verification, DMG checksum, and `git diff --check` passed. The
executable SHA-256 is
`98a5b719f3de6e7c366c08134db45f9c7bd37ba792b7d76409cf8e5c89c772cec`; the DMG
SHA-256 is
`3449f76f02e21c3cc8fe28847c762af8187c0d64ddb74d113c05968437667617`. No saved
repository content or explanation was changed. The broader screen-by-screen,
accessibility, and responsive audit remains open under `13yp` and `x21h`.

## Snapshot refresh button alignment — 29 October 2026

The live workspace header showed “Load latest changes” sitting lower than the
repository and connection controls. The shared secondary-button style adds a
top margin intended for page actions; the snapshot refresh control now resets
that margin with a header-scoped selector so it aligns with the other controls.
This targeted CSS correction does not change snapshot behavior or stored
repository data. The packaged app was rebuilt and reopened at the existing
`pillow-rs` workspace; the refresh button now shares the header row's vertical
alignment with the repository and connection controls.

## Packaged Flow / Search / Changes spot-check — 29 October 2026

Inspected the rebuilt Code Atlas 0.1.0 app from commit `d107bc4` at
2880×1688, using `pillow-rs` `main@e843f864fc` (partial snapshot) and the
existing `canvas.rs` answer. Flow retained the answer and showed its saved
visual; **Fit View** brought the full diagram into the pane. Search restored
the local query `NativeDrawCanvas`, reported matches in captured files, and
kept the selected source visible. No question was sent.

Changes restored a local comparison from `e843f864…` to `c5104c…`, but its ref
fields were disabled and **Stop comparison** was visible while no result was
shown. I left that state untouched; this does not establish a successful
comparison. The screenshot was inspected live but not retained because it
contains local source. The packaged executable SHA-256 was
`d92ca3eae9b52b437fc07eb8f00df11ccfe347437393bb73a97c50a0f1a2b171`.
First-run, nested follow-ups, notification outcomes, theme/accessibility, and a
completed Changes result still need their own evidence under `13yp` and `x21h`.

## Header and Guide button alignment — 29 October 2026

Reopened the rebuilt app at 2880×1688 on the existing `pillow-rs`
`main@e843f864fc` Guide. The header's **Load latest changes** control now sits
on the same vertical center as the repository, connection, and appearance
controls. The **Previous / Section / Next section** row is fully visible below
the Guide/Flow/history tabs, and the code-selection actions stay in the source
path row. The header reset explicitly clears the shared secondary-button
margin; the sticky reader navigator now keeps a small inset and lower stacking
priority so it cannot crowd the view tabs. No repository snapshot, saved answer,
or narration was changed. The rebuilt executable SHA-256 is
`4059e01548a752d6005bd2d465457f8af988f6d03a4875cd218059a2814a293c`.
Broader screen-by-screen, accessibility, responsive, and nested-exploration
acceptance remains open under `13yp` and `x21h`.

## Changes opens on the current commit comparison — 29 October 2026

In the installed build above, I opened Changes from the populated
`pillow-rs` `main@e843f864fc` workspace. The activity showed the local commit
history and automatically compared local HEAD `c5104cbaaefbd359c88746801a1fd60e52a1f6d1`
with its first parent `e843f864fcbdf6b53feead608f7ab53025541d8e`, without
selecting a commit. The first accessibility read caught the comparison still
running; the next read showed a completed result. The result accurately says
**Partial comparison**, lists the 128-file cap and 111 omitted changes, and
labels indexed cross-file relationships as partial. The comparison is read-only;
no provider operation or working-tree change occurred. I returned the app to
its original Explorer/Guide view and kept `canvas.rs`, section 1 of 4, selected.

This resolves the previously unverified populated Changes entry path for this
repository state. It does not cover empty or initial-commit repositories,
custom ref persistence, alternate commit selection, the full diff/navigation
matrix, or accessibility/reflow. Keep the cross-screen audit open under
`13yp` and `x21h`.

## Guide navigation button grouping — 29 October 2026

The packaged `pillow-rs` Guide showed its **Previous / Section / Next section**
controls spread across the full reader width, and **Scene 1 of 2** was separated
from its scene arrows and dots. Both sets now stay together: section controls
form a centered group, while the scene count sits beside its arrows and dots;
**Full map** remains at the right edge. Narrow-pane and zoom-compact layouts use
the same grouping.

`npm run bundle:macos` passed, including frontend compilation, license and
bundle-metadata checks, app/DMG bundling, and DMG checksum verification. The
reopened app showed the centered section controls and the grouped scene count
and navigation on the same `pillow-rs` `main@e843f864fc` Guide. `canvas.rs`,
section 1 of 4, scene 1 of 2 remained selected; no question was sent and no
saved explanation was changed. Executable SHA-256:
`53b61abee6178df3e0a71046f9c926ec22f1d214a9ee5713df08906096dc2591`.
DMG SHA-256:
`b47056aa0fea67cf752f25e7803f0745c516da56cf6806df8eddf2da787caf7f`.

## Center scene controls in the Guide toolbar — 29 October 2026

The first alignment pass still left **Scene 1 of 2** and its previous/next
controls at the left side of a compact Guide pane. The scene count, arrows, and
dots now occupy the center column at both regular and compact widths; **Full
map** stays right-aligned. The scene title remains left-aligned on its own row
when the pane is compact.

`npm run bundle:macos` completed, and the installed app was reopened on the
existing `pillow-rs` `main@e843f864fc` Guide. The live screenshot showed the
scene navigation centered beneath the title and **Full map** at the right.
No test suite was run. No repository snapshot, saved answer, or narration was
changed. Executable SHA-256:
`7afe02c741efbcca77000cfb89aeee66ecb31c1d7b532b189d26cc2ddd2578d8`.
DMG SHA-256:
`32218d539ab4666819844db415daaa88f514c8a39788282a0a441954f90ff342`.

## Keep Guide actions visible in a narrow pane — 29 October 2026

Guide section navigation now reflows inside the Guide pane itself: the section
counter gets its own row and Previous/Next share the next row. This works when
the file and source panes are resized, even if the overall window remains wide.
The Guide me / Show flow / Show source actions also wrap instead of colliding.

`npm run tauri -- build --no-sign --bundles app` completed. The rebuilt app was
opened on the existing `pillow-rs` Guide; after narrowing the Guide pane, the
section controls remained visible in two rows, then returned to their original
layout when the pane was widened. No test suite was run, and no repository
snapshot, saved answer, or narration was changed. The screenshot was inspected
live and not archived. Executable SHA-256:
`69a29bef6728990291caa65ba2ed108fa0be634ecd11f192f001bf3a78104403`.

This addresses the narrow-pane placement defect only; the broader packaged UI
acceptance tracked by `13yp` remains open.

## Explorer and Changes recheck — 29 October 2026

On the same installed executable at 2880×1688, the populated
`pillow-rs` `main@e843f864fc` Explorer/Guide showed the header actions,
Guide/Flow/history tabs, section Previous/progress/Next controls, scene
count/arrows/dots, and **Full map** without overlap. In Changes, commit history
and the local compare card were present, but its ref fields were disabled and
**Stop comparison** was visible while the main pane still asked the reader to
select a commit and showed no result. I returned to Explorer/Guide without
changing refs, source, snapshot, answer, or narration. The screenshot was
inspected live and not archived because it contains repository source. This
records an unresolved Changes state, not a completed comparison; the remaining
screen matrix and packaged acceptance under `x21h`/`13yp` stay open.

## Search pane recheck — 29 October 2026

From the same populated Explorer/Guide, Search retained the existing local
query `NativeDrawCanvas` and showed matches across captured files, including 23
matches in `canvas.rs`, while indicating that more captured-code matches were
still being checked. Long paths were visually shortened in the results pane;
the selected source and Guide remained beside the results. I did not start an
AI explanation. The screenshot was inspected live and not archived because it
contains repository source. This is one populated-query state only; no-results,
partial-index, keyboard-shortcut, Explore with AI, and 200% reflow states remain
uninspected.

## Flow view recheck — 29 October 2026

The same exploration's Flow tab preserved the selected `canvas.rs` source and
rendered its saved eight-node relationship diagram in the explanation pane.
The Guide/Flow/history tabs stayed at the top, and zoom/fit controls remained
inside the diagram's lower-left corner. The visible graph fit within its pane;
I did not select a node or trigger an explanation. This is a single populated
diagram state, not a dense-graph, no-visual, keyboard, accessibility, or
200%-zoom review. No saved answer or repository data changed.

## Chat history view recheck — 29 October 2026

The same Guide's Chat history showed its question grouped under the repository
and level, labeled **Partially completed**, with an **Open explanation** action.
Search is available at the top; Diagnostics and the four saved explanations
remain collapsed until requested. This is the current root question only, not
a nested branch or notification-origin return. I did not open diagnostics or
change any saved state.

## Notification action alignment — 29 October 2026

The packaged build now uses the shared quiet-button treatment for notification
actions, with a stable minimum target height and spacing between actions. A
single notification shows one **New** state badge; grouped notifications retain
their event count and unread count. I opened the panel in the installed
`pillow-rs` workspace and visually checked the header and several action rows.
The actions remained aligned and readable at the current window size. I closed
the panel without activating an action; the unread total remained 38. The
verified executable SHA-256 is
`d86df9d68c8bb7919943d8ba48046eba1ba6af53e84cdd8d6fa6a9b7e274e6a2`, matching
the packaged bundle. This is a focused notification fix; the wider screen and
accessibility acceptance under `x21h` and `13yp` remains open.

## Connections dialog account controls — 29 October 2026

Opening **Connections** in the packaged workspace showed its tabs and footer,
but the Explanations panel was blank even though Codex was connected. Source
inspection found the authenticated account details inside a closed `<details>`
element whose summary is intentionally hidden in the embedded Connections
panel. The content therefore had no visible disclosure control. The source fix
keeps that account section open while embedded; the standalone generation
dialog retains its collapsed-details behavior.

The rebuilt app was installed and reopened on the existing `pillow-rs`
workspace. Explanations now shows the connected-account status and an explicit
Model settings disclosure; opening it shows the saved GPT-6-Luna/max controls.
The Voice tab shows Pocket TTS as ready on this Mac, and the footer actions stay
aligned below the content. No connection or voice preference was changed and no
AI/TTS request was sent. The screenshot was inspected live and not archived
because it exposes account details. The verified executable SHA-256 is
`1328dc1fd4dba061fa3ff185ba671a58219b31af7b0d100c582c9a05b2649a44`. No test
suite was run; the broader `x21h`/`13yp` screen and accessibility acceptance
remains open.

## Changes comparison action placement — 29 October 2026

The comparison form placed both version fields, the swap control, and the
Compare/Stop action on a single row. The action consumed width that should be
available to the version fields and became stretched across its row at narrower
pane widths. The form now keeps the two version fields and swap control on one
row and places the action beneath them, aligned to the right. At the narrowest
layout, the ending field remains on its own row and the action stays right
aligned. The rebuilt app was installed and inspected at 2880×1688 on the
existing `pillow-rs` workspace: both long commit IDs remain readable in the
wider fields, **Compare versions** sits on its own right-aligned row, and the
partial comparison result remains below the form. I returned the app to
Explorer/Guide afterward. No repository files, saved explanations, connection
settings, or narration changed; no AI request was sent. Executable SHA-256:
`3705ee558d7d9101a7efd1e3128e055b6aeb96e8336d999e865bb73360df51ab`. No test
suite was run; `git diff --check` and the production app build completed.

The packaged check exposed one remaining control-size mismatch: Radix's ghost
button padding was being added outside the swap control's fixed width and
height. The swap button therefore looked taller and wider than the version
fields. It now uses border-box sizing, zero padding, and a 36px grid track. In
the rebuilt app at 2880×1688, the square control aligns with the input height
and the Compare/Stop action remains right-aligned. Build completed; no test
suite was run.
Installed executable SHA-256:
`5a1f955959194e05282425d0b6d6dc134e50cf1fcff5da164c062507e05b6e08`.

The same Changes view also hid the inactive **PR Guide** tab trigger because a
selector intended for inactive panels matched every inactive tab descendant.
It now hides only inactive `tabpanel` elements, keeping **Diff** and **PR
Guide** visible together.

The final packaged build was opened in the existing `pillow-rs` workspace. Both
Change view buttons render side by side; the swap control matches the version
field height and the compare action is aligned below them on the right. The
existing large comparison remains explicitly partial. Installed executable
SHA-256: `cc51410ad3a308639cd9e5f76407cae84faa8dab442109d03a55ab7a8b202f24`.
No test suite or AI request was run.

## Changes view controls move above result details — 29 October 2026

In the populated `pillow-rs` Changes screen, the **Diff** and **PR Guide**
buttons appeared after version metadata, omission notices, and the static
relationship warning. That pushed the changed-file list and active diff below
the fold. The two view controls now sit directly under the result heading;
version and analysis details follow the file/diff workspace. The packaged app
was reopened and the tabs were confirmed above the visible changed-file list
in the partial comparison state. The saved comparison remained marked partial.
No AI request or test suite was run; `npm run build` and the macOS bundle,
metadata, and DMG checksum checks passed. No source screenshot was retained
because it contains repository code. Keep the full-screen and responsive UI
audit open.

## Guide section buttons were too far apart — 29 October 2026

The current packaged `pillow-rs` Guide screenshot showed Previous and Next
anchored to opposite edges of the reader while the section count sat in the
middle. This made the three controls read as unrelated actions. The Guide
section navigator now keeps Previous, the section count, and Next together as
a centered group at regular widths. Existing narrow-pane behavior remains a
two-row layout, with the count above the two actions.

Implemented in `src/workspace/workspace.css` and pushed as `f18a3ac`. The
unsigned macOS app was rebuilt and reopened in the existing 2880×1688
`pillow-rs` workspace at `main@e843f864fc`. The live Guide shows Previous,
“Section 1 of 4,” and Next as one centered group; the selected `canvas.rs`
source, answer, section, and narration state were retained. No repository
snapshot, saved explanation, or audio was changed, and no test suite or model
request was run.

Installed executable SHA-256:
`14392f3e716308ada5c126d415b2818a3685e87ee79e06250362ab8bf3f5fcae`.
DMG SHA-256:
`390c80152af9a82980122a7bf442b3a6684754e3d040a4e6ecb7b11fd7ecedbc`.
This verifies one populated Guide layout; responsive, keyboard, accessibility,
and cross-screen acceptance remain open under `13yp` and `x21h`.

## Put section navigation after the section content — 29 October 2026

The packaged Guide still placed **Previous / Section / Next section** before
the current slide. Centering the buttons fixed their horizontal grouping but
left the primary placement wrong: a user reached the next-section action before
reading or hearing the section it advances. The pager now renders after the
complete slide, including its visual, narration, and source/details controls,
so navigation follows the content it controls. The existing centered grouping
is preserved.

The rebuilt app was reopened in the existing `pillow-rs`
`main@e843f864fc` workspace. The selected `canvas.rs` Guide remains at section
1 of 4, and the live view shows the pager below the scene player, details,
section notes, and **Explore further**. Accessibility order also places the
pager after the slide. No question, snapshot, saved explanation, or narration
was changed; no test suite or provider request was run. `git diff --check`, the
frontend build, license checks, native bundle, and DMG metadata/checksum
verification completed.

Installed executable SHA-256:
`6eefd11bc24d4e8ad9d56fcb341705aea288dd9fb24acb1ca1451c9f60a1d047`.
DMG SHA-256:
`7641c1a8108a13e55808fd850c058f7f87c2c64cc686827bbf72d75df8bc1eea`.
Responsive, keyboard, accessibility, and cross-screen acceptance remain open
under `13yp` and `x21h`.

## Native split-pane reflow observation — 29 October 2026

During a follow-up packaged-app review, a screenshot showed the code pane
covering the available reading area even though the accessibility tree still
contained the Guide and section pager. Focusing the source/explanation divider
and moving it left changed its saved ratio from 43% to 38%; the next screenshot
showed source and Guide side by side, with scene controls and the section pager
visible. No source, snapshot, answer, audio, or provider state changed. The
immediate layout is usable after the resize, but the initial discrepancy was
not explained or reproduced after restart. Track as an unresolved native
reflow/layout observation; do not claim a durable code fix from the manual
resize. Include initial/reopened split restoration in the `13yp`/`x21h`
packaged layout matrix.

## Default Changes comparison completes — 29 October 2026

On the same installed build, opening Changes first showed the expected loading
state: the default parent-to-head comparison was running, the ref fields were
disabled, and **Stop comparison** was available. After the native comparison
finished without another user action, the screen displayed **Partial
comparison**, both **Diff** and **PR Guide** tabs, 128 visible files, and an
omission notice for 111 further changes. Static relationships were explicitly
marked partial. The earlier loading observation was not a stuck comparison;
do not classify it as a failure or add a retry workaround. No branch switch,
working-tree change, AI request, or content edit occurred.

This is one populated comparison on `pillow-rs`, local `e843f864` to
`c5104cbaaefbd359c88746801a1fd60e52a1f6d1`, at 2880×1688. The screenshot was
inspected live but not retained because the diff exposes repository content.
The executable and DMG identities are recorded above. Empty repositories,
error/cancellation states, alternate ref pairs, and responsive/accessibility
checks remain open.

## Guide button reflow threshold — 29 October 2026

The surrounding Guide content reflows at an 820px pane width, while the
visual scene toolbar and narration controls waited until 700px. Between those
widths, a resized Guide could keep its desktop control row in a pane already
treated as compact, crowding scene navigation, playback actions, and audio
seeking. The scene-control container query now switches at 820px
(`src/visual-story.css`, commit `f6d0de5`), matching the workspace Guide
breakpoint.

The unsigned macOS bundle rebuilt successfully with `npm run bundle:macos` at
`d552a2b`, including license, frontend, bundle, and DMG metadata/checksum
verification. It was installed over the existing app after preserving the old
bundle as `Code Atlas.pre-f6d0de5.app`, then reopened on the existing
`pillow-rs` `main@e843f864fc` workspace. At 2880×1688, the scene title is on its
own row, scene count/arrows/dots stay together below it, and **Full map** stays
at the right. Narration status sits above Replay/Play/Stop; elapsed time is
aligned at the far edge and the seek track spans its own row. Section
Previous/Next remains grouped below the full slide. The selected `canvas.rs`,
section 1 of 4, and scene 1 of 2 survived the restart. No snapshot, saved
answer, audio, or AI request changed.

Installed executable SHA-256:
`3da6d65e6b7e2b89d17f18801be20e12379fabec94d70a31a43aff98ad85724d`.
DMG SHA-256:
`39445d594aad88f479657d28e5c2ea95054aa9f0491ffe973ca012a2f721cdd6`.
This checks one real populated Guide at one window size; other widths,
keyboard, accessibility, and cross-screen acceptance remain open under
`13yp` and `x21h`. No test suite or provider request was run.

## Packaged Guide, Flow, Search, Changes, and history check — 29 October 2026

On the rebuilt app recorded above, the existing `pillow-rs`
`main@e843f864fc` workspace was used without changing repository data or
calling an AI provider. Flow showed the saved eight-node explanation beside
the selected `canvas.rs` source. Search restored `NativeDrawCanvas`, showed
36 matches across 12 files, and displayed its **Local search may be
incomplete** notice; switching to Search did not replace the current Flow or
source. Chat history showed the current question and four saved explanations.

Changes restored the local commit history and completed its default comparison
from `e843f864fcbdf6b53feead608f7ab53025541d8e` to
`c5104cbaaefbd359c88746801a1fd60e52a1f6d1`. It correctly labeled the result
**Partial comparison**, listed 128 visible files, disclosed 111 omitted
changes, and labeled static cross-file relationships partial. **Back to
exploration** returned to the same Flow and selected source; Chat history and
Guide tabs remained available, and Guide restored scene 1 of 2 and section 1
of 4. No source selection, explanation, snapshot, or audio changed.

The screen was inspected live in the installed executable recorded above; no
image containing local source/diff content was retained. This covers one
populated repository state and the normal Flow/Search/Changes/history path.
It does not cover nested follow-up navigation, empty/error states, keyboard,
VoiceOver, high zoom, or other window sizes. Those acceptance gates remain
open under `13yp` and `x21h`.

## Guide navigation button alignment — 29 October 2026

The section pager grouped Previous, its counter, and Next in the middle of the
reader, and the narrow-pane scene toolbar also centered its controls away from
the reading edge. The pagers now use a three-column layout: Previous on the
left, progress centered, and Next on the right. In compact Guide panes, scene
navigation stays left and the optional map control stays right. The compact
zoom rule uses the same scene-toolbar placement.

After `npm run bundle:macos` completed, the rebuilt app was installed over the
existing application and reopened on Fontdone `main@5249b72371`. The saved
Guide restored without changing its selected file, question, answer, snapshot,
audio, or provider data. The populated pager visibly showed Previous at the
left, “Slide 2 of 6” centered, and Next slide at the right. The scene pager
stayed grouped at the left edge in its narrow pane. `git diff --check` passed;
no automated test suite or provider request ran. The build output reported the
existing Rust unused-code warnings and Vite's large-chunk warning.

Installed executable SHA-256:
`565a5043cba48609ee5879b1bc1d1c5354f3fd3c0f632a3f36b21fa049c29c32`.
DMG SHA-256:
`6bf0445fc1969d09874f77d774da3b04ed9184db5d4c26cb621313026309bba4`.
This checks one populated Guide at its restored pane width; other widths,
keyboard navigation, accessibility, and high-zoom layouts remain open under
`13yp` and `x21h`.

## Compact scene controls re-centered — 29 October 2026

The user reported that the buttons still appeared misplaced. In the reopened
Fontdone Guide, the section pager was aligned, while the scene count and its
previous/next/dot controls were still clustered at the left in the compact
Guide pane. The compact and zoom-compact layouts now center that group beneath
the scene title; the optional map action stays on the title row.

`npm run bundle:macos` completed. The installed app was reopened on the same
Fontdone snapshot and Guide, and the live screenshot confirmed the centered
scene group. No test suite or provider request ran, and no repository snapshot,
saved answer, or narration changed. Installed executable SHA-256:
`285816cc91e3d590b8b48052bf4c32bc85fb771c549948f87264db38703528fe`.
DMG SHA-256:
`1013bfe209635f49ed14c356bdf1ec8ae2ee45f555cd38cf5702544e8ed82cde`.

## Scene navigation edge alignment — 29 October 2026

The user reported that the scene buttons still looked misplaced. The live
Fontdone Guide showed Previous and Next arrows grouped beside the dots at the
center of the bar, making the navigation target hard to scan against the
section pager below. The scene title and optional **Full map** action now occupy
the top row; Previous and Next sit on opposite edges of the next row, with the
scene count and dots centered between them. A one-scene visual omits disabled
arrows and redundant dots. This applies at normal pane sizes, compact panes,
and app zoom's compact layout.

`npm run build` passed (TypeScript and Vite production build), and
`npm run bundle:macos` produced the app and DMG from commit `4a8c13c`. App
executable SHA-256:
`a5f4bb7d57dbcf72ad7234ae2376f4b7ca7ca6117cdcfa82ad2c8bce6e3adb3c`.
DMG SHA-256:
`8583edbf10052f241b4405d244bf769480bab448022a01b8e9e07c09444ce08c`.

After the four other answers and attention items were confirmed in the saved
workspace, Code Atlas was closed through its guarded Quit flow so local state
could flush. The previous app bundle was preserved as
`Code Atlas.pre-31de6f2.app`; the new bundle was installed and reopened. The same
Fontdone snapshot, selected `glyf.rs`, Guide, code selection and saved answer
returned. The live app screenshot shows `Scene 1 of 1` centered, with the
disabled arrows and one-scene dot removed; **Next section** remains on the
right. No provider request or narration playback was started during this check.

Installed executable SHA-256:
`a5f4bb7d57dbcf72ad7234ae2376f4b7ca7ca6117cdcfa82ad2c8bce6e3adb3c`.
DMG SHA-256:
`8583edbf10052f241b4405d244bf769480bab448022a01b8e9e07c09444ce08c`.

The available saved visual has one scene, so it verifies the removal of
redundant controls but does not visually exercise the new multi-scene edge
placement at other pane widths. Keep broad responsive and multi-scene acceptance
under `13yp`/`x21h` open.

## Recenter Guide navigation buttons — 29 October 2026

The edge-aligned scene arrows and section pager still separated each action
from the progress it controls. Both navigation groups are now centered: scene
Previous / count and dots / Next sit together under the scene title, and the
section Previous / count / Next sit together after the section content. Narrow
layouts keep the progress label above a centered pair of section buttons.

`npm run build` passed. `npm run bundle:macos` passed, including notice checks,
app and DMG bundling, metadata verification, and the DMG checksum. The rebuilt
app was installed and reopened on the existing Fontdone `main@5249b72371`
workspace with `src/tt/glyf.rs` and the saved Guide answer restored. The native
screenshot shows the section controls clustered at the center of the Guide
pane. Its saved visual has one scene, so multi-scene arrow placement and narrow
pane reflow still need a populated visual for direct screenshot acceptance.
No test suite, AI request, narration request, repository capture, or saved
explanation change occurred. The build reported the existing Vite chunk-size
warning and 36 Rust warnings.

Installed executable SHA-256:
`e37fbf64783e1b51dc27559d8849fa4f2f83f7656018b25707577ebabb432c36`.
DMG SHA-256:
`5a4f5777434514a05f19c6f01aafb5a07bea7bfa75c3f536e64e31df1e73c157`.

## Remove navigation overlap and header overflow — 29 October 2026

The user reported that scene navigation and the top menu overflowed as if they
were positioned over the content. In section-sequence mode, both the guide
pager and the scene toolbar could become sticky in the same scrolling pane.
The guide pager also used content-sized columns, which could exceed a narrow
pane. Both navigation layers now remain in document flow; the centered pager
uses bounded tracks and wraps to two rows in compact panes. The app header now
reflows to two rows below 1150 CSS pixels, before its repository and account
controls can collide.

`npm run build` passed. The unsigned native app bundle built and was installed
over the previous app, which was preserved as
`Code Atlas.pre-nav-overflow-20261029.app`. The reopened Fontdone workspace
retained its snapshot, selected file, and saved answers; its section pager is
visible after the answer in normal reader flow. The current saved Fontdone
visuals do not provide a multi-scene player for a direct scene-arrow screenshot,
and a narrow-window resize check remains open. No test suite, provider request,
or repository-content change occurred. Installed executable SHA-256:
`001fe24fe254ce19f7f4e407eed8155c3eacf29a7d295f862595bc9aefa4aa41`.

## Follow-up: scene strip and header reflow — 29 October 2026

The user reported that the scene navigation and top header still overflowed.
The current scene index let every scene button wrap into more rows as guides
grew, while the header waited until 1150 CSS pixels to split its repository and
account controls. Scene buttons now stay in a single horizontal scroll area
between fixed previous/next controls; the header reflows into two normal-flow
rows up to 1400 CSS pixels. The existing scene-layout source-contract
assertions were updated to describe the title-row map action and bounded
horizontal scene strip; they were not executed.

`npm run tauri -- build --no-sign --bundles app` completed on 29 October. The
new bundle was installed over the existing app and reopened on the saved
Fontdone workspace; the installed executable SHA-256 matches the build:
`ccf51824e4a6b3477b55b3d6355c2339d0413ac75cfbe1f03a2ddc2358dc01f2`. The live
screenshot shows the repository/header controls within the window. The saved
Fontdone answer scenes could not be reviewed because its stored sections now
report a schema mismatch, so multi-scene navigation and narrow-window reflow
remain unverified. No tests, provider requests, narration requests, or saved
content changes were made. The frontend build retained its existing large
chunk warning and Rust build warnings. Keep packaged multi-scene and viewport
acceptance open under `x21h` and `13yp`.

## Follow-up: remove floating scene controls and reflow earlier — 29 October 2026

The user reported that both the scene controls and application header still
looked detached from the content and overflowed the window. Source review found
the scene toolbar and audio bar still defaulted to sticky positioning, with a
workspace-only override that depended on a particular reader ancestry. Scene
controls and playback now stay in normal document flow wherever the shared
visual player is used. The application header switches to its two-row layout at
1600 CSS pixels so repository and connection controls reflow before a typical
window reaches the previous breakpoint.

Source-only update in `src/visual-story.css`, `src/workspace/workspace.css`, and
`src/App.css`. No build, test, screenshot, or packaged runtime check ran. Verify
the top header and a populated multi-scene Guide at the affected native window
widths before claiming acceptance; `x21h` and `13yp` remain open.

## Replacement build observation — 29 October 2026

`npm run bundle:macos` completed for `6fe1350` with the existing license
inventory and bundle metadata checks. The unsigned app and DMG were built and
the DMG checksum/metadata verified. The app was replaced in place and reopened
on the existing Fontdone snapshot. The top bar reflowed into two normal-flow
rows with the repository and connection controls visible within the window;
the prior top-level saved-chapter schema error no longer appeared, and an
existing answer with its visual was available. Other saved answer parts still
report unavailable. The saved reader state did not show the scene pager, so
populated multi-scene placement and narrow-window acceptance remain open. No
tests, provider request, narration request or repository-content change ran.
The installed executable matches the built bundle:
`346265301244249c77fde19f9fde2f4e1b46d8b1c9a13d82cfaa0bfacd62dfe6`.

## Follow-up: keep header labels and scene navigation within their tracks — 29 October 2026

The user still reports that the scene navigation and top header overflow.
Source review found remaining intrinsic-sizing paths: the section pager's
center grid track used `max-content`; long repository/snapshot labels and
exploration links could retain their flex min-content width. The pager now gives
all three controls bounded tracks; scene arrows and dots stay in a centered,
capped strip; header labels may shrink and ellipsize; long exploration titles
wrap inside their card. `npm run tauri -- build --no-sign --bundles app`
completed. The app was replaced in place, reopened on Fontdone
`main@5249b72371`, and its installed executable SHA-256 matches the built one:
`585471e4919cccc7d51bb990f727b07f80716e80536fb43affa1bf4f7084f957`. The
current screenshot shows the top header within the window and a long nested
exploration title wrapping within its card. Keep narrow-window and populated
multi-scene acceptance open: the selected saved answer has only one scene and
the active app window remained full-width during this review. No test suite or
provider request ran; existing saved content was not changed.

## Follow-up: explicit header and scene-navigation tracks — 29 October 2026

The user reported that the scene pager and top header still looked misplaced
and overflowed. The final header layout now uses named grid areas: at ordinary
wide widths, the repository sits between the brand and account actions; below
1600 CSS pixels, repository and account actions occupy separate in-flow rows.
Small-width and native zoom layouts retain the selected snapshot label while
allowing control labels to shrink. The scene pager now reserves fixed columns
for the previous/next controls and a capped center column for the count and
scrollable scene strip. These controls stay in the reader's normal flow.

`npm run build` and `npm run tauri -- build --no-sign --bundles app` passed.
The new app was copied over the installed bundle after the previous process
closed cleanly. The built and installed executable SHA-256 is
`8c16c44b91abd83d8679152f7487662b2decc6ac836213fe5413f12d2a40f28d`; the
replacement launch was requested. The active desktop remains covered by the
screen saver, so I could not visually recheck the rendered layout. No tests,
provider requests, narration requests, or repository-content changes were
made. Keep narrow-window, populated multi-scene, zoom, and accessibility
acceptance open under `x21h` and `13yp` until a current packaged screenshot is
captured.

## Follow-up: stop header and scene controls escaping their layout — 29 October 2026

The user again reported the top menu and scene navigation overflowing. The
application header was entering its stacked layout below 1600 CSS pixels, which
put a two-row header on ordinary desktop windows. Its repository and snapshot
menus also used fixed vertical offsets, so they did not follow a reflowed
header. The header now stays on one row until 1180 CSS pixels; repository and
snapshot menus are anchored to their control groups and constrained to the
window height. The close-menu scrim covers the app body without relying on a
fixed header height. The scene pager now has a centered, 604-pixel maximum
track, with a flexible bounded center for its count and scrollable scene dots.

`npm run bundle:macos` passed, including the frontend build, license inventory,
bundle metadata, and DMG checksum. The rebuilt app was installed and reopened
on the existing Fontdone snapshot; the installed binary hash matches the
bundle (`b7f62d3d1370194535ac33a8b53b67828eeab1729c8e7cd1e37a3eeb0f500041`). The
current saved answer has one scene, so multi-scene arrows and narrow-window
reflow still need a populated native visual check. No tests,
provider requests, narration requests, or repository-content changes were
made. Keep full viewport, multi-scene, zoom, and accessibility acceptance open
under `x21h` and `13yp`.

## Follow-up: keep the notification count and scene controls in flow — 29 October 2026

A resized native window showed the unread count detached from the bell, while
the user still reported scene controls overflowing like absolute-positioned
elements. The notification count now participates in the bell button's flex
layout. Scene title, map action, and scene pager wrap as normal-flow items; the
workspace preview player wraps its controls instead of holding a fixed grid.

The focused badge and scene-layout checks pass, and `npm run build` passes. The
unsigned app/DMG bundle was rebuilt, installed, and reopened on the existing
Fontdone snapshot; its header and notification count fit together in the live
native screenshot. This saved answer has one scene, so multi-scene arrows could
not be visually exercised. Four unrelated assertions in the broader focused
test files still fail (notification outcome grouping/source contract, Full map
visibility, and narrow Guide source actions); the directly affected assertions
pass. No provider request or repository-content change ran. Keep multi-scene
and narrow-window review open under `x21h` and `13yp`.

## Follow-up: wrap the header and scene controls as normal-flow groups — 29 October 2026

The user again reported that scene navigation and the top application menu
overflowed as if they were positioned over the page. The header used
intrinsic-width grid tracks around independently wrapping control groups, and
the repository popover shifted left at narrow widths. Replaced the header row
with wrapping flex groups so project controls and account actions move to
separate rows when the available width runs out; long connection labels now
shrink safely and the repository menu stays anchored within the header. Replaced
the scene pager's nested three-column grid with a centered flex row whose scene
dots scroll inside the bounded reader width. The design preview now uses the
same in-flow, wrapping header and its preview search control can wrap below the
brand without covering neighboring controls.

Updated the existing source-contract assertions for these layout contracts.
`npm run build` and `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run bundle:macos`
passed. The build checked notices for 148 npm packages and 391
Cargo crates, verified bundle metadata, and verified the DMG checksum. The app
was installed at `/Users/lazytrot/Applications/Code Atlas.app`; the prior app
is preserved as `Code Atlas.pre-flex-header-scenes.app`. The installed binary
SHA-256 is `55eee375b546febc618a5b896149fd0e40eaa449117246bb2d78a4d16712b7dc`;
the DMG SHA-256 is `806dc06710f0b94f53ce25858cbeab881029779c6984a1347971498d071189f3`.
The reopened native app shows the header controls contained in one normal-flow
row at the current full-width window. Its saved Guide has one scene, so there
are no previous/next buttons or dots to inspect. No test suite, provider call,
narration request, or repository-content change ran. Keep narrow-window,
multi-scene, zoom, and accessibility acceptance open under `x21h` and `13yp`.

## Follow-up: explicit header and scene rows — 29 October 2026

The user still reported overflow in the app header and scene navigation. The
previous wrapping-flex change did not fully resolve their concern. These groups
now use named CSS Grid regions: at widths up to 1560px, account actions move to
their own in-flow row beside the project identity; below 1180px the brand folds
away and project controls and actions occupy separate rows. High zoom uses that
same compact layout. Scene title and optional map action share a bounded first
row, with a separately centered, width-limited scene pager below; long dot lists
scroll within the pager.

Focused source-contract checks for the Guide layout and compact app header pass.
The macOS app and DMG built successfully and the replacement app was installed
and reopened on Fontdone. A native screenshot shows the repository controls on
their own row, connection/theme/notification/help controls on the next row, and
the scene title and centered `Scene 1 of 1` counter contained in the Guide
reader. The saved answer has one scene, so previous/next arrows cannot be
visually exercised. Narrow-window and multi-scene acceptance remains open. The
broader one-off run of the two surrounding UI test files also reports three
failures outside these layout rules (Full map visibility, saved-visual state,
and relationship-preview wrapping); they were not changed by this fix.
