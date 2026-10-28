---
kind: task
key: t13
parent: m2
title: Edit, validate, accept, or reject chapters in Chapter Studio
priority: 0
labels: phase-2, chapters, revisions
blocked_by: t12
---

## User outcome
The user can verify and improve every generated explanation before it becomes part of the guide.

## Contract
Show section text and claims beside the exact cited source; provide actions to open Code Lens, edit prose and spoken text, reorder sections, validate, accept, or reject. Invalid claims identify the failed path/range and block acceptance until fixed or explicitly removed. Accepted revisions are immutable. Reject keeps the draft payload and reason/history; later edits create a new revision.

## Acceptance criteria
- Editing a draft persists locally and never changes the captured repository.
- Accept is available only after schema and evidence validation; acceptance records snapshot and revision identity.
- Reject does not delete the draft; rejected content cannot later be accepted without creating a new revision.
- The user can inspect every citation and see labels for inference and repository context.
- Chapter Studio distinguishes missing, draft, rejected, stale, and accepted states.

## Excludes
Changing source code, silent acceptance, and direct editing of an accepted revision.

## Done evidence
Create, edit, validate, accept, reject, and repair a chapter; verify the old payload and accepted revision remain unchanged and every invalid anchor blocks acceptance.

## Source-only evidence (2026-10-28)
- `src/App.tsx:4277-4292` skips saved-history reads outside the native runtime. `src/App.tsx:4386-4387` now reports that state as unavailable; `src/lib/chapterStudioStatus.ts:14,28,44-50` maps unconfirmed history to unavailable and permits generation only after a ready read proves the target is missing or stale.
- `src/App.tsx:4698-4707,4751-4753,4792-4793` disables overview generation outside the Mac app and tells users to load history there before deciding a file is missing.
