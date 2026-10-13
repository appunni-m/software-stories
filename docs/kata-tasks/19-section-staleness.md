---
kind: task
key: t19
parent: m4
title: Detect changed evidence and mark only dependent sections stale
priority: 0
labels: phase-4, revisions, repository
blocked_by: t04, t06, t11, t13
---

## User outcome
After a code change, the user sees exactly which accepted explanations may no longer match and why.

## Contract
Compare accepted section anchors and relationship dependencies against a new immutable snapshot. Report each section as Current, Safe to carry forward, or Stale with the reason and source diff. Use stable symbol identity plus exact excerpt/hash to propose unambiguous line remaps. Track reverse dependencies from source anchors/relationships to sections and derived assets.

## Acceptance criteria
- A line-only shift with identical symbol/excerpt can be Safe to carry forward with old/new ranges shown.
- Changed/deleted cited code, changed resolved relationship, changed test evidence, and missing source produce explicit stale reasons.
- Unreferenced sections and unrelated chapter revisions remain Current.
- Freshness computation does not mutate an accepted chapter, snapshot, or asset.
- The app distinguishes source change from incomplete or unsupported indexing evidence.

## Excludes
Automatic regeneration, automatic acceptance, nearest-line guessing, and treating every file change as a repository-wide stale event.

## Done evidence
Use a multi-file fixture with one line shift, one cited edit, one caller/test relationship change, and one unrelated edit; verify only exact dependents become stale and prior payloads are byte-identical.
