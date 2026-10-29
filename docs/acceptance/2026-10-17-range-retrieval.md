# Fontdone local retrieval checkpoint

Date: 17 October 2026. Source changes after `cc3d4cc`; source-file hashes and machine-readable observations are in the [metrics artifact](artifacts/2026-10-17-fontdone-retrieval-metrics.json). This is a native Rust service measurement, not an installed-package or Codex generation acceptance run. The [earlier broad-answer failure](2026-10-17-fontdone.md) remains unresolved.

## Frozen evidence set

- Fontdone commit: `5249b723719445813b8771a602ea5b96891d4450`.
- Existing captured snapshot: `snap-1792140879637485000-11155-0`.
- [Twelve expected declaration spans](fixtures/fontdone-retrieval-ranges.json) were selected by reading the pinned code: face entry/format selection, glyph dispatch/loading, hinting, rendering, FFI and C ABI entry points. File hashes were checked against the captured manifest; coordinates came from the independently selected source lines. Leading indentation is excluded because a syntax range can start at its first declaration token.
- This is a principal-journey retrieval set. It is not exhaustive coverage of supported font formats, rendering branches or implementation correctness. Retrieval output did not generate the expected set.

## Results

One cold-version-cache debug run on this Mac, without a concurrent build or an AI request:

| Measurement | Observation |
| --- | --- |
| Eligible text indexed | 1,621 files, 19,296,074 bytes, 3,422 ranges |
| Syntax / text fallback files | 81 / 1,540 |
| Captured files unavailable to this service | 1,141 unavailable text entries plus six secret-like entries; 25 additional paths were skipped during capture |
| Cold `prepare` call | 17,428 ms; internal build counter 16,120 ms |
| Content layouts calculated / reused in cold run | 1,597 / 24; identical eligible blobs can share a layout even in a clean cache |
| Disk-loaded `prepare` without search | 1,293 ms |
| Native service disk load + `memory_face` search | 1,903 ms |
| Same query with in-memory prepared index | 256 ms, 20,543 serialized match bytes, first hit `src/font.rs:4053` |
| Named query recall, repository scope, top 32 | **8 / 12** expected spans |
| Named query recall, expected file scope, top 32 | **12 / 12** expected spans |
| Broad lexical query recall, top 32 | **2 / 12** for each of the two queries below |

The two broad queries were `font bytes rendered glyph bitmap entry parsing outline hinting rasterization` and `font buffer glyph image format outline hinting pixel coverage`. They returned the expected glyph-dispatch and render-dispatch spans, missing ten expected spans. These are explicit lexical search inputs, not the original full explanation prompt or an agent's search plan. Their request times were 251 ms and 220 ms; match payloads were 20,731 and 20,280 bytes. All searches reported truncation when more than 32 candidates matched.

At this 17 October source revision, named global searches missed the four FFI/C adapter spans. File-scoped searches found all four in the 613,652-byte `src/ffi/handles.rs` and 1,032,893-byte `fontdone-c-abi/src/implementation.rs`; both were text fallback, not parser-indexed. The current 21 October parser now indexes these files, as recorded below. Finding source text still does not prove a resolved definition/reference. The 375,700-byte `src/font.rs` contributes a bounded 6,152-byte range beginning at the actual `memory_face` declaration.

These observations establish local availability of important previously excluded text. **They fail a broad repository retrieval sufficiency claim.** The service test asserts the file-scoped expectations, records global/broad misses, and does not assert that a broad explanation is ready. There was no new Codex request, narrative/visual generation, narration test or packaged-app walkthrough.

Cold means no compatible versioned index/layout cache; the OS page cache was not cleared. Cache disk sizes and unavailable-file reason counts are recorded in the metrics artifact. A previous compatible cache was preserved separately before the initial cold check.

## Deterministic checks

The default test suite checks UTF-8/CRLF coordinates, huge declarations and long lines, exact identifier/component ranking, repeatable ordering, snapshot isolation, deletion/rename, chunk-layout reuse on a one-file TypeScript change, secret/path/invalid-text filtering, source/hash changes, corrupted metadata/Tantivy recovery, read/query caps, expired deadlines, cancellation ownership and preservation of the last complete publication.

