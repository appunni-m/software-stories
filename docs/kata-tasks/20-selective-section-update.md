---
kind: task
key: t20
parent: m4
title: Edit or regenerate one selected stale section and reuse unaffected assets
priority: 0
labels: phase-4, regeneration, revisions
---

> **Current contract — 9 November 2026:** formal accessibility audits are not first-release gates. Connect once and grant the captured snapshot once. A user request to repair one selected section is sufficient intent; the app rechecks the existing grant and source scope in the background, validates the result locally, then saves and shows a new immutable revision automatically. Do not add per-repair source review or an acceptance dialog. Historical notes below describe earlier draft/review flows and are not current requirements.

## User outcome

The user can repair one part of a walkthrough without recreating the whole guide, while seeing any other sections affected by shared evidence.

## Contract
From the stale-section view, show the source change and affected section. The user selects one section and requests a repair using the saved header connection. The app quietly checks the current connection, snapshot-scoped grant, and exact bounded source before transmission; it does not ask the user to select or review files again. Locally validate the result, then save and show that section as a new immutable revision automatically. Keep unrelated sections and their media unchanged unless their declared inputs changed. A prose-only edit can be local.

## Acceptance criteria
- Regeneration targets one selected section and its declared evidence; no whole-chapter or whole-guide call is hidden behind the action.
- If a referenced source change also affects another section, list it separately; do not silently rewrite it.
- Unchanged sections retain revision IDs; diagrams/audio whose inputs did not change retain cache IDs.
- The saved project/snapshot grant is validated in the background; cancellation, revoked access, rejection, or provider failure leaves prior history untouched.
- A valid changed section is published as a new immutable chapter revision without a separate accept action; an invalid section remains unpublished.

## Excludes
Background/bulk regeneration, replacing all sections by default, silent asset deletion, and overwriting accepted content.

## Done evidence
Change one cited function, repair one selected section, and verify the new revision appears automatically while unrelated sections, unchanged visual/audio cache assets, and prior history retain their identities and payloads.

## Historical pending work — 2026-10-15 (superseded by the current contract above)

- [ ] After Pocket cache identity is integrated, change one cited function and review exactly one stale section.
- [ ] Regenerate/accept only that section using the saved connection and exact applicable request permission. Record new section/chapter IDs without repeated backend setup.
- [ ] Compare unaffected section, diagram and narration IDs/payloads; verify shared dependencies appear separately and old accepted history survives.
- [ ] Exercise cancellation/rejection/provider failure with no hidden whole-guide regeneration.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## Codex source delta — 2026-10-15

Codex now shares the transport-independent section context with the API adapters. The generation chooser permits Codex for a single proven-stale section, fixes its declared source selection, binds the accepted chapter/section revisions into the review and rebuilds that context before dispatch. Current source is citable; the old section and old source are reviewed context only, with old/current bytes bounded together. Returned section output enters the existing preservation/citation canonicalizer and saved unaccepted-draft flow. Old source is additionally screened for secret-like content and bounded before/while reading. No runtime, generation, tests or build were executed; preservation, cancellation and packaged acceptance remain pending. Manual handoffs still do not cover section regeneration.

## Shared scene repair boundary — 17 October 2026

The current selected-section schema requests nullable version-2 HTML
compositions. New-result preparation targets only that section; regression
checks preserve carried legacy presentation/script bytes and reject a missing
selected ID. Existing stale-section/history/asset-ID/provider checks pass in
the 394-test native suite. [Scope and evidence](../acceptance/2026-10-17-shared-chapter-scenes.md).

Pending: perform a real Codex repair from the saved header connection, with an
old accepted guide and actual cached Pocket clips. Accept only the repaired
section, compare unaffected IDs/payloads/audio/visual fingerprints, and exercise
Stop/rejection/failure and shared dependencies in the packaged app. The new
ordinary Fontdone file chapter is not this selective-repair qualification.
Keep the task open.

## Multiple stale sections source audit — 28 October 2026

Static inspection found that both `provider::build_section_update_context` and
`chapter_generation::prepare_section_update` reject the selected repair when
any other section is neither `Current` nor `SafeToCarry`. Draft preparation
also remaps every carried citation to the new snapshot and fails when a changed
anchor cannot be verified. This protects the current single-snapshot chapter
contract, but it means a guide with two stale sections cannot be repaired one
section at a time from the same accepted revision: each request is blocked by
the other stale section.

Do not remove these checks by themselves. The product needs an explicit repair
session or section-level snapshot/freshness representation so one section can
be regenerated at a time, other affected sections remain visibly stale and
unchanged, and no invalid current-snapshot chapter can be accepted. Keep the
accepted chapter immutable throughout. No source change, test, build, provider
request, or app run was made for this audit; packaged selective-repair
acceptance remains pending.

