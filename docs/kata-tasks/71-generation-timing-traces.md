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

Research and contracts only. This task narrows the existing timing checkbox in parent Kata `nw1h`; the parent retains durable scheduling, progressive section publication, restart and the broader cancellation matrix.

## First local cost measurement — 17 October

The owned-tool investigation exposed repeated local hashing of the 246 MB installed Codex executable. The original standalone diagnostic took 8.7–9.4 seconds per hash. After enabling and optimizing the existing RustCrypto accelerated backend, the actual native identity check took 122–128 ms across four calls. This is a bounded local diagnostic, not a measured end-to-end generation speedup or an implemented stage trace. [Artifact and timing boundaries](../acceptance/artifacts/2026-10-17-runtime-digest-timing.json) preserve the distinction.

Full monotonic stage tracing, bounded export/restart behavior, controlled repeated Fontdone questions and the two-minute first-admitted-answer gate remain open.

The fourth broad agent investigation took 125.603 seconds in native generation and 127.82 seconds for the full test. It delivered 52 calls, read 47,986 distinct code bytes, consumed 239,661 metadata bytes, and then failed a search matcher without producing an answer. [Preserved result](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-search-terms.json). Earlier transport failures and the old 606-second stopped attempt remain in the record. This is one failed sample with a changed integration, not a speedup measurement, percentile, or two-minute useful-answer pass.
