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

## Follow-up: isolate unsupported chapter lineage — 29 October 2026

Current chapter revisions with an unsupported origin schema remain unavailable
and are never rewritten. The chapter store now inspects only their bounded,
duplicate-key-checked identity header, keeps their byte and record counts within
the existing storage limits, and skips their explanation content. This lets
current-schema revisions load and new current-schema work continue without
restoring old origin validation. Supported descendants may remain visible when
their parent is an opaque unsupported record; that ancestry is partial, and the
unsupported parent cannot be selected for review. Source-only review; no tests,
build, or packaged runtime check ran.

## Follow-up: strict Guide outline records — 29 October 2026

Repository Guide records now use schema 9 and contract
`repository-guide-v10-five-initial-units`. Validation accepts two to five
initial outline units and up to twenty total units after incremental topics are
appended. The former twelve-initial-unit allowance was removed. Older bounded
records remain on disk and show as unavailable; shared answer accounting holds
the maximum current Guide allowance for each opaque older record, so skipping
old content cannot release its capacity. A record claiming a newer schema fails
capacity inspection because its allowance cannot be known. The macOS data-format
manifest now publishes the Guide schema and contract. Existing source-level
fixtures were aligned to the current outline shape; no tests or build were run.

## Follow-up: do not reserve future work for unsupported guides — 31 October 2026

The packaged Fontdone workspace reached the 1,000-answer limit with 450 saved
answer receipts and 37 unsupported Guide records. Each unavailable schema-6/7/8
Guide reserved all 63 possible current answer slots, although the current app
cannot resume those guides. This counted hypothetical work as stored data and
blocked new explanations. Unsupported guides now reserve no future answer
slots; every answer receipt remains counted separately from its file, including
unreadable receipts. Old guide and answer bytes are preserved in place, and
current-schema guides continue to reserve the attempts they can still publish.
No test suite was run; package and fresh-section qualification remain pending.
