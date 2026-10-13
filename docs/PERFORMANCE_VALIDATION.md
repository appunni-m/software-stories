# Large repository performance report

Measured on 2026-10-13 against source revision
581cc8422d144229a783f1c794e877b40a31dddd. The benchmark is intentionally
small enough to reproduce locally; it is not a general performance guarantee.

## Environment and fixture

- Environment: macOS 15.7.7, Apple Silicon (aarch64), rustc 1.98.1.
- The restricted session did not expose the Mac model or CPU count.
- Fixture: a committed temporary Git repository with 2,000 TypeScript files in
  20 directories, each exporting one small function. The tracked source totals
  162,000 bytes. Fixture generation and Git commit are outside the timed
  capture/index/save phases.
- Method: one warmup followed by three sequential samples. Capture, static
  indexing, and atomic local index save are timed separately. Reported raw
  samples are milliseconds; median is the middle value.

## Results

| Phase | Samples (ms) | Median |
| --- | --- | ---: |
| Snapshot capture | 7,411; 7,390; 7,915 | 7,411 ms |
| Static indexing | 345; 346; 348 | 346 ms |
| Index save | 155; 155; 156 | 155 ms |

All 2,000 files were captured and indexed; the index contained 2,000 symbols
and 6,000 relationships. The capture phase includes Git metadata checks,
repository enumeration, reads, hashing, object writes, and manifest finalizing.
Indexing reads the captured snapshot and runs the Rust tree-sitter parsers.
Saving serializes and atomically writes the local index.

The capture and index commands run on Tauri's blocking worker pool and expose
progress and cancellation in the app. This timing does not measure rendered UI
responsiveness, VoiceOver, or end-to-end time on other repositories.

## Reproduction

Run from the repository root:

    cargo test --manifest-path src-tauri/Cargo.toml --lib report_representative_large_repository_performance -- --ignored --nocapture

The ignored test creates and removes its fixture under the system temporary
directory. It performs no provider request, opens no port, and does not modify
the working repository.
