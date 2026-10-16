---
kind: task
key: notification-center
parent: design-parity
title: Collect background outcomes and recovery behind a notification bell
priority: 1
labels: product, design, ux
---

## Outcome
Background updates do not cover code or push content down. The header bell opens a compact list with plain outcomes and the next useful action.

## Work
Use standard Radix Popover: non-modal, Escape/outside close, proper focus return, bounded scrolling and keyboard actions. Badge represents available updates; never open or steal focus when work completes/fails. Aggregate durable interrupted work, saved previews and job outcomes; identify the associated project/question without dumping provider prompts, fingerprints or validator IDs. The relevant answer can show a concise failure and Retry/Stop. Notifications reopen existing work; they do not dispatch inference or accept a chapter. Dismiss does not delete saved explanations; any actual discard must be accurately named. Retain diagnostics separately for optional support details.

Move generation progress/results out of blocking dialogs only after background job ownership exists. Keep necessary explicit setup/scope decisions contextual; do not turn a required decision into a hidden autoapproval. Do not migrate a job to notification storage and lose its original evidence/account ownership. Define dedupe, read/dismiss, resolution and retention so app refresh cannot resurrect cleared duplicates or lose unresolved work.

## Current implementation
Global runtime, workspace, interrupted-operation and pending-preview banners now use a shared bell popover; actions still call existing explicit recovery methods. Existing underlying recoveries remain persisted natively. General notices currently remain one transient item and are not a durable event history. Capture-specific views and generation dialogs still need migration. No automatic panel opening. This partial UI implementation does not satisfy the full job lifecycle.

## Acceptance
Check empty/multiple/project-scoped entries, live completion while typing/playing, Escape/outside close, action opens source/guide, disabled missing previews, retry failures, focus into opened dialog and back, restart persistence, resolved dedupe, 200% zoom, dark/light contrast and screen reader announcements. Verify no banner stack in the main reading surface and no implicit save/accept/retry.

## 16 October workspace migration

The bell receives workspace outcomes; legacy recovery remains reachable. New workspace jobs do not open stage modals. A durable notification history remains open.
