# Large repository performance report

Measured on 2026-10-14 against source revision
4133f1491b33b77e80b38677c2884f786e8a213f. The benchmark is intentionally
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
| Snapshot capture | 5,760; 5,577; 6,089 | 5,760 ms |
| Static indexing | 358; 353; 352 | 353 ms |
| Index save | 153; 163; 152 | 153 ms |

All 2,000 files were captured and indexed; the index contained 2,000 symbols
and 6,000 relationships. The capture phase includes Git metadata checks,
repository enumeration, reads, hashing, object writes, and manifest finalizing.
Indexing reads the captured snapshot and runs the Rust tree-sitter parsers.
Saving serializes and atomically writes the local index.

The capture and index commands run on Tauri's blocking worker pool and expose
progress and cancellation in the app. This timing does not measure rendered UI
responsiveness, VoiceOver, or end-to-end time on other repositories.

## Atlas inventory UI follow-up

The Atlas inventory tree now filters with retained-path sets and renders a
bounded virtual window with ten overscan rows on either side. The focused
`tests/atlas.test.mjs` helper fixture contains 25,000 rows and checks that the
computed window remains at most 39 rows for a 510 px viewport at the default
29 px row height. This checks the window calculation and filtering behavior;
it is not a browser render benchmark and does not establish end-to-end UI
responsiveness, VoiceOver behavior, or accessibility at enlarged text sizes.
Those still need a measured desktop fixture and manual acceptance.

## Production JavaScript bundle

Measured on 2026-10-14 at source revision
`4133f1491b33b77e80b38677c2884f786e8a213f` with `npm run build`. The initial
JavaScript entry is 451.72 kB (130.26 kB gzip); Walkthrough is a 92.39 kB
(26.52 kB gzip) on-demand chunk, Git Changes is 11.55 kB (4.08 kB gzip), and
PR Guide Studio is a 31.86 kB (8.00 kB gzip) on-demand chunk. The initial
JavaScript entry is below Vite's 500 kB advisory threshold. These are
compressed artifact sizes, not browser startup timings; no rendered
performance or accessibility claim follows from them.

## Reproduction

Run from the repository root:

    cargo test --locked --manifest-path src-tauri/Cargo.toml --lib report_representative_large_repository_performance -- --ignored --nocapture

The ignored test creates and removes its fixture under the system temporary
directory. It performs no provider request, opens no port, and does not modify
the working repository.
