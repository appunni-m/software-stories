---
kind: task
key: current-design-gap-audit
parent: contract
title: Record concrete UI gaps against the approved designs
priority: 1
labels: product, design, audit, ux
---

> **Current workspace direction — 16 October:** the [persistent code/chat/exploration workspace](../design/WORKSPACE_EXPLORATION.md) supersedes earlier primary-navigation proposals. It keeps the explorer left, chat at the top, Guide / Flow / history within each exploration, inline code details, and background notifications. The interactive preview remains illustrative; the production app now implements this workspace with repository-backed data. Historical observations below remain dated evidence and do not prove the current packaged build matches the approved design. The packaged audit and outstanding UI findings are tracked in [workspace UI audit](64-workspace-ui-audit.md).

Latest packaged spot check: [29 October native workspace observation](../acceptance/2026-10-29-workspace-native-spotcheck.md). It confirms one populated Guide, Search, Changes, and Explorer state at 2880 × 1688 against the installed executable hash. Screenshots were inspected live but not retained; the five-reference screenshot ledger, narrow-window/multi-scene states, and accessibility review remain incomplete. Keep the issue open.

## Native workspace refinement — 2026-10-16

The user clarified that understanding code comes first, with navigation resembling VS Code, a dynamic player, an understandable code path, and no repeated backend selection. This supersedes any interpretation that matching three navigation labels alone establishes design parity.

Implemented in this iteration:
- Removed the permanent Project context sidebar and large Files hero from the populated workspace. The explorer, actual source and explanation tool pane use the window height with independent scrolling.
- Added explicit Explanation / Flow / Player controls. Flow is focused on verified calls/imports/test references for the selected file, with clickable source/evidence nodes; it does not claim to be a runtime trace.
- Reduced normal-state source metadata and duplicate explanation actions. Captured paths are the default; skipped/excluded paths remain accessible on demand. Folder rows are no longer misleadingly marked excluded.
- Codex is the default generation entry; an explicit backend choice is remembered across file, planning and PR entry points. Existing Codex model/reasoning preferences remain in use. A one-file Codex action prepares local source review automatically without sending or reusing consent.
- Kept Previous/Continue in a separate, always-visible player row. Removed generic side cards and empty “no diagram” placeholders; evidence quotes and audio controls remain available on demand.
- Kept the learning-plan manager behind its relevant Guide disclosure.

See [native observations, screenshots and remaining deviations](../acceptance/2026-10-16-code-workspace.md). Full Changes composition, first-run polish, generated visual quality, question-to-source discovery, and accessibility acceptance are not established by this iteration. The question-to-source work is separately defined in Kata `xvjk` and `55-question-to-source-discovery.md`.


## Updated native gap ledger — 16 October

| Approved reference | Classification | Observed result / remaining difference | Component / priority |
| --- | --- | --- | --- |
| First run | Partial | Outer project sidebar is removed. The welcome still shares a Files-oriented hero and shell controls rather than the dedicated welcome composition. | `AtlasEmptyState`, `App` / P1 |
| Guide | Partial | Plan/coverage managers are now collapsed and the tour action is visible. Suggested files can still begin with configuration because the ranking follows inventory order. | `GuidePlanScreen`, `RepositoryPlans` / P1, `xvjk` |
| Files | Partial, major layout gap repaired | Native source, explorer and numbered explanation are now visible together with one Explain action. Flow opens verified code links and Player opens accepted sections. Resizable panes, editor tabs and a reliable concise summary contract remain. | `AtlasWorkbench`, `FileExplanationPanel`, `FileFlow` / P1 |
| Walkthrough | Partial, navigation gap repaired | Native Previous/Continue remain visible even with long expanded evidence. Generic help panels and empty diagram placeholders are removed. The synthetic chapter has no generated diagram, so visual-story quality and the approved section-specific concept panel remain unqualified. | `WalkthroughScreen`, `VisualPanel` / P1 |
| Changes | Deviation in source; populated native state not inspected this pass | The stacked comparison/PR controls still need the reference’s persistent changed-file/diff/explanation composition. This pass does not establish native Changes parity. | `GitChangesScreen`, `PrGuideStudio` / P1 |

The [dated native evidence](../acceptance/2026-10-16-code-workspace.md) records the window size, executable identity, native interactions, local captures and published synthetic screenshots. The historical source-only baseline below is retained for comparison.

## User outcome
A newcomer can open Code Atlas and find the intended starting point, inspect what real files do, follow a section at their own pace, and understand a change without learning the app’s internal workflow names first.

## Relationship to existing work
This is a separate, evidence-first design audit. It records exactly where the current app diverges from the approved screens and what those differences cost the user. Issue `13yp` owns implementation and packaged-app parity; this task does not replace or close it.

## Design authority
Use `docs/design/proposed-00-first-run.svg` through `proposed-04-changes.svg` and `docs/design/PROPOSED_SIMPLIFICATION.md`. The old Atlas / Code Lens / Walkthrough PNG concepts are historical. The primary labels Guide, Files, and Changes in `src/lib/shell.ts` already match the approved direction; do not treat relabeling as a design fix.

## Static source gap baseline — 2026-10-15
This baseline comes from reading the current checkout only. The app was not built, launched, or screenshot-reviewed for this task. Packaged-app behavior and build identity are therefore unverified. Treat the entries below as source-confirmed layout or flow differences, then verify what reaches the user in the packaged build.

