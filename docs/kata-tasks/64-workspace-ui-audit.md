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
