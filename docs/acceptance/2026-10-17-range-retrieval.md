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

Named global searches missed the four FFI/C adapter spans. File-scoped searches found all four in the 613,652-byte `src/ffi/handles.rs` and 1,032,893-byte `fontdone-c-abi/src/implementation.rs`. They remain parser-unavailable text fallback: finding their text is not a resolved definition/reference claim. The 375,700-byte `src/font.rs` now contributes a bounded 6,152-byte range beginning at the actual `memory_face` declaration.

These observations establish local availability of important previously excluded text. **They fail a broad repository retrieval sufficiency claim.** The service test asserts the file-scoped expectations, records global/broad misses, and does not assert that a broad explanation is ready. There was no new Codex request, narrative/visual generation, narration test or packaged-app walkthrough.

Cold means no compatible versioned index/layout cache; the OS page cache was not cleared. Cache disk sizes and unavailable-file reason counts are recorded in the metrics artifact. A previous compatible cache was preserved separately before the initial cold check.

## Deterministic checks

The default test suite checks UTF-8/CRLF coordinates, huge declarations and long lines, exact identifier/component ranking, repeatable ordering, snapshot isolation, deletion/rename, chunk-layout reuse on a one-file TypeScript change, secret/path/invalid-text filtering, source/hash changes, corrupted metadata/Tantivy recovery, read/query caps, expired deadlines, cancellation ownership and preservation of the last complete publication.

The one-file fixture reuses unchanged layouts but still verifies all eligible source and builds a fresh Tantivy generation. Fontdone incremental-update work, global cache retention, full boundary/cancellation qualification, definition/reference/module-map commands, and agent evidence-budget enforcement remain pending. The [Kata contract](../kata-tasks/69-local-range-retrieval.md) stays open.

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
