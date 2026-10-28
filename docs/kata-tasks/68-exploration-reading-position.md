---
kind: task
key: exploration-reading-position
parent: workspace-ui-audit
title: Restore the exact answer section when returning from an exploration
priority: 1
labels: navigation, workspace, ux
---

## User outcome

The reader can follow a detail and return to the explanation they were reading without finding the section again.

## Observed failure

In Fontdone's saved three-section Outline draft, navigate to section two, “When `is_empty` returns true.” Choose Explore further, stop the child request, then Back. The parent returns to section one. Switching through Chat history or restarting also recreates the section state at one. The selected source range and the exploration tree survive; answer section position does not. `ChapterAnswer` owns its position only in component-local state.

## Required behavior

- Persist reader position per exploration, answer turn and exact saved revision, using stable section identity.
- Returning through Back, root, a branch link or the sidebar restores the parent section and reading position. Keep each exploration's last selected Guide/Flow/history view.
- Returning to Guide also restores the selected source-backed visual node or edge within that section; do not restore a stale selection missing from the current revision.
- Switching views and restarting restore the same section without resending a question or creating a revision.
- A missing section or changed revision falls back safely to an existing section. Invalid or oversized persisted position data cannot crash the workspace.
- Restoring position must not automatically restart narration. Keep the existing source hash and citation admission rules.

## Acceptance

- Reproduce the observed Fontdone path against the saved draft in the [acceptance record](../acceptance/2026-10-17-fontdone.md), then verify section two remains selected after each return path.
- Exercise two nested levels, independent sibling positions, Guide/Flow/history switches, app restart, a replaced chapter revision and corrupt local state.
- Select a node and an edge, switch to another workspace activity, then return to Guide; verify the focused visual cue and its linked source are restored. Repeat after reopening the app.
- Confirm the source range, stopped child outcome and original saved revision remain intact and no new model request occurs during navigation.
- Include deterministic state serialization/restoration regressions and packaged app evidence before closing this task.

## Implementation checkpoint — 17 October 2026

The production workspace now records stable section identity for each answer turn and exact revision. Guide selection and Flow diagram selection are independent. Scroll position is stored per exploration and Guide/Flow/history view, with migration from the previous single scroll value. Navigation restores after saved chapters load and ignores layout-generated scroll events during restoration. A delayed event targets its captured exploration rather than whichever exploration is currently active. Section changes stop narration; reopening a view starts with narration off.

Five new deterministic regressions cover two nested levels and independent siblings, JSON round-trip, last selected views, stopped child/source/revision preservation, reordered/deleted sections, changed revisions, stale updates, legacy scroll migration and invalid/oversized position values. The full frontend suite passes **141 tests** and the production frontend build passes (existing bundle-size warning remains).

**Still open:** build and install the same app, then exercise the saved Fontdone three-section draft through Back, root, branch links, sidebar, view switches and restart. No packaged-app pass or production readiness is claimed by the state regressions.

## Current state regression recheck — 2026-10-21

Ran the focused workspace state suites on current source:

`node --experimental-strip-types --test tests/workspace.test.mjs tests/workspace-outcomes.test.mjs tests/answer-versions.test.mjs tests/repository-guide.test.mjs`

Result: **67 passed**. Coverage includes exact exploration/turn/revision/view ownership, stable section IDs after reorder, safe fallback for changed or deleted sections, stale-event isolation, corrupt/oversized position rejection, History reopening without changing reading state, revision switching, nested branch state, and preserving reader position through guide follow-ups.

This recheck does not extend the earlier packaged observation. Third-level navigation, independent Flow selections, exact scroll restoration, replaced/deleted revision behavior and malformed stored state still need the complete packaged matrix; keep the task open.

## Packaged parent-section return — 2026-10-21

On the running Mac app with Fontdone `main@5249b72371`, the root exploration
was on a one-section Guide answer with the source pane at
`src/tt/mod.rs:149`. I opened an existing Level 2 child from the exploration
list. It retained its stopped/saved state and selected source at
`src/outline.rs:48–52`. Back returned to the root with the same parent answer
section selected (`1 / 1`) and restored `src/tt/mod.rs:149`; narration remained
paused at its existing end position and did not autoplay. I did not retry or
submit a question, and navigation initiated no model or narration request.

[Manual evidence and limits](../acceptance/2026-10-21-navigation-position-followup.md).
This covers one parent/child/Back path. It does not exercise a multi-section
parent, two nested levels, sibling-specific positions, independent Flow
selection/viewport, restart, revision fallback, corrupt state or accessibility.
Keep `erzc` open.

## Native navigation checkpoint — 17 October

