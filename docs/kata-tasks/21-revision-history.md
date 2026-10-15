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
- The comparison is field-level and content-safe: source paths, ranges, and hashes may be summarized, but raw source excerpts are not copied into diff output.
- Restore creates a distinct draft while retaining the selected old version and current accepted version.
- Invalid restored anchors block acceptance and show exact repair options.
- Rejected draft history remains visible and cannot be accepted without a new revision.

## Excludes
Mutating Git history, deleting accepted revisions during restore, and silently pinning old line numbers to new source.

## Done evidence
Accept two distinct revisions, compare every changed field family, restore the older version, and verify all three records and their asset references remain separate.

## Pending work — 2026-10-15

- [ ] Keep deleted/excluded/non-text file chapter histories reachable; static fix applied, acceptance pending.
- [ ] Accept two distinct revisions and inspect every comparison field; restore an older revision as a new current-snapshot draft.
- [ ] Verify previous accepted/history/assets survive, invalid restored anchors block acceptance, and rejected content needs a new draft.
- [ ] Exercise edit/save/validate/navigation races against the patched UI; do not count source inspection as acceptance.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.