### Chosen direction — durable selective-repair session (planned)

Use a bounded, durable repair session scoped to one project, the exact accepted
base revision, and one target snapshot. This is a staging record, not a
`ChapterRevision`: it may retain an unselected stale section with its original
snapshot-bound citations, but must never label those citations current or pass
them to the normal chapter validator as target-snapshot evidence.

The session records the source/target snapshot IDs, base revision ID, a stable
freshness assessment, and each affected section's ID, revision ID, status, and
any validated replacement. Starting or resuming a session does not change the
accepted chapter. Each user-requested repair binds to the same base and target,
one selected section ID/revision, the saved connection, and the current exact
permission. Only that section is sent for regeneration. Its response is
canonicalized against the target snapshot and persisted in the session. Stop,
provider failure, rejection, or app restart leaves the accepted chapter and
previously completed session replacements intact.

Unchanged sections may be carried only after existing freshness logic proves
their anchors and dependencies safe in the target snapshot. Stale sections
remain pending until explicitly repaired. `NeedsReview` is not equivalent to
stale or safe: the session cannot finalize while source/index uncertainty
remains unresolved. A changed base revision, target snapshot, or freshness
fingerprint invalidates the session for new requests and presents a clear
restart/reassess action; it never broadens an earlier request.

Only when every affected section is resolved does native code assemble one
ordinary draft pinned to the target snapshot and run the complete chapter
validator. The normal preview/accept flow then creates a new immutable chapter
revision/manifest. If final validation fails, retain the session for a scoped
repair and leave accepted history untouched. Reuse section, visual, and audio
identities only where the current input fingerprints prove they are unchanged.

Required implementation evidence: a multi-section fixture where edits make
two sections stale; repair one and verify only its replacement is recorded
while the other remains pending; restart and repair the second; then verify the
assembled draft has only target-snapshot anchors, unchanged safe sections and
assets retain identities, and acceptance creates a distinct revision. Also
cover cancel/failure, corrupt or oversized session data, changed base/target,
and `NeedsReview` without automatic publication. This architecture was planned
at the time of this audit. A native session workflow was implemented on
29 October; provider and packaged-app acceptance remain open.

### Durable session storage foundation — 29 October 2026

The native backend now has a versioned, bounded durable session record scoped to
the accepted base revision, source and target snapshots, and a fingerprint of
the full freshness assessment. It records each original section revision and
freshness state separately; a canonicalized replacement can be saved for one
stale section while other stale or `NeedsReview` sections remain pending.
Session files use private local storage, atomic publication, strict identity and
size checks, and restart-safe loading. Five focused tests cover persisting one
of two stale replacements, rejecting changed freshness, rejecting corrupt or
oversized state, exact coverage of the accepted sections, and keeping
`NeedsReview` unresolved.

### Native selective-repair workflow — 29 October 2026

The user action now creates or resumes the exact local session for the accepted
base, selected snapshot, and freshness report. The session ID is included in the
request target and provider fingerprint. Direct API and account-agent reviews
revalidate that binding before dispatch. The user can repair multiple stale
sections one at a time; each response is canonicalized and its visuals are
prepared against the target snapshot before the replacement is stored. Until
all stale sections are replaced, the operation reports progress without
creating a chapter draft. The accepted chapter remains unchanged.

When the session is ready, native code assembles the target-snapshot revision,
remaps only sections classified current or safe to carry, and runs the full
chapter validator before returning the ordinary unaccepted draft. `NeedsReview`
still blocks repair assembly. The focused session tests (5) and the
multi-section selective-repair pipeline test (1) pass. `cargo check --lib` and
`npm run build` pass.

The full native suite is not green: 758 passed, 67 failed, and 18 were ignored.
The failures span multiple subsystems and have not been triaged; release
readiness must not be inferred from the focused passes. No packaged-app run,
real provider request, acceptance of the resulting draft, audio-cache identity
comparison, cancellation/rejection exercise, or restart-from-installed-app
qualification was performed. Keep `tbsh` open until those acceptance checks and
the outstanding suite failures are resolved.

### Repair stale sections while another section needs review — source correction (2026-10-29)

The source previously stopped a targeted stale-section repair whenever any
other section was `NeedsReview`, even though the durable session contract keeps
that section unresolved and prevents final assembly until it is reviewed. The
direct-provider review and native response canonicalizer now permit a verified
one-section replacement within the same bound repair session;
`is_ready_to_assemble` still rejects a session with any `NeedsReview` section.
Chapter Studio also allows the stale-section action and explains that
unresolved review still blocks creation of the final draft. Progress feedback
distinguishes remaining stale sections from sections that need index refresh
and reassessment.

