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
Static inspection: snapshot.rs read_catalog/load_snapshot_manifest, project.rs load_recent_projects and snapshot finalization existing-object verification still use unrestricted fs::read. Index/recovery/source/WAV reads were bounded in this pass; these remaining sites need implementation.

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
Read-site inventory, selected bounds rationale, code changes and later boundary/corruption acceptance; no tests run in this pass.