| Reference | Current source behavior | Concrete gap and user impact | Source evidence / status |
| --- | --- | --- | --- |
| First run (`proposed-00`) | The initial route is `atlas` (the Files destination). With no repository, `AtlasEmptyState` provides an Open repository action, a three-step explainer, and local-browsing reassurance inside the normal app shell. | The approved reference is a dedicated welcome surface with one clear action. The current route also renders primary workspace navigation, repository/snapshot controls, search, More, help, and the persistent Project context sidebar. The user first lands in Files and must visually parse the full workspace shell. This is a partial match, not a missing open-repository action. | `src/lib/navigation.ts` (`initialNavigationState`); `src/App.tsx` (`AtlasEmptyState`, persistent shell). Packaged result unknown. |
| Guide (`proposed-01`) | When no accepted repository overview exists, `GuidePlanScreen` makes the hero file-first: “What does an important file do?”, opens the first file awaiting explanation, and says repository tours are optional. Accepted overviews get a Read the guide action; accepted guides are also reachable through More. | The approved Guide makes the short repository tour the primary start-here action, with file coverage and next files alongside it. A first-time user with no accepted overview has no obvious tour action and is steered away from the repo-level orientation shown in the design. | `src/App.tsx` (`GuidePlanScreen`, `acceptedOverview` fallback, guide actions). Source-confirmed; native presentation unknown. |
| Files (`proposed-02`) | The selected-file route composes the inventory tree, inline captured source, and `FileExplanationPanel`; relationship exploration is collapsed under an optional disclosure. | The core three-pane structure and optional map are present. Two visible hierarchy gaps remain: the persistent outer Project context sidebar makes the Files work area effectively a fourth column, and the explanation panel does not follow the reference's one-sentence summary, short numbered behavior steps, and explicit connected-file list. This makes the first-glance answer harder to find. | `src/App.tsx` (`SnapshotReadyState`, `AtlasWorkbench`, `FileExplanationPanel`); `docs/design/proposed-02-files-explainer.svg`. Source-confirmed; native presentation unknown. |
| Walkthrough (`proposed-03`) | `WalkthroughScreen` has a chapter/section rail, a long center column containing prose, evidence cards, file links, optional visual, narration, and transcript citations, plus a right rail with generic progress/help cards. Previous and Continue are rendered after all section content. | The design keeps a behavior diagram and source excerpt central, gives the right panel a section-specific key idea and term definitions, and keeps Previous/Continue visible at the bottom. Current controls can fall below long evidence/narration content; the right rail does not explain the selected section’s key idea or vocabulary. This undermines deliberate stop, inspect, and continue pacing. | `src/Walkthrough.tsx` (`WalkthroughScreen`, `EvidencePanel`, `VisualPanel`); `src/App.css` (`.walkthrough-section-actions`, `.walkthrough-side-notes`). Source-confirmed; common-window behavior unknown. |
| Changes (`proposed-04`) | `GitChangesScreen` stacks a full-width base/head form above `ComparisonResults`. Results stack the changed-file disclosures and `PrGuideStudio`; plain-language generation is a separate provider-backed workflow. | The design keeps changed files, focused before/after code, a behavior explanation, and evidence links visible together. Current source does not provide that persistent comparison layout or an always-visible explanation of the selected change; the reader must work through stacked disclosures and the separate PR Guide path. | `src/GitChanges.tsx` (`GitChangesScreen`, `ComparisonResults`); `src/PrGuideStudio.tsx`; `src/App.css` (`.git-changes-workflow`). Source-confirmed; native presentation unknown. |

## Required work
1. Open the packaged Mac app and record its app version/build identity and window size. Capture the real first-run, Guide home, selected-file Files view, one Walkthrough section, and populated Changes comparison. Use the same viewport for the SVG reference and app capture.
2. For each of the five references, classify the shipped screen as match, partial, deviation, or not inspectable. Attach the screenshot evidence and note the repository/snapshot/state used; never substitute the web preview or source labels for native evidence.
3. Review and correct the source baseline table above. Record the exact screen element that differs, the user consequence, the responsible component/route, severity, and the smallest specific fix. Mark unknowns instead of guessing.
4. Separate true mismatches from intended complexity. Preserve exact source citations, snapshot identity, explicit provider consent, local-only Git safety, and optional relationship exploration while simplifying the visible path.
5. Turn implementation changes into focused updates to `13yp` or a narrowly scoped follow-up Kata issue. Keep this issue as the complete, reviewable gap ledger.

## Source follow-up tracked under `13yp` — 2026-10-15

After this point-in-time baseline was recorded, the Guide no-overview source state changed: it now offers an optional repository-tour creation action through the existing exact-context review flow, keeps a per-file explanation action beside it, surfaces a saved current-snapshot draft for review, and offers an older accepted guide as an explicitly historical route. See the source implementation follow-up in `43-design-implementation-parity.md`. This updates source evidence only; it does not change the baseline's date or establish packaged-app parity. The five native screenshots and build/window metadata are still missing.

## Acceptance criteria
- One screen-by-screen gap ledger covers all five approved SVG references and includes the app build identity, viewport, evidence screenshot, source component, observed deviation, impact, priority, and proposed fix for each non-matching screen.
- The ledger confirms both what already matches and what does not. It explicitly records that the Guide / Files / Changes labels and the selected-file tree/source/explanation composition exist in source, while distinguishing that from verified packaged-app parity.
- The first-run state, no-overview Guide state, Files explanation hierarchy, Walkthrough controls/context panel, and Changes result layout are each directly compared against their specified design content and action hierarchy.
- Findings are specific enough to become implementation criteria without phrases such as “make it cleaner” or “match the mockup”; every fix names the control, content, placement, and expected navigation or return behavior.
- Packaged-app unknowns remain labeled unknown until observed. This task does not claim visual acceptance, accessibility sign-off, or completion of `13yp`.

