---
kind: task
key: design-parity
parent: contract
title: Make the packaged app follow the approved product designs
priority: 0
labels: product, design, ux, acceptance
---

> **Current design authority — 16 October:** the [persistent code/chat/exploration workspace](../design/WORKSPACE_EXPLORATION.md) supersedes the earlier Guide / Files / Changes navigation. Its VS Code-style activity bar contains Explorer, Search, Changes, and Explorations. Chat remains pinned above content; Guide, Flow, and Chat history are views within one exploration; code details stay inline. The workspace preview is illustrative, not packaged-app acceptance. Dated iteration notes below are historical evidence unless a later packaged recheck is explicitly recorded.

## Native workspace refinement — 2026-10-16

The user clarified that understanding code comes first, with navigation resembling VS Code, a dynamic player, an understandable code path, and no repeated backend selection. This supersedes any interpretation that matching Guide / Files / Changes labels alone establishes design parity.

Implemented in this iteration:
- Removed the permanent Project context sidebar and large Files hero from the populated workspace. The explorer, actual source and explanation tool pane use the window height with independent scrolling.
- Added explicit Explanation / Flow / Player controls. Flow is focused on verified calls/imports/test references for the selected file, with clickable source/evidence nodes; it does not claim to be a runtime trace.
- Reduced normal-state source metadata and duplicate explanation actions. Captured paths are the default; skipped/excluded paths remain accessible on demand. Folder rows are no longer misleadingly marked excluded.
- Codex is the default generation entry; an explicit backend choice is remembered across file, planning and PR entry points. Existing Codex model/reasoning preferences remain in use. A one-file Codex action prepares local source review automatically without sending or reusing consent.
- Kept Previous/Continue in a separate, always-visible player row. Removed generic side cards and empty “no diagram” placeholders; evidence quotes and audio controls remain available on demand.
- Kept the learning-plan manager behind its relevant Guide disclosure.

See [native observations, screenshots and remaining deviations](../acceptance/2026-10-16-code-workspace.md). Full Changes composition, first-run polish, generated visual quality, question-to-source discovery, and accessibility acceptance are not established by this iteration. The question-to-source work is separately defined in Kata `xvjk` and `55-question-to-source-discovery.md`.

## Dense Flow canvas correction — 2026-10-21

Large disconnected or cyclic Flow diagrams now use a compact grid instead of a single horizontal fallback row. Long ranked paths wrap into alternating bands. The Flow canvas uses more of the available window height, and a layout-version field drops old saved zoom positions so the new arrangement is fit on first open. Small graphs keep their existing positions. Linear paths continue to use the source-linked step reader.

The focused Flow/persistence checks pass **55/55**, and the production frontend build passes. The unsigned Mac app and DMG built successfully; DMG checksum and bundle metadata verification passed. In the installed Code Atlas 0.1.0 app (arm64, executable SHA-256 `f82913d8584744290627e0afaa5a2571af8da5162f3f9d8950abac2f1f85ea9d`), Fontdone `main@5249b72371` reopened with its saved Flow selection. All nine nodes were visible in a 3×3 layout, and selecting the first node retained Flow and targeted `src/tt/mod.rs:94`.

The screenshot was inspected live but is not saved as a durable file. This verifies one dark/system Flow state only. Other workspace states, themes, accessibility, large-repository behavior, and the full screen-by-screen acceptance remain open; keep `13yp` open.

## Preserve source lines in the workspace editor — 2026-10-23

The installed Fontdone workspace displayed long Rust statements as wrapped fragments in the source pane, weakening the editor-like reading experience and hiding original line shape. The source editor now leaves CodeMirror soft wrapping disabled so long code lines can be inspected with the editor's horizontal scroll. The focused editor contract check passes **2/2**, and `npm run build` passes with the existing large-bundle warning. This is a source-level correction only: the installed bundle still shows the prior wrapping behavior, and the full packaged/zoom/keyboard matrix remains open. Keep `13yp` open.


## User outcome

A new developer can open Code Atlas and immediately understand where to start, what a file does, and what changed between two revisions. The shipped Mac app follows the approved screens and interactions, rather than only containing matching workspace labels in source code.

## Initial evidence and current source status — 2026-10-15

