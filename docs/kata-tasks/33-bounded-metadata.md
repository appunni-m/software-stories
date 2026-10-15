---
kind: task
key: bounded-metadata
parent: t24
title: Bound remaining local metadata and existing-object reads
priority: 2
labels: storage, reliability
---

## User outcome
Oversized or damaged local metadata produces a recoverable error instead of unbounded memory allocation.

## Current evidence
The persisted catalog, manifest, Recents, and existing content-object paths now use bounded reads. Corrupt or unsupported metadata is reported rather than replaced with an empty default; existing source objects are verified by streaming SHA-256.

## Contract
Inventory remaining persisted-data reads. Set documented byte/count limits consistent with supported capture/history sizes. Enforce limits during the read, before deserialization; use bounded streaming hashing for existing objects. Preserve unreadable/unknown-schema data and return a recovery action instead of overwriting it with empty defaults.

## Acceptance criteria
- Catalog, manifest, Recents and existing-object checks cannot allocate the entire oversized input.
- Exact-limit and supported large histories remain readable; count/byte overflow is reported explicitly.
- Wrong identity/schema/truncated JSON/corrupt objects do not silently reset or overwrite saved data.
- Bound values and compatibility behavior are documented, with later acceptance evidence.

## Excludes
Automatic deletion of data and introducing arbitrary limits that make valid existing history unreadable without recovery.

## Done evidence
Read-site inventory, selected bounds rationale, code changes, and focused boundary/corruption tests. Packaged-app recovery messaging and compatibility review remain separate release acceptance evidence.

## Implementation update — 2026-10-15

- Recent-project JSON is capped at 16 MiB and 10,000 entries. Snapshot catalogs are capped at 16 MiB and 20,000 entries. Snapshot manifests are capped at 128 MiB. These limits exceed the supported capture and history count bounds without removing or rewriting older data.
- The reader continues to accept snapshot catalog schema 1 and manifest schemas 1 and 2. Unsupported versions and mismatched manifest identities return errors while preserving the original bytes; no automatic migration or deletion occurs.
- The shared bounded reader reads at most limit plus one byte before reporting overflow. Existing source objects are verified in fixed-size chunks and rejected above the captured-file limit.
- Catalog, manifest, and Recents decode, count, and size errors leave their original files untouched. Capture cannot begin from a damaged catalog, and a corrupt content-addressed object is not replaced during a recapture.
- Focused tests cover exact and over-limit reads, readable maximum supported histories, invalid persisted JSON, count overflow, oversized Recents, schema and identity compatibility, streaming object hash/size handling, and preservation of damaged snapshot data.
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib` passed with 177 passed, 0 failed, and 1 ignored (manual large-repository performance report). The suite was rerun after adding the metadata boundary cases; the existing dead-code warning in `narration.rs` remains.

## Scope of recorded validation — 2026-10-16 audit

The dated validation counts above are historical records, not a new run against the current checkout. No tests or builds ran during this audit. They do not qualify the later chapter-version-4, agent-runtime, narration, packaged-app or complete language/narrative changes. Current release acceptance remains governed by the open parent/capability tasks and the deferred cases in `xymb`.
