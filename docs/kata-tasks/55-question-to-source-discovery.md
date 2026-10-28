---
kind: task
key: question-to-source-discovery
parent: design-parity
title: Find relevant code from a question without manual context hunting
priority: 1
labels: product, design, agents, source
---

## User outcome
A reader asks a question such as “How does a font file become pixels?” and gets a short, source-grounded reading path without manually guessing which files to select.

## Required behavior
- Search the captured inventory and local symbols/relationships to propose relevant entry points and supporting files. Operate against the selected immutable snapshot.
- Show a compact proposed scope, why each file is relevant, and capture/language gaps. Never present a static link as a proven execution sequence.
- Default to the remembered supported agent, model and reasoning settings. Opening the planner must not repeatedly ask the reader to choose a backend.
- Keep the question and selected scope when changing between Files, Flow and Player, or restarting the app. Clearly separate unsent local drafts from sent requests and accepted plans.
- Prepare the exact bounded source review locally. Require fresh consent before transmission. Respect secret, byte, file-count and source validation limits; explain when a question cannot fit one request.
- Present accepted steps beside real code. Clicking a step, graph node or citation opens the exact snapshot source and preserves the reading position.
- An empty or unsupported index must offer a clear fallback; never invent a path or silently broaden what is sent.

## Acceptance
Use a Rust multi-crate project, a TypeScript project, a missing-index case, and a question whose relevant source exceeds one request. Record the proposed files, reasons, omissions, consent scope and return-state behavior. Verify backend/model settings survive repeat file requests. User-visible controls must describe the question and code rather than internal request-schema concepts.

## Exclusions
No paid service requirement, automatic private-source upload, runtime-trace claims, or automatic acceptance of generated narratives. This task remains open for the broader acceptance matrix; the source implementation below does not establish exhaustive discovery.

## Existing baseline to preserve
The 16 October workspace iteration already remembers the backend/model, restores a saved source version and last file, and retains an unsent planning question and selected paths locally. These reduce repeated setup; they do not select the relevant source automatically. See the native workspace acceptance record before reimplementing those pieces.

## October 16 implementation delta
The Codex path now prepares a local index automatically, reviews metadata, asks Codex to select implementation, validates up to 12 code paths/250,000 bytes, then automatically prepares the exact source review. File and overview requests share this discovery path. The workspace header restores and displays the saved connection/model/reasoning. Code-behavior claims backed only by documentation are rejected; explicit deprecated/archive labels and unverified docs are visible in planning. Routes support one reviewed sequential chapter batch with saved-step skipping and independent drafts.

Remaining: Rust multi-crate and large TypeScript native runs, missing/unreadable index and metadata/source overflow acceptance, persisted discovery scope/reasons across restart, multi-window coverage for large repositories, and deferred non-Codex agent discovery. See [current flow](../AI_GUIDE_FLOW.md) and [batch qualification](56-route-chapter-batch.md).

## Fontdone qualification — October 17

The real Rust multi-crate repository exposes a gap between capture/index eligibility and question retrieval: the central 375,700-byte `src/font.rs` is indexed but excluded from discovery, and larger adapter implementations are above both request and parser limits. The broad font-to-bitmap request staged nine smaller files without the core font implementation or public Rust API. Keep large-repository acceptance open. Bounded source ranges and staged retrieval have a separate contract in [task 66](66-large-source-discovery.md), Kata `0wdn`; [native observations](../acceptance/2026-10-17-fontdone.md) preserve the limitations and failed attempts.

The subsequent [industry study](../research/2026-10-17-repository-context-and-indexing.md) recommends a local lexical/range index plus owned agent search/read tools. Implementation contracts are `a699` and `gk46`; neither is complete. Discovery acceptance must measure expected-range recall independently, rather than using the model's selected files as its own ground truth.

## Index fallback implementation — October 25

Discovery now records the local map state as `complete`, `partial`, `cancelled`, `missing`, or `unreadable` in the review metadata. A partial/cancelled map can still contribute the symbols and static links it contains; a missing or unreadable map falls back to eligible paths in the immutable captured inventory. Failure to load, start, or observe the optional indexing job no longer blocks discovery preparation. The dialog uses “captured code files” and gives an explicit limitation message for each incomplete state.

Focused source-discovery coverage passes for complete/partial baseline metadata, missing and corrupt indexes with captured-path fallback, partial/cancelled map labeling, and scope validation (5 tests). `npm run build` also passes. This closes the missing-index fallback implementation gap only; the Rust multi-crate and large TypeScript recall matrix, metadata/source overflow, restart persistence, multi-window behavior, and user-visible consent/return-state checks remain open.

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
