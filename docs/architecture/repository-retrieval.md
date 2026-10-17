# Local repository retrieval

Implementation checkpoint: 17 October 2026. This service powers workspace text search in source. The installed macOS package and Codex generation have **not** been qualified with it. The existing generation selector still sends whole files. See the [measurement report](../acceptance/2026-10-17-range-retrieval.md) and [remaining contract](../kata-tasks/69-local-range-retrieval.md).

## Native interface

| Command | Contract |
| --- | --- |
| `search_repository_code` | Input object: request UUID, active project, captured snapshot, query, optional exact file/directory scope, result limit. Returns ranked range metadata, bounded snippets, coverage, truncation and local timing. |
| `read_repository_range` | Input object: active project, captured snapshot, original path, expected full-file SHA-256, start/end byte offsets. Reads only eligible captured text; returns verified coordinates, range digest and text. |
| `cancel_repository_context` | Cancels a request UUID, including a bounded tombstone when cancellation precedes dispatch. |
| `search_workspace_source` | Adapts the same search service to the existing workspace search UI. Every debounced query has an ID; replacing it or leaving the view cancels that request. |

These commands require an active native project session. Local search does not require an AI connection, permission to transmit source, an API key, a new server or a port. They do **not** authorize an agent to call them. The dependent [agent tool contract](../kata-tasks/70-codex-retrieval-tools.md) must bind its own transmission permission, account/runtime identity and cumulative limits.

## Source and range identity

All reads go through the existing immutable snapshot reader, which verifies actual blob size and SHA-256. The service additionally rejects invalid relative paths, excluded directories, non-file entries, unavailable text and native secret-like matches. A parser limit does not exclude captured text. Missing or corrupt syntax metadata permits text fallback; it does not create a parser-support claim.

Ranges carry snapshot, original path, full-file digest, range digest, absolute UTF-8 byte offsets and one-based inclusive line numbers. End byte offsets are exclusive. UTF-8 boundaries and coordinates are checked again before returning search evidence. Symbol/enclosing-parent context and the saved parser identity are optional. These are static syntax records, not compiler-resolved or observed runtime relationships.

Chunking prefers existing Tree-sitter declaration boundaries in the second half of an 8 KiB window. It otherwise prefers line boundaries, splitting very long lines at UTF-8 boundaries. Ranges partition a file without overlap. Oversized enclosing declarations retain symbol context in the windows they contain. This does not implement the future agent's merging/deduplication of separately requested overlapping reads.

## Search and publication

- Tantivy indexes source tokens, original identifiers, snake/camel/acronym components, path terms and declarations where syntax metadata exists. Exact declarations rank before exact identifier mentions, then lexical candidates, with score/path/byte ordering. Candidate sets are capped; truncation is reported. Equal-score selection beyond the candidate cap still needs boundary qualification.
- The ripgrep project's Rust libraries locate matching snippet lines inside retrieved ranges, including parser-unavailable files. A full repository regex/substr fallback when lexical search is unsuitable is still pending.
- A generation fingerprint binds the complete manifest, saved syntax index, chunking/tokenizer version and schema. A bounded content-addressed chunk-layout cache can reuse unchanged source. A new generation currently rehashes all eligible blobs and reinserts all search documents; this is **not** an incremental Tantivy update implementation.
- A staging directory receives the complete Tantivy commit and metadata. Publication renames it and atomically replaces a small current-generation pointer on the same filesystem. Failed/cancelled work does not replace that pointer. Metadata digests, schema, document count and Tantivy checksums are checked on disk load. The current and previous complete generations are retained during cleanup.
- The native owner serializes search/build operations and retains one prepared index in memory. The cache binds manifest identity, syntax-file and publication-pointer metadata, with a 60-second checksum refresh interval. Evidence blobs are still hash-checked on every search; each distinct file is read once per search. A parser finishing during a build invalidates reuse on the following search. Search errors invalidate the in-memory index.

## Current local bounds

| Resource | Bound |
| --- | --- |
| Captured paths admitted to a build | 25,000 |
| Verified source bytes per build | 100 MiB; existing capture limits also apply |
| Indexed ranges | 50,000, each at most 8,192 bytes |
| Index metadata / one chunk-cache record | 32 MiB / 4 MiB |
| Query | 2–240 characters, up to 32 derived terms; workspace UI currently caps 120 characters |
| Candidate sets | At most 256 each for lexical, exact identifier and exact declaration searches |
| Returned matches | 1–32; snippets at most 360 characters each |
| One explicit range read | At most 32,768 source bytes |
| Request registry / owned queued or running operations | 64 / 8; cancelled jobs retain ownership until return |
| Build/search operation deadline | 90 seconds, checked cooperatively between native operations |
| Tantivy indexing workers / memory arena | One / 30 MB |

The deadline does not preempt a synchronous Tantivy call. `resultBytes` measures serialized match records, including snippets/coordinates, not tokens or total wire framing. The cached build counters describe the originating index build, not new work performed on each warm search. Total request timing in the acceptance probe includes manifest/cache preparation; `elapsedMs` in a search result describes search/read work after preparation.

The current 250,000-byte model-operation source limit is unchanged. No model retrieval tool is enabled here. Aggregate source/structural-context accounting, overlapping-read merging, tool-call limits and per-generation deadlines must pass before the agent path is enabled. Global disk retention/eviction and complete cache-boundary corruption/cancellation qualification remain open; disposable cache files must not be confused with saved explanations.

## Dependencies and qualification

Direct versions are locked in Cargo:

| Dependency | Declared licence / compatibility | Use |
| --- | --- | --- |
| Tantivy 0.26.2 | MIT; declared Rust minimum 1.86 | Embedded lexical index; default features off, `mmap` and `lz4-compression` enabled. |
| grep-regex 0.1.14 | Unlicense OR MIT; minimum Rust not declared in inspected metadata | Bounded literal snippet matching. |
| grep-searcher 0.1.17 | Unlicense OR MIT; minimum Rust not declared in inspected metadata | Search captured range bytes in-process. |

Versions/licences were checked against their downloaded Cargo package metadata. This is not a transitive licence audit or minimum-toolchain qualification. Current development checks use rustc 1.98.1 on this Mac. The implementation adds no external search executable or Node process. Native tests and `cargo check` compile these dependencies on macOS; a newly packaged application still needs qualification.

Run deterministic retrieval tests with:

```sh
env RUSTC_WRAPPER= cargo test --manifest-path src-tauri/Cargo.toml --lib repository_context
```

The [Fontdone report](../acceptance/2026-10-17-range-retrieval.md) gives explicit local measurement commands and the frozen source-range set. Passing these tests does not establish broad-question recall or a completed explanation.
