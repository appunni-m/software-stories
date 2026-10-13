---
kind: task
key: t08
parent: m1
title: Inspect exact snapshot source and citations in Code Lens
priority: 1
labels: phase-1, code-lens, repository
blocked_by: t05, t06
---

## User outcome
The user can verify an explanation against the exact code that was captured for that guide.

## Contract
Render read-only source with stable line numbers, syntax highlighting where supported, and a highlighted citation range. Show the snapshot ID/hash and explicit states for text, binary, oversized, uncaptured, invalid encoding, or missing anchor. Provide related callers/imports/tests only when indexed. Back restores originating chapter, section, and playback position.

## Acceptance criteria
- Rust validates path, snapshot, hash, and size before returning bytes; the UI cannot read arbitrary filesystem paths.
- A citation hash mismatch is shown as stale/unavailable and never redirected to live source or a nearby range.
- Unsupported syntax remains readable as plain text when it is safely captured.
- Links to callers/imports/tests retain relationship type and confidence.
- Open-from-Atlas and open-from-Walkthrough both return to their exact prior selections.

## Excludes
Editing source, writing to the repository, and accepting an explanation based only on line numbers.

## Done evidence
Navigate from both Atlas and a chapter citation; verify correct highlighted bytes, mismatch handling, read-only behavior, unsupported text view, and position-preserving return.
