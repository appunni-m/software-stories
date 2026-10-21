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
- Switching views and restarting restore the same section without resending a question or creating a revision.
- A missing section or changed revision falls back safely to an existing section. Invalid or oversized persisted position data cannot crash the workspace.
- Restoring position must not automatically restart narration. Keep the existing source hash and citation admission rules.

## Acceptance

- Reproduce the observed Fontdone path against the saved draft in the [acceptance record](../acceptance/2026-10-17-fontdone.md), then verify section two remains selected after each return path.
- Exercise two nested levels, independent sibling positions, Guide/Flow/history switches, app restart, a replaced chapter revision and corrupt local state.
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

## Native navigation checkpoint — 17 October

The unlocked Mac allowed replacement with the updated unsigned debug app. The old installed build first reproduced the reset from section 3 to section 1 after a Flow/Guide switch. In the updated app, Fontdone's saved three-section draft retained section 2 through Flow/history/Guide and Back from its stopped child branch. A sibling retained section 3 independently of the parent's section 2 through root breadcrumb/sidebar navigation. Quitting on the sibling's Flow view and reopening restored that view and section 3; returning to Guide showed 3/3. A citation still opened captured `src/outline.rs` lines 48–52, and Listen remained idle. The saved Codex connection resumed without a setup dialog. [Detailed evidence and limits](../acceptance/2026-10-17-owned-correction-and-navigation.md#packaged-navigation).

**Remaining packaged matrix:** a third exploration level, independently selected available Flow diagrams, exact scroll restoration, replaced/deleted revision/section fallback and malformed persisted state. Existing deterministic regressions cover state migration and fallback; this manual check does not close the complete task. Historical stopped/failed questions and saved drafts were retained, with no new inference during navigation. This debug bundle is not signed/notarized release acceptance.
