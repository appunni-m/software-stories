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
1. Files: the compact-heading selector applies .has-snapshot and .view-atlas to the same element even though they are ancestor/descendant. The huge hero is therefore still visible. Hide the contextual heading visually while preserving the focus/landmark target.
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


Installed observation: existing origin-v1 answers/diagram now load; clicking Validation selects captured lines 2–4. A new explicit empty-list retry saved an origin-v2 file chapter and new diagram; subtotal node selected captured line 12. Shared Connections and direct local Listen were inspected in the actual app. Two sections and a second answer used the same global voice. Retain remaining UI gaps: graph accessibility currently exposes library edge-selection/delete instructions despite a reading surface; some diagram nodes lack button semantics; the hidden legacy landmark copy is outdated; prose backticks need a safe inline-code renderer; branch “answers” counts include failed questions. These need explicit keyboard/accessibility and content-rendering work without changing saved evidence.

## Fontdone findings — October 17

The [large-repository acceptance run](../acceptance/2026-10-17-fontdone.md) reproduced remaining user-facing inconsistencies:

- [ ] Returning from a child, switching Guide/history or restarting restores the parent answer at section one. Preserve exact reader position under separate [task 68](68-exploration-reading-position.md), Kata `erzc`.
- [ ] History labels stopped questions with no saved revision “Open the saved explanation.” Show the actual stopped/interrupted/failed state and an appropriate navigation/retry label; only a matching revision may be called saved.
- [ ] A stopped child with no generated result is counted as “1 answer.” Distinguish questions/explorations from completed answers without hiding their preserved history.
- [ ] Headings and prose display literal Markdown backticks. Render supported inline code safely; preserve stored prose and citation identity and do not execute model HTML.
- [ ] A shared search term carries from Search into Explorer, leaving an empty filtered tree for a symbol name. Make the filter's current scope obvious or retain separate search/filter values.

The snapshot, source range, question tree, selected view and saved draft did survive restart; Pocket playback also reused its recording. These passes do not close the unresolved UI, reader-position, accessibility or deeper-branch gates.
