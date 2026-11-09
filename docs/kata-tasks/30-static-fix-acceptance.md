---
kind: task
key: static-fix-acceptance
parent: t24
title: Complete packaged regression acceptance for first-release fixes
priority: 1
labels: reliability, validation
---

> **Scope lock — 7 November 2026:** this issue covers only the original snapshot, storage, transport, navigation, Chapter Studio, and narration regressions listed under **Acceptance criteria** below. Formal VoiceOver/screen-reader, 200% zoom, contrast-preference, and reduced-motion audits are removed. Claude/Antigravity setup, broad agent/planning qualification, AI explanations for PRs, whole-repository narrative/visual quality, and language qualification are separate from this regression task; provider/API and PR features are post-1.0. Keep ordinary keyboard actions when they are part of a listed user workflow.
>
> Dated execution notes and broader checklists from the former omnibus version are preserved in [the history archive](../acceptance/2026-11-07-static-fix-acceptance-history.md); they are not additional acceptance criteria.

## Current native evidence — 2026-10-16

The packaged app automatically discovered the existing desktop Codex with no saved executable selection, reused the ChatGPT account and generated a synthetic file chapter using **GPT-6 Luna / max**, with no direct API key. Exact source review and consent preceded generation. A locally validated preview was saved as r1 Draft and explicitly accepted as r2; a citation opened verified source lines and the manually paced reader advanced sections. Named permission profiles replaced unsupported protocol fields without removing source/citation validation. See [dated evidence](../acceptance/2026-10-16-codex-validation.md).

`npm run check` passed: corpus, 98 frontend tests, 185 Rust tests passed/two ignored, frontend build and Cargo check. The opt-in installed-account metadata test passed separately. Local unsigned packaging and native file flow were exercised. The synthetic permission probe allowed staged reads and denied outside reads/writes; no network probe ran.

**Still required:** native evidence for every remaining operation; account/runtime/change/cancel/quota/crash/recovery boundaries; bounded repair and actionable diagnostics after invalid chapter output; usable real-repository narratives/visuals; narration, clean-account packaging and distribution qualification. One prior native response failed validation; the later success did not prove automatic recovery. Keep this task open. Claude is post-1.0.

## Validation authorization — 2026-10-16

The user explicitly authorized building, testing and validating Codex with a small synthetic example through the installed account. Execute the relevant build/tests now and record failures and fixes. Earlier no-test statements are historical. Preserve the user's named GPT-6 Luna/max model constraint. Claude runtime/product qualification is post-1.0; formal accessibility audits are out of scope.

## User outcome
The fixes from the October 15 code review have recorded behavioral evidence before release.

## Current evidence
The original October 15 pass was source-only. October 16 authorization and executed checks supersede that restriction. The baseline above does not complete the broader cases below; keep this task open.

## Contract
Exercise the changed paths below when validation is authorized. Record commit, environment, exact trigger, expected/actual outcome, and evidence. Fix any regression before closing; do not run this acceptance merely because the task was created.

## Acceptance criteria
- Snapshot ignore handling: ignored file, negated pattern, re-included directory, escaped literal exclamation mark, and force-tracked ignored file retain correct captured/skipped status.
- While a later file is read, change an earlier already-dirty captured file. Capture reports inconsistent, publishes no snapshot, and preserves history.
- [x] Oversized index (64 MiB), recovery journal (1 MiB), source object (recorded size, at most 2 MiB), and WAV (12 MiB) stop reading at cap plus sentinel. Exact-size, empty, truncated and corrupted inputs fail or load according to their contracts. Seven focused Rust cases passed on 8 November 2026; results are recorded in Kata `xymb`.
- [x] All direct-provider and narration production clients reject 301/302/303/307/308 without a second request, forwarded body/credentials, provider fallback, or history mutation. The recorded 7 November suite covers both production transports, and those source files are unchanged through current `main` (`8b20924`). See [redirect rejection sub-gate](#redirect-rejection-sub-gate--8-november-2026); no test was rerun for this status update.
- [x] Resolve Walkthrough history refreshes in reverse order; only newest data/error/loading survives. Switch projects during Git comparison; old success/failure cannot affect the new project. Focused `git-history`, `workspace-refresh`, and `gitComparisonController` tests passed on 8 November 2026.
- [x] Switch providers with a credential read pending; old status cannot enable the new provider. Provider selection/close remain unavailable during Keychain save/remove and controls recover on failure. Focused `providerKeychainLifecycle` tests passed on 8 November 2026.
- During Chapter Studio save/validate/accept/reject/restore, editing and revision selection cannot discard new input or show results for a different revision. Navigate away: completion must not alter the new route. Saved revisions remain discoverable.
- [x] A file deleted, excluded, or changed to binary in a newer snapshot keeps its chapter history visible with original citations; unavailable current source is not offered for generation. `chapterStudioTargets` now asserts original anchor identity and that deleted, skipped, excluded, or binary paths are review-only. Its focused test passed on 8 November 2026.
- During narration review/cache/synthesis, navigate away or rapidly use playback controls: no late autoplay/state, abandoned reviews, or adopted cancelled output. Cancel remains reachable; script controls resume after completion/failure. Previous audio and transcript survive.
- Cached playback position survives normal mount and React effect restart. Malformed/truncated/oversized WAV is rejected safely.

## Excludes
AI PR explanation (post-1.0), non-Codex agent qualification, broad chapter-quality claims, and closing other acceptance gates from this checklist alone. This task does not authorize live paid calls.

## Done evidence
Attach results for every remaining case on the patched commit; checked items retain dated evidence. Keep this issue open until all criteria pass.

## Historical evidence

Focused and dated results from the former omnibus checklist are preserved in [the acceptance history archive](../acceptance/2026-11-07-static-fix-acceptance-history.md).

## Local regression baseline — 2026-11-07

At source revision `6e1799480b9d4567d7ff1695513c1ea4fe1cd8e7`, with the current worktree changes, `npm run check` exited 0. This ran the frontend test suite, seven-case chapter-quality corpus, TypeScript/Vite production build, Rust library tests, and `cargo check`. The build emitted the existing large frontend chunk advisory; Rust reported unused-code warnings. No app server, live provider request, model generation, or VoiceOver run was started.

This baseline does not establish the task's per-case snapshot, size-cap, race, recovery, Pocket, or packaged-app acceptance. The redirect rejection cases are covered by the recorded Rust suite, as detailed below. Those other gates remain open; do not close `xymb` from this check alone.

## Redirect rejection sub-gate — 8 November 2026

Review of the recorded 7 November `npm run check` confirms the Rust suite includes loopback coverage for all five redirect statuses (301/302/303/307/308) in both the direct-provider transport and narration transport. The fixtures use the same production client-builder functions with only loopback HTTP and proxy settings adjusted; redirect handling remains `Policy::none()`. They assert no follow-up request and preserve the reviewed request body and credential boundary. The current `provider_api.rs` and `narration.rs` match the tested revision. No test, build, provider request, or app launch was repeated for this review. This closes only the redirect sub-gate; the remaining `xymb` criteria stay open.
