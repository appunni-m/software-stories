---
kind: task
key: generation-timing-traces
parent: background-generation
title: Measure each generation stage and time to the first validated answer
priority: 1
labels: performance, agents, diagnostics
---

## User outcome

Questions finish in a useful time, and a slow or failed question can be diagnosed without asking the reader to understand the generation pipeline. Waiting questions are labelled honestly and Stop remains responsive.

## Confirmed gap

Fontdone's broad explanation was manually stopped at 606 seconds without a validated answer. No persistent stage trace identifies where that time went. The observation is neither a native timeout nor a benchmark percentile. The [industry review](../research/2026-10-17-repository-context-and-indexing.md) recommends measurement before model or indexing-performance claims.

## Required contract

- Attach a stable job/turn identity to queue wait, connection preparation, index load/build, retrieval, each inference/correction, validation and publication. Record monotonic start/end or elapsed times, terminal status and explicit gaps; preserve event order across the native/renderer boundary.
- Record model/reasoning, runtime identity digest, prompt/contract version, request fingerprint, evidence bytes, output bytes, tool counts and cache outcomes. Token usage is nullable and recorded only when reported by the installed runtime. Do not label bytes as tokens.
- Define first useful answer as the first complete independently validated answer section published for that question; record separately from first model token, partial JSON, completed whole guide and narration readiness. Until independent-section publication exists, report that fact and measure first admitted chapter.
- Preserve failed, stopped and timed-out attempts. Distinguish native timeout, user Stop, queue cancellation and model failure. Correlate narrow correction turns without hiding their latency or treating them as user retries.
- Keep traces bounded, versioned and exportable through diagnostics. Exclude source contents, credentials and model reasoning by default. A trace write failure must not lose or duplicate a saved chapter. Restart reconciles active/uncertain state without reissuing inference.
- Main UI shows compact queued/working states and existing Stop; detailed process/timing belongs in diagnostics or notifications. One managed runtime and the existing server port are preserved.

## Acceptance

1. Deterministic regressions cover queued/active cancellation, stage errors, missing runtime usage, clock/event ordering, correction attempts, delayed/lost publication replies, retention limits and safe diagnostic export.
2. Run the agreed frozen Fontdone question set with GPT-6 Luna/max, no concurrent build. Separate cold/warm index runs and queue wait from active time; record repeated samples and failures. Report sample count, median and tail only when supported by the samples.
3. Compare old selection, local range retrieval and agent retrieval using identical questions/evidence expectations. Identify measured bottlenecks and review semantic usefulness separately from schema/citation validity.
4. Evaluate the proposed first useful answer goal of two minutes for the agreed questions. Record pass/fail without hiding omitted core code, lowering reasoning, truncating valid answers or treating a generic introduction as success.

## Evidence

The dated probes below record local identity costs, coarse generation phases and owned-tool counters. They do not implement or qualify the complete versioned stage trace required above. This task remains open and narrows the timing checkbox in parent Kata `nw1h`; the parent retains durable scheduling, progressive section publication, restart and the broader cancellation matrix.

## Broad answer latency after retrieval feedback — 17 October

The [fifth broad attempt](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-budget-identifier.json) delivered 58 calls and 94,809 source bytes across nine implementation paths. First observed agent-message delta progress arrived at 484.557 seconds; validation started at 528.769 seconds and rejected a declared identifier missing from claim prose. Native execution ended at 529.629 seconds; the whole test took 531.81 seconds. No preview was published and no limit notice was needed. The two-minute first useful answer target fails; no semantic or speed pass follows from improved code coverage. Do not attribute all of this time to context size, search or model reasoning without the missing stage/event measurements. This is one failed sample, not a latency distribution.

## First local cost measurement — 17 October

The owned-tool investigation exposed repeated local hashing of the 246 MB installed Codex executable. The original standalone diagnostic took 8.7–9.4 seconds per hash. After enabling and optimizing the existing RustCrypto accelerated backend, the actual native identity check took 122–128 ms across four calls. This is a bounded local diagnostic, not a measured end-to-end generation speedup or an implemented stage trace. [Artifact and timing boundaries](../acceptance/artifacts/2026-10-17-runtime-digest-timing.json) preserve the distinction.

Full monotonic stage tracing, bounded export/restart behavior, controlled repeated Fontdone questions and the two-minute first-admitted-answer gate remain open.

## Packaged focused-section timing — 2026-10-21

One explicitly approved packaged Codex request against Fontdone `main@5249b72371` published a focused `Font::face_count` section after **174,204 ms (2m54s)**, above the proposed two-minute first-useful-answer goal. The run read 10,395 captured source bytes; this is not the total request/context size. A 679,724-byte local Pocket WAV appeared about six seconds after the saved revision, so narration synthesis did not hold up section publication in this sample. The exact sample and observation limits are recorded in [the prewarm acceptance](75-narration-prewarm.md). This is one narrow sample, not a latency distribution. Current traces still cannot assign uncovered time among queue, index/retrieval, runtime preparation, model inference and validation; do not attribute the result to context size or any single stage.

## Safer chapter-validation correction feedback — 2026-10-21

