# Recorded agent generation and plan-step links

Source checkpoint: October 16, 2026. This implementation has not been built or exercised. Codex native acceptance is pending; Claude automatic generation remains unavailable behind the empty qualification registry.

## What the user sees

- New native Codex/Claude chapter previews, recovered previews and saved revisions show **Original generation** with the recorded agent, model, reasoning/approval settings, request ID and original snapshot. Planned chapters also retain the original question and exact accepted plan revision/step.
- An accepted learning step lists **Chapters started from this step**, using the full accepted revision/digest/step identity. Accepted revisions can be opened for reading; drafts, accepted and rejected revisions can be reviewed. Two steps targeting the same file do not share a link merely because their filename matches.
- Recovery distinguishes a **Recorded request match**, **Same recorded learning step · another request**, another recorded file request, and legacy/unrecorded filename candidates. Pending preview discovery also reports whether the original content digest matches.
- None of these labels accepts a chapter, completes a step, marks it read or establishes answer quality. Local metadata is not cryptographic account attestation.

## Native creation and persistence

Chapter schema **4** adds optional `generationOrigin`. Only the native Codex and Claude chapter result closures attach it after independent source/claim/visual validation and before guarded preview publication. They reuse the existing cancellation owner and publication stores. No additional model request or process is involved.

The record has its own schema version 1 and strict fields:

| Field | Recorded meaning |
| --- | --- |
| `projectId`, `snapshotId`, `targetPath`, `purpose` | Original reviewed generation scope: file, overview or one-section update |
| `requestId`, `requestFingerprint`, `requestBodySha256`, `contractVersion` | Exact original native review/request identity |
| `settings` | Agent-specific model/effort and fixed approval; Claude also records bounded turns |
| `sourceFiles` | Distinct relative paths, SHA-256 values and byte lengths; no source contents |
| `sectionUpdate` | Original section/base revision selector, when applicable |
| `plannedChapter` | Complete original planned-step scope, when applicable |
| `contentSha256` | Original project/snapshot/target/title/sections serialization digest |

`settings.backend` is `codex` or `claude-agent`. Codex records `never`; Claude records `dontAsk` and 1–8 turns. Unsupported contract/purpose combinations, malformed UUIDs/hashes, duplicate source paths, oversized metadata/source scope and inconsistent planned/section identity are rejected. This structural validation does not independently qualify the account/runtime or authenticate a file against local tampering.

Content hashing streams a borrowed, ordered JSON object containing `projectId`, `snapshotId`, `target`, `title`, and `sections`, using the chapter serializer, into SHA-256. Serialization is bounded to 4 MB and does not clone the chapter or retain another serialized payload. Root revision IDs, revision numbers, timestamps, lifecycle, decision notes, schema version and the origin record itself are excluded. Acceptance/rejection can therefore retain the original content identity while appending their own immutable decision metadata.

Recoverable previews must retain the original content digest and model ID. Their existing 2 MB per-record, 16 MB total and count limits still apply. Saving retains the record inside the immutable chapter, so there is no separate association file to lose between two writes. Reading the record does not rerun an agent or grant consent.

## Edits, restores and new generation

Manual editing ignores renderer-authored origin replacements and clones the native base's record. Acceptance and rejection retain lineage. Restore clones the selected historical source record; remapping to another snapshot can change the content digest. Descendant revisions may therefore retain original generation metadata while their current content differs. The UI says **Original generation**, rather than claiming every later edit was produced by that agent or still answers the old question.

A new section-generation operation clears inherited origin while building the new candidate, before validation. An actual native Codex/Claude result receives a new record for that request; a direct API or manual handoff does not receive agent provenance. Earlier revisions and their original plan links remain immutable. This does not automatically carry a plan association onto a newly generated section repair whose own request did not include that plan scope.

Plan links are descriptive local lineage, with the complete recorded plan selector. They do not reassign older same-file chapters or silently choose the newest chapter for another question. The reader uses the existing project history loader and explicit revision routes. Missing/corrupt accepted plan evidence still prevents opening that plan or preparing another planned request; an existing saved chapter retains its historical record independently.

## Compatibility and pending acceptance

Readers retain strict versions 1, 2 and 3, including version-3 overviews. New writes use version 4; editing/reviewing an older revision appends a new version-4 revision. Existing accepted bytes are not migrated or overwritten. Version-3 pending previews remain recoverable without fabricated lineage. Origin metadata is omitted when absent; a present null/invalid origin is rejected. Versions before 4 cannot carry it. Unknown future chapter/origin versions are rejected. Older app releases have not been qualified to read version-4 output.

Both actual agent workflows, preview restart/save, history decisions, content/visual round trips, cross-snapshot restore, wrong-step/request recovery, legacy compatibility, accessibility and narrative quality remain unexecuted acceptance work in Kata `xymb`. The schema task must be revalidated for version 4; its prior version-3 evidence is insufficient. Formatting and Git whitespace checks do not establish compilation or runtime correctness.
