---
kind: task
key: t20
parent: m4
title: Edit or regenerate one selected stale section and reuse unaffected assets
priority: 0
labels: phase-4, regeneration, revisions
blocked_by: t10, t17, t18, t19
---

## User outcome

The user can repair one part of a walkthrough without recreating the whole guide, while seeing any other sections affected by shared evidence.

## Contract
From the stale-section view, show the source diff, stale reason, dependent sections, and fingerprints for narration/visual assets. The user selects one section and requests a repair using the saved header connection, including Codex account generation where supported. Revalidate the exact request, connection settings and applicable permission before transmission; do not make the user choose the same backend again. Receive a new draft section revision. A prose-only edit can be local. Preserve the accepted section and chapter manifest until explicit acceptance. Reuse unaffected sections and assets by input fingerprint.

## Acceptance criteria
- Regeneration targets one selected section and its declared evidence; no whole-chapter or whole-guide call is hidden behind the action.
- If a referenced source change also affects another section, list it separately; do not silently rewrite it.
- Unchanged sections retain revision IDs; diagrams/audio whose inputs did not change retain cache IDs.
- Exact per-request consent is revalidated before send; cancellation, rejection, or provider failure leaves accepted history untouched.
- Accepting a changed section creates a new chapter manifest; it never mutates an accepted revision.

## Excludes
Background/bulk regeneration, replacing all sections by default, silent asset deletion, and overwriting accepted content.

## Done evidence
Change one cited function, regenerate one selected section, accept it, and verify an unrelated section, unchanged diagram/audio assets, and prior accepted manifest retain their IDs and payloads.

## Pending work — 2026-10-15

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
