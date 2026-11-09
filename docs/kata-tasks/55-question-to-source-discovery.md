---
kind: task
key: question-to-source-discovery
parent: design-parity
title: Find relevant code from a question without manual context hunting
priority: 1
labels: product, design, agents, source
---

> **Scope update — 8 November 2026:** automatically select relevant code in the background. Do not ask the reader to pick reference files, approve a route, or review the same source scope for every question. A revocable permission for the selected repository snapshot authorizes bounded source access; re-check that permission, snapshot, account, and source limits before each dispatch. Show the chosen files and ranges as answer evidence, not as a blocking wizard.

## User outcome
A reader asks how a behavior works and gets a concise, source-grounded answer without manually guessing which files to select.

## Required behavior
- Search the captured inventory and local index, then let the connected agent retrieve relevant files or bounded ranges automatically. Operate only within the selected immutable snapshot and the user's existing permission.
- Run discovery and explanation as one background operation. Show a useful answer and quiet progress; do not expose a planner, route approval, reference checklist, or per-request scope confirmation.
- Keep the saved connection/model choice and repository permission. At dispatch, re-check the exact snapshot, account/runtime, revocation state, and source limits. Never silently expand access beyond the selected snapshot.
- Keep the question and answer when switching between Files, Flow, and the player or restarting. Cite the actual files/ranges in the answer so the reader can inspect how the agent used them.
- If the index is missing, partial, or a relevant file exceeds whole-file limits, use safe captured-source fallback or bounded range retrieval. Explain a real coverage limit in plain language; never invent a path or claim an unverified execution sequence.
- Preserve direct code selection and drag-and-drop as optional ways to narrow a question; neither is required for repository-wide discovery.

## Acceptance
Use a Rust multi-crate project, a TypeScript project, a missing-index case, and a question whose relevant source exceeds one request. Verify automatic source selection reaches the expected implementation, citations open the exact snapshot ranges, no file picker or repeated scope confirmation appears, the existing snapshot permission is enforced, and answer position survives navigation. Verify the saved connection is reused for repeat questions. User-visible controls describe the question and code rather than internal request-schema concepts.

## Workspace-flow regression check — 8 November 2026

On the current working tree based on `8b20924`, focused checks passed: `node --experimental-strip-types --test tests/code-explanation-one-action.test.mjs tests/workspace.test.mjs tests/repository-guide.test.mjs` — **53 passed, 0 failed**. This covers direct selected-code questions, snapshot-bound sharing, AI-selected file routing, existing-grant reuse in the workspace, incremental Guide recovery, and nested/background answer behavior. No provider request, app launch, or full build ran. This source/test evidence does not establish recall quality across the Rust/TypeScript repository fixtures, missing-index fallback, large-file retrieval, or packaged operation; keep the task open for those acceptance cases.

## Empty overview recovery no longer asks for files — 8 November 2026

The normal Guide and file-question paths already run source discovery in the
background. An empty overview recovery now returns to the workspace and asks
the reader to resubmit the question so Code Atlas can find the source
automatically; it no longer exposes a file checklist. Existing saved answers
remain available. The focused source-discovery suite passes 13/13 and the
production frontend build passes with the existing large-chunk warning. No
provider request was sent. Rust/TypeScript retrieval quality, missing-index and
large-file acceptance, and packaged operation remain open.

## Exclusions
Manual per-question file selection, accepted-route approval, repeated source-scope consent, paid service requirements, automatic access outside the selected snapshot, runtime-trace claims, or automatic acceptance of generated narratives. This task remains open for the broader repository recall and installed-app acceptance matrix.

## Existing baseline to preserve
Keep local search, optional file/code attachments, saved source version, last file, answer history, and repository permission controls. Automatic discovery is the default; manual scope selection is not a prerequisite. See the native workspace acceptance record before reimplementing existing pieces.

## October 16 implementation delta
The Codex path now prepares a local index automatically, reviews metadata, asks Codex to select implementation, validates up to 12 code paths/250,000 bytes, then automatically prepares the exact source review. File and overview requests share this discovery path. The workspace header restores and displays the saved connection/model/reasoning. Code-behavior claims backed only by documentation are rejected; explicit deprecated/archive labels and unverified docs are visible in planning. An accepted-route batch path existed in source but was removed from active product scope on 8 November; the current flow uses progressive Guide slides and file-level exploration.