The one-file fixture reuses unchanged layouts and Tantivy postings while still verifying all eligible source. The 21 October current-source Fontdone update profile is recorded below. Global cache retention, full boundary/cancellation qualification, definition/reference/module-map commands, and agent evidence-budget enforcement remain pending. The [Kata contract](../kata-tasks/69-local-range-retrieval.md) stays open.

Automated checkpoint: 224 native tests passed, six default tests remained ignored; both explicit Fontdone probes passed separately. All 136 frontend tests, the frontend build and `cargo check` passed. The documentation audit found zero errors and four existing review notes. None of these checks closes the broad retrieval, generation or package gates.

## Reproduce

Use the existing saved public Fontdone snapshot. This does not execute Fontdone or contact a model. The cold run needs an empty **disposable retrieval cache**, not deleted snapshots or chapters. The recorded run preserved its previous cache separately before measurement.

```sh
env RUSTC_WRAPPER= \
  CODE_ATLAS_FONTDONE_DATA_DIR="$HOME/Library/Application Support/com.codeatlas.desktop" \
  cargo test --manifest-path src-tauri/Cargo.toml --lib \
  repository_context::tests::fontdone_local_index_profile -- --ignored --exact --nocapture

env RUSTC_WRAPPER= \
  CODE_ATLAS_FONTDONE_DATA_DIR="$HOME/Library/Application Support/com.codeatlas.desktop" \
  cargo test --manifest-path src-tauri/Cargo.toml --lib \
  repository_context_commands::tests::fontdone_service_retrieval_profile -- --ignored --exact --nocapture
```

The probe logs metadata, coordinates, counts and timings, excluding source snippets, account credentials and reasoning. Measurements are single local debug observations, not performance guarantees or a product comparison. The test snapshot/project identity is intentionally frozen; other snapshots require a separately reviewed expected set.

## Current-source rerun — 19 October 2026

Reran the service profile twice from Code Atlas source `e161110` against the same immutable Fontdone snapshot and independently selected twelve-span fixture. Both runs used an isolated disposable storage copy containing only that snapshot's manifest, captured objects, syntax indexes and retrieval cache (81,279,554 bytes); the app's original storage was not modified. No model or speech request was made. The test binary completed both runs successfully.

| Measurement | Run 1 | Run 2 |
| --- | ---: | ---: |
| First disk-load + `memory_face` search | 8,260 ms | 8,551 ms |
| Same-process cached `memory_face` search | 387 ms | 347 ms |
| Serialized `memory_face` match result | 18,121 bytes | 18,121 bytes |
| Retrieval cache after run | — | 11,946,480 bytes across 27 files |
| Syntax index copied into the isolated store | — | 37,411,046 bytes |

Coverage remained 1,621 indexed files, 19,296,074 indexed source bytes, 3,422 ranges, 81 syntax-parsed files, 1,540 text-fallback files and 1,147 unavailable files (including the six secret-like entries). The `memory_face` declaration remains the top result at `src/font.rs:4053`.

All twelve named expected declarations were found globally and file-scoped, improving on the older 8/12 global observation. The two broad lexical queries found only 1/12 and 2/12 spans, respectively, and both hit the bounded result limit. This is useful exact-name retrieval, but broad repository discovery remains insufficient. It does not qualify packaged search, Codex answers, or narrative quality.

Run logs are `/private/tmp/code-atlas-fontdone-service-profile-e161110.log` and `/private/tmp/code-atlas-fontdone-service-profile-e161110-disk.log`. The disposable storage and temporarily restored build fixture were removed after the run; the user's deleted fixture remains deleted.

## Current-source rerun — 20 October 2026

Reran the same frozen profile on committed search source `e7bcc8a` using an isolated 273 MB copy of the app-data `snapshots`, `objects`, `indexes`, and `repository-context` directories. The original app data was left untouched. The copy contains the saved Fontdone snapshot and its existing prepared indexes, so this measures disk-loaded and same-process cached searches, not a cold index build. No Codex, model, or speech request was made.

Environment: Apple M3 Pro, arm64, macOS 15.7.7, Rust/Cargo 1.98.1. One profile run, no separately warmed repetitions or variance estimate. Command:

```sh
RUSTC_WRAPPER= \
CODE_ATLAS_FONTDONE_DATA_DIR=/private/tmp/code-atlas-a699-current \
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib \
  repository_context_commands::tests::fontdone_service_retrieval_profile -- --ignored --exact --nocapture
```

| Measurement | Observation |
| --- | ---: |
| Indexed files / bytes / ranges | 1,621 / 19,296,074 / 3,438 |
| Syntax / text-fallback files | 83 / 1,538 |
| Unavailable files / snapshot paths skipped | 1,147 / 25 |
| First `memory_face` disk-load + search | 1,731 ms |
| Same-process cached `memory_face` search | 203 ms |
| Named identifier recall, global | **12 / 12** |
| Broad query 1, first ranked page; all ranked pages | **1/12; 12/12** |
| Broad query 1, paged file discovery; scoped range recall | **7/7 files; 12/12 ranges** |
| Broad query 2, first ranked page; all ranked pages | **2/12; 7/12** |
| Broad query 2, paged file discovery; scoped range recall | **7/7 files; 12/12 ranges** |

Both broad file-discovery scans visited eight 32-result pages and found all seven expected files. The ranked range search still missed five expected spans for the second broad query even after paging; a follow-up search scoped to the discovered files recovered all twelve. This demonstrates a useful local discovery-then-scope route, but the weak first-page ranking and need for iterative calls mean that broad question-to-code retrieval is not yet automatic or proven sufficient. Keep the related retrieval and guide-quality issues open.

## Current-source rerun — 21 October 2026

Ran the frozen service profile twice at source `39d1fd73a081047f16d7c2d5e0aad59fcafa427c` against an isolated 273 MB copy of the saved Fontdone snapshot and its prepared indexes. The user's original app data was not modified. No model, Codex, or speech request was made during either measurement.

Environment: Apple M3 Pro, arm64, macOS 15.7.7, Rust/Cargo 1.98.1. Both runs used copied prepared indexes, so these are disk-loaded and same-process cached searches, not cold index-build measurements. Timings below are observed ranges across two runs, not a variance estimate. Command:

```sh
RUSTC_WRAPPER= \
CODE_ATLAS_FONTDONE_DATA_DIR=/private/tmp/code-atlas-a699-recall-20261021 \
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib \
  repository_context_commands::tests::fontdone_service_retrieval_profile -- --ignored --exact --nocapture
```

| Measurement | Observation |
| --- | ---: |
| Indexed files / bytes / ranges | 1,621 / 19,296,074 / 3,438 |
| Syntax / text-fallback files | 83 / 1,538 |
| Unavailable files / snapshot paths skipped | 1,147 / 25 |
| `memory_face` disk-load + search / same-process search | 1,787–1,893 ms / 213–222 ms |
| Serialized `memory_face` result | 23,279 bytes; top hit `src/font.rs:4053` |
| Named identifier recall, global | **12 / 12** |
| Broad query 1, first ranked page; all ranked pages | **1/12; 12/12** |
| Broad query 1, file-discovery first page; scoped recall on those paths | **5/7 paths; 9/12 ranges** |
| Broad query 1, all file-discovery pages; scoped recall | **7/7 paths; 12/12 ranges** |
| Broad query 2, first ranked page; all ranked pages | **2/12; 7/12** |
| Broad query 2, file-discovery first page; scoped recall on those paths | **4/7 paths; 8/12 ranges** |
| Broad query 2, all file-discovery pages; scoped recall | **7/7 paths; 12/12 ranges** |

Each file-discovery scan used eight 32-result pages. Scoping the expected files from all pages recovered 12/12 ranges for both broad queries. For query 1, all ranked pages took 9,351–9,611 ms and serialized 166,094 result bytes; discovery took 1,195–1,227 ms and serialized 143,546 bytes. For query 2, all ranked pages took 9,038–9,116 ms and serialized 157,869 result bytes; discovery took 1,184–1,290 ms and serialized 142,831 bytes. These are local debug timings and aggregate page payloads, not a user-facing latency or token measure.

Path-level discovery followed by scoped searches improves the simulated first-page path result to 9/12 and 8/12 expected ranges; paging discovery and scoping all expected paths reaches 12/12. A single global range page remains weak, and query 2's global range pages still miss five spans. This service profile does not establish automatic agent tool use, sufficient broad-answer evidence, packaged offline coverage or narrative quality. Keep `a699`, its dependent agent qualification and guide-quality work open.