This is source-only review; no tests, build, provider operation, or packaged
repair session ran. The previously recorded full native-suite failures remain
untriaged, and the packaged multi-section acceptance is still open.

### Search-subsystem test triage follow-up — 29 October 2026

The earlier full-suite output that exposed three candidate-ranking failures
and one broad-range retrieval failure was recovered from the already-built test
binary. The three ranking tests had incomplete fixtures: valid documents did
not populate the required `end_byte` sort field. The broad-range test exposed
a real inconsistency between file discovery and range ranking. The fixtures
now supply complete sort identity, and broad natural-language ranges prefer
distinct-term coverage before IDF when declaration signals tie.

The complete repository-context test group now passes: 75 passed, 0 failed, 3
ignored. This is subsystem evidence only; the historical full-suite result of
758 passed / 67 failed / 18 ignored has not been rerun, and failures in other
modules remain untriaged. This does not change the selective-repair acceptance:
real repair, unaffected assets, cancellation/failure, and packaged workflow
checks remain open.

### Keep asset fingerprints available without cluttering the repair view — 31 October 2026

Chapter Studio now keeps per-section asset reuse fingerprints in a collapsed
“Asset reuse details” disclosure. Stale status, reasons, affected sections,
source diffs, and the one-section repair action remain visible. The IDs and
narration script hash remain available on demand for review. Source-only UI
change; selective-repair provider, unaffected-audio reuse, cancellation, and
packaged acceptance remain open.

### Reject ambiguous repair-session JSON — 2026-11-03

Session loading and project listing now decode through the shared duplicate-key
rejecting JSON parser before applying the repair-session schema and identity
checks. The previous direct `serde_json::from_slice` accepted duplicate JSON
keys using the last value, so a damaged record could be interpreted
ambiguously. The file and project-size bounds remain applied before decoding.
Source-only correction; no tests, build, provider request, or packaged
corruption-recovery check ran. Keep `tbsh` open for the full selective-repair
acceptance.

### Focused native regression rerun — 7 November 2026

The saved-session and selected-section pipeline tests now pass on the current
checkout:

- `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib section_repair_session::tests::` — **7 passed**.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib chapter_pipeline_tests::` — **8 passed, 1 explicit live-account probe ignored**.

This covers duplicate-key/corrupt session rejection, persistence of one
replacement while another section stays pending, changed-freshness rejection,
`NeedsReview` blocking assembly, and a selected stale-section repair that
preserves unrelated section and asset IDs. No provider request or packaged app
run occurred. Real Codex repair/acceptance, Pocket cache identity comparison,
restart/cancel/failure behavior in the packaged UI, and the remaining suite
qualification are still open; keep `tbsh` open.

### Match the cache-key fixture to the shipped Pocket voice — 2026-11-07

The focused selective-repair fixture now computes the preserved section's
audio-cache key with the shipped Pocket model ID, Alba voice, empty style, and
`audio/wav` format. The selected-section regression passes **1/1** after this
change. It verifies deterministic cache-key reuse for an unchanged narration
script under the shipped identity; it does not synthesize audio or qualify the
packaged Pocket runtime/cache. Real regeneration/acceptance and packaged
cancel/restart/failure behavior remain open. Keep `tbsh` open.

### Confirm the Pocket cache-key regression on the current checkout — 2026-11-08

Re-ran `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib selected_stale_section_regenerates_without_changing_other_section_or_asset_ids` on `main@8b20924` with the current fixture change: **1 passed**. The unchanged section retains its narration cache identity under the shipped Pocket model/voice/format values, while the regenerated section gets a different key. This does not synthesize or listen to audio, qualify packaged cache reuse, or complete selective-repair acceptance. Keep `tbsh` open.

### Verify one-action repair publication wiring — 9 November 2026

The packaged UI's selected-section action prepares the fixed review automatically,
uses the saved Codex connection and snapshot grant, and submits without a second
source-review or consent action. After a locally validated response, it saves
the candidate and publishes an accepted immutable revision automatically. It
checks the accepted revision is valid, based on the saved candidate, preserves
all section identities/revisions, and changes the selected section revision.
If the local save response is uncertain, it looks up the same immutable revision
before deciding whether to continue or report recovery.

Added `tests/section-repair-publication.test.mjs` to guard this automatic path
and its preservation checks; the focused test passes **2/2**. This is a source
contract regression only. No installed-app repair, real Codex response, Pocket
audio generation/reuse, restart, cancellation or provider-failure acceptance
was run. Keep `tbsh` open.