## Exclusions
Do not implement UI changes in this audit task, regenerate the design images, add product features, or treat source inspection as a native-app review. Do not close `13yp`.

## Done evidence
A dated gap ledger with five packaged-app screenshots, build/window metadata, corrections to the source baseline, prioritized specific fixes, and links from each fix to its implementation issue.

## Historical contract gap ledger — 16 October

This records the native state observed on 16 October, before the persistent workspace was integrated. The later packaged app has since adopted the Explorer / chat / Guide workspace shell, so these rows are historical implementation gaps rather than claims about the current build.

| User requirement on 16 October | Application gap observed then | Delivery task |
| --- | --- | --- |
| Explorer remains left in every view; actual code beside explanation | Existing explorer/source composition belongs to Files; Guide is a separate destination. | nnsf |
| One pinned chat captures intent in any view | Guide route questions, selected-code dialogs and independent generators are separate entry points. | nsfd |
| Explain a selection at the current chat level | Exact-range explanation still enters the reviewed modal workflow; no persisted current-level chat turn. | nsfd, eapn |
| Ask once; everything runs in background | Discovery, source review, route acceptance and chapter batch are visible process stages. Native work is still owned by dialog lifetimes. | nw1h |
| Guide / Flow / history are views of one exploration | Main navigation is still Guide / Files / Changes; no shared persisted exploration tree. | eapn, nnsf |
| Flow drills from project context into implementation; details work in both views | Existing focused file flow and separate player do not implement this abstraction/branch navigation contract. | mrpj |
| Headed answer cards and real nested exploration counts | Saved routes/chapter revisions exist, but there is no stable originating-card/child-exploration relationship. | eapn, mrpj |
| Open last explored view and return parent/root | Existing source/reader route state is not the requested per-exploration view/graph/player persistence. | eapn |
| Standard components, consistent spacing and keyboard interactions | Preview uses Radix, CodeMirror, React Flow and resizable panels; production source/graph/forms still need migration. | nnsf |
| Notifications stay behind a bell, with no settings taking reading space | Global notices moved to the bell and compact header controls are implemented; capture/generation modal migration and durable outcomes remain open. | hstr, nw1h |

The preview demonstrates sample interactions and cannot close these native gaps. Do not replace actual generated narrative depth, source correctness, narration or accessibility with screenshots or source-level labels.

## Native Guide observation — 21 October 2026

The running Code Atlas window showed the Fontdone `main@5249b72371` snapshot and the persistent workspace: Explorer on the left, source beside Guide, a chat composer above the content, and Guide / Flow / Chat history views in the explanation pane. The selected Guide section was “PFR0: dispatch, validate, adapt”; its scene title was “One dispatch, distinct exits,” and Pocket narration was ready at 00:00 / 00:23. The story canvas appeared empty, and its accessibility subtree exposed the graph controls and attribution but no visible node labels. The code pane showed `src/font.rs:4062`.

This is a concrete Guide visual gap to reproduce. The running app bundle reported version `0.1.0`, arm64 ad-hoc signature, and executable SHA-256 `0fe1889c8cd503e0a631f606bc4377c570977464e4ab3a6f7d07b35adb59e278`; its source revision was not embedded or independently matched. No screenshot was saved with the audit, so this observation does not establish the cause or count as packaged acceptance. Recheck with a verified source/build identity, durable screenshot, and screen-reader-visible scene content. The saved answer and reader state were left unchanged. Other first-run, Explorer, Changes, history, accessibility, zoom, contrast, motion, and theme cases remain unreviewed in this pass; keep this task and parent `13yp` open.

### Follow-up — 21 October 2026

The same installed executable hash was still running. Its filesystem mtime is 21 October at 01:59 local time. The saved selected visual is valid and contains **8 nodes and 7 edges**; its reader progress is scoped to the selected revision/section and stores a viewport at zoom `0.18`. The live Guide canvas remained blank after using its Fit View control, and the accessibility tree exposed no node labels. The serialized reader position and viewport were unchanged by this check.

Current source contains the post-layout React Flow rehydration in `VisualStory.tsx` from `c91630b` (03:56 local) and dense graph readability follow-up `1609d27` (05:22 local), both later than the installed executable mtime. The installed build therefore predates these source changes by its observed timestamp, though its exact source revision is still unknown. `node --experimental-strip-types --test tests/visualStory.test.mjs` passes **39 tests**, including rehydration after data arrives and after the first layout frame. This points to stale installed code as the likely reason for the blank canvas; it is not proof that the current source build renders this saved scene correctly. A durable screenshot and packaged verification of a build containing these fixes are still required. Other approved screens and accessibility/theme cases remain unreviewed; keep this task and parent `13yp` open.

### Packaged Flow layout correction — 21 October 2026

On the installed Code Atlas 0.1.0 app (arm64, executable SHA-256 `f82913d8584744290627e0afaa5a2571af8da5162f3f9d8950abac2f1f85ea9d`), Fontdone `main@5249b72371`, the saved `parse_table_directory_at` Flow contained nine nodes. The prior build displayed them as a tiny horizontal strip in a tall canvas. The replacement build displays the same evidence as a readable 3×3 grid in a taller canvas; clicking **Base past input** kept Flow selected and opened `src/tt/mod.rs:94`. The nine source-link buttons were present in the accessibility tree.

