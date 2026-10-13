---
kind: task
key: t26
parent: m6
title: Build bounded read-only evidence for local Git base and head refs
priority: 1
labels: phase-6, pull-request, repository
blocked_by: t04, t06, t25
---

## User outcome
The user can compare two local Git revisions and inspect exactly which source lines and symbols changed, without changing their checkout.

## Contract
Accept explicit local base/head refs or full commit IDs, resolve them to immutable object IDs, and compute a bounded tree/diff. Classify added/deleted/modified files and changed hunks. Attach parser-backed symbols/relationships only when resolved at the corresponding revision. Preserve line context and skipped reasons. This is separate from working-tree snapshots and does not checkout or modify refs.

## Acceptance criteria
- Ambiguous, missing, and invalid refs are rejected with a choose-again state.
- Diff output is deterministic for resolved object IDs and distinguishes base/head source anchors.
- Rename, binary, oversized, unsupported, and parser-uncertain changes are visibly classified.
- No fetch, push, checkout, commit, PR API call, or GitHub authentication occurs.
- Existing repository chapters remain unchanged when PR evidence is created.

## Excludes
Remote GitHub PR integration, comments/reviews, merge operations, and inferred runtime impact.

## Done evidence
Compare temporary base/head commits containing add/edit/delete/rename/binary and parser-resolvable changes; confirm anchors, uncertainty, deterministic results, and unchanged refs/worktree.