## Current-source one-file incremental update — 21 October 2026

Ran the ignored native Fontdone update profile from Code Atlas source `b7ff94c` against a marked disposable copy of the saved Fontdone snapshot. The original app data was not modified. No model, Codex, speech request, or Fontdone code execution was involved. The profile appends a 66-byte comment to `src/font.rs`, prepares the updated snapshot, compares it with a fresh Tantivy build, and checks frozen expected ranges.

The current parser index serializes to 41,616,863 bytes for 2,768 parser-index file entries, 7,507 symbols and 59,534 relationships. The old 32 MiB persistence bound rejected that index; the bounded per-snapshot limit is now 64 MiB. A first parser-index build took 6,485 ms; loading the saved index took 518 ms in the final profile. Both the incremental and fresh Tantivy generations contain the same 3,462 live documents in one segment. Single-segment publication keeps broad BM25 ordering stable across cold and incremental builds.

| Measurement | Observation |
| --- | ---: |
| Retrieval coverage | 1,621 indexed files; 19,296,140 source bytes; 3,462 ranges |
| Syntax / text fallback files | 153 / 1,468 |
| Unavailable files / snapshot paths skipped | 1,147 / 25 |
| Cold retrieval-index preparation | 13,941 ms |
| Updated syntax-index build | 4,255 ms |
| One-file incremental retrieval-index preparation | 4,387 ms (3,217 ms build work) |
| Fresh retrieval-index preparation | 14,313 ms (13,154 ms build work) |
| Unchanged files / documents reused | 1,620 / 3,409 |
| Changed-file documents added / deleted | 53 / 53 |
| Tantivy segment bytes copied | 5,051,978 bytes |
| Named expected ranges | 12/12 global and 12/12 file-scoped |
| Broad first-page expected ranges | 1/12 and 1/12; identical between incremental and fresh builds |

The result confirms one-file reuse and search-result stability for this frozen profile. Broad first-page retrieval remains weak, and this does not qualify other change shapes, packaged search, cancellation/pressure boundaries, or AI answer quality. See the [machine-readable profile](artifacts/2026-10-21-fontdone-incremental-profile.json). Keep `a699` and its dependent generation task open.

## Current-source large-file parser and search profile — 21 October 2026

Ran the native parser and retrieval profiles from Code Atlas source `b7ff94c` against the same marked disposable Fontdone copy. The real 613,652-byte `src/ffi/handles.rs` and 1,032,893-byte `fontdone-c-abi/src/implementation.rs` both report `Indexed`, with 673 and 902 symbols respectively; each includes `FT_Load_Glyph` and `FT_Render_Glyph`. Both files are under the current 1 MiB per-file parser limit, so this real-repository result qualifies syntax indexing, not parserless fallback. A separate deterministic synthetic test covers a larger parserless C adapter.

| Measurement | Observation |
| --- | ---: |
| Parser index build for the captured snapshot | 5,291 ms; partial status due to other omissions |
| Retrieval coverage | 1,621 indexed files; 19,296,074 source bytes; 3,462 ranges |
| Syntax / text fallback files | 153 / 1,468 |
| Named expected ranges | 12/12 globally and 12/12 file-scoped, including all four FFI/ABI ranges |
| Broad ranked first page | 1/12 for each query |
| Broad ranked pages within the 256-candidate window | 11/12 and 7/12 |
| File discovery, first 32 paths | 5/7 and 4/7 expected files |
| File discovery across eight pages, then scoped retrieval | 7/7 expected files and 12/12 ranges for both queries |
| Discovery / all ranked-page time | 1,176–1,202 ms / 8,242–8,751 ms |

This establishes local service retrieval of the pinned real FFI/C declarations and a multi-step discovery-to-scope path. Global first-page recall is still too weak to infer that an agent will find every path without refining its searches. The profile does not qualify packaged behavior or answer quality. The machine-readable observations are in the [large-file retrieval artifact](artifacts/2026-10-21-fontdone-large-file-retrieval.json). Keep `a699` and `0wdn` open.

