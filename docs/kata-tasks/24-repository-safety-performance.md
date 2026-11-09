---
kind: task
key: t24
parent: m5
title: Keep repository operations safe and responsive
priority: 1
labels: phase-5, reliability, security
blocked_by: t02, t07, t08, t13, t16, t17
---

> **Scope update — 7 November 2026:** the user removed formal accessibility and screen-reader audits from the product/release backlog. Historical observations are retained in `HISTORICAL_ACCESSIBILITY_VALIDATION.md` and older dated task notes, but they are not open acceptance gates. Normal keyboard controls and readable layouts remain part of the developer workspace; this task covers repository safety and responsiveness only.

## User outcome
Opening, indexing, searching, and asking about a large repository stays responsive; repository content and generated visuals never execute as code.

## Contract
Keep filesystem reads, parsing, indexing, Git subprocesses, and provider payload handling bounded and cancellable. Never execute repository files or model-authored HTML/JavaScript. Preserve captured source and saved work when an operation is cancelled or rejected.

## Acceptance criteria
- Representative large Rust and TypeScript repositories remain usable during capture, indexing, search, and explanation-context retrieval; limits and cancellation behavior are recorded.
- Path traversal, symlink escape, malformed metadata/packages, oversized provider responses, and executable generated content are rejected at the native boundary.
- Git subprocesses have bounded lifetime, cancellation, and cleanup.
- Boundary and performance evidence identifies the tested source revision, fixture size, configured limits, observed result, and known gaps.

## Excludes
Do not claim universal language support or replace security review with UI validation.

## Done evidence
Attach a representative large-repository responsiveness report and documented adversarial path/content checks. Keep the issue open only for unresolved repository safety, cancellation, or performance requirements.

## Fontdone retrieval-index profile — 2026-11-07

The current source was exercised against a disposable copy of the saved 2,768-path Fontdone snapshot. Fresh retrieval-index preparation took 15.565 s; a one-file change updated the index in 4.236 s, reusing 1,620 files and 3,409 documents while preserving an equivalent 3,462-document/one-segment generation. All 12 named ranges were found globally and scoped; the broad first-page range result was still 5/12. Exact method and limitations: [current-source profile](../acceptance/2026-11-07-fontdone-current-source-index-profile.md).

This does not qualify capture, cancellation, packaged UI, TypeScript, adversarial boundaries, or owned-agent behavior. Keep `bcn2` open.

## Native boundary regression map — 2026-11-07

The current-source `npm run check` also exercised these existing safety regressions successfully:

- Snapshot capture: `snapshot::tests::interrupted_staging_cleanup_rejects_traversal_and_symlinked_paths`, `policy_skips_ignored_generated_symlink_and_nested_repository_but_keeps_tracked_ignored_file`, `existing_snapshot_object_hash_is_streamed_and_object_size_is_bounded`, and `corrupt_snapshot_catalog_does_not_reset_history_or_start_capture`.
- Derived storage: `repository_context::cache::tests::symlinks_cannot_redirect_cache_reads_locks_or_eviction_outside_derived_storage`.
- Provider and narration boundaries: API and narration transports reject redirects without a second request; provider parsing rejects oversized/incomplete output; cached WAV reads stop at limit plus sentinel and invalid audio preserves prior cache bytes.
- Visual content: `scene_presentation::tests::raw_markup_and_executable_fields_are_not_in_the_contract`; model-authored markup/CSS/scripts are not renderer instructions.
- Git subprocesses: `bounded_process::tests::timeout_terminates_the_child`, `cancellation_terminates_a_running_child`, and cleanup-failure cases bound and reap the process group.

These tests prove their local invariants, not every OS race, the real capture/index/search experience on both Rust and TypeScript repositories, or the installed app's cancellation and error UX. Keep `bcn2` open for those broader requirements.

## Reuse validated range identities during broad search — 9 November 2026

Prepared indexes now validate a bounded document-address-to-range map once;
file discovery and source ranking reuse it instead of repeatedly deserializing
stored index fields for matching documents. Search ranking, returned evidence,
and index schema are unchanged. The focused native repository-context suite
passes 59 tests (1 ignored).

One release-profile run against clean Fontdone `main@5249b72` on macOS arm64,
Code Atlas source `2e67da6` plus the uncommitted change, measured broad file
discovery at 16.745 ms and broad range search at 98.795 ms. The previous
9 November profile recorded 28.815 ms and 134.787 ms respectively on a different
source revision. Treat this as directional, not a controlled before/after
claim. The additional address map is bounded by the 50,000-range index cap.
The earlier 100,000-symbol timeout, representative TypeScript search,
cancellation-under-load, packaged behavior, and adversarial OS-boundary
evidence remain open; keep `bcn2` open.
