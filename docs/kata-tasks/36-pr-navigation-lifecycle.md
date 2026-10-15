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
