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

The fourth broad agent investigation took 125.603 seconds in native generation and 127.82 seconds for the full test. It delivered 52 calls, read 47,986 distinct code bytes, consumed 239,661 metadata bytes, and then failed a search matcher without producing an answer. [Preserved result](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-search-terms.json). Earlier transport failures and the old 606-second stopped attempt remain in the record. This is one failed sample with a changed integration, not a speedup measurement, percentile, or two-minute useful-answer pass.


## Native tool boundary checkpoint — 17 October

- [x] Add a named monotonic clock and start/prepared/finish offsets to newly persisted native retrieval calls. Record rejected and uncertain calls, preserve legacy missing values as unknown, and validate event order/state on inspection. Deterministic regressions cover successful/rejected/uncertain calls, restart inspection, invalid clock/missing/backwards/overlapping offsets and source-free records.
- [ ] Complete the full stable job/turn trace contract above: queue, connection/index preparation, inference/correction, validation, publication, first useful answer, export/retention and restart reconciliation. Call-level offsets alone cannot attribute gaps to provider/model reasoning.

The first attached `memory-face` sample ended after 391.557 seconds without a validated answer because one exact source quote was reflowed. It delivered 29 calls, 69,653 source bytes and 63,143 metadata bytes. Its old ledger has no native call timings, so none were reconstructed. This is one failed sample and fails the two-minute useful-answer target. [Exact candidates and counters](../acceptance/artifacts/2026-10-17-fontdone-agent-memory-face.json), [measurement boundaries](../acceptance/2026-10-17-fontdone-quotes-and-tool-timing.md). The new retest records the new native timing fields; repeated cold/warm comparisons remain pending.


The separate [attached-function retest](../acceptance/artifacts/2026-10-17-fontdone-agent-memory-face-exact-quotes.json) failed after 380.156 seconds with no preview. Its 36 delivered native calls total 12.568 seconds; final native call finish is 113.301 seconds on the context clock. First observed agent-message delta is 286.377 seconds and final validation 379.455 seconds on the outer probe clock. [Native timing companion and explicit clock origins](../acceptance/artifacts/2026-10-17-fontdone-memory-face-native-timing.json) preserve the new observations. Different clock origins and unmeasured gaps prevent a full stage attribution; no model-reasoning or provider-time claim follows. The two-minute gate still fails. The native-reference reliability dependency is [yk6c](72-agent-evidence-references.md); it does not replace this task's complete trace/benchmark contract.

## Native-reference timing observation — 17 October

Frozen Fontdone operation `da7f3bb9-b3a8-4ce1-973e-e5ccd9916340` failed after 358.714 seconds, publishing no answer. Its 40 native calls totalled 18.542 seconds (longest 1.742); the last finished at 92.239 seconds on the separate captured-context clock. On the outer run clock: first answer delta 229.125, first preflight 258.780, correction start 270.063, correction first delta 323.996 and second preflight 351.580 seconds. [Source-free measurements and interpretation](../acceptance/2026-10-17-native-citation-references.md). These clocks differ in origin; do not attribute all uncovered time to model reasoning or provider latency.

The two-minute gate fails. Full stage tracing and first-section publication remain absent. [Kata 3bz5](73-progressive-answer-sections.md) now specifies the latter. The deterministic suite and a working citation protocol are not a latency pass. The approximately eleven-second first preflight is a bounded profiling opportunity; measure its actual owners before changing runtime identity or source checks.