The unsigned app/DMG build and metadata/checksum verification passed. The screenshot was reviewed live but not saved as a durable file. This verifies one Flow state only; retain the broad screen, accessibility, theme, and zoom/reflow checks as open.

### Guide prompt chrome reduction — 21 October 2026

In the live Fontdone workspace, a collapsed Guide question heading exposed generated request scaffolding such as “Create exactly one fresh Guide section for Fontdone main@…”, even though the answer card directly below already had a useful section title. That added repository/version metadata and generation instructions to the reader’s main content. The exact full prompt remains important for history and recovery, so it stays available by expanding the question.

The production reader now summarizes recognized generated section prompts to their code topic and first sentence, for example `parse_head` in `src/tt/head.rs`. The same summary is used in Chat history and as the accessible label for the answer card; the full prompt is exposed when its disclosure is expanded. This is provider- and repository-independent, retains the original prompt in the disclosure, and no longer exposes the entire prompt as a hover title. Verification: `node --experimental-strip-types --test tests/workspace-outcomes.test.mjs tests/workspace-refresh.test.mjs` passed (35/35); `npm run build` passed. Vite reports the existing large main-chunk warning. The source edit has not been installed into the running app because its conversation-save warning is still present; packaged visual confirmation is pending. This fixes one hierarchy issue only; screen-by-screen comparison, other pages, themes, zoom, and accessibility remain open.

### Reopen a compact explanation in place — 21 October 2026

The same Guide view showed older answer cards reduced to a heading and narration/player actions; the title itself had no action to reveal the interactive visual or its source hotspots. The reader could select another answer only by navigating through another surface. Compact answer titles are now buttons that focus that turn and render its existing visual story in the Guide pane. This uses the persisted focused-turn field and does not regenerate or alter the explanation. Verification: `node --experimental-strip-types --test tests/workspace-outcomes.test.mjs tests/workspace-refresh.test.mjs` passed (36/36); `npm run build` passed with the existing large-main-chunk warning. The running app has not been replaced while its conversation-save warning is active, so packaged visual and persistence confirmation remain pending.

### Selected-snapshot prompt summary — 21 October 2026

The live Guide showed another generated request form, “Create exactly one fresh Guide section from the selected snapshot. Explain only …”, whose setup text was not covered by the first prompt-summary patterns. The reader now removes that request prefix, presents the code subject as the compact heading, and keeps the complete prompt in its collapsed disclosure. A colon after the subject is treated as the boundary before implementation instructions. Verification: focused workspace tests passed (36/36); `npm run build` passed with the existing large-chunk warning. This source is not installed in the open app because a failed conversation save is still visible; packaged confirmation and the other page/design/accessibility checks remain pending.

### Cyclic Flow readability — 21 October 2026

The native Fontdone Flow for `TableDirectory::find` had seven nodes and a real record-scan loop. The cycle forced the prior layout into a source-order grid; return edges crossed the forward path and their labels sat directly over the lines. Source now ranks the forward reading spine by dependency, retains return edges, routes graph edges with smooth-step paths, and gives labels a theme-aware opaque background. The diagram layout version increments so saved viewports from the prior geometry are discarded and the new canvas can fit the graph. Focused Flow/reader checks passed (24/24); the native conversation journal layout/version tests passed (19/19); `npm run build` passed with the existing large-chunk warning. Opening version-2 diagrams now drops only their stale viewport in memory and leaves durable conversation bytes unchanged. The screenshot was reviewed live but not saved, and the running app has not been replaced while its conversation-save warning remains. Packaged verification and all other screen/theme/accessibility cases remain open.

### Remove the workspace-tools detour — 21 October 2026

The compact header's gear menu exposed separate “Browse accepted guides” and “Review a draft” destinations, plus a second click to reach keyboard help. These routes made the new conversational workspace feel like several old workflows. The header now opens keyboard shortcuts directly from one help icon; accepted explanations and draft editing remain available from their answer cards and history. This is a source-level navigation simplification, not proof that the native packaged screen has updated. Focused shell tests and `npm run build` passed; the full keyboard and VoiceOver journeys remain open.

### Current populated workspace snapshot — 27 October 2026

The live Code Atlas window opened to Fontdone `main@5249b72371` and shows the Explorer / Search / Changes / Explorations activity rail, the repository tree on the left, a CodeMirror source pane, the pinned composer, and Guide / Flow / Chat history within the explanation pane. The header shows the saved Codex connection and notification bell. The app's accessibility state reports 112 questions and 3 child explorations for the root, plus 183 unread updates. The visible Guide stack includes older unavailable Outline answers, a saved composite-glyph explanation, and a failed `gasp.rs:29` question; earlier saved explanations remain visible.

This verifies the layout hierarchy in one populated native state, but not the identity of the running build: the window did not expose a version or source revision, and the screenshot was inspected live rather than saved as a durable audit attachment. It also exposes a content-density concern: the root has accumulated many questions and unread outcomes, and unavailable/failed attempts remain interleaved with usable explanations. Keep the history intact; audit how the reader focuses the latest useful answer and how the bell groups these outcomes before changing retention. No user data was dismissed, deleted, or regenerated. The first-run, selected-file, populated Flow, Search, Changes, zoom/theme, accessibility, and verified-build screenshot checks remain open.

### Exploration-history scan overload — 28 October 2026