## Current-source retrieval strategy comparison — 25 October 2026

Reran the ignored native service profile at Code Atlas source `2a55680` against the existing Fontdone `main@5249b72371` snapshot and syntax index in a separate temporary retrieval cache. The app's original storage was not modified. No model, Codex, speech request, or Fontdone code execution was involved. The profile passed; this was a disk-loaded and same-process cached search measurement, not a cold index build.

The current baseline found all twelve named declarations globally and scoped. Both broad ranked first pages found 1/12 expected ranges. The 256-result candidate window found 8/12 and 7/12 respectively. File discovery found 6/7 and 4/7 expected paths on its first page; across eight pages it found 7/7 paths, and scoping range search to those paths recovered 12/12 ranges for both queries. The first-page file path sets covered 11/12 and 8/12 expected ranges when followed by scoped search.

A temporary experiment unioned per-term candidates with the combined lexical ranking to recover high-coverage ranges outside its bounded candidate pool. Its synthetic 600-decoy regression passed, but the real profile did not improve: first-page recall remained 1/12 for both queries, and the first query's candidate window fell to 6/12 while the second stayed 7/12. The code change and its test were discarded. The measured next path is file discovery followed by scoped range reads; automatic agent orchestration and useful global first-page recall remain unqualified. The experiment's profile completed in 56.94 seconds. No source snippets or provider output were saved.

## Current-source Fontdone profile after automatic file-first default — 25 October 2026

Reran the ignored local service profile at Code Atlas commit `34d67b9` against Fontdone `main@5249b72371`. The fixture was an isolated clone of the saved `snapshots`, `objects`, `indexes`, and `repository-context` directories under `/private/tmp/code-atlas-a699-post-default-20261025`; the running app's storage was not changed. The search profile calls the local range/file-discovery services directly. It makes no model, Codex, or speech request and does not execute Fontdone.

| Measurement | Observation |
| --- | ---: |
| Indexed files / source bytes / ranges | 1,621 / 19,296,074 / 3,462 |
| Syntax / text-fallback files | 153 / 1,468 |
| Unavailable files / snapshot paths skipped | 1,147 / 25 |
| `memory_face` first disk-load + search / warm search | 2,181 ms / 174 ms |
| Named independently selected ranges | 12/12 globally and file-scoped |
| Broad range first page | 1/12 for both queries |
| Broad range pages, within the 256-result window | 8/12 and 7/12 |
| File discovery, first page | 6/7 and 4/7 expected paths |
| File discovery across eight pages, then scoped retrieval | 7/7 paths and 12/12 ranges for both queries |
| First-page discovery paths followed by scoped retrieval | 11/12 and 8/12 ranges |
| Eight-page file discovery time | 1,305 ms / 1,255 ms |
| All broad range pages time | 11,195 ms / 10,804 ms |

This repeats the service-level discovery-then-scope result after the `search_code` automatic default change; it does not measure an agent choosing and using that default. The profile used an existing prepared parser index, so the first value is a disk-loaded search, not a cold index build. Results are one run without a variance estimate, and the all-page totals aggregate bounded request payloads rather than a single model context. A single broad range page remains at 1/12. Keep `a699`, `gk46`, and dependent guide-quality tasks open; this is not an explanation-quality or packaged acceptance result. The temporary test output is in `/tmp/code-atlas-a699-post-default-20261025.log`; no Fontdone source was copied into this repository.

## Current-source Fontdone profile with an empty retrieval cache — 25 October 2026

Ran the ignored native-service profile at Code Atlas source `6cbb30d382df97335af2bc01fe728a34d22c5b62` against Fontdone `main@5249b72371`. The fixture was an isolated copy of the saved snapshot, objects and syntax indexes at `/private/tmp/code-atlas-fontdone-profile-qOJQ59`; its pre-existing retrieval cache was moved aside before this run. The app's original storage was not modified. No model, Codex, speech request, or Fontdone code execution was involved. The test passed. This creates a fresh retrieval generation, while reusing the copied syntax index; it is not a cold parser-index build.

Environment: Apple M3 Pro, arm64, macOS 15.7.7, Rust/Cargo 1.98.1. One run, without a variance estimate. Command:

