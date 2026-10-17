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