The foreground Code Atlas window still shows Fontdone `main@5249b72371` with a partial capture. Its accessibility tree reports 109 root questions, four child explorations, 39 unread notifications, and 215 saved explanations. The selected **Chat history** view renders a very long individual-entry list: repeated `glyf.rs` line-range questions, repeated PFR0 questions, and repeated whole-project questions appear alongside saved, partial, stopped, incomplete, and failed attempts. Diagnostics are interleaved with these entries. The view exposes no history search, status filter, or grouping control in this captured state, so finding a useful answer means scanning duplicate retries and failures. The tree currently has no selected guide section in view.

This is a concrete density and retrieval gap in the populated workspace. Keep every stored attempt and receipt. Improve the history view by grouping retries for the same question, making saved answers easy to distinguish from failed attempts, and putting diagnostics behind a compact disclosure; add direct search/filtering only if the approved workspace design keeps it lightweight. The current accessibility capture is live evidence only: its executable/source revision and durable screenshot were not verified. No history record was opened, edited, deleted, or regenerated, and no provider request was sent. Keep `x21h` open pending review of the other activities and first-run state.

### Bundle identity follow-up — 27 October 2026

The installed bundle at `~/Applications/Code Atlas.app` reports version `0.1.0`; its packaged metadata contains schema versions but no source revision. A second running process comes from the workspace's debug-bundle path, whose on-disk `Contents/Info.plist` is absent. The live accessibility capture did not expose a process ID or executable path, so the visible window cannot be attributed to either bundle. Treat the observed build identity as unknown; the version in the installed bundle alone is not evidence that it supplied the inspected window. No screenshot was saved.

### Packaged history search and retry grouping — 28 October 2026

Built the clean `main` checkout at `10a40ffa75603136279f8eafa98045dd22747705` with `CARGO_INCREMENTAL=0 npm run bundle:macos`. The TypeScript/Vite production build, unsigned macOS app bundle, metadata verifier and DMG checksum all passed. Replaced `~/Applications/Code Atlas.app`; the installed executable hash matches the build (`ba265121e47818a6e5514cc5485c8d0683e207c76b34dca4ebfd9f0e38da2425`). Removed the generated Rust target cache afterward; no additional app backup was created.

The packaged app restored Fontdone `main@5249b72371`, the selected `src/tt/glyf.rs` source pane, and the saved Codex GPT-6 Luna/max connection. In Chat history, the accessibility tree now exposes **Search chat history** and groups repeated prompts into collapsible attempt groups; the populated root shows groups of 2, 36, and 27 attempts, with the current/latest attempt directly openable and prior attempts under **Show all attempts**. The 215 saved explanations remain under one collapsed library. No question, answer, receipt, notification, or source snapshot was changed, and no provider request was sent.

This verifies one populated packaged history state and the executable identity. The screenshot was inspected live but not saved as an audit attachment. Search matching, nested explorations, other activity screens, first-run states, themes/zoom, keyboard traversal, VoiceOver, and the complete design gap ledger remain open; keep `x21h` open.

### Search and group exploration history — source change, 28 October 2026

The Chat history view now has a small search field. While a query is present it
matches questions and outcome summaries across all explorations in the current
snapshot, and matches saved answers by chapter or section title. Matching saved
answers appear immediately, with a result count and a clear action. Identical
questions after case and whitespace normalization are grouped within each
exploration. The current or latest attempt remains directly openable; earlier
attempts and their diagnostics are available under **Show all attempts**.
Clearing the query returns to the ancestry history and collapsed saved-answer
library. Search and grouping are local presentation state; they change no
stored questions, answers, receipts, or source order.

`npm run build` passes for the frontend. This source change is not installed or
checked in the packaged app yet. Status filtering, remaining diagnostics
density, other activity views, first-run parity, themes/zoom, and accessibility
remain open. Keep `x21h` open.

### Notification recovery action — source change, 28 October 2026

An earlier packaged notification showed an explanation needing attention but
disabled **Open explanation**, leaving no way to inspect the saved recovery
state from that notice. The action now reads **Review recovery** and opens the
local recovery dialog even when the preview is unavailable. That dialog shows
the validation/load failure and retains the explicit discard action; opening
it does not send a provider request or remove the saved record. This is a
source-only correction. It has not been built, installed, or checked in the
packaged app, and the other activity, first-run, theme, zoom, and accessibility
gaps remain. Keep `x21h` and parent `13yp` open.

### Sticky history search overlap — packaged correction, 28 October 2026

In the packaged Fontdone history view, scrolling moved retry rows beneath the
sticky search field. The input itself did not fully cover the rows at its lower
edge, so one line of question text remained visible through the field. Moving
stickiness to an opaque wrapper around the input and its spacing keeps the
scrolling history behind a solid surface. The earlier field-only CSS adjustment
did not fully fix this overlap.

`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run bundle:macos` passed: frontend
build, release compile, app/DMG bundling, metadata verification, and DMG
checksum. Installed bundle executable SHA-256 matches the built app
(`969c7414504f7cb1398c127bd6be3bc64c703efd144cdb6b7685bf7a835568e4`). In the
reopened packaged app, Fontdone `main@5249b72371` restored and the scrolled
history search covered the question rows cleanly. The screenshot was inspected
live but not saved. No question, answer, receipt, notification, or snapshot was
changed; no provider request was sent. This verifies one populated Chat history
state only. Keep `x21h` and parent `13yp` open pending review of other screens,
first-run states, themes/zoom, and accessibility.

### Packaged Search and Changes spot check — 28 October 2026

Built and installed the current `main` checkout (`1b72521`) with
`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run bundle:macos`. The installed
Code Atlas 0.1.0 executable SHA-256 is
`5e084a1978895f46acdfa1c4d2ac3ab92905e99c868a3a1dc5244b5504cef174`; the app
and DMG metadata/checksum verifier passed. No test suite or provider request
was run for this checkpoint.

