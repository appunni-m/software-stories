---
kind: task
key: large-source-discovery
parent: question-to-source-discovery
title: Explain large implementation files through bounded source ranges
priority: 1
labels: agents, source, quality
---

## User outcome

A question about a large repository follows the relevant code even when one implementation file is larger than a single model request. The reader does not have to split files or guess a smaller question.

## Reproducer and confirmed boundary

Fontdone at `5249b723719445813b8771a602ea5b96891d4450` has a captured, indexed `src/font.rs` of 375,700 bytes. `source_discovery::metadata` excludes every file larger than 250,000 bytes before asking the agent to select sources. Opening that file as a fixed target therefore fails eligibility. The 613,652-byte `src/ffi/handles.rs` and 1,032,893-byte C adapter implementation also exceed the indexer's file limit. Capturing a file is not the same as making it available to question generation.

The native replacement build independently reproduced this after connection recovery: an attached `src/font.rs` question failed with “This target is not eligible captured implementation code. Choose a code file in the snapshot.” The source itself opened correctly. This failure is distinct from the stale-runtime failures observed on the initial queued questions.

## Required behavior

- Advertise relevant declarations in large captured files, with explicit index omissions when parsing is unavailable.
- Select bounded, immutable source ranges using available parser anchors and local text search. Keep original paths, hashes and absolute byte/line coordinates.
- Resolve callers and dependencies through staged retrieval or multiple bounded requests when needed. Preserve the original question and connection. Do not silently substitute docs for omitted implementation.
- Validate claims and visual references against the exact selected ranges and immutable source. Preserve file/byte limits, secret exclusions, cancellation, consent scope and request fingerprints.
- State the useful answer and its evidence boundary in ordinary language. Avoid requiring a manual reference-file checklist.
- Retain the discovery rationale and omitted relevant ranges so a result can be reviewed after restart.

## Acceptance

- Run the font-bytes-to-bitmap question against this Fontdone revision. Verify that the relevant `src/font.rs` implementation is actually available and inspect claims against it.
- Ask about `Font::memory_face` with `src/font.rs` explicitly attached. Generate a useful range-grounded answer rather than the eligibility error.
- Exercise an implementation above the parser limit using bounded local search; disclose unresolved links and never present guessed calls as observed execution.
- Cover Unicode/CRLF offsets, overlapping ranges, altered source hashes, excluded files, range overflow, cancellation and a follow-up across two retrieval stages with deterministic regressions.
- Record failed and successful attempts, request identity, selected ranges and reviewer findings. A raised global byte cap or a successful small sibling file does not satisfy this task.

## Evidence

See [Fontdone acceptance](../acceptance/2026-10-17-fontdone.md). This task records a confirmed source-selection limitation; it does not infer the semantic quality of a generated answer from metadata alone.

## Industry review and implementation units — October 17

The [repository context study](../research/2026-10-17-repository-context-and-indexing.md) compares ten published product approaches and existing indexing components. The recommended implementation is split into [local range retrieval](69-local-range-retrieval.md), Kata `a699`, then [owned Codex retrieval tools](70-codex-retrieval-tools.md), Kata `gk46`. This replaces whole-file selection with bounded search/read operations while preserving exact-source validation. Semantic embeddings and compiler indexes remain evaluated extensions. These are open implementation contracts, not shipped fixes or accepted performance results.