The task began from a real mismatch between the proposal and the installed Mac app. The inspected bundle (Code Atlas 0.1.0) showed Atlas, Code Lens, and Walkthrough in its primary bar, with Guide Plan and Chapter Studio outside that navigation. On fontdone it showed 2,768 captured files and 25 skipped paths in the persistent project panel. The inspected Code Lens view for `tests/support/generated_constant_lookup.rs` had repeated TestReference cards that crowded the source explanation. That was a layout and information-architecture problem, not just spacing.

The checkout already labeled the primary bar Guide, Files, and Changes, but its behavior still diverged from the proposals: Guide opened directly on the dense coverage manager, and selecting a file navigated to the separate Code Lens route. Source-only changes are now in progress:

- The current product authority is `docs/design/WORKSPACE_EXPLORATION.md`: Explorer, Search, Changes, and Explorations on the activity bar; Guide, Flow, and Chat history inside each exploration. The earlier Guide / Files / Changes proposal remains a historical capability description, not navigation authority.
- Guide now has a repository home that reads an accepted overview for the current snapshot. If none exists, it offers to create an optional tour through the exact-context review flow and keeps a per-file action visible beside it; a guide from another snapshot remains separately available. Coverage remains under **File-by-file coverage**.
- Files source links now target the existing Files route. The tree, selected snapshot source, exact citations, and selected file's current accepted explanation are composed together; relationship exploration remains optional.
- The source route keeps navigation history so citation and related-file links can return to the previous file or Guide reader context.
- Changes has not yet been compared against the approved design in the packaged app.

These edits have not been built, tested, launched, or inspected in the native app. The installed 0.1.0 bundle has not been replaced or confirmed to match the checkout. Source-level changes and updated docs are progress evidence, not packaged-app acceptance.

## Source implementation follow-up — 2026-10-15

The no-current-overview state now offers **Create repository tour**, which opens the existing reviewed overview request flow. The user selects the captured files and reviews the exact context before any request is sent. The same card keeps a direct action for the next file explanation, and its adjacent coverage card continues to track file chapters independently. When a current-snapshot tour draft exists, Guide offers **Review tour draft**; an accepted guide from an older snapshot remains explicitly reachable without being presented as current. If the snapshot has no captured text files, the tour action is omitted and file browsing remains available.

This is source-only progress in `src/App.tsx` and `src/App.css`. No test, build, app launch, or packaged screenshot was run. Native hierarchy, keyboard/focus behavior, and provider-review usability remain unverified; `13yp` and `x21h` stay open.

## Concrete design-to-app mismatch record

This is the dedicated design-parity task. The table records the specific mismatch that motivated it and separates direct observation from work that still needs packaged-app evidence.

| Approved reference | What the app was doing or showing | What the design requires | Evidence status |
| --- | --- | --- | --- |
| Workspace shell | The 25 October packaged build restores the persistent Explorer, captured source, pinned composer, and Guide / Flow / Chat history. | Keep the four activity-bar destinations, with connection and notifications in the compact header; keep source and explanation in the same workspace. | One dark/System Fontdone state was inspected after reinstall; no durable screenshot, first-run review, or full navigation matrix. |
| Guide and Flow views | Saved Code Lens answers and source selection restored after app restart. This check did not create a new answer. | Each exploration restores its last view; Guide presents headed answer cards and paced sections, Flow shows evidence-supported relationships, and source cues return to the same answer position. | Workspace structure is visible; player progression, Flow, citation navigation, and Back restoration still need packaged checks. |
| Explorer and Search | The selected `src/tt/glyf.rs:564–583` source is displayed beside saved answer cards; no new explanation was dispatched. | Explorer keeps captured source beside the current exploration. Search runs locally and offers **Explore with AI** only when the user chooses it. | Reopen state is confirmed; exact file/citation hops, no-result search, and return focus need checks. |
| Changes | The packaged app displays 80 recent commits and a local comparison from `7c48b706482b` to `5249b7237194`, with two changed files. README's textual hunk remains inspectable with an unsupported-grammar notice. | Keep commit history, explicit local base/head refs, changed files, focused diff, and grounded evidence together without repository writes. | One populated dark/System state was inspected inline. Base/head editing, error/empty cases, and durable screenshot remain unverified. |

The remaining mismatch must be judged against the current activity-bar workspace, not the historical Guide / Files / Changes proposal. The packaged app now visibly restores the intended Explorer/chat/Guide structure and one Changes comparison. The unreviewed states and return behaviors below remain acceptance gaps; source edits and inline screenshots do not establish full packaged parity.