The same run's first proposal was rejected with only the generic `sectionValidation` finding. A second proposal began about 40 seconds later; the section was published about 44 seconds after the first rejection. The saved receipt does not say whether rejection came from authored-section canonicalization or the final chapter validator, so it cannot establish the root cause or quantify a speedup. Its single app-server turn reports thread-cumulative usage of 276,783 input tokens (215,808 cached), 18,436 output tokens and 14,832 reasoning tokens. Those totals are not broken down by source, instructions, tool schemas, or stage, so they do not prove source files were the main context cost. Native final-validator failures now return a deduplicated set of up to 16 fixed `ChapterIssueCode` categories; dynamic issue messages, paths and source values are never sent back to the agent. The prompt distinguishes correctable evidence findings from native snapshot/integrity boundaries. Focused section-ledger tests pass: 17 passed, including bounds, unknown-code rejection and source-free feedback. Full stage timing and another authorized packaged sample remain necessary to evaluate latency impact.

## Repository-guide renderer queue handoff — 22 October

Guide start, explicit resume and playback-driven refill now measure time waiting in the shared WebView question queue with `performance.now()`. The frontend passes the bounded duration to the matching native command. The native worker attaches it to the first progressive answer run created by that operation, consuming the optional value once; later slides and automatic retries do not inherit the same queue wait. A guide operation that fails during outline/planning before starting an answer run has no answer trace to carry the observation. The timing report names guide planning duration as unmeasured and describes this queue observation as a separate WebView clock, not part of native monotonic stage durations.

Deterministic frontend coverage held one answer operation in the queue, advanced a controlled clock by 125 ms, and observed that value at `start_repository_guide`; it also checked resume/refill payloads. Native guide tests cover the 24-hour diagnostic bound, and the focused suites passed: 27 workspace-conversation tests and 77 repository-guide tests (2 ignored). This closes only the renderer-side Guide queue handoff gap. Cold/warm indexing, pre-manager connection, retrieval vs inference attribution, validation/publication event completeness, renderer adoption, narration readiness, bounded export/retention/restart, and the agreed frozen Fontdone sample set with the two-minute target still require evidence.

The fourth broad agent investigation took 125.603 seconds in native generation and 127.82 seconds for the full test. It delivered 52 calls, read 47,986 distinct code bytes, consumed 239,661 metadata bytes, and then failed a search matcher without producing an answer. [Preserved result](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-search-terms.json). Earlier transport failures and the old 606-second stopped attempt remain in the record. This is one failed sample with a changed integration, not a speedup measurement, percentile, or two-minute useful-answer pass.


## Native tool boundary checkpoint — 17 October

- [x] Add a named monotonic clock and start/prepared/finish offsets to newly persisted native retrieval calls. Record rejected and uncertain calls, preserve legacy missing values as unknown, and validate event order/state on inspection. Deterministic regressions cover successful/rejected/uncertain calls, restart inspection, invalid clock/missing/backwards/overlapping offsets and source-free records.
- [ ] Complete the full stable job/turn trace contract above: queue, connection/index preparation, inference/correction, validation, publication, first useful answer, export/retention and restart reconciliation. Call-level offsets alone cannot attribute gaps to provider/model reasoning.

The first attached `memory-face` sample ended after 391.557 seconds without a validated answer because one exact source quote was reflowed. It delivered 29 calls, 69,653 source bytes and 63,143 metadata bytes. Its old ledger has no native call timings, so none were reconstructed. This is one failed sample and fails the two-minute useful-answer target. [Exact candidates and counters](../acceptance/artifacts/2026-10-17-fontdone-agent-memory-face.json), [measurement boundaries](../acceptance/2026-10-17-fontdone-quotes-and-tool-timing.md). The new retest records the new native timing fields; repeated cold/warm comparisons remain pending.


The separate [attached-function retest](../acceptance/artifacts/2026-10-17-fontdone-agent-memory-face-exact-quotes.json) failed after 380.156 seconds with no preview. Its 36 delivered native calls total 12.568 seconds; final native call finish is 113.301 seconds on the context clock. First observed agent-message delta is 286.377 seconds and final validation 379.455 seconds on the outer probe clock. [Native timing companion and explicit clock origins](../acceptance/artifacts/2026-10-17-fontdone-memory-face-native-timing.json) preserve the new observations. Different clock origins and unmeasured gaps prevent a full stage attribution; no model-reasoning or provider-time claim follows. The two-minute gate still fails. The native-reference reliability dependency is [yk6c](72-agent-evidence-references.md); it does not replace this task's complete trace/benchmark contract.

## Syntax-index preparation stage — 25 October 2026

Progressive workspace generation now records `syntaxIndexLoad` as its own
monotonic pre-dispatch stage, before retrieval-session creation. A missing or
unreadable optional index still uses the existing text-search fallback. The
already loaded syntax index is passed into the session, avoiding a second
unmeasured read. The diagnostic retains only the fixed stage name, elapsed
offsets and normal stage completion; it contains no index contents or error
text. The record schema is unchanged.