The unlocked Mac allowed replacement with the updated unsigned debug app. The old installed build first reproduced the reset from section 3 to section 1 after a Flow/Guide switch. In the updated app, Fontdone's saved three-section draft retained section 2 through Flow/history/Guide and Back from its stopped child branch. A sibling retained section 3 independently of the parent's section 2 through root breadcrumb/sidebar navigation. Quitting on the sibling's Flow view and reopening restored that view and section 3; returning to Guide showed 3/3. A citation still opened captured `src/outline.rs` lines 48–52, and Listen remained idle. The saved Codex connection resumed without a setup dialog. [Detailed evidence and limits](../acceptance/2026-10-17-owned-correction-and-navigation.md#packaged-navigation).

**Remaining packaged matrix:** a third exploration level, independently selected available Flow diagrams, exact scroll restoration, replaced/deleted revision/section fallback and malformed persisted state. Existing deterministic regressions cover state migration and fallback; this manual check does not close the complete task. Historical stopped/failed questions and saved drafts were retained, with no new inference during navigation. This debug bundle is not signed/notarized release acceptance.

## Persist Guide visual focus — 21 October 2026

Guide reader progress now stores the selected node and edge alongside the existing exact section, scene, narration position and viewport metadata. Values are bounded and section-scoped; when a saved edge is missing from the current visual, the player falls back to a still-valid source node or no focused cue. Selection does not resume playback. The selected node/edge now flows through the existing workspace persistence and decode path.

**Source verification:** `node --experimental-strip-types --test tests/visualStory.test.mjs tests/reader-progress.test.mjs` passes **56/56**; `npm run build` and `git diff --check` pass. The state regression covers workspace serialization and restoration of the focused node and edge, and malformed optional focus metadata does not discard saved narration position.

**Still open:** packaged-app recheck of Guide → another activity → Guide and after restart, plus the broader navigation, viewport, revision-fallback, corrupt-state and accessibility matrix above. This source/build check does not close `erzc`.

## Native malformed reader-position recovery — 23 October 2026

Native conversation loading now drops malformed optional scroll and section-bookmark entries in memory while retaining valid entries. It applies to Guide/Flow bookmarks, per-view scroll offsets and per-answer-version bookmarks. It does not alter saved answer content or durable bytes; reopening the conversation does not rewrite the journal. Required conversation fields and ownership checks remain strict.

**Source verification:** `cargo test --manifest-path src-tauri/Cargo.toml --lib workspace_conversation::tests` passes **21/21**. The new regression injects malformed Guide/Flow/history bookmarks and scroll offsets into an otherwise valid saved record, then verifies that valid reading positions and the answer survive while the on-disk bytes remain unchanged.

The combined locked native library suite then passes **720 tests, 0 failed, 18 explicitly ignored**.

**Still open:** the complete packaged navigation and accessibility matrix above, including malformed state through the installed app. This focused native regression does not close `erzc`.

## Packaged saved-answer position spot check — 23 October 2026

On the running Code Atlas 0.1.0 app with Fontdone `main@5249b72371`, opened a
saved four-section answer and selected section two, “The probes look for
different kinds of signatures.” Guide → Flow → Guide retained the section.
Opening a saved child exploration and returning with Back also restored the
parent with section two visible. A second saved child was likewise opened and
returned from. These actions reused saved content; no model request or narration
was started. The observation was captured from the app accessibility tree.

[Procedure and limits](../acceptance/2026-10-23-navigation-position.md). This
same-process spot check does not establish restart, exact scroll, third-level
navigation, independent Flow viewport, revision fallback, corrupt state through
the app or accessibility. The installed bundle's source revision was not
verified. Keep `erzc` open.

## Exact viewport restoration and corrupt-position recovery — 28 October 2026

Source inspection found that Guide restoration always called `scrollIntoView`
for the focused answer, even when the exploration had a saved Guide offset. That
overrode the per-view scroll value on Back and view switches. Restoration now
uses a saved offset (including zero) when present; it scrolls to an answer only
for an explicit one-shot reveal or an uninitialized Guide. Selecting another
slide inside the already-open progressive answer updates its section without
creating another answer reveal.

Workspace decoding now sanitizes optional reader and view metadata independently.
Malformed per-view scroll, answer positions, answer versions, visual progress,
file-flow selection, drafts, and invalid optional file selections are discarded
without rejecting the conversation tree or other valid positions. Valid saved
offsets and answer revisions remain scoped to their original exploration.

The focused reader-progress checks cover saved Guide offsets (including zero),
explicit reveals, answer focus changes and corrupt optional state. All 15 cases
passed across the final focused runs; the frontend production build and
`git diff --check` passed. These are source-level checks. Packaged exact-scroll,
third-level, independent Flow viewport, restart, revision fallback, malformed
state and accessibility acceptance remain outstanding; keep `erzc` open.

## Packaged exact-answer return check — 28 October 2026

