---
kind: task
key: t23
parent: m5
title: Onboard users locally and recover from interrupted work
priority: 0
labels: phase-5, onboarding, reliability
blocked_by: t02, t03, t07, t08, t10, t13, t16
---

## User outcome
A first-time user reaches a useful local Atlas before configuring AI and can recover from cancellation, app restart, or provider failure without losing work.

## Contract
First-run explains the local-only path and offers **Open repository**. API setup is optional and appears only when a user chooses generation or speech. Persist projects, snapshots, drafts, accepted chapters, playback position, and retryable work locally. Show operation-specific errors with Retry/Cancel/Choose another path actions where valid.

## Acceptance criteria
- First-run reaches Atlas with networking disabled and no key.
- Closing/reopening during capture or generation produces a clear interrupted state; no partial result is accepted.
- A locally validated generated preview is durably saved before it is shown, appears after restart for explicit review, and can only be saved as an unaccepted draft or discarded. It never stores the key, provider prompt, or consent; a new provider request requires a new exact-context review and fresh consent.
- A failed request can be retried only after reviewing the new exact manifest; no old consent is silently reused.
- Quota, billing, region, keychain denial, permission, malformed model output, and disk errors are distinct and actionable.
- Previously accepted chapters and local source browsing remain available after provider failure.

## Excludes
Mandatory account creation, background jobs after app quit, cloud sync, and telemetry.

## Done evidence
Perform first launch offline, interrupt capture and generation, relaunch, recover/cancel/retry, reopen a generated-but-unsaved preview and save/discard it, simulate provider and storage failures, and verify local accepted work is intact.