Remaining: Rust multi-crate and large TypeScript retrieval, missing/unreadable index and metadata/source overflow, persisted discovery reasons across restart, and multi-window coverage for large repositories. Codex is the only first-release agent backend. See [current flow](../AI_GUIDE_FLOW.md), [progressive answer sections](73-progressive-answer-sections.md), and [bounded large-source retrieval](66-large-source-discovery.md).

## Fontdone qualification — October 17

The real Rust multi-crate repository exposes a gap between capture/index eligibility and question retrieval: the central 375,700-byte `src/font.rs` is indexed but excluded from discovery, and larger adapter implementations are above both request and parser limits. The broad font-to-bitmap request staged nine smaller files without the core font implementation or public Rust API. Keep large-repository acceptance open. Bounded source ranges and staged retrieval have a separate contract in [task 66](66-large-source-discovery.md), Kata `0wdn`; [native observations](../acceptance/2026-10-17-fontdone.md) preserve the limitations and failed attempts.

The subsequent [industry study](../research/2026-10-17-repository-context-and-indexing.md) recommends a local lexical/range index plus owned agent search/read tools. Implementation contracts are `a699` and `gk46`; neither is complete. Discovery acceptance must measure expected-range recall independently, rather than using the model's selected files as its own ground truth.

## Index fallback implementation — October 25

Discovery now records the local map state as `complete`, `partial`, `cancelled`, `missing`, or `unreadable` in the review metadata. A partial/cancelled map can still contribute the symbols and static links it contains; a missing or unreadable map falls back to eligible paths in the immutable captured inventory. Failure to load, start, or observe the optional indexing job no longer blocks discovery preparation. The dialog uses “captured code files” and gives an explicit limitation message for each incomplete state.

Focused source-discovery coverage passes for complete/partial baseline metadata, missing and corrupt indexes with captured-path fallback, partial/cancelled map labeling, and scope validation (5 tests). `npm run build` also passes. This closes the missing-index fallback implementation gap only; the Rust multi-crate and large TypeScript recall matrix, metadata/source overflow, restart persistence, and multi-window behavior remain open. The per-question consent requirement was removed by the 8 November scope update above.

## Fixed-target metadata priority — 27 October 2026

The source discovery metadata rank now puts a caller-specified eligible target
ahead of relevance-ranked candidates as a separate sort key. A sufficiently
long question can no longer outrank the target merely by accumulating more
path, symbol, or relationship matches. Remaining candidates keep their
relevance score and lexical tie break. This only orders candidates within the
existing 96 KB metadata budget; it does not relax file/source limits or the
large-file bounded-range route. This was a source-only change: no tests or build
ran. Multi-repository recall, overflow, restart, multi-window, consent and
packaged return-state acceptance remain open.

## Oversized fixed-target metadata — 27 October 2026

If a fixed target's single indexed metadata record cannot fit the bounded
discovery window, preparation now returns a focused limitation instead of
silently omitting that target and asking the agent to choose from unrelated
files. Ordinary non-target candidates still use the remaining shared budget.
Source-only change; no tests or build ran. The multi-project recall, aggregate
metadata/source overflow, restart, multi-window, consent and packaged
return-state acceptance remain open.

## Complete discovery-envelope bound — 27 October 2026

The 96 KB discovery limit now applies to the complete serialized review sent to
the agent, including the question, capture status, documentation notices and
coverage gaps, rather than only the per-file records. If envelope overhead
pushes the request over budget, the lowest-ranked non-target records are
removed and the omitted-file count is refreshed. The fixed target remains
first; if the target plus its mandatory coverage context cannot fit, preparation
returns a focused limitation before consent or transmission. The existing
metadata-size assertion now checks the actual 96 KB cap. Source-only change; no
tests or build ran. Recall quality, explicit consent, restart/multi-window and
packaged return-state acceptance remain open.

## Large-file discovery handoff — 27 October 2026

Captured readable code above the whole-file request limit is now offered as a
separate range-only candidate during discovery. The agent can select up to one
such target while selecting small supporting files under the existing 250 KB
whole-file budget. Discovery guidance prefers a materially relevant large
implementation as the primary target and uses small files as supporting
context. A range-only result is accepted even when it has no small
supporting files, then routes into the existing bounded-reader consent flow.
The original question, selected snapshot and up to 12 source paths are kept;
the target and supporting-file selection reasons plus any discovery limitation
are saved with the answer card for review after restart. Source-only change;
no build, tests, agent request or packaged-app acceptance ran. Recall quality,
multi-stage retrieval, omitted-range reporting and Fontdone acceptance remain
open.