```sh
RUSTC_WRAPPER= \
CODE_ATLAS_FONTDONE_DATA_DIR=/private/tmp/code-atlas-fontdone-profile-qOJQ59 \
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib \
  repository_context_commands::tests::fontdone_service_retrieval_profile -- --ignored --exact --nocapture
```

| Measurement | Observation |
| --- | ---: |
| Indexed files / bytes / ranges | 1,621 / 19,296,074 / 3,462 |
| Syntax / text-fallback files | 153 / 1,468 |
| Unavailable files / snapshot paths skipped | 1,147 / 25 |
| First search including retrieval-index preparation / warm same-process search | 15,530 ms / 167 ms |
| Serialized `memory_face` result; top hit | 23,276 bytes; `src/font.rs:4053` |
| Named independently selected declarations | 12/12 globally and 12/12 file-scoped |
| Broad first ranked page | 1/12 for each query |
| Broad pages inside the 256-result candidate window | 8/12 and 7/12 |
| File discovery first page | 6/7 and 4/7 expected paths |
| Eight-page file discovery, then scoped search | 7/7 expected paths and 12/12 ranges for both queries |
| First-page discovered paths, then scoped search | 11/12 and 8/12 ranges |
| File discovery time / result bytes | 1,230 ms / 173,839 bytes; 1,267 ms / 170,760 bytes |
| All broad range pages time / result bytes | 10,399 ms / 165,137 bytes; 19,298 ms / 159,096 bytes |
| Retrieval cache / copied syntax-index directory after the run | 14 MiB / 234 MiB (index directory includes other saved snapshots) |

This confirms the local service can discover all seven expected files and recover all twelve pinned ranges when it pages file discovery and scopes range searches. It also reconfirms weak global first-page recall and an incomplete first-page discovery result. The test invokes local services directly; it does not prove that an agent automatically follows the multi-step path, that the packaged app works offline, or that an explanation is accurate or useful. These are debug-profile observations, not user-facing latency or token measurements. Keep `a699`, `gk46`, and guide-quality tasks open.

## Current-source Fontdone profile with the owned file-page cap — 28 October 2026

Ran the ignored native-service profile with its file-discovery helper set to the
same eight-result page size exposed by the owned `search_code` operation. The
fixture is an isolated copy of Fontdone `main@5249b72371` snapshot data and its
V6 syntax index at `/private/tmp/code-atlas-a699-post-default-20261025`; the
running app's data was not changed. An earlier profile attempt against this copy
without the V6 syntax index reported zero syntax files and is discarded. The
corrected run used the copied V6 index. No model, Codex, speech request, or
Fontdone code execution was involved.

| Measurement | Observation |
| --- | ---: |
| Indexed files / source bytes / ranges | 1,621 / 19,296,074 / 3,462 |
| Syntax / text-fallback files | 153 / 1,468 |
| Unavailable files / snapshot paths skipped | 1,147 / 25 |
| `memory_face` first search including index preparation / warm search | 2,617 ms / 580 ms |
| Named expected ranges | 12/12 globally and 12/12 file-scoped |
| Broad range first page | 1/12 for each query |
| Broad range pages within the 256-result window | 6/12 for each query |
| File discovery first page | 4/7 expected paths for each query |
| File discovery across 32 eight-result pages, then scoped search | 7/7 paths and 12/12 ranges for both queries |
| First query discovery time / serialized bytes | 6,066 ms / 160,166 bytes |
| Second query discovery time / serialized bytes | 4,541 ms / 160,287 bytes |

This measures direct local-service paging and scoped retrieval. It does not show
that an agent automatically performs 32 file-discovery calls, that global
first-page ranking is useful, or that a generated explanation is complete.
Result-byte totals span bounded service responses and are not one model context.
The profile also does not qualify packaged search, cancellation/corruption, or
answer quality. Keep `a699`, `gk46`, and dependent generation tasks open.

## Fontdone broad file discovery with a 32-path page — 28 October 2026

The native search service accepts 32 results, while the owned Codex MCP wrapper
caps file-discovery replies at eight paths to bound model context. This ignored
Fontdone service profile bypasses that wrapper and measures the native service
at 32 results; it does not qualify the model-facing page size. This run used
the same isolated saved
`main@5249b72371` snapshot plus V6 syntax index described above; no app storage
was changed and no model, Codex, speech, or Fontdone code was run.

