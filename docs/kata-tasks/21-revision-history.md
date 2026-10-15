---
kind: task
key: t21
parent: m4
title: Compare immutable guide revisions and restore as a new draft
priority: 1
labels: phase-4, history, revisions
blocked_by: t20
---

## User outcome
The user can understand what changed in an explanation and recover an earlier version without erasing history.

## Contract
Chapter Studio lists revision time, source snapshot, status, and parent revision. Compare explanation prose, sections, claims, anchors, visuals, spoken text, dependencies, and assets. Restore selected prior content as a new draft and revalidate its anchors against the target snapshot.

## Acceptance criteria
- Accepted chapter and section revisions cannot be edited in place.
- Diff identifies changed/added/removed fields and source references.
- The comparison is field-level and content-safe: source paths, ranges, and hashes may be summarized, but raw source excerpts are not copied into diff output.
- Restore creates a distinct draft while retaining the selected old version and current accepted version.
- Invalid restored anchors block acceptance and show exact repair options.
- Rejected draft history remains visible and cannot be accepted without a new revision.

## Excludes
Mutating Git history, deleting accepted revisions during restore, and silently pinning old line numbers to new source.

## Done evidence
Accept two distinct revisions, compare every changed field family, restore the older version, and verify all three records and their asset references remain separate.

## Pending work — 2026-10-15

- [ ] Keep deleted/excluded/non-text file chapter histories reachable; static fix applied, acceptance pending.
- [ ] Accept two distinct revisions and inspect every comparison field; restore an older revision as a new current-snapshot draft.
- [ ] Verify previous accepted/history/assets survive, invalid restored anchors block acceptance, and rejected content needs a new draft.
- [ ] Exercise edit/save/validate/navigation races against the patched UI; do not count source inspection as acceptance.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.


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
