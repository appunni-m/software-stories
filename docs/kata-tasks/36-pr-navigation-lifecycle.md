---
kind: task
key: pr-navigation-lifecycle
parent: t27
title: Define recoverable PR generation and decisions across navigation
priority: 2
labels: pull-request, reliability
---

## User outcome
Leaving Changes during an operation never loses a generated draft or changes another comparison's UI.

## Current evidence
This pass guards/discards late local PR review builds and locks draft selection during decisions. The complete sent-request/navigation recovery contract still needs implementation review and runtime acceptance.

## Contract
Bind every build/send/cancel/accept/reject completion to project, resolved base/head pair and operation identity. On navigation, cancel unsent work and discard its review. For a sent request choose and document one behavior: local cancellation with late output discarded, or a validated draft saved and discoverable in that comparison's history. Never retry or switch providers automatically. Persisted decisions survive unmount; callbacks must not redirect or clear another comparison.

## Acceptance criteria
- Navigate during each phase, then resolve success/failure late; no abandoned review, stale UI mutation, hidden accepted result or duplicate send.
- Returned drafts remain discoverable under the correct immutable commit pair if recoverable completion is selected.
- Cancel and network errors preserve prior history and show what completed versus what stopped.
- Reopening the same comparison finds saved decisions/drafts with exact provider and source identities.

## Excludes
Background auto-generation, remote PR writes and retrying billable calls without a new review.

## Done evidence
Documented lifecycle decision, implementation where needed and later native navigation/cancellation evidence; current evidence is static only.

## Preserve submitted PR Guide sends across navigation — 2026-10-30

Lifecycle decision: after the user chooses **Send**, keep the operation owned by
the native command. Navigation fences all late renderer callbacks, while native
validation saves a successful result as an unaccepted draft under the reviewed
comparison identity before returning it. Reopening that same comparison reloads
its local draft history. This makes a late validated result recoverable without
allowing it to mutate a different comparison's UI.

Static review found an admission race: component unmount discarded the request
review even after **Send** had begun, so navigation could delete the review
before the native command consumed it. Unmount now discards only unsent previews;
the send path releases its review after the command resolves, including early
failures where it was never consumed. This is safe after successful consumption
and does not cancel or retry a provider request.

On the macOS checkout based at `663fd94` with the changes above, the focused PR
Guide lifecycle and operation-fence suites passed (**10/10**), and
`npm run build` passed. No provider request or native app run occurred. Delayed
navigation success/failure, history identity and decisions in the packaged app
remain unqualified; keep the task open.
