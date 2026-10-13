---
kind: task
key: t09
parent: m2
title: Track chapter coverage for every inventory path
priority: 0
labels: phase-2, coverage, chapters
blocked_by: t05, t11
---

## User outcome
The user knows which files have useful explanations and which still need work, with no hidden gaps.

## Contract
Guide Plan shows one row per snapshot inventory path, stable total counts, search/filter/pagination, and a path-specific chapter status: missing, draft, accepted, rejected, stale, excluded with reason, unsupported with reason, or unavailable with reason. Every eligible file defaults to its own chapter. A user may create a named group only by selecting members and entering a rationale; each member remains listed, searchable, and linked. Group membership is visible metadata and never satisfies an individual file's chapter coverage. Grouping never happens automatically.

## Acceptance criteria
- Coverage totals reconcile with inventory totals; capture exclusion does not imply guide completion.
- A file's draft/accepted status is tied to its own path and snapshot/revision.
- Group membership and rationale are visible; each member's explanation remains individually identifiable and citable.
- A group or group-scoped revision never changes an individual file's missing/draft/accepted/rejected/stale state; only a revision whose target is that exact file and snapshot contributes to its coverage.
- No bulk “generate all” action exists in the initial release.
- An exclusion requires a reason and is reversible; unsupported status names the limitation and offers local navigation where possible.

## Excludes
Automatically grouping files, silently excluding paths, and counting the optional repository overview as file coverage.

## Done evidence
On a mixed fixture, coverage rows equal inventory rows; add/remove a user-created group; verify default independent files, reversible exclusions, and draft/accepted status counts.
