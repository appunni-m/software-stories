---
kind: task
key: t21
parent: m4
title: Compare immutable guide revisions and restore as a new draft
priority: 1
labels: phase-4, history, revisions
blocked_by: t20
---

## User outcome
The user can understand what changed in an explanation and recover an earlier version without erasing history.

## Contract
Chapter Studio lists revision time, source snapshot, status, and parent revision. Compare explanation prose, sections, claims, anchors, visuals, spoken text, dependencies, and assets. Restore selected prior content as a new draft and revalidate its anchors against the target snapshot.

## Acceptance criteria
- Accepted chapter and section revisions cannot be edited in place.
- Diff identifies changed/added/removed fields and source references.
- Restore creates a distinct draft while retaining the selected old version and current accepted version.
- Invalid restored anchors block acceptance and show exact repair options.
- Rejected draft history remains visible and cannot be accepted without a new revision.

## Excludes
Mutating Git history, deleting accepted revisions during restore, and silently pinning old line numbers to new source.

## Done evidence
Accept two distinct revisions, compare every changed field family, restore the older version, and verify all three records and their asset references remain separate.