## Required work

1. Keep one design source of truth across the product blueprint, greenfield contract, interaction contract, workspace design, source implementation, and packaged app. Mark the older Guide / Files / Changes proposals as historical navigation, while retaining still-useful behavior requirements.
2. Make each exploration reopen its saved Guide, Flow, or Chat history view. The pinned composer remains available above all three views; a Guide request produces headed, paced answer cards, and file coverage stays contextual rather than becoming a separate primary destination.
3. Keep Explorer, captured source, and the active Guide/Flow answer together. Selecting a file, opening a citation, or following a source cue must preserve the current exploration and restore its prior answer, selection, and reading position on Back. Search remains a separate activity with local results and an explicit **Explore with AI** action.
4. Make Changes work as the VS Code-style activity: local commit history, explicit base/head selection, changed-file list, focused diff, plain-language explanation, and evidence links. Keep the local-only Git contract intact.
5. Apply the design’s content hierarchy to real repositories, including large or generated files. Repeated low-value static references must be collapsed, grouped, or clearly secondary; they must not crowd out the explanation of what the file actually does. Partial capture must present an actionable next step. Put hashes, counts, and advanced controls behind the relevant details affordance where the reference calls for it.
6. Align the packaged Mac app with the source version being reviewed. Keep the native bundle’s navigation, routes, labels, and behavior in sync with the current checkout. Do not count a successful web preview, component screenshot, or source-level label check as native acceptance.
7. Keep the visual language coherent with the selected reference set. Use the actual project, snapshot, and source content; do not substitute static mock data for core screen content.

## Acceptance criteria

- One current design source of truth describes Explorer, Search, Changes, and Explorations on the activity bar; pinned chat; and Guide / Flow / Chat history within each exploration. The blueprint, contract, source, and packaged app agree on this hierarchy.
- In the packaged Mac app, each of the four activity-bar destinations opens the intended surface. The main journey works without a detached Code Lens, Guide Plan, or Chapter Studio destination.
- Each exploration restores its last view. Guide cards advance one section at a time, expose exact source cues, return to the same section after a source hop, and keep file coverage and next actions contextual. Flow supports evidence-backed investigation from the current question.
- Explorer displays one captured file beside its Guide/Flow answer and exact evidence. Citation and related-file links open exact local snapshot ranges and return without losing exploration, selection, or scroll context.
- Changes displays a real local comparison with selected base and head, focused diff, explanation, and exact source evidence; empty, loading, and error states explain the next action.
- On the fontdone fixture and a second non-trivial repository, generated/repetitive relationship rows do not swamp the primary explanation. Partial-snapshot status leads directly to skipped-path review.
- Capture packaged screenshots for Explorer with selected source, one Guide answer, one Flow answer, Search results, Changes with a selected diff, and the first-run/empty state. Review each against the current workspace preview at the same window size, record pass/fail and deviations, and include the exact app build identity.
- Verify all activity destinations, section Previous/Next, source/citation links, and Back/root state restoration in the packaged app. Record accessibility issues under the dedicated accessibility task rather than silently treating this task as an accessibility sign-off.

## Exclusions

Do not add new model providers, generation behavior, PR hosting, or unrelated parser/index features here. Do not remove evidence, safe local Git boundaries, revision history, or accessibility work. This task does not close the separate accessibility, packaging, or chapter-quality acceptance gates.

## Done evidence

A single reconciled reference set, updated product/design contracts, the implementation commit, and a native-app acceptance record with the required screenshots, build identity, interaction results, and remaining deviations.

## Edge-free Flow presentation — 23 October 2026

The packaged Fontdone Flow for “The Rust example classifies the constructor result” contained four source-backed nodes and zero edges. The canvas placed them as isolated graph nodes, which looked broken and called them “steps” even though no order or transition was present. Source now renders an edge-free visual as an unordered set of clickable source-point cards with exact source locations, and states that the evidence does not define a path. Real graphs remain interactive; the graph accessibility label now calls their items nodes.

The focused Flow suite passes **6/6**, and `npm run build` passes with the existing large-bundle warning. The app package was not rebuilt because available disk space was 962 MiB, below the previously established 5 GB safe-build threshold after a prior truncated package. This is source/build evidence only; `13yp` still needs a verified native package and screenshot plus its broader screen, theme, and accessibility matrix.
