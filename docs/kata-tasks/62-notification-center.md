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
Global runtime, workspace, interrupted-operation and pending-preview banners now use a shared bell popover; actions still call existing explicit recovery methods. General notices use native schema-1 history with typed scoped recovery and replay-safe dismissal, bounded to 1,000 visible records, 2,048 dismissal keys and 16 MiB. Workspace outcomes keep the latest 50 non-dismissed terminal questions per snapshot and group repeated outcomes by question. Capture-specific views and generation dialogs still need migration. No automatic panel opening. This partial UI implementation does not satisfy the full job lifecycle.

## Acceptance
Check empty/multiple/project-scoped entries, live completion while typing/playing, Escape/outside close, action opens source/guide, disabled missing previews, retry failures, focus into opened dialog and back, restart persistence, resolved dedupe, 200% zoom, dark/light contrast and screen reader announcements. Verify no banner stack in the main reading surface and no implicit save/accept/retry.

## 16 October workspace migration

The bell receives workspace outcomes; legacy recovery remains reachable. New workspace jobs do not open stage modals. A durable notification history remains open.

## Dismiss repeated attempts together — 23 October 2026

In the packaged Fontdone workspace, one notification group contained 21 attempts of the same question, each marked new. The group exposed only the latest attempt's Dismiss action; clearing all notices required repeated work. The notification center now offers **Dismiss all N attempts** for a repeated question group. It acknowledges every matching terminal outcome in one workspace update and preserves the turns, saved answers, source links, and run history. Single outcomes retain their existing Dismiss action. Expanding earlier attempts still allows individual acknowledgment.

The focused notification/history/outcome suites pass **64/64**, and `npm run build` passes with the existing large-bundle warning. The running v30 app was not replaced, so the new group action remains source-level evidence until packaged acceptance. Retain the restart, focus/keyboard, theme, zoom and broader recovery acceptance above; keep `hstr` open.

The same one-click group dismissal now covers repeated durable app notices (for example, repeated save failures). It queues the exact dismissal for every notice in the visible group through the existing serialized history writer; opening/retry actions and saved repository content remain unchanged. No live notices were dismissed. The installed app still exposes only individual Dismiss actions, so this addition also needs packaged verification.

## Keep repository updates out of the reading surface — 27 October 2026

Active capture progress appears in the header notification center with a Cancel
action. Terminal results are saved in the existing notice journal and direct the
user to choose the new snapshot from the header; notices can be read or
dismissed after restart. The old inline capture banner over an existing
snapshot and its styles were removed, leaving the source workspace at full
height. A first capture with no selected snapshot still keeps its capture state
and recovery action in the empty workspace. The notification panel associates
its visible heading with the popover through `aria-labelledby`. Source-only
change; keyboard/focus, persistence, theme, zoom and packaged acceptance remain
open.

Active captures across repositories now each retain their own progress and
Cancel action in the bell. Switching the selected repository leaves the other
capture visible in the collapsed **Other repositories** group; cancellation is
scoped to that capture's snapshot ID rather than whichever repository is
currently selected. Source-only correction; multi-repository packaged behavior
and the remaining acceptance checks stay open.

Question outcomes from conversations already loaded in this renderer now remain
in the bell when a background question finishes after navigation to another
project or snapshot. The feed is capped at the 50 latest unresolved outcomes
across those loaded scopes; each row retains its owning project/snapshot for
grouping, opening, read and dismiss actions. Untouched conversations are not
preloaded. Source-only change; restart discovery for unvisited scopes and the
packaged, focus, persistence, contrast, zoom and screen-reader gates remain
open.
