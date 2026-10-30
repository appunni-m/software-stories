---
kind: task
key: local-range-retrieval
parent: large-source-discovery
title: Index and retrieve immutable code ranges across large files
priority: 1
labels: indexing, source, retrieval
---

## User outcome

Ask about a repository or a function inside a large file without selecting reference files. Relevant captured implementation remains discoverable even when a whole file exceeds one inference request.

## Scope and decision

Implement the local retrieval layer recommended by the [industry review](../research/2026-10-17-repository-context-and-indexing.md). Reuse existing Tree-sitter parsers, add Tantivy for ranked lexical search, and use the ripgrep project's search libraries for bounded parser-independent fallback. Check dependency licences, macOS packaging and locked compatible versions before adoption. Do not add a vector service or embedding model to this baseline.

## Required contract

- Build from the selected immutable snapshot, retaining inventory entries independently of parser/request limits. A parse failure or oversized file stays searchable through eligible captured text; unavailable text has an explicit reason.
- Index syntax-aware declarations and bounded fallback windows. Every range has snapshot, original path, full-file hash, absolute UTF-8 byte/line coordinates, chunk hash and optional symbol/parent/parser identity. Oversized declarations retain enclosing context when split. Reuse existing source readers and validators.
- Provide bounded local search, source-range reads, symbol lookup, relationship lookup and a compact module map. Search original identifiers, identifier components and actual source contents. Rank exact identifiers before fuzzy matches; preserve deterministic tie breaks and result ordering.
- Version chunking/tokenization and index schema. Reuse unchanged content by digest, publish index generations atomically, remove deleted results, and reject stale/changed-source results. A branch/snapshot switch cannot mix code versions.
- Deduplicate overlapping evidence windows before counting budgets. Keep the current 250,000-byte code budget per model operation; record all transmitted code and structural context across stages. Define tested result, tool-call, cumulative-byte and deadline caps before enabling the agent path. No raised whole-file cap as a substitute for ranges.
- Respect native exclusions and secret filtering for every read. Return coverage, truncation and unresolved relationship labels. Text/AST matches must not be described as compiler-resolved calls or runtime traces.
- Remain in the existing native application process; no new Node server, port or mandatory AI connection for local search. A corrupt index is rebuildable without altering source or saved explanations.

## Acceptance

1. Fontdone at `5249b723719445813b8771a602ea5b96891d4450`: find and read the actual `Font::memory_face` implementation in the 375,700-byte `src/font.rs`; include independently reviewed expected ranges for the broad font-to-bitmap query.
2. Retrieve eligible code inside the 613,652-byte FFI and 1,032,893-byte C adapter files; disclose unresolved relationships. On the pinned Fontdone snapshot both files are under the current 1 MiB parser limit, so separately retain a >1 MiB parserless fixture and verify useful text retrieval without a false parser-support claim.
3. Deterministic regressions cover Unicode/CRLF offsets, huge declarations, overlap/deduplication, repeated names, identifier components, empty queries, corrupted indexes, changed digests, deletion/rename, excluded paths, bounded search, cancellation and atomic publication.
4. Record cold/warm indexing, retrieval recall against an independent expected-range set, result bytes, disk size and one-file incremental update work. Show unchanged blobs are reused. Keep a TypeScript fixture and a parser-unavailable fixture.
5. Package qualification checks search with no AI connection and no extra server. Codex answer quality is a separate dependent task; index success alone cannot close large-repository generation acceptance.

## Evidence

Partial implementation: native range indexing/search/read commands, structural lookup and workspace search wiring are implemented. See the [interface and limits](../architecture/repository-retrieval.md) and [Fontdone measurements](../acceptance/2026-10-17-range-retrieval.md). Captured text from the three previously problematic large files is retrievable. The 12-span corpus gives 8/12 named global, 12/12 named file-scoped, and 2/12 broad lexical retrieval recall. These observations do not pass broad repository understanding.

Remaining acceptance work:

- Structural definition/relationship/module-map native commands are implemented; qualify their full cancellation/corruption and packaged invocation boundaries. Static relationship lookup is not a precise reference resolver. See the [structural checkpoint](../acceptance/2026-10-17-structural-context.md).
- Broader bounded substring/regex fallback, deterministic tie qualification at the candidate cap, and full mid-operation cancellation/corruption boundaries. Ranked and literal search widen their bounded candidate prefixes when cross-source overlap consumes the unique-page sentinel; the focused overlap-pagination regression passed on 28 October (details below). Global index retention is implemented: the native cache caps aggregate snapshot scopes and bytes, keeps the chunk cache under a separate bound, prunes obsolete generations, and preserves active readers with leases. Existing deterministic regressions cover global byte eviction, pinned scopes, generation cleanup, and failed publication in `src-tauri/src/repository_context/cache/tests.rs`; their execution on the current revision and packaged storage-pressure behavior remain unverified.
- Deduplicated overlapping evidence reads and aggregate source/structural/tool-call budgets are implemented; an installed-account synthetic has exercised the owned Codex tool path. Full exhaustion/cancellation/corruption matrices and Fontdone answer-quality qualification remain with dependent `gk46`; see the [agent evidence accounting checkpoint](#agent-evidence-accounting-checkpoint--17-october).
- The current-source Fontdone one-file update profile now passes: 1,620 unchanged files and 3,409 unchanged Tantivy documents are reused; 53 changed-file documents are replaced; incremental preparation takes 3,217 ms of build work versus 13,154 ms for a fresh build. All eligible source is still verified. Both generations are normalized to one Tantivy segment so broad BM25 ordering matches the fresh build. See the [21 October profile](../acceptance/2026-10-17-range-retrieval.md#current-source-one-file-incremental-update--21-october-2026). Further file-change shapes and packaged boundaries remain open.
- A full Fontdone parser index is 41,616,863 bytes, above the former 32 MiB persistence bound. The per-snapshot cap is now 64 MiB and has an exact-limit/next-byte regression. Reassess peak memory and total on-disk use under concurrent/active readers as part of the remaining storage-pressure acceptance.
- Current-source native search indexes both pinned large FFI/C files and finds all twelve reviewed named ranges globally and file-scoped. File discovery over eight pages finds all seven expected paths; scoped retrieval returns 12/12. Global broad first-page recall remains 1/12 for each query. The installed UI is now spot-checked separately; these local/native range metrics do not qualify agent selection or answer quality. See the [large-file profile](../acceptance/2026-10-17-range-retrieval.md#current-source-large-file-parser-and-search-profile--21-october-2026) and [packaged observation](../acceptance/2026-10-22-packaged-local-search.md).
- An installed-app local-search spot check now finds and opens `src/font.rs:4053–4115` from the large-file snapshot without manual attachment. The broad global query still misses that declaration on its first page. See the [packaged observation](../acceptance/2026-10-22-packaged-local-search.md). The complete cancellation/corruption, retention, parserless-over-1-MiB, and broad automatic-retrieval matrix remains open; Codex answer quality is tracked separately in dependent `gk46` and parent `0wdn`.

Kata `a699`, parent `0wdn` and dependent `gk46` remain open. The [earlier broad explanation failure](../acceptance/2026-10-17-fontdone.md) is not resolved by this search checkpoint.

## Overlap pagination regression run — 28 October 2026

`RUSTC_WRAPPER= cargo test --locked --manifest-path src-tauri/Cargo.toml --lib overlapping_ranked_and_literal_windows_expand_until_a_page_sentinel_or_cap` passed (1 passed, 837 filtered out). Compilation emitted the existing Claude dead-code warnings. This confirms the local regression for widening ranked/literal candidate prefixes until a unique-page sentinel or cap; it does not qualify candidate tie ordering at the cap, broader fallback recall, cancellation/corruption, packaged storage pressure, or Fontdone answer quality.

## Expanded industry review — 17 October

The updated [research](../research/2026-10-17-repository-context-and-indexing.md) distinguishes persistent ranked text search, exact regex/substring search, syntax metadata and precise language intelligence. Cursor's local regex indexing, Sourcegraph's search/navigation loop and Aider's compact map reinforce the need for several bounded operations; a vector database is not a prerequisite.

Retain the existing Tantivy/ripgrep/Tree-sitter foundation. Compare scoped/query-refined and relationship-guided retrieval against the frozen twelve-span Fontdone set before choosing another engine. If repeated scans become a measured bottleneck, evaluate existing Zoekt tooling rather than building a custom trigram index. Preserve one local context contract so a later qualified LSP/SCIP adapter can add precise links without replacing captured-text fallback. No new backend, runtime acceptance or task closure is established by this review.

## Agent evidence accounting checkpoint — 17 October

The new native ledger implements overlapping-range deduplication plus aggregate 12-file, 250,000-code-byte, metadata/wire/call/fragment/deadline bounds. It reserves possible delivery durably, rechecks the versioned grant, and rejects final quotes outside delivered code. Correctable eligible-path read errors carry bounded metadata and no source; unknown paths/scope changes/uncertain delivery remain terminal. A real installed-account synthetic has used this service through five owned Codex MCP tools. See [scope and executed evidence](../acceptance/2026-10-17-codex-owned-retrieval.md).

This supersedes the earlier statement that no agent path exists in source. It does not close the task: full exhaustion/cancellation/corruption matrices, efficient incremental index updates, disk retention, fallback/search limits, packaged offline search and the Fontdone agent-quality gates remain pending. `gk46` and the parent remain open.

## Current-source Fontdone retrieval rerun — 24 October 2026

The ignored native service profile was rerun at Code Atlas HEAD `2fe8b16` against Fontdone `main@5249b72371`. It used a temporary retrieval-cache root and the existing immutable snapshot, object store and parser index. The isolated result indexed 1,621 files / 19,296,074 source bytes into 3,462 ranges (153 syntax-indexed files, 1,468 text-fallback files); 1,147 files and 25 snapshot paths remain unavailable/skipped. Initial index preparation plus query took 9,386 ms; a warm search took 179 ms.

For `font bytes rendered glyph bitmap entry parsing outline hinting rasterization`, the first 32-file page contained 6/7 expected implementation paths; eight file pages found all 7, and path-scoped searches recovered all 12 independently selected ranges. For `font buffer glyph image format outline hinting pixel coverage`, the first page contained 4/7 paths; eight pages again found all 7 and scoped searches recovered 12/12 ranges. Global range search still found only 1/12 expected ranges on the first page and 8/12 or 7/12 inside the 256-candidate window. Exhausting the range window took 17,504 ms and 12,659 ms; file discovery took 2,804 ms and 1,312 ms. The named declaration queries found 12/12 globally and file-scoped.

An initial isolated run omitted the parser index and reported text fallback only; it was discarded. The accepted measurements above used the existing parser index and a separate writable retrieval cache. They confirm the staged local-search path, while the weak global first page, cancellation/corruption and retention matrices, packaged answer path and semantic answer quality remain open. No provider request was made. Profile output remains in temporary storage; no Fontdone source or provider response was copied into this repository. Keep `a699`, `0wdn`, and `gk46` open.

## Automatic file-first discovery for broad agent queries — 25 October 2026

`search_code` now defaults to `auto`. An unscoped natural-language query with
at least three words discovers a bounded file shortlist first; short phrases,
paths, identifiers, code syntax, and scoped queries retain range search. The
agent can still explicitly choose `files` or `ranges`. This makes the measured
file-first workflow the tool default instead of relying on the agent to set
`resultKind` correctly. The search remains lexical and discovery paths remain
hints, not evidence; claims still require exact `read_code` references.

Focused regressions verify default selection, explicit overrides, path scope,
symbol-like queries and the advertised schema. The end-to-end MCP fixture also
exposed that the index computed `queryCoverage`, but the evidence ledger had
stripped it from the agent response. The bounded matched terms and total are
now preserved for file results; range results omit this optional metadata.
The 12-span Fontdone broad range recall has not been rerun through the agent
with this default, so no retrieval-recall or explanation-quality improvement
is claimed. Keep `a699`, `0wdn`, and `gk46` open.

### Post-change service profile — 25 October 2026

The frozen local service benchmark was rerun at commit `34d67b9` against an
isolated copy of the Fontdone snapshot, objects and indexes. Across both broad
queries, eight-page file discovery found all 7/7 expected paths and subsequent
path-scoped reads found 12/12 expected ranges. The first discovery page found
6/7 and 4/7 paths; a single broad range page still found 1/12 expected ranges.
This confirms the retrieval primitives after the default change but does not
exercise the MCP `auto` dispatch or an agent's tool choice. See the detailed
[current-source profile](../acceptance/2026-10-17-range-retrieval.md).
Keep the agent orchestration, full retrieval recall and answer-quality gates
open.

## Scope-bounded file discovery — 27 October 2026

File discovery queries now apply the indexed exact `scope` term before
collecting matching body and declaration documents. The in-memory path-hint
pass also skips files outside the requested file or folder scope. This avoids
scanning and then discarding repository-wide matches for a request that is
already scoped; final path checks remain in place. This is a source-level
retrieval improvement only: no tests, build, large-repository profile, or
packaged qualification ran. Keep `a699`, `0wdn`, and `gk46` open.

## Declaration-frequency ranking and source excerpt correction — 27 October

Broad file discovery now weights each declaration hint by the number of
distinct in-scope files that declare it. A declaration unique to a few files
can therefore outrank a generic body coincidence, while a returned evidence
range still has to match captured source text. File result previews now use the
first literal matching line when available, and recheck cancellation after
searching the selected range. Static review also restored the ripgrep UTF-8
sink's explicit stop result; without it, the callback did not satisfy the
searcher's result contract. No tests, build, retrieval profile, or packaged
qualification ran, so no recall improvement is claimed. Keep `a699`, `0wdn`,
and `gk46` open.

## Distinctive-term weighting in range results — 27 October 2026

Natural-language range search now weights each matched query term by its
document frequency in the scoped range corpus, matching the information-value
signal already used by file discovery. Exact identifier and declaration
matches retain precedence; weighted distinct-term coverage then orders broad
range candidates, with term count and the existing BM25 score as tie-breakers.
The bounded per-term candidate window and result budget are unchanged. This is
a source-level ranking change only: no tests, build, retrieval profile,
Fontdone recall measurement, or packaged qualification ran. Do not claim
improved recall until the frozen expected-range corpus is rerun. Keep `a699`,
`0wdn`, and `gk46` open.

## Full bounded term coverage before range ranking — 27 October 2026

Natural-language range search now considers the complete per-term match set
from the local index before applying distinct-term coverage ranking. Each
per-term TopDocs collection is bounded by the existing 50,000-range index cap;
the 256-candidate window, 32-result page, source-byte budget and exact
snapshot/hash checks remain unchanged. This removes the earlier per-term BM25
cutoff that could discard a lower-scoring match before its distinctive-term
coverage was considered. Source-only change; no tests, build, retrieval profile
or packaged qualification ran. The Fontdone recall and other acceptance gates
remain open; do not claim improved recall until the frozen expected-range
corpus is rerun.

## Reuse verified files during identifier fallback — 27 October 2026

Identifier literal fallback now recognizes parserless and partially parsed
declarations from the same captured-file read and SHA-256 verification used
for literal matches. It no longer reopens and rehashes those files in a second
pass; declaration matching still scans the already verified in-memory text.
The existing 100 MiB verified-source cap is reserved before each file read,
so a final file cannot push a scan past the cap. Range sizes, search-result
limits, the 250,000-byte model-operation budget, snapshot/hash validation,
secret filtering and cancellation checks are unchanged. Source-only change;
no tests, build, profile or packaged qualification ran. Broad recall and the
remaining acceptance gates stay open; do not claim a recall improvement.

## Preserve a capped index when one file exceeds the range budget — 28 October 2026

Index preparation no longer aborts the complete generation when adding one
file would exceed the 50,000-range cap. It leaves that file in the inventory
with reason `localRangeLimit`, omits its ranges, and continues so later files
that fit can still be indexed. An incremental base's postings for an omitted
file are removed by the existing cleanup pass. The cap is unchanged. This is
a source-only correction; no tests, build, profile, or packaged qualification
ran. Keep broad recall and the remaining acceptance gates open.

## Expose bounded index-limit omissions — 28 October 2026

Search responses now include full counts for the `localRangeLimit` and
`localSourceByteLimit` reasons, plus up to eight safe affected paths with their
reason. Range-limited paths are listed first and `filesTruncated` indicates
when more path details remain. The owned Codex search response and workspace
search command both expose this summary. Excluded and secret-filtered paths
remain aggregate-only. This response-only change does not alter the persisted
index schema or raise the search or source budgets. No tests, build, profile,
or packaged qualification ran; the retrieval acceptance gates remain open.

## Overlap pagination regression — 28 October 2026

The existing regression for ranked/literal candidate overlap and page-sentinel
expansion now passes on `8da4add3bdbc1e9f71eb0103b19d5e5cb063eb2c`:

```text
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib overlapping_ranked_and_literal_windows_expand_until_a_page_sentinel_or_cap
1 passed; 0 failed; 836 filtered out
```

This verifies that a page continues widening bounded candidate prefixes when
overlap leaves no unique next-page sentinel, and stops at the configured cap.
It is deterministic unit evidence only. Cancellation/corruption boundaries,
packaged storage pressure, broad Fontdone recall, and agent answer quality
remain unqualified; keep `a699`, `0wdn`, and `gk46` open.

The cache-retention suite also passed on the same source revision:

```text
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib repository_context::cache::tests
16 passed; 0 failed; 1 ignored; 820 filtered out
```

This executes deterministic LRU/byte-quota eviction, reader pinning, old
generation leases, bounded maintenance, symlink rejection, interrupted cleanup,
and refused-publication preservation. The ignored entry is the cross-process
lease helper, which is invoked by the passing disposable lease test. These are
local native tests; packaged disk-pressure behavior and the broader retrieval
gates remain open.

### Current Fontdone profile using the owned file-page size — 28 October 2026

The current ignored local-service profile ran against the isolated saved
Fontdone `main@5249b72371` snapshot with its V6 syntax index. It reports 1,621
indexed files, 3,462 ranges, 153 syntax files and 1,468 text-fallback files.
Named expected ranges are 12/12 globally and scoped. Broad first range pages
find 1/12, and the 256-result windows find 6/12 for each query. Eight-result
file pages find 4/7 expected paths on their first page; 32 pages followed by
scoped range search find 7/7 paths and 12/12 ranges for each query. Discovery
takes 6,066 ms / 4,541 ms and serializes 160,166 / 160,287 bytes over all
bounded responses. `memory_face` first search including index preparation is
2,617 ms and the warm search is 580 ms.

This direct local-service profile makes no model request and does not measure an
agent's pagination decisions, packaged behavior, or explanation quality. An
earlier attempt without the V6 syntax index was discarded. Details and the
limits are in [the acceptance record](../acceptance/2026-10-17-range-retrieval.md#current-source-fontdone-profile-with-the-owned-file-page-cap--28-october-2026).
Keep `a699`, `0wdn`, and `gk46` open.

The subsequent native-service profile requested 32 results directly and
returned all seven independently expected paths on its first file-discovery
page for both queries, with 12/12 expected ranges recoverable by scoped service
searches. The owned Codex wrapper still caps file-discovery replies at eight
paths; this service-level measurement does not improve broad range first-page
recall (still 1/12) or prove automatic agent behavior. It remains an
un-packaged local-service measurement.

## Symbol-aware ranking checkpoint — 29 October 2026

Broad local range search now gives separate weight to terms found in indexed
declaration names and terms found only in source bodies. In a focused regression,
a declaration matching the requested behavior outranks an unrelated body that
contains every query term. The focused test passed (1 passed; 843 filtered
out), and the ignored Fontdone service profile passed against an isolated copy
of `main@5249b72371` with its V6 syntax index.

First-page broad-query recall improved from 1/12 to 5/12 for each of the two
queries. Recall across the bounded 256-result window improved from 6/12 to
10/12 for the first query and 7/12 for the second. Named identifier lookup
remains 12/12, and discovery followed by file-scoped search still recovers
12/12. The ranking does not yet make a broad first page complete. It also does
not show that Codex automatically performs discovery and scoped retrieval or
that explanations are accurate. See the dated measurement and exact command in
the [acceptance record](../acceptance/2026-10-17-range-retrieval.md#symbol-aware-range-ranking-on-fontdone--29-october-2026).

Keep `a699`, `0wdn`, and `gk46` open; the remaining source, packaging, bounds,
cancellation, parserless-file and agent-quality acceptance has not passed.

## Packaged local memory-face lookup — 29 October 2026

The rebuilt native app was checked on the existing Fontdone
`main@5249b72371` snapshot. Local Search for `memory_face` reported 79 matches
across 22 files and explicitly marked coverage incomplete with more matches
available. It found the actual `src/font.rs` implementation at lines
4053–4115; opening that hit displayed the captured function body and its
format-dispatch branches in the source pane. The selected source returned to
`src/tt/glyf.rs` afterward.

This was a packaged, native local-search interaction; no extra server was
started and no Codex, speech, or model request was made. The Codex account was
connected, so a disconnected-account run is still outstanding. The partial
snapshot and incomplete search label were retained; this is not exhaustive
repository coverage or evidence for broad-query recall, cancellation, index
corruption, parserless fixtures, or answer quality. Keep `a699`, `0wdn`, and
`gk46` open.

## Align owned-agent file discovery with the measured native page — 29 October 2026

The owned Codex `search_code` tool now permits up to 32 file-hint results per
page, matching the local search service. Previously the wrapper returned at
most eight paths even though the isolated Fontdone service profile showed all
seven reviewed expected paths on the first 32-path page. File hints contain no
source code; the existing source and wire budgets are unchanged. A focused MCP
catalog regression passed after the cap change (1 passed; 843 filtered out).

This removes the wrapper's artificial eight-path truncation. It does not prove
Codex will use every hint, read all required ranges, or produce an accurate
repository explanation. The corresponding Fontdone agent run and remaining
large-file, cancellation, corruption, offline, and packaging gates remain
open under `a699` and `gk46`.

## Interruptible incremental segment copies — 29 October 2026

Incremental Tantivy segments are now copied in 64 KiB chunks with a budget and
cancellation check between chunks. Copying opens only regular source files
without following Unix symlinks, creates new staging files without overwriting
unexpected entries, and rejects a source that shrinks or grows while it is
being copied. A failed or cancelled copy returns through the existing staging
guard, which removes the partial generation before publication.

The focused native incremental-index tests pass: 9 passed, 0 failed, 1 ignored
(`fontdone_one_file_incremental_profile`), 837 filtered out. New regressions
prove cancellation after the first bounded chunk and reject short/extended
segment streams. Changed Rust files pass rustfmt. This is source and focused
unit evidence only; packaged storage pressure, real mid-copy cancellation,
independent ranking review, Fontdone broad recall, and Codex answer quality
remain unqualified. Keep `a699`, `0wdn`, and `gk46` open.

## Current-source Fontdone incremental profile — 29 October 2026

Ran the opt-in `fontdone_one_file_incremental_profile` once against a fresh,
marked disposable copy of Fontdone `main@5249b72371`. Only the selected snapshot,
its 2,762 non-secret captured objects, and its parser index were copied from app
storage; the original snapshot and saved explanations were not modified. The
profile appended a 66-byte comment to copied `src/font.rs`, then compared the
incremental index with a fresh reference build. No model or speech request ran.

The incremental build copied 5,295,592 segment bytes in 4.234 s versus 14.157 s
for the fresh reference. It reused 1,620 files and 3,409 documents, added and
deleted 53 chunks, and produced the same 3,462-document, single-segment index
as the fresh build. The previous generation's hashes remained unchanged. All
12 named ranges were found globally and file-scoped. Each broad query found
5/12 expected ranges on its first page, matching the fresh build; this is better
than the 1/12 observation in the 25 October run, but still leaves seven spans
unretrieved. The run is one local observation without a variance estimate and
does not qualify Codex's tool choices or answer quality. Exact metrics and
limits: [profile artifact](../acceptance/artifacts/2026-10-29-fontdone-incremental-profile.json).

## Ignore one- and two-character tokens in broad-query coverage — 29 October 2026

Natural-language query coverage now excludes tokens shorter than three bytes,
matching the existing file-discovery candidate filter. This keeps common
single-letter words from consuming a coverage bit or affecting relevance
weights. Code-like and scoped searches still retain their exact terms. The
focused `broad_question_coverage_ignores_prompt_scaffolding` native regression
passes (1 passed; 846 filtered out). This narrows broad-query noise; it does
not change or qualify the preserved Fontdone recall measurements. Keep `a699`,
`0wdn`, and `gk46` open.

## Prefer multi-term evidence in broad range ranking — 29 October 2026

Targeted triage of the existing repository-context suite exposed a real
inconsistency: file discovery ranks distinct query-term coverage before inverse
document frequency, but range retrieval did the reverse. A range matching only
one rare word could therefore outrank a range matching four distinct, common
terms. Natural-query range ordering now uses distinct body-term coverage
before its summed IDF score when declaration signals tie. The ranking fixture
also now supplies the required `end_byte` sort value, so its tie-cap tests
exercise valid index documents instead of failing on malformed fixtures.

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib
repository_context:: -- --quiet` passes 75 tests, with 3 ignored. This covers
the targeted range-ranking regression and the surrounding repository-context
tests; it is not a rerun of the full native suite. The recorded Fontdone
first-page recall measurements are unchanged and need a fresh local profile to
show whether this ranking change improves them. Agent selection, packaged
search, and explanation quality remain unqualified. Keep `a699`, `0wdn`, and
`gk46` open.

The 29 October current-source Fontdone profile has now been rerun once. The
first 32-path discovery page contains all 7/7 reviewed paths for both broad
queries, and discovery followed by scoped range search recovers 12/12 expected
ranges. Single broad-range first-page recall remains 5/12 for each query, and
the 256-result windows remain 10/12 and 7/12. Thus the distinct-term-first
comparator fixes the synthetic rare-term case but does not further improve this
Fontdone corpus. The concrete retrieval strategy for broad questions is
file discovery followed by bounded scoped searches; automatic agent use and
answer quality still need independent qualification. Exact profile details:
[current-source recheck](../acceptance/2026-10-17-range-retrieval.md#recheck-after-distinct-term-first-range-ordering--29-october-2026). Keep
`a699`, `0wdn`, and `gk46` open.

## Full bounded file-discovery window — 30 October 2026

Automatic source discovery can now consume all 256 unique file candidates in
one local ranked-search operation while the user-facing search endpoint keeps
its 32-result page cap. Truncation at the internal candidate ceiling remains
visible as a partial result. Focused `source_discovery` regressions passed for
a relevant large implementation behind 36 higher-ranked files and for the
partial result at the 256-path ceiling (4 passed, 854 filtered out). This does
not change direct range-search recall or qualify the agent's subsequent range
selection; the Fontdone profile and answer-quality gates remain open. See
[large source discovery](66-large-source-discovery.md).
