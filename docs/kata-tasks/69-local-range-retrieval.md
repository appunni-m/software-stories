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
- Broader bounded substring/regex fallback, deterministic tie qualification at the candidate cap, full mid-operation cancellation/corruption boundaries, and global disk eviction.
- Deduplicated overlapping evidence reads and aggregate source/structural/tool-call budgets are implemented; an installed-account synthetic has exercised the owned Codex tool path. Full exhaustion/cancellation/corruption matrices and Fontdone answer-quality qualification remain with dependent `gk46`; see the [agent evidence accounting checkpoint](#agent-evidence-accounting-checkpoint--17-october).
- The current-source Fontdone one-file update profile now passes: 1,620 unchanged files and 3,409 unchanged Tantivy documents are reused; 53 changed-file documents are replaced; incremental preparation takes 3,217 ms of build work versus 13,154 ms for a fresh build. All eligible source is still verified. Both generations are normalized to one Tantivy segment so broad BM25 ordering matches the fresh build. See the [21 October profile](../acceptance/2026-10-17-range-retrieval.md#current-source-one-file-incremental-update--21-october-2026). Further file-change shapes and packaged boundaries remain open.
- A full Fontdone parser index is 41,616,863 bytes, above the former 32 MiB persistence bound. The per-snapshot cap is now 64 MiB and has an exact-limit/next-byte regression. Reassess memory and global disk-retention behavior as part of the remaining storage-pressure acceptance.
- Current-source native search indexes both pinned large FFI/C files and finds all twelve reviewed named ranges globally and file-scoped. File discovery over eight pages finds all seven expected paths; scoped retrieval returns 12/12. Global broad first-page recall remains 1/12 for each query. The installed UI is now spot-checked separately; these local/native range metrics do not qualify agent selection or answer quality. See the [large-file profile](../acceptance/2026-10-17-range-retrieval.md#current-source-large-file-parser-and-search-profile--21-october-2026) and [packaged observation](../acceptance/2026-10-22-packaged-local-search.md).
- An installed-app local-search spot check now finds and opens `src/font.rs:4053–4115` from the large-file snapshot without manual attachment. The broad global query still misses that declaration on its first page. See the [packaged observation](../acceptance/2026-10-22-packaged-local-search.md). The complete cancellation/corruption, retention, parserless-over-1-MiB, and broad automatic-retrieval matrix remains open; Codex answer quality is tracked separately in dependent `gk46` and parent `0wdn`.

Kata `a699`, parent `0wdn` and dependent `gk46` remain open. The [earlier broad explanation failure](../acceptance/2026-10-17-fontdone.md) is not resolved by this search checkpoint.

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