The packaged app restored Fontdone `main@5249b72371`:

- Search for `memory_face` reported 49 matches across 23 files, with more
  matches available. The old indefinite “Searching” state is no longer shown.
- Changes shows the local `7c48b706482b` → `5249b7237194` comparison and two
  changed files. The README text hunk is inspectable; the oversized TSV is
  labeled, and static relationship coverage is explicitly partial.
- The Explorer/source pane and composer remain present beside the Changes
  content. The Guide, Flow, and history remain in the exploration workspace.

This is a populated-state spot check, not screen-by-screen design acceptance.
The screenshot was inspected live but not saved as a durable artifact. First-run
and error states, nested restoration, other theme/zoom settings, keyboard,
VoiceOver, contrast, and motion remain uninspected. Keep `x21h` and parent `13yp`
open.

### Guide/Flow and keyboard spot check — 28 October 2026

On the same installed build and Fontdone snapshot, the packaged Guide showed
the saved repository guide with six slides ready, the current position at slide
2, and a notice that the next slide builds as the reader reads or listens. One
older Code Lens answer remains visible as an unavailable saved explanation;
another saved visual is explicitly marked as needing refresh. This is saved
content state, not evidence that the current generator failed or that audio was
requested.

The selected Flow answer, “Rebuilding and transforming component points,” had
no verified diagram. Flow stated that it would not show an unevidenced graph
and directed the reader to source links in Guide. This is an honest empty state,
but it leaves substantial blank space for this answer; the source pane showed
`src/tt/gasp.rs` while the selected saved Code Lens answer concerned
`src/tt/glyf.rs`. The user had separately selected `gasp.rs` in Explorer, so
this observation alone does not establish a navigation defect. A native
keyboard-only Tab traversal reached the header controls, Explorer/Search/
Changes/Explorations activity items, repository-sharing action, file filter and
selected row, composer/attachment, source-pane control, and Guide/Flow/history
tab group without sending a question or opening a provider request. Activation,
focus visibility, VoiceOver, zoom/reflow, contrast and motion were not checked.

The native screenshot was viewed at 2880×1688 but is not retained as a file. This
observation adds no durable screenshot evidence and does not satisfy the
screen-by-screen acceptance matrix. Keep `x21h` and `13yp` open.

### Packaged Flow fallback and source return — 28 October 2026

Built commit `50e2551` with `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run bundle:macos`. The frontend build, unsigned macOS app bundle, package metadata verification, and DMG checksum passed. Installed the bundle into the existing Code Atlas app without retaining another backup; the installed executable SHA-256 matches the build: `bfe5202bb49e8a2655560aa93afe5b7b7c99bfd9f47a62368c8e30d7cb52e63d`.

On Fontdone `main@5249b72371`, Flow for “Rebuilding and transforming component points” now shows a section-specific no-verified-diagram state instead of an empty pane. Expanding its details reveals two source-backed claims with buttons for `src/tt/glyf.rs:564–582`. Opening one selects that exact captured range while retaining the Flow view and answer. “Read this section in Guide” opens the matching Code Lens answer at section 1 of 4. The saved Codex connection reconnected; no provider request was sent.

The native screenshot was inspected live at 2880×1688 but was not retained as a file. This qualifies this one populated Flow fallback and navigation path only. First-run/error states, other modes, high zoom, themes, VoiceOver, contrast, and motion remain uninspected; keep `x21h` and `13yp` open.

### Packaged Changes changed-file status — 28 October 2026

Observed the installed Code Atlas 0.1.0 app on Fontdone
`main@5249b72371`. Its executable SHA-256 is
`bfe5202bb49e8a2655560aa93afe5b7b7c99bfd9f47a62368c8e30d7cb52e63d`, matching
the package identity recorded for the build from `50e2551` above. The current
checkout is `64e5f6f`; that commit adds documentation only.

The Changes activity showed the local comparison
`7c48b706482b` → `5249b7237194`, 80 recent commits, and two changed files. The
workspace kept its composer and selected source visible beside the comparison.
Selecting `doc/FILE_RETENTION_INVENTORY.tsv` retained the view and showed the
explicit 524,288-byte evidence limit, zero readable hunks, and a next step to
inspect a smaller revision or another file. The separate comparison banner
said relationship coverage is partial, may omit callers, and does not infer
runtime impact. The `README.md` row was labeled unsupported with one textual
hunk. Returning to the exploration restored Guide; reopening Search restored
the existing `memory_face` query and then showed 79 results across 22 files.
No provider request or saved-content/source mutation occurred.

At the inspected 2880×1688 size, the long TSV path wraps across multiple lines
inside the changed-file card. Truncating long paths while preserving their full
accessible names would make the list easier to scan. This is a concrete visual
deviation to link to `13yp`; it does not by itself establish the full Changes
layout or interaction acceptance.

The screenshot was inspected live but not retained; it displayed the connected
account identifier in the header. Base/head editing, selecting another
commit, empty/error states, and the rest of the screen matrix were not checked.
Keep `x21h` and `13yp` open.

### History search count clarity — 28 October 2026

In the packaged Fontdone workspace, searching `Outline::is_empty` showed
“2 question groups · 0 saved explanations” above two question results whose
cards each offered **Open explanation**. The count mixed question matches with
the separate saved-answer library and made the visible results harder to
interpret. `historySearchResultSummary` now labels only nonzero categories and
uses “matching questions” for grouped question results.