In the running Code Atlas window on Fontdone `main@5249b72371`, the parent was
in Chat history with the saved answer “Explain what glyf.rs:564–583 does and
how it works.” marked current and `src/tt/glyf.rs` open. I opened the existing
child **Explore remaining topics**; its Guide opened with `src/api.rs`. Back
returned to the parent with Chat history still selected, the same saved answer
still marked current, and `src/tt/glyf.rs` restored. No question or provider
request was started, and narration did not play. Navigation state moved to the
child and returned; no question, answer, receipt, or source snapshot was
edited.

This confirms one same-process child/Back restoration path for the exact
answer, tab, and source file. The running executable revision and a durable
screenshot were not verified. It does not exercise a multi-section answer,
exact scroll offset, sibling/nested-level independence, restart, revision
fallback, corrupt state or accessibility. Keep `erzc` open for the remaining
acceptance matrix.

## Additional packaged Guide/Flow/child position check — 28 October 2026

In the running Code Atlas 0.1.0 app on Fontdone `main@5249b72371`, a saved
two-slide Guide was advanced to slide 2 of 2, switched Guide → Flow → Guide,
and remained on slide 2. Opening the saved level-2 child “Explain When
is_empty returns true in more detail” and returning with Back restored the
parent's slide 2 and previously selected `src/outline.rs`; the child had
opened `src/font.rs`. No question, provider request, answer, or narration was
started. The installed executable's source revision was not verified.

This covers one same-process section/tab and child/Back path. Third-level and
sibling independence, exact scroll, restart, revision fallback, malformed
state in the packaged app, visual-node/edge restoration and accessibility
remain open. See the combined [navigation acceptance record](../acceptance/2026-10-23-navigation-position.md).

## Explorer activity preserves saved answer position — 28 October 2026

In the running Code Atlas window on Fontdone `main@5249b72371`, the root
Explorations view showed the saved four-section answer “Explain what
glyf.rs:564–583 does and how it works.” at section 4 of 4. I moved to section
3, switched to the Explorer activity, and returned to Explorations. The answer
remained at section 3 of 4. I then restored the original section 4 position.
No question, provider request, or narration was started. The app's executable
revision was not verified.

This verifies one same-process answer-position restoration across the Explorer
activity and restores the user's previous reading position. It does not verify
restart, exact scroll, a return from a child, third-level/sibling state,
revision fallback, malformed state in the packaged app, or accessibility;
keep `erzc` open.

## Saved child return to its parent Guide — 28 October 2026

In the running packaged Code Atlas app on Fontdone `main@5249b72371`, opened
the existing **Explain When `is_empty` returns true in more detail** branch
from its root Guide card. The saved child opened at Level 2 with its Guide
answer visible at section 4 of 4 and `src/font.rs` selected. Back returned to
the root with Guide still selected, the original one-section
`Outline::is_empty` card and its saved child link visible, and
`src/ffi/handles.rs` restored. This reused saved work; no question, provider
request, answer, or narration was started.

The app remained open. Its executable revision and a durable screenshot were
not verified. Because the root answer has one section, this does not verify
multi-section parent-position restoration; third-level/sibling independence,
exact scroll, restart, revision fallback, malformed state in the packaged app,
visual-node/edge restoration, and accessibility remain open.

## Cross-activity and repository-switch position check — 28 October 2026

In the running native app, Fontdone `main@5249b72371` was on the saved
six-slide repository Guide **How does this project work, from its entry points
through its main paths?**, slide 2 of 6, with `src/tt/glyf.rs` open. I switched
among Search, Explorations and Explorer, then opened `pillow-rs` and returned to
Fontdone. The Guide reopened at slide 2 of 6 with `glyf.rs` still selected. No
question, provider request, answer edit, or narration was started. I then
restored the original `pillow-rs` workspace, file and Guide view.

The running executable revision was not verified. This adds a same-process
cross-activity and repository-switch observation; it does not cover app
restart, exact scroll offset, third-level/sibling positions, revision fallback,
malformed persisted state, independent Flow viewport, source-linked visual
focus, or accessibility. Keep `erzc` open.

## Stale Flow layout no longer hides saved conversations — 28 October 2026

Native inspection of the rebuilt app opened Fontdone `main@5249b72371` but
reported **Saved conversation unavailable**. The source cause was a Flow
viewport-layout version bump: the loader rejected the whole conversation
before its existing recovery sanitizer could discard obsolete reader
positions. The saved conversation's `layoutVersion: 4` predates current
version 5. Reader layout state is optional UI metadata, so load now discards
incompatible Guide/Flow positions in memory and validates the remaining
conversation. It does not rewrite the original record; normal saves still
require the current layout version.

The focused native conversation suite passes **25/25**, including a v4
regression that preserves the question/status, removes stale presentation
positions, and byte-compares the durable record before and after loading. The
old packaged app remains open; rebuild and reopen it to confirm Fontdone's
saved answers become available in the app. No provider request, snapshot
refresh, or saved-content edit was made. Keep `erzc` open pending packaged
confirmation and the remaining navigation matrix.
