---
kind: task
key: t11
parent: m2
title: Define versioned chapter, section, claim, anchor, and hotspot records
priority: 0
labels: phase-2, validation, chapters
blocked_by: t06
---

## User outcome
Every chapter is structured, editable, linkable, and mechanically checkable instead of being a long untraceable video script.

## Current greenfield contract — 18 October 2026
Chapter schema 5 is the only supported chapter-record format. Do not read, migrate, or silently upgrade older chapter versions; unsupported bytes remain unchanged and require explicit regeneration. Current v5 records round-trip without mutation, and unsupported versions fail closed. This matches the live Kata issue, which is closed. The dated v1–v4 notes below preserve implementation history only; they are not current compatibility requirements.

## Contract
Define a versioned schema for chapter IDs/revisions, file/group/overview target, snapshot, ordered sections, stable section/claim IDs, claim category, prose, source anchors, explicit dependencies, optional typed visual, typed hotspot destination, editable spoken text, and validation status. An anchor stores path, optional stable symbol ID, inclusive range, snapshot/hash, and exact excerpt. Claim categories are code behavior, repository context, or labeled inference. The current supported chapter schema is version 5; older versions are not read or migrated.

## Acceptance criteria
- Every code-behavior claim has at least one exact anchor; an inference records its basis and visible label.
- Every hotspot has a valid typed destination; executable HTML/script output is not accepted.
- Parser line shifts can be remapped only through stable symbol identity plus exact excerpt/hash checks.
- Validation reports structured issue codes for wrong snapshot, missing path, range/hash/excerpt mismatch, invalid claim, and invalid destination.
- Valid v5 records round-trip without mutation; unsupported versions fail without changing their bytes.

## Excludes
Proving that prose is semantically true, accepting AI output automatically, or storing a whole guide as one media blob.

## Done evidence
Valid and invalid payload fixtures exercise every field family, anchor mismatch, typed link, schema version, and immutable round-trip behavior.


### Original agent generation and exact step links — historical schema-4 checkpoint, 2026-10-16

The following schema-4 and legacy-compatibility details are historical. The current v5-only greenfield policy above supersedes them.

Both native Codex/Claude chapter result closures now attach schema-4 `generationOrigin` metadata after independent source/claim validation and before guarded preview publication. It retains the original request/fingerprint/contract, agent-specific model/reasoning/fixed approval/turn settings, distinct source digests, optional section target and complete optional accepted-plan step scope. API/manual import paths do not call this creator. Previews and immutable chapter history persist the record together; no sidecar transaction or agent process was added. Original-content hashing uses bounded streaming serialization and excludes root decision metadata.

The app exposes Original generation in native agent previews, recovered previews and chapter review. The accepted plan reader lists saved chapters for the exact accepted revision/digest/step, with explicit lifecycle/snapshot labels and separate read/review actions. Recovery distinguishes recorded request/step matches from older filename candidates. These are local recorded labels, not account attestation, complete-repository coverage, proof that a question was answered or step/read completion.

At this checkpoint, legacy v1–v3 reads and version-3 overview/pending-preview recovery were still present, and new writes used schema 4. The later greenfield change removed that compatibility path. The details about generation origin, edits and restore above describe this historical schema-4 implementation only.

The later greenfield change replaced this checkpoint's v4/legacy-read requirements with schema 5 only. Live Kata issue `8bw6` is closed; do not reopen it to restore legacy compatibility.
- [ ] Complete actual native Codex and Claude planned/ordinary chapter and section flows with matching settings/records, cancellation ownership, interrupted-save recovery and correct step links. No API prerequisite or fallback is allowed.
- [ ] Verify wrong request/backend/step, malformed/duplicate/oversized metadata, digest mismatch, forged renderer replacement, missing history, two same-file learning questions and cross-snapshot derived lineage. Use the detailed deferred cases in `xymb`.
- [ ] Review pacing, source-grounded answer quality, navigability and comprehension across representative languages. Recorded metadata/citation validity cannot satisfy this. Multi-unit orchestration and selective updates remain separate pending work.

No tests, builds, app launches, agent/SDK probes, login or model requests ran. Existing fixture constructors only received the required absent-metadata field; no tests were added or executed. Changed Rust regions were formatted and Git whitespace checks are separate from compilation/runtime acceptance. Codex remains Source, every automatic Claude cell remains Missing behind the empty registry, and the multi-agent release requirement remains open.

### Saved plan-step compatibility evidence — 17 October 2026

Local native regressions now reopen the actual synthetic accepted route and its independently saved chapter after correcting a shared-enum serialization digest break. They retain the original record/revision/claim/anchor identities, revalidate chapter source, and exercise synthetic older request contracts plus new schema-2 plan-record acceptance. Plan-record schema 2 is independent of chapter schema 4; existing schema-1 plan bytes and chapter lineage remain unchanged. [Exact scope and remaining qualification](../acceptance/2026-10-17-saved-route-replay.md). This does not qualify both live agent workflows, the full origin/legacy/history matrix, native UI, speech or semantic quality; the schema task remains open.