The focused four-case summary regression passes, as does `npm run build`. A
broader run of `tests/workspace.test.mjs` still fails unrelated older
expectations; for example, its progress-copy assertion expects “Building your
explanation…” while the current progressive reader says “Your first section is
taking shape…”. The summary change is source/build qualified only and is not
installed in the native app. Keep `x21h` and `13yp` open for packaged review
and the remaining screen matrix.

### Packaged history count correction — 28 October 2026

Built the unsigned macOS app and DMG from `dbf3bcd` with
`npm run bundle:macos`; notice/license, frontend, optimized Rust build,
embedded metadata and DMG checksum verification all passed. Replaced the
installed app and reopened the existing Fontdone `main@5249b72371` workspace.
After entering the same `Outline::is_empty` search, the reader showed exactly
“2 matching questions”; it no longer showed “0 saved explanations.” The
selected `src/ffi/handles.rs` source pane and saved conversations were still
present. No explanation, answer, narration or provider request was started.
The installed executable SHA-256 is
`1f8e0ccd3c35f35a6c9310886181ea5699346f45ef20772e3f96a532bee510fc`.

This verifies one packaged history-count inconsistency against the current
source. The app is unsigned, and this does not complete other activity, theme,
zoom, keyboard, VoiceOver or accessibility reviews. Keep `x21h` and `13yp`
open.

### Long changed-path source correction — 28 October 2026

The changed-file heading now keeps paths on one line with an ellipsis in a
narrow pane. Each rendered path retains its complete title and accessible
name; renamed files keep separate starting/ending path names. The focused
`gitComparisonController` suite passes 12/12 and `npm run build` passes. This
is source-level evidence only: the installed package above still contains the
wrapping style, and no replacement native bundle was built for this CSS/markup
change. Keep packaged Changes acceptance and the full screen audit open.

### Narrow navigation and sidebar action placement — source audit, 28 October 2026

Compared the current production workspace against
[`WORKSPACE_EXPLORATION.md`](../design/WORKSPACE_EXPLORATION.md). This pass read
the route shell, workspace activity/sidebar markup and responsive CSS. It did
not open the packaged app or measure a viewport; the findings below are
source-confirmed behaviors to reproduce in the screen-by-screen acceptance
run.

| Priority | Concrete gap and user impact | Source evidence | Smallest actionable fix |
| --- | --- | --- | --- |
| P1 | **Narrow sidebar has no visible way back to the current explanation.** Every activity-button click selects `narrowPane='explorer'` (`Workspace.tsx:904`). At widths up to 680 px, the stylesheet shows the repository sidebar and hides the reading workspace (`workspace.css:342–349`). The sidebar header renders **Back to explanation** only when the route is a separate tool (`Workspace.tsx:908`), so ordinary Guide/Flow/history users who open Explorer, Search or Explorations cannot return to the current answer from the visible controls; clicking an activity button selects the sidebar again. | `src/workspace/Workspace.tsx:904,908`; `src/workspace/workspace.css:342–349` | In the narrow sidebar state, show a **Back to explanation** control in the sidebar header. It should set the pane to explanation while preserving the selected activity, search text, and current exploration. At ≤680 px and native compact zoom, verify that each activity can be opened and left without opening a file or relying on a keyboard shortcut. |
| P2 | **Repository-sharing settings occupy the activity sidebar header.** The shield action is rendered beside the changing Explorer/Search/Explorations/Source control heading for every activity (`Workspace.tsx:908`). It puts a project permission control in the code-navigation rail, where the contract reserves the sidebar for repository tools and asks that settings stay out of the reading workspace. This also consumes scarce sidebar-header width in the narrow layout. | `src/workspace/Workspace.tsx:908`; header contents are defined in `docs/design/WORKSPACE_EXPLORATION.md` (“The header contains search, connection status, appearance, and a notification bell”). | Move **Repository sharing** to the project/header menu with the other project-level controls. Keep the sidebar header for the current activity and its relevant navigation actions; confirm the revoke action remains findable without adding a settings pane. |

At the time of this audit, no source changes or package interactions were made. The
responsive-navigation behavior and the placement finding have not been
reproduced in the native app; neither establishes that all routes, themes,
zoom levels, or accessibility states pass. Keep `x21h` and `13yp` open.

### Move repository sharing out of the navigation rail — 28 October 2026

Repository sharing is now opened from the active repository's header menu; the
activity sidebar header keeps only activity navigation and the relevant Back
action. The dialog remains bound to the currently opened immutable snapshot and
still offers the same permission status and revoke/cancel behavior. This is a
source-level correction to the P2 placement gap above. No packaged screen,
keyboard/menu, or revoke interaction was inspected; keep the audit and design
parity tasks open.

### Live workspace review — 29 October 2026

Inspected the open Code Atlas window on Fontdone `main@5249b72371` (partial
snapshot) at 2920×1688. The running bundle's source revision was not verified;
this is a live UI observation, not release evidence. Explorer, Search, Changes,
Explorations, Guide, Flow, Chat history, Connections, and Notifications were
reviewed without opening a saved answer, changing notifications, or sending a
provider request.

