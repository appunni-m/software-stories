---
kind: task
key: t26
parent: m6
title: Build bounded read-only evidence for local Git base and head refs
priority: 1
labels: phase-6, pull-request, repository
blocked_by: t04, t06, t25, parser-binding-resolution
---

## User outcome
The developer can compare two locally available Git revisions and inspect changed source, declarations, and parser-resolved cross-file references without changing the checkout.

## Contract
Accept an explicit local base/head ref or full commit ID, resolve each once to an immutable commit object ID, and build deterministic bounded diff evidence from those commits. Do not fetch or invoke network-backed Git operations.

Build a separate static parser index for each revision. Prioritize changed paths. The relationship scan is limited to 10,000 regular tree entries plus changed paths from the 128-file diff cap, 5,000 indexed files and 50 MiB of source per revision, 512 KiB per file, and eight seconds across both revisions. Retain at most 16 sorted relationship records for each changed file and revision.

Attach high-confidence resolved imports when their target is a changed file. Attach calls and test references only when they resolve to an exact changed declaration. Keep base and head records separate. Every relationship source anchor and resolved declaration anchor identifies the exact commit object ID, blob ID, path, and line range.

Report relationship analysis as complete, partial, or unavailable. Complete means the bundled Rust, TypeScript, TSX, JavaScript, Python, and Go parser variants were indexed within these bounds; it does not mean complete semantics in any language. Python module imports and Go package imports remain unresolved. Parser uncertainty, unsupported files, dynamic/unresolved edges, tree/source/time limits, and per-file relationship caps must never be described as proof of no other reference. This is static source evidence; it does not infer runtime impact.

The comparison must not checkout, change refs, update the index, or write repository files. It must not modify existing repository chapter history.

## Acceptance criteria
- Ambiguous, missing, invalid, and non-commit refs return a choose-again state; full object IDs and unambiguous local refs resolve to immutable IDs.
- Diff order and evidence are deterministic for the same resolved commit pair. Base and head anchors use the correct revision and blob.
- Added, deleted, modified, renamed, binary, oversized, unsupported, and parser-uncertain changes retain an explicit classification and skipped reason where needed.
- High-confidence imports to changed files and calls/test references to exact changed declarations carry correct base/head source and target anchors. Unrelated files and same-file edges are not attached.
- Limits or parser uncertainty produce partial/unavailable relationship status with an explanation; edge truncation is surfaced and deterministic.
- Temporary-repository verification confirms refs, working-tree files, and Git index are unchanged.
- No fetch, push, checkout, commit, PR API call, GitHub authentication, runtime claim, or existing chapter-history mutation occurs.

## Excludes
Remote GitHub PR integration, comments/reviews, merge operations, and inferred runtime impact.

## Done evidence
Use temporary base/head commits to cover ambiguous/missing/invalid refs; add/edit/delete/rename/binary and parser uncertainty; exact import/call evidence in both revisions; per-file edge caps and incomplete-index reporting; deterministic anchors; and unchanged refs, index, worktree, and chapter history. Run the focused Git-evidence tests and the repository validation suite.

## Pending work — 2026-10-15

- [x] Use the shared indexer's conservative lexical shadowing/export resolution. The targeted parser-binding task `129x` is closed with Rust/TypeScript/TSX/JavaScript and separate base/head regressions; this is not a claim of full language semantics.
- [ ] Keep existing bounded base/head parser evidence implementation; verify both immutable sides with add/edit/delete/rename/binary/uncertain and capped cases.
- [ ] Record exact relationship anchors, deterministic truncation and complete/partial/unavailable labels without runtime-impact claims.
- [ ] Complete packaged Changes acceptance and unchanged refs/index/worktree/history evidence after prerequisites.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## Python and Go revision-index parity — 2026-10-28

Source review found that the separate Git revision index filtered to Rust,
TypeScript, TSX and JavaScript even though the shared snapshot index and this
contract include Python and Go. The Git path parser now admits `.py`, `.pyi`,
`.pyw` and `.go` into both changed-file declaration extraction and the bounded
base/head revision indexes. The complete-analysis message names all six shipped
grammars and preserves the known unresolved Python module and Go package import
boundary. The revision result also becomes partial when a source-like path is
not indexed, including a language without a shipped parser, instead of
silently presenting the remaining indexed files as complete. This is a
source-only correction; no tests or build ran. Keep the temporary-repository,
deterministic anchor, unchanged-worktree and packaged Changes acceptance gates
open.

## Count only parser candidates toward the revision-index file cap — 28 October 2026

The base/head source scan previously took the first 5,000 tree entries before
filtering for files supported by a bundled parser. Repositories with many
lexically earlier docs or assets could therefore use the source-index slots
without contributing source. The cap now applies after parser and per-file-size
eligibility checks. A candidate that does not fit the remaining shared byte
budget is skipped so a smaller later changed source can still be indexed.
Changed paths still sort first, and the existing aggregate source-byte and tree
bounds remain. Source-only change; no tests or build ran.
The temporary-repository and packaged Changes acceptance gates remain open.