## Automatic discovery sharing notice — 27 October 2026

The automatic repository-discovery dialog now displays its data boundary
while local preparation and agent discovery run: the question and a codebase
summary are sent to the saved AI connection to identify relevant files; file
contents are reviewed separately. It adds no file-picking step. This copy-only
source change was not built or tested; the consent, recall, restart,
multi-window and packaged acceptance above remain open.

## Packaged no-result search handoff — 28 October 2026

In the running Code Atlas app on Fontdone `main@5249b72371` (partial capture),
the local Search activity was given the unique query `zzqzxvbnm987654321`.
It showed **No matching code in this search** and an explicit **Explore with
AI** button. I did not activate the AI action, so this check sent no question
and used no provider quota. Restoring the previous `memory_face` query returned
captured source results; switching back to Explorer restored the previously
selected `src/tt/glyf.rs` file. This confirms one packaged no-result and
return-to-Explorer path only. The running bundle's source revision and a
durable screenshot were not verified; Rust/TypeScript retrieval recall,
missing-index fallback, restart, consent, and multi-window acceptance remain
open.

## Distinguish discovery coverage causes — 28 October 2026

The UI previously used the same label for two separate unreadable-source causes
and for two separate missing-index causes. Coverage notes now distinguish
captured non-text paths, files the local code map could not read, files left
unmapped when indexing stopped, and files absent from the saved map. The native
gap categories and their evidence are unchanged; this is a user-facing
clarification, not a claim of improved retrieval coverage. No tests or build
ran; the broader multi-repository recall, overflow, restart, consent and
packaged acceptance remains open.

## Keep discovery work stable across parent renders — 28 October 2026

The discovery effect no longer depends on the inline `onReady` callback from
its parent. Callback changes update a ref, while each discovery result is bound
to the effect instance that started it. Cleanup requests cancellation and
invalidates late results before they can update the new scope. This prevents
ordinary workspace re-renders from discarding a prepared review and repeating
the metadata/model-selection operation. Source-only change; no tests, build,
provider request, or packaged race qualification ran. Multi-project recall,
restart, consent, multi-window and packaged acceptance remain open.

## Keep automatic discovery in the background — 28 October 2026

Automatic discovery no longer blocks the workspace with a preparation modal.
Its in-progress status appears in the notification center as renderer-local
state and disappears when discovery completes or its owning view closes; it
does not create a saved notification or dismissal record per run. Missing
connection setup and actionable discovery errors still use their existing
dialogs. The exact selected-source review and fresh consent remain in the
subsequent explanation request. This is a source-only UX change; no tests,
build, model request, or packaged keyboard/focus qualification ran. Retrieval,
restart, multi-window, consent and packaged acceptance remain open.

## Share one in-flight snapshot index — 28 October 2026

Workspace flow loading and question discovery now use the same per-renderer,
per-project/snapshot index promise. Discovery waits for the reader's active
index job instead of racing a second `start_snapshot_indexing` call (which the
native job store rejects) and proceeding with less relationship context. The
shared helper still falls back to immutable captured inventory when indexing
is absent or unreadable. The native command now returns the existing active job
to another window and starts a worker only for the window that created it, so
the per-renderer promise is no longer the sole duplicate-work guard.
Packaged multi-window and large-repository recall remain unqualified.
Source-only change; no tests, build, provider request or app acceptance ran.

## Make index panics terminal — 28 October 2026

The native indexing worker now catches an unwind from the parser, marks its
shared job failed, clears the current path, and emits a retryable message. This
prevents all renderers joining that job from polling a permanent running state;
no partial index is saved. Source-only change; no tests or build ran. Parser
panic recovery and cross-window retry still need native acceptance.

## Snapshot index join regression — 28 October 2026

A focused native regression now proves that a second request for the same
project/snapshot joins the existing running index job and shares its
cancellation token, while the strict `begin` API still rejects duplicates and
a different project receives a distinct job. The exact command
`cargo test --locked --manifest-path src-tauri/Cargo.toml --lib index::tests::concurrent_index_start_for_same_snapshot_joins_one_owned_job`
passes (1 test). Two unrelated Claude dead-code warnings remain.