The first 32-path page contained all seven independently expected paths for
both broad queries, and scoped searches over those paths recovered all 12/12
expected ranges. Under the former eight-path page, the first page contained
4/7 expected paths for each query, and 32 pages were needed to enumerate all
seven. The eight 32-path pages together returned 160,142 bytes for query one
and 160,263 bytes for query two; those are serialized local-service responses
over the full 256-result window, not the size of the first page or one model
context. Discovery across all eight pages took 1,479 ms and 1,412 ms.

The native-service profile makes the seven expected paths available in its
first discovery response in this fixture. It does not establish that Codex
receives those 32 paths, that a generated explanation reads or uses those paths,
or that broad range ranking is sufficient: the broad range first page still
finds 1/12 expected ranges and the 256-result range window finds 6/12 for each
query. Agent answer quality and packaged acceptance remain open. This is a
single local-service profile with no variance estimate.

## Symbol-aware range ranking on Fontdone — 29 October 2026

The local ranker now searches indexed declaration names separately from source
body text and orders broad-query results by exact/declaration matches, weighted
symbol coverage, then body-term coverage and the existing BM25 score. A focused
synthetic regression places all query words in an unrelated body but only the
behavioral declaration name in the relevant function; the relevant declaration
must rank first. Search budgets, page bounds, stored ranges and result schema
are unchanged.

The focused regression passed:

```text
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib repository_context::tests::broad_range_search_prefers_query_terms_in_indexed_declarations_over_body_noise -- --exact
1 passed; 0 failed; 843 filtered out
```

The ignored Fontdone local-service profile also passed against a disposable
copy of Fontdone `main@5249b72371` and its syntax index, stored at
`/private/tmp/code-atlas-a699-current-20261029`. No live app data, model,
Codex, TTS, or Fontdone code was used.

```text
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib repository_context_commands::tests::fontdone_service_retrieval_profile -- --ignored --exact --nocapture
1 passed; 0 failed; 843 filtered out; 40.71s
```

| Measurement | Before ranking change | After ranking change |
| --- | ---: | ---: |
| Broad range first-page recall, query 1 | 1/12 | 5/12 |
| Broad range first-page recall, query 2 | 1/12 | 5/12 |
| Broad range recall in the 256-result window, query 1 | 6/12 | 10/12 |
| Broad range recall in the 256-result window, query 2 | 6/12 | 7/12 |
| Expected files discovered on first page | 4/7 with eight-path pages; 7/7 with a direct 32-path service page | 7/7 with a direct 32-path service page |
| File discovery followed by scoped range search | 12/12 expected ranges | 12/12 expected ranges |
| Paged broad range search time, query 1 / query 2 | 9,820 / 9,492 ms | 9,094 / 10,434 ms |

The post-change profile preserves 1,621 indexed files, 19,296,074 source bytes,
3,462 ranges, 153 syntax files and 1,468 text-fallback files; named identifier
recall remains 12/12. Temporary profile output and the isolated copy were
removed after these measurements were recorded; reproduce with the command
above and supply the accepted fixture/index described in the earlier profile
entries.

Symbol-aware ranking improves broad global retrieval, but still puts only 5/12
reviewed ranges from each broad query on the first page; the 256-result window
misses two and five ranges respectively. The measured file-first plus scoped
retrieval path recovers 12/12 locally, but automatic agent use of that path and
explanation quality are not qualified. These are single-run debug measurements,
not latency guarantees. Keep `a699`, `gk46` and guide-quality acceptance open.

## Packaged local `memory_face` search — 29 October 2026

The rebuilt Mac app was opened on the existing Fontdone
`main@5249b72371` snapshot. Searching locally for `memory_face` returned 79
matches across 22 files, with **Local search may be incomplete** and more
matches available. The results included the actual `src/font.rs`
`memory_face` implementation at lines 4053–4115. Opening that result displayed
the captured function body in the source pane, including the `PFR0` dispatch to
`pfr_face`; Explorer and the previous `src/tt/glyf.rs` selection were restored
afterward. Installed executable SHA-256:
`e37fbf64783e1b51dc27559d8849fa4f2f83f7656018b25707577ebabb432c36`.