| Surface | Observed behavior | Remaining user-facing gap |
| --- | --- | --- |
| Header and section navigation | Repository, snapshot, branch, **Load latest changes**, connection/model, appearance, notification bell, and keyboard-help controls fit at this width. The selected answer's `Section 1 of 3` / **Next section** controls are visible in Guide. | The reported overflow is not present at this viewport; narrow-window and 200% zoom placement remain unverified. Keep the existing responsive acceptance gate. |
| Explorer and Source | The activity rail keeps Explorer, Search, Changes, and Explorations together; the captured tree and source pane remain beside Guide. | Long code lines extend past the pane width. Keep source horizontally navigable and ensure the explanation remains useful without forcing repeated pane resizing. |
| Search | Local results appear beside the persistent composer; the example query `memory_face` returned 47 matches across 22 files and showed a **Local search may be incomplete** disclosure. | The warning and result ordering need to distinguish complete local matches from parser/range coverage limits in plain language. Search's direct next action should remain obvious without another workflow screen. |
| Changes | The pane combines an 80-commit history with a two-file comparison, diff / PR Guide tabs, and explicit unsupported/oversized file statuses. | At this window the layout fits. Confirm narrow widths preserve the changed-file list and diff controls together; do not let internal evidence status dominate the code change. |
| Guide | Guide, Flow, and Chat history are within the exploration. Saved answers expose section navigation, visuals, narration and follow-up actions. | The populated Guide mixes the current answer with incomplete-topic prompts, “0 slides ready,” and “5 in progress.” Move operational progress into Notifications and keep the active section and its visual/narration in the reading focus. |
| Flow | Flow states that links are static and shows source-backed calls/imports/test references for the selected file. | The current file had 25 links and showed up to 12 at once. The first view should lead with behavior-relevant relationships and keep the remainder optional. |
| Chat history | Search is present; repeated prompt attempts are grouped, and saved explanations are separately collapsed. | The live repository contains hundreds of saved explanations and many failed attempts. Keep grouped history searchable and keep diagnostics/attempt detail behind disclosures so it does not crowd Guide. |
| Connections | Explanations and Voice are separate; the connected Codex account/model and Pocket TTS engine are summarized, with advanced details collapsed. | The one-time setup is comparatively direct; retain the single header entry point and keep configuration out of the reading surface. |
| Notifications | The bell opens a scrollable drawer; attention items group repeated attempts and expose open/read/dismiss actions. | The drawer reports 24 attention items and 47 unread updates, while each large card repeats action/status text. Keep outcomes grouped and make the most recent useful action faster to scan. No read/dismiss action was taken. |

The scene/header positioning fix is already in commit `8d76957`; this live
observation confirms fit only at the captured wide viewport. This audit supplies
concrete page findings to the existing `x21h` task; it does not create a second
overlapping issue. First-run, light appearance, narrow/high zoom, keyboard,
VoiceOver, contrast, motion, and a verified packaged source identity remain
open. Keep `x21h` and `13yp` open.

### Guide label and summary cleanup — source change, 29 October 2026

Repeated question groups now say **earlier attempts** instead of **other
answers**, because the group can include failed or stopped questions. The
collapsed repository-guide heading no longer repeats topic/ready-slide counts;
coverage status remains in the heading and per-topic details stay available
when expanded. This removes redundant process status from the reading surface.
`npm run bundle:macos` then completed: notice inventory, TypeScript/Vite
production build, unsigned app/DMG creation, bundle metadata, and DMG checksum
passed. No tests were run. The running app showed five in-progress attempts, so
the new bundle was not installed over that session or runtime-checked; keep the
visual and responsive acceptance gates open.

### File Flow density and ordering — source change, 29 October 2026

File Flow now shows six static links by default, orders direct calls before
imports and test references, and retains an explicit **Show all** control for
the rest. This gives the reader a shorter starting path while preserving access
to every indexed relationship. Source review only; no tests or build were run,
and packaged narrow/wide behavior remains unqualified. Keep `x21h` and `13yp`
open.

### Collapse incomplete Guide detail by default — source change, 29 October 2026

The Guide now keeps the concise **Partial guide** status visible and puts the
long uncovered-topic list inside a closed disclosure. This leaves the current
visual and section controls in the reading focus while preserving the details
on demand. The existing source-contract assertion was updated; no tests or
build were run, and native narrow/wide behavior remains unqualified. Keep
`x21h` and `13yp` open.

### Production frontend build after source refinements — 29 October 2026

At source `e8bfa84`, `npm run build` passed TypeScript checking and the Vite
production build after the File Flow and Guide disclosure refinements. Vite
reported the existing 500 kB chunk-size warning. No tests ran; this did not
rebuild or install the Mac package, and the visual/responsive acceptance remains
open.

### Unsigned Mac package after UI refinements — 29 October 2026

`npm run bundle:macos` completed at repository HEAD `e7d83a7`, including the
license inventory, TypeScript/Vite build, optimized Rust release build, app and
DMG packaging, metadata verification, and DMG checksum. Rust emitted 37
existing warnings; no tests ran. The app executable SHA-256 is
`93bf52eb22e5e0e87e1979cc92f43c030e805243d98438da9f7114020efa0cf5`; the DMG
SHA-256 is `72567f0d1e2bdb473ec46b32b9e038a8e3b421b8e2973790f4683ecce28a610a`.
This package was not installed or opened, so the new visuals and responsive
behavior have no native acceptance yet. Keep `x21h` and `13yp` open.

### Remove no-op Guide history control — source correction, 29 October 2026

The current native accessibility tree reported “Showing 4 of 4 questions” and
offered **Show all 4 questions**, even though all four question cards were
already present. The hidden entries were repeated earlier attempts inside those
cards. The Guide now keeps a concise count and grouping hint but only renders
the expand/collapse button when older question cards are hidden or the full
stack is already expanded. Earlier attempts remain available under their
existing per-question disclosure. No saved content changed. This source-only
correction was not tested, built, packaged, or visually reviewed; keep
`x21h` and `13yp` open.