This covers the job-registry contract only; it does not exercise two packaged
windows, parser panic/retry, cancellation during indexing, or large-repository
retrieval recall. Keep `xvjk` open.

## Rank proposed files using captured source text — 30 October 2026

Source discovery now reuses an existing snapshot-scoped Tantivy range index to
rank its bounded file shortlist by actual code matches before applying symbol,
relationship and path hints. It never builds an index as a side effect of the
review action; a missing or stale text index is reported as `contentSearch`
`notBuilt`, and discovery falls back to the existing metadata ranking. The
fixed target remains first. The metadata reports whether content ranking was
ready, partial or unavailable, and the fingerprint version was advanced so
pending reviews cannot be sent against the old shortlist contract.

Source-only implementation; no tests, build, provider request or packaged
qualification ran. This changes shortlist ranking, not recall proof. The
Fontdone 5/12 first-page profile, multi-language recall, missing-index behavior,
consent, restart and multi-window acceptance remain open. See
[`a699`](66-large-source-discovery.md) for large-file range gates.

## Scope reuse and rediscovery — 30 October 2026

Successful planner discovery still saves only its bounded file paths, reasons
and limitation for the exact project/snapshot/question/target. The selected
scope can now be refreshed directly with **Find code again**; that action skips
the saved result for the current discovery and replaces it only after a new
successful, cacheable result. Large-file range handoffs are not stored as
ordinary file selections. One-off file explanations now keep discovery
request-local instead of persisting a selection that may silently shape a later
request. This keeps automatic agent selection while making the saved-plan
reuse choice explicit.

Focused acceptance passed: source-discovery store/integration tests 8/8,
Codex connection tests 9/9, commit-navigation tests 3/3, and the TypeScript/Vite
production build. No agent request, native package, or Fontdone retrieval run
was made. Multi-repository recall, missing/overflow indexes, restart, consent,
multi-window and packaged return-state checks remain open; keep `xvjk` open.

## Expand question candidates through indexed static links — 4 November 2026

Discovery now seeds bounded expansion from up to 12 content-ranked paths,
including an eligible explicit target. It adds up to 24 eligible neighbors
from high-confidence indexed calls/imports in either direction. After the
existing target and test/configuration intent ordering, the top 24 direct
content matches remain ahead of those linked candidates; the linked candidates
precede lower-ranked direct matches and path/symbol fallback. Expanded records
carry the exact indexed relation and endpoint paths as a search hint. The prompt
explicitly says that this metadata does not prove execution, order, or
relevance. It does not change source transmission, consent, file-count, or byte
limits.

A synthetic TypeScript regression proves that a linked helper with no query
terms is included after the matching entry point and carries its exact hint.
The focused Rust discovery suite passed **8/8**. This verifies candidate
construction only: no post-change Fontdone profile, live Codex selection,
packaged workflow, or broad-recall improvement has been established. Keep
`xvjk` and `a699` open for multi-language recall, agent choice, multi-window,
consent, restart and packaged acceptance.

### Fair bounded expansion across matching files — 4 November 2026

A dense first-ranked implementation could previously fill all 24 relationship
hints, hiding linked candidates from other content-ranked files. Candidate
expansion now de-duplicates each seed's neighbors, keeps that seed's existing
call/import and path ordering, then emits one neighbor per seed in ranked
rounds until the shared 24-candidate cap is reached. This preserves the
strongest direct content matches and prevents one high-fanout file from
monopolizing the relationship shortlist.

A focused Rust regression constructs a first seed with more than 24 neighbors
and a second seed with one eligible call target; both remain in the bounded
hint set and their stable ranks are checked. The focused native discovery
module passed **17/17**; rustfmt and `git diff --check` passed. This source
change improves candidate fairness only. Fontdone recall, agent selection,
packaged behavior, consent, restart, and the broader multi-language matrix
remain open; keep `xvjk` and `a699` open.

GitHub package verification run `41798014426` for pushed commit `5ab8c0a`
completed successfully on 4 November 2026, including the unsigned macOS bundle
build and inspection. This establishes package build integrity for this source
revision; it does not run the retrieval-quality matrix or establish native
workspace navigation and visual acceptance.
