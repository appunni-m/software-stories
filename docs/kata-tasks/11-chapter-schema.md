---
kind: task
key: t11
parent: m2
title: Define versioned chapter, section, claim, anchor, and hotspot records
priority: 0
labels: phase-2, validation, chapters
blocked_by: t06
---

## User outcome
Every chapter is structured, editable, linkable, and mechanically checkable instead of being a long untraceable video script.

## Contract
Define a versioned schema for chapter IDs/revisions, file/group/overview target, snapshot, ordered sections, stable section/claim IDs, claim category, prose, source anchors, explicit dependencies, optional typed visual, typed hotspot destination, editable spoken text, and validation status. An anchor stores path, optional stable symbol ID, inclusive range, snapshot/hash, and exact excerpt. Claim categories are code behavior, repository context, or labeled inference. Overview chapters use schema version 3 and stable IDs `chapter:overview` and `target:overview`; versions 1 and 2 remain readable for existing revisions.

## Acceptance criteria
- Every code-behavior claim has at least one exact anchor; an inference records its basis and visible label.
- Every hotspot has a valid typed destination; executable HTML/script output is not accepted.
- Parser line shifts can be remapped only through stable symbol identity plus exact excerpt/hash checks.
- Validation reports structured issue codes for wrong snapshot, missing path, range/hash/excerpt mismatch, invalid claim, and invalid destination.
- Schema versioning can read existing accepted revisions without mutating their payloads.

## Excludes
Proving that prose is semantically true, accepting AI output automatically, or storing a whole guide as one media blob.

## Done evidence
Valid and invalid payload fixtures exercise every field family, anchor mismatch, typed link, schema version, and immutable round-trip behavior.
