---
kind: task
key: greenfield-no-legacy
title: Remove legacy compatibility from the greenfield app
priority: 0
labels: architecture, greenfield, cleanup
---

## User outcome

The app has one supported product contract and one clear implementation path. Older external libraries remain inspectable; obsolete app-owned formats and readers do not remain as supported product behavior.

## Requirements

- Use one supported version and strict validation for persisted app records and generation protocols. Reject unsupported versions clearly without migration, rewrite, deletion, retry, or budget reset.
- Remove app-only aliases, fallback storage keys, format upgrades, historical replay rules, and obsolete generation/UI paths.
- Preserve current Codex, Pocket TTS, direct API, source navigation, nested exploration, Guide/Flow/history, cancellation, and bounded-index behavior.
- Test current behavior and rejection of unsupported formats; update current docs and macOS metadata to describe the supported contracts.
- Do not delete existing user data or regenerate it automatically. Unsupported content may require an explicit user action.

## Completion evidence recorded in Kata

Kata `zch9` was closed on 18 October. The source/test/manifest change was pushed as `50d9725beb603d125baee0505d9c5b537eb812e7`; the recorded check passed 443 frontend tests, 585 native tests, the corpus, TypeScript/Vite build, and Rust checks. The unsigned debug app was installed and opened. The closed Kata issue records the exact app/data hashes and remaining product-quality gates. This closure covers compatibility removal only.

## Follow-up source audit — 29 October 2026

The current native recovery journal had since gained a schema-1 reader that
silently rewrote interrupted-operation data to schema 2. The generation-job
store also accepted schema 3 as schema 4, and chapter origins accepted schema 5
without a recorded index identity. These app-owned compatibility paths
contradicted the single-supported-format contract. They are removed: only
recovery schema 2, job-store schema 4, chapter-origin schema 6, and timing
trace schema 5 are accepted. Older files/records are reported as unsupported
and remain untouched; a new record cannot overwrite the old recovery or job
store. Timing diagnostics require the complete schema-5 envelope, including
`promptContractVersion`, and no longer normalize schema 3/4 records. Existing
regressions assert rejection and byte preservation for recovery, jobs, chapter
origins, and timing traces. Static source review only; no tests, build, or
packaged recovery exercise was run for this follow-up.
