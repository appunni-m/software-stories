---
kind: task
key: background-generation
parent: design-parity
title: Run code discovery and explanation as owned background jobs
priority: 1
labels: product, design, ux
---

## Outcome
Ask once; the assistant selects code, plans and builds the explanation in the background using the saved Codex connection. The reader sees useful content and a quiet cancellable working state, without a discovery/review/route/batch wizard.

## Native lifecycle
Move job ownership out of generation dialog mount/unmount. Persist stable job IDs, project/snapshot, turn, exact source fingerprint, model/account/runtime, progress, completed artifacts and terminal outcome. UI view/project switching cannot cancel work. Stop is explicit; cancellation and late-result admission are race-safe. Saves are idempotent and independently preserve earlier chapters if a later step fails. Reconcile save uncertainty before a user retry. No silent inference retry, backend/model fallback or autoacceptance. Restart exposes recoverable state without secretly reissuing requests.

## Sharing and connection
Keep one connection in the header; model/reasoning/approval are configured once in settings. Establish a clear revocable project/snapshot/source-scope grant before automatic code sharing. Bind it to destination/account and permitted source classes, limits and exclusions, enforce it natively for each freshly built request, and invalidate it when trust scope changes. Do not reinterpret an old per-request fingerprint as this grant. The initial grant needs a meaningful scope disclosure; only an actual grant/scope/account problem interrupts normal intent. Preserve current exact-source admission until this replacement is implemented and qualified. No private-source validation without human authorization of source and destination.

## Acceptance
Installed GPT-6 Luna/max account path without mandatory API keys; no repeated backend/file selection. Close/change view/project during discovery, generation and save; restart, explicit Stop, quota failure at step two, bad evidence, lost save reply, changed account, revoked grant and wider question. Check bounded source, unchanged saved artifacts and no unexpected transmission/retry. All internal stages use one user intent and one result surface. Claude stays deferred; Pocket remains narration default.

## 16 October workspace migration

New question jobs have module ownership independent of views, explicit Stop, native interruption records, fingerprinted question/range context and revocable native snapshot/account/runtime sharing. A native persisted scheduler and complete restart/reconciliation qualification remain open.

Questions are now serialized before native inference, so follow-ups queued while another question runs do not collide with the installed Codex adapter. A stopped waiter releases its slot without inference. The adapter supplies explicit source-only base instructions instead of inheriting a coding-agent prompt, while retaining all tool-event rejection, permission, provenance, model and grant checks. Restart scheduling and the full cancellation/race matrix remain unqualified.

Workspace generation routes a sole AI-selected implementation file to the file chapter contract. Multi-file workspace overviews have a fingerprinted `anchored-claims-v1` presentation recipe: validate exact claim evidence and diagram references, then compose display/narration from complete claim sentences. Discard separate model prose/narration, preserve inference labels and keep native storage copy validation. No automatic retry or weaker evidence admission is introduced. Qualify multi-file dispatch and malformed-claim failure alongside the existing queue/lifecycle matrix.

## Fontdone qualification — October 17

The broad native question had no validated answer after 606 seconds and was manually stopped. Two queued questions then failed because the frontend reused a cached connection after native cancellation reaped the process. The narrow correction checks/restores the actual saved runtime at the queue boundary before preparing a fresh review; [task 67](67-queued-connection-recovery.md), Kata `xs7j`, owns its regression and native cancellation qualification. Preserve the original failed cards.

- [ ] Profile large-repository time to the first useful, validated answer separately from discovery, generation, validation and queue wait; retain exact request/source identities and do not call this single run a performance benchmark.
- [ ] Distinguish waiting questions from the active question with compact, reader-facing status. In this run every queued card said “Building your explanation…” for the duration of the slow first request.
- [ ] Complete the wider restart, cancellation/race, durable scheduler and lost-save matrix; the narrow connection correction does not close this parent.
