---
kind: task
key: t05
parent: m1
title: Show every repository path and its capture disposition
priority: 1
labels: phase-1, repository, coverage
blocked_by: t04
---

## User outcome
The user can tell what Code Atlas inspected and why any file is absent from source browsing, indexing, or a future AI request.

## Contract
For a selected snapshot, inventory every path seen by the scanner, including files excluded or skipped by limits. Each row shows path, kind, byte size when known, capture/index eligibility, and a stable reason. Keep capture disposition separate from guide coverage and Gemini selection. Add path search and filters for language, parser readiness, config/test/generated status, and capture outcome.

## Acceptance criteria
- The total count equals captured plus skipped/excluded inventory entries; pagination retains stable total and visible range.
- Binary, oversized, invalid-encoding, secret-like, ignored, generated, symlink, and parser-unsupported states are distinguishable where applicable.
- Excluded/skipped paths remain searchable and visible; no path is silently omitted.
- Opening an eligible file uses snapshot source retrieval and never falls back to the working tree.
- Inventory filtering does not mutate capture state or imply that a path has a chapter.

## Excludes
Changing ignore rules, automatically generating chapters, or inferring a file's responsibility from its name.

## Done evidence
Mixed fixture shows all path categories, stable pagination and filters, correct path totals, and explicit reasons for every non-captured or unsupported path.
