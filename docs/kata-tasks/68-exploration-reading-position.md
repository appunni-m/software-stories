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
