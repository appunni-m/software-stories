---
kind: task
key: queued-connection-recovery
parent: background-generation
title: Recover the saved Codex connection before the next queued question
priority: 1
labels: agents, reliability, workspace
---

## Reproducer

In the packaged app, start a broad Fontdone explanation, queue two further questions, then stop the first. The first stops locally. Both queued questions immediately fail with “The Codex account or runtime changed or is unavailable,” while the header still looks connected.

## Confirmed cause and correction

Native cancellation intentionally reaps the process and invalidates its runtime identity. The workspace's next queued question used the frontend's cached authenticated/running state and skipped observation/restoration. Its new native discovery review therefore found no running process. Refresh the shared connection once after acquiring the question queue and before preparing discovery. Existing saved-account restoration can restart the runtime without sign-in; it continues to respect explicit disconnect. Fresh reviews retain all identity and consent checks. Do not retry a sent request or revive the cancelled question.

## Acceptance

- Regression: simulate a connected runtime followed by reaping; the next restore observes status, resumes the saved account and refreshes models exactly once without login or a model request.
- Wiring regression: every dequeued workspace question forces a runtime observation before discovery; chapter dispatch remains a single send, with no retry loop.
- Packaged app: cancel an active Fontdone operation with a question queued behind it. The next question starts with the saved connection without a settings detour, repeated backend selection or new sharing prompt for unchanged account/snapshot scope.
- Confirm the stopped question remains stopped and no late result is saved; retain the pre-fix failed questions and verify restart preserves them.
- Explicit disconnect, missing login, different account/runtime and altered snapshot permissions remain enforced.

## Evidence

Results belong to [Fontdone native acceptance](../acceptance/2026-10-17-fontdone.md). A successful recovery does not resolve source-size limits or establish larger-repository narrative quality.

## Completed narrow qualification — October 17

Code `96a40a3` passed 136 frontend tests and the production build. The replacement macOS app passed bundle metadata and DMG verification. With a real Fontdone question queued behind an active request, Stop reaped the active operation; the queued question restored the saved Codex runtime and produced a saved source-validated three-section draft, without setup or repeated sharing selection. Its origin records GPT-6 Luna/max. Seven anchors independently match the exact source. The stopped operation had no saved late revision. Restart retained the original stopped/failed questions, child and draft without resending them. Explicit disconnect and unavailable-account regression cases remain passing. Close this narrow recovery task only; parent lifecycle, large-source retrieval and narrative quality remain open.
