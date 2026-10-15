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

## Contract
Define a versioned schema for chapter IDs/revisions, file/group/overview target, snapshot, ordered sections, stable section/claim IDs, claim category, prose, source anchors, explicit dependencies, optional typed visual, typed hotspot destination, editable spoken text, and validation status. An anchor stores path, optional stable symbol ID, inclusive range, snapshot/hash, and exact excerpt. Claim categories are code behavior, repository context, or labeled inference. Overview chapters use schema version 3 and stable IDs `chapter:overview` and `target:overview`; versions 1 and 2 remain readable for existing revisions.

## Acceptance criteria
- Every code-behavior claim has at least one exact anchor; an inference records its basis and visible label.
- Every hotspot has a valid typed destination; executable HTML/script output is not accepted.
- Parser line shifts can be remapped only through stable symbol identity plus exact excerpt/hash checks.
- Validation reports structured issue codes for wrong snapshot, missing path, range/hash/excerpt mismatch, invalid claim, and invalid destination.
- Schema versioning can read existing accepted revisions without mutating their payloads.

## Excludes
Proving that prose is semantically true, accepting AI output automatically, or storing a whole guide as one media blob.

## Done evidence
Valid and invalid payload fixtures exercise every field family, anchor mismatch, typed link, schema version, and immutable round-trip behavior.


### Original agent generation and exact step links — source checkpoint, 2026-10-16

Both native Codex/Claude chapter result closures now attach schema-4 `generationOrigin` metadata after independent source/claim validation and before guarded preview publication. It retains the original request/fingerprint/contract, agent-specific model/reasoning/fixed approval/turn settings, distinct source digests, optional section target and complete optional accepted-plan step scope. API/manual import paths do not call this creator. Previews and immutable chapter history persist the record together; no sidecar transaction or agent process was added. Original-content hashing uses bounded streaming serialization and excludes root decision metadata.

The app exposes Original generation in native agent previews, recovered previews and chapter review. The accepted plan reader lists saved chapters for the exact accepted revision/digest/step, with explicit lifecycle/snapshot labels and separate read/review actions. Recovery distinguishes recorded request/step matches from older filename candidates. These are local recorded labels, not account attestation, complete-repository coverage, proof that a question was answered or step/read completion.

Strict legacy v1–v3 chapter reads and version-3 overview/pending-preview recovery remain in source. New writes/review decisions use version 4 and preserve older immutable bytes. Missing origin is omitted; present null/invalid origin and future versions fail. Manual edits ignore renderer-supplied origin replacement and preserve native lineage. Restore retains the historical source origin; edited/remapped content can differ. New section generation clears inherited origin and receives its own record only from a native agent result, preserving earlier revisions/links. See [the record, compatibility and evolution contract](../CHAPTER_GENERATION_RECORDS.md).

**Pending:**
- [ ] Qualify schema-4/legacy decoding, preview restart/save and immutable decision/restore behavior; prior version-3 evidence is insufficient. The schema task `8bw6` requires reopening/revalidation for this change.
- [ ] Complete actual native Codex and Claude planned/ordinary chapter and section flows with matching settings/records, cancellation ownership, interrupted-save recovery and correct step links. No API prerequisite or fallback is allowed.
- [ ] Verify wrong request/backend/step, malformed/duplicate/oversized metadata, digest mismatch, forged renderer replacement, missing history, two same-file learning questions and cross-snapshot derived lineage. Use the detailed deferred cases in `xymb`.
- [ ] Review pacing, source-grounded answer quality, navigability and comprehension across representative languages. Recorded metadata/citation validity cannot satisfy this. Multi-unit orchestration and selective updates remain separate pending work.

No tests, builds, app launches, agent/SDK probes, login or model requests ran. Existing fixture constructors only received the required absent-metadata field; no tests were added or executed. Changed Rust regions were formatted and Git whitespace checks are separate from compilation/runtime acceptance. Codex remains Source, every automatic Claude cell remains Missing behind the empty registry, and the multi-agent release requirement remains open.
