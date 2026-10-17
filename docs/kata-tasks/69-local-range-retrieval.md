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
2. Retrieve eligible code inside the 613,652-byte FFI and 1,032,893-byte C adapter files despite the parser cap; disclose unresolved relationships. Unsupported languages have useful text retrieval without a false parser-support claim.
3. Deterministic regressions cover Unicode/CRLF offsets, huge declarations, overlap/deduplication, repeated names, identifier components, empty queries, corrupted indexes, changed digests, deletion/rename, excluded paths, bounded search, cancellation and atomic publication.
4. Record cold/warm indexing, retrieval recall against an independent expected-range set, result bytes, disk size and one-file incremental update work. Show unchanged blobs are reused. Keep a TypeScript fixture and a parser-unavailable fixture.
5. Package qualification checks search with no AI connection and no extra server. Codex answer quality is a separate dependent task; index success alone cannot close large-repository generation acceptance.

## Evidence

Partial implementation: native range indexing/search/read commands and workspace search wiring are implemented. See the [interface and limits](../architecture/repository-retrieval.md) and [Fontdone measurements](../acceptance/2026-10-17-range-retrieval.md). Captured text from the three previously problematic large files is retrievable. The 12-span corpus gives 8/12 named global, 12/12 named file-scoped, and 2/12 broad lexical retrieval recall. These observations do not pass broad repository understanding.

Remaining acceptance work:

- Definition/reference/compact module-map commands and unresolved relationship labels.
- Broader bounded substring/regex fallback, deterministic tie qualification at the candidate cap, full mid-operation cancellation/corruption boundaries, and global disk eviction.
- Deduplicated overlapping evidence reads and tested aggregate source/structural/tool-call budgets before enabling agent tools. No Codex tool path is enabled by this checkpoint.
- Efficient incremental Tantivy updates and a Fontdone one-file update measurement. The current fixture proves chunk-layout reuse, while every new generation still rehashes source and reinserts search documents.
- Actual packaged local search without AI and the complete acceptance matrix. Codex's current whole-file selector is unchanged.

Kata `a699`, parent `0wdn` and dependent `gk46` remain open. The [earlier broad explanation failure](../acceptance/2026-10-17-fontdone.md) is not resolved by this search checkpoint.

## Expanded industry review — 17 October

The updated [research](../research/2026-10-17-repository-context-and-indexing.md) distinguishes persistent ranked text search, exact regex/substring search, syntax metadata and precise language intelligence. Cursor's local regex indexing, Sourcegraph's search/navigation loop and Aider's compact map reinforce the need for several bounded operations; a vector database is not a prerequisite.

Retain the existing Tantivy/ripgrep/Tree-sitter foundation. Compare scoped/query-refined and relationship-guided retrieval against the frozen twelve-span Fontdone set before choosing another engine. If repeated scans become a measured bottleneck, evaluate existing Zoekt tooling rather than building a custom trigram index. Preserve one local context contract so a later qualified LSP/SCIP adapter can add precise links without replacing captured-text fallback. No new backend, runtime acceptance or task closure is established by this review.
