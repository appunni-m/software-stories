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

## Historical reproducer and confirmed boundary

On Fontdone `5249b723719445813b8771a602ea5b96891d4450`, captured `src/font.rs` is 375,700 bytes. The October 17 implementation excluded files larger than 250,000 bytes from source discovery, so a fixed target could not enter question generation. The 613,652-byte `src/ffi/handles.rs` and 1,032,893-byte C adapter also exceeded the parser/index file limit. The native replacement build reproduced the original `src/font.rs` eligibility error after connection recovery; the source itself opened correctly. This was separate from stale-runtime failures on the initial queued questions.

That eligibility failure is no longer the current source state: the October 27 discovery handoff keeps readable captured files above 250 KB as range-only candidates, and the guide reads bounded immutable ranges instead of staging the whole target. The parserless files still need large-repository retrieval and answer-quality qualification; see the dated source progress below. Capturing a file alone still does not prove the agent can find, explain and cite the relevant ranges.

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

The [repository context study](../research/2026-10-17-repository-context-and-indexing.md) compares ten published product approaches and existing indexing components. The implementation work was split into [local range retrieval](69-local-range-retrieval.md), Kata `a699`, followed by [owned Codex retrieval tools](70-codex-retrieval-tools.md), Kata `gk46`. This replaces whole-file selection with bounded search/read operations while preserving exact-source validation. Semantic embeddings and compiler indexes remain evaluated extensions. Current implementation and acceptance state is recorded below.

## Current implementation and package evidence — 22 October 2026

The local indexing and bounded search/read implementation is present in the
current source; the owned Codex retrieval tools are also implemented. A
packaged-app spot check found and opened `Font::memory_face` inside Fontdone's
375,700-byte `src/font.rs` without attaching a reference file. The same app's
broad global query still failed to surface that declaration on its first page.
See [packaged local-search observations](../acceptance/2026-10-22-packaged-local-search.md)
and the detailed [retrieval profile](../acceptance/2026-10-17-range-retrieval.md).
Implementation presence and a successful exact-name search do not pass the
broad-retrieval or generated-answer acceptance gates. Keep Kata `0wdn`, `a699`,
and dependent `gk46` open until the remaining retrieval, agent, package, and
answer-quality evidence is recorded.

## Discovery handoff improvement — 27 October 2026

Automatic source discovery now keeps captured text files larger than 250 KB in
a range-only candidate list instead of treating them as whole-file sources or
excluding them entirely. It may return one large target and up to 11 small
supporting files; the target remains subject to the existing bounded local
range reader and fresh workspace grant. Discovery guidance now prefers a
materially relevant large implementation as the primary target, with small
whole-file sources as supporting context. The handoff preserves the user's
question, snapshot identity, supporting paths, and per-file selection reasons
as a collapsed disclosure on the saved answer card. A range-only result with
zero smaller supporting files is now accepted. Source-only change; no build,
tests, agent request, restart, or packaged-app acceptance ran. Search recall,
exact range selection, omitted relevant ranges, citations, cancellation,
multi-stage follow-up, and the Fontdone scenarios above remain unqualified.

## Persist discovery coverage limits with the range answer — 27 October 2026

The range handoff now carries the bounded discovery review's omitted-candidate
count, coverage-gap counts and one example path per gap, together with the
agent's limitation note. These join the already persisted target/supporting
file reasons in the answer card's collapsed source-selection record. The same
coverage labels now drive both the review dialog and saved note. This preserves
discovery-level omissions across navigation and restart; it does not identify
which omitted files or byte ranges are semantically relevant, or qualify local
range-search recall. Source-only change; no tests, build, agent request or
packaged-app acceptance ran. Keep the full acceptance above open.

## Do not advertise unreadable large paths as range targets — 28 October 2026

Static review found that `discovery_coverage_gaps` checked the 250 KB size
threshold before source readability. A code-like path with binary or invalid
UTF-8 content could therefore be labeled `largeFiles`, whose message says a
separate bounded range lookup can recover it, even though large non-text paths
are excluded from `rangeOnlyFiles` and the source reader cannot return them as
code. The gap classification now reports `unreadableSource` first and only
uses `largeFiles` for readable captured text. This improves the explanation of
the omission; it does not change candidate inclusion, source limits, retrieval
recall, or generated-answer quality. No tests, build, profile, provider request
or packaged qualification ran; keep the acceptance task open.

### Regression for unreadable large-path disclosure — 28 October 2026

A focused regression now verifies the coverage-gap order: a readable
implementation above the whole-file request limit is categorized as
`largeFiles`, while equally large binary and invalid-encoding implementation
files are categorized as `unreadableSource`. The fixture also checks the
reported counts and example paths. This prevents discovery from promising a
text-range lookup for bytes its reader cannot decode.

`RUSTC_WRAPPER= cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
large_unreadable_code_paths_are_not_advertised_as_range_retrievable` passed
(1 passed, 837 filtered out). Existing Claude dead-code warnings were emitted.
This verifies the disclosure classification only; broad Fontdone recall,
selected ranges, provider answer quality, cancellation and packaged behavior
remain open.