Focused verification passed: generation-timing 18/18, workspace-job 18/18,
progressive-generation 16/16 (two account probes ignored), and agent-context
24/24, including the preloaded-index handoff regression. Formatting and
whitespace checks pass. This measured session syntax-index loading before
retrieval-session creation. The per-search cache and preparation steps are now
measured separately below. Repository-guide planning, provider/runtime wait,
renderer adoption and audible narration remain outside these observations.
Frozen repeated Fontdone comparisons and the two-minute semantic first-answer
gate remain unqualified. Keep `ps02` open.

## Per-search cache and index stages — 25 October 2026

Each owned `search_snapshot` call now records fixed stage names and monotonic
offsets on the evidence-ledger context clock: input validation, local cache
lock, manifest load, cache check, optional syntax-index load, local search-index
preparation, cache publication check and query execution. Warm calls omit the
index-load and preparation stages. The maximum is eight stages per search; the
trace stores no query, source, path or dynamic error text. Missing or unreadable
syntax indexes are marked as a fixed fallback outcome, while captured-text
search remains available.

Stages are persisted with the owned Search call, including completed local work
when cancellation or response handling prevents delivery. The serialized timing
schema remains backward-compatible through an optional empty stage list.
Focused verification passed: generation-timing 25/25, repository-context
commands 6/6 (one explicit Fontdone profile ignored), evidence-ledger stage
persistence/cancellation 2/2, and the updated agent-context search-delivery
case 1/1. Formatting passes. These deterministic tests do not qualify repeated
Fontdone latency, queue or provider time, guide planning, independent section
publication, or the two-minute first-useful-answer goal; `ps02` remains open.

## Native-reference timing observation — 17 October

Frozen Fontdone operation `da7f3bb9-b3a8-4ce1-973e-e5ccd9916340` failed after 358.714 seconds, publishing no answer. Its 40 native calls totalled 18.542 seconds (longest 1.742); the last finished at 92.239 seconds on the separate captured-context clock. On the outer run clock: first answer delta 229.125, first preflight 258.780, correction start 270.063, correction first delta 323.996 and second preflight 351.580 seconds. [Source-free measurements and interpretation](../acceptance/2026-10-17-native-citation-references.md). These clocks differ in origin; do not attribute all uncovered time to model reasoning or provider latency.

The two-minute gate fails. Full stage tracing and first-section publication remain absent. [Kata 3bz5](73-progressive-answer-sections.md) now specifies the latter. The deterministic suite and a working citation protocol are not a latency pass. The approximately eleven-second first preflight is a bounded profiling opportunity; measure its actual owners before changing runtime identity or source checks.

## Fresh version-2 broad timings — 17 October

The isolated committed-helper Fontdone run publishes at 76.328, 336.939 and 475.887 seconds; probe adoption observes 76.699, 337.700 and 477.074 seconds. Terminal result is Partial at 491.831 seconds, with eight proposals exhausted before rasterization and final bitmap publication. All 61 tool calls deliver. These are one native probe's observations, not installed UI timing, distribution/tail evidence or a first-useful-answer quality pass. [Exact artifact and qualification limits](../acceptance/2026-10-17-scene-v2.md). Keep this task open.

## No-publication failure category — 23 October 2026

The active v30 Fontdone Guide's failed topic receipts contain a valid
`answerManifestV2` with an incomplete-topic entry, but no accepted answer
section. The native manifest validator correctly rejects completion without a
published section; because the error category previously combined this case
with manifest-version mismatch, the saved operation `failureCode` was
`unknown`. The native code now gives the empty-publication case its own fixed
`NoPublishedSections` category and keeps final-manifest size, shape, version,
and invalid unfinished-topic failures distinct as well. The focused
section-ledger suite passes 22/22, including the empty-publication behavior and
failure-category mapping regressions; format and diff checks pass.

This source change does not rewrite existing receipts, update the running v30
app, or add the application outcome to the separate Codex timing trace. The
active Guide therefore still has its original `unknown` receipts. Timing
diagnostics still need an explicit application-result event/correlation and the
broader cold/warm benchmark acceptance. Keep `ps02` open.

A later read-only snapshot of that same Guide showed four topics failed, the
parser topic building, and the final three topics pending; the Guide still had
zero slides. The Rust-example topic had failed since the prior measurement.
No stop or retry was initiated. This is an in-progress UI observation, not a
terminal timing sample.

## Optional syntax-index fallback outcome — 27 October 2026

Pre-dispatch timing now distinguishes an available syntax index from a missing
or unreadable index that falls back to bounded text search. At that point the
trace and job-store schemas advanced to v4; the contemporaneous source review
recorded older records as readable. Diagnostics retain only the fixed
`syntaxIndexLoad` stage and `fallback` outcome, never the error text or index
contents.

The 29 October greenfield compatibility audit removed those old-format readers.
The current timing trace accepts schema 5 only, and the current job store
accepts schema 4 only. Older records are left byte-for-byte intact but are
reported as unsupported; they are not upgraded or deleted. The timing reader
also requires the complete current envelope, including `promptContractVersion`.
This source-only review did not run builds or tests. Packaged Fontdone timing,
complete stage coverage and the two-minute useful-answer acceptance remain
open; keep `ps02` open.