No extra server, model, Codex, or speech request was used. The Codex connection
was present, so this verifies that the local search interaction did not need a
model request, but it does not qualify operation with the account disconnected.
The selected snapshot is partial and the UI correctly reported incomplete
coverage. This verifies one packaged named-definition lookup and source read;
it does not qualify broad query recall, offline/disconnected operation,
cancellation, corruption, disk pressure, parserless fallback behavior, or
answer quality. Keep the wider retrieval and agent qualification open.

## Expose the measured file-discovery page to the owned agent — 29 October 2026

The native file-discovery service already supports pages of up to 32 paths,
but the owned Codex tool capped file hints at eight. The owner now advertises
and enforces the same 32-result ceiling as other search pages. These results
contain path, range, and relevance metadata; source is still delivered only by
bounded `read_code` calls. The 250,000-byte source budget and 128 KiB MCP reply
bound are unchanged.

The existing 28 October Fontdone service profile found all seven independently
reviewed expected paths on its first 32-path page, then recovered all 12/12
expected ranges with scoped searches. The focused MCP catalog regression passed
after this change (1 passed; 843 filtered out) and confirms the advertised
limit follows the owner cap. No agent or model request was run, so actual
Codex use of the larger hint page, source-read quality, and broad explanation
quality remain unqualified. The direct-service profile is evidence for the
search ranking only; it is not a packaged-agent pass. Keep `a699` and `gk46`
open.

## Recheck current installed bundle — 29 October 2026

On the installed app (`843dfba7bdebbf200a9d755f516716187c83650190067e56096582924b3a4001`), local search for `memory_face` returned 79 matches across 22 files and surfaced the partial-coverage warning. Selecting `src/font.rs:4053–4115` opened the captured `Font::memory_face` body in the source pane, including its `PFR0` dispatch to `pfr_face`. No model or speech request was made. Explorer, `src/outline.rs`, and Guide were restored after this read-only check.

This is a packaged named-search and exact-source-open check on the current UI build. The app was connected during inspection, so disconnected operation remains unqualified. It does not establish that an agent selects the range, that a generated explanation is useful, or that broad retrieval meets recall targets; keep `a699`, `0wdn`, and `gk46` open.

## Recheck after distinct-term-first range ordering — 29 October 2026

Reran the ignored native Fontdone service profile at Code Atlas source
`92a507b` against the existing isolated copy of Fontdone
`main@5249b72371` at `/private/tmp/code-atlas-a699-post-default-20261025`.
This reused the already-copied snapshot and syntax index; the profile wrote its
retrieval cache only inside that disposable directory. The running app's data
was untouched. No provider, Codex, speech, or Fontdone code ran. The full log is
`/tmp/code-atlas-a699-post-ranking-20261029.log`.

| Measurement | Observation |
| --- | ---: |
| Indexed files / source bytes / ranges | 1,621 / 19,296,074 / 3,462 |
| Syntax / text-fallback files | 153 / 1,468 |
| `memory_face` disk-loaded / cached search | 2,767 ms / 554 ms |
| Named expected ranges | 12/12 globally and 12/12 scoped |
| Broad range first page, query 1 / query 2 | 5/12 / 5/12 |
| Broad range recall in the 256-result window, query 1 / query 2 | 10/12 / 7/12 |
| First 32-path discovery page, query 1 / query 2 | 7/7 / 7/7 expected paths |
| File discovery followed by scoped search | 12/12 expected ranges for both queries |
| Discovery time across eight pages, query 1 / query 2 | 1,456 ms / 1,397 ms |
| Paged broad-range search time, query 1 / query 2 | 9,015 ms / 8,861 ms |

Distinct-term-first ordering fixes the synthetic rare-term counterexample, but
does not improve these Fontdone broad-range recall counts over the prior
symbol-aware profile. The tool's file-discovery-then-scoped-search path still
finds all expected paths and ranges locally. The mismatch confirms that a
single broad lexical range query is not a dependable repository explanation
plan; the agent must use file discovery and then investigate concrete code
operations. These are one-run local service measurements, not an agent or
answer-quality pass. Keep `a699`, `gk46`, and guide-quality tasks open.
