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
From the stale-section view, show the source diff, stale reason, dependent sections, and fingerprints for narration/visual assets. The user selects one section, chooses an available direct API provider/model, reviews its exact request manifest if regeneration is requested, confirms that request, and receives a new draft section revision. A prose-only edit can be local. Preserve the accepted section and chapter manifest until explicit acceptance. Reuse unaffected sections and assets by input fingerprint.

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
- [ ] Regenerate/accept only that section with explicit provider/model/request review. Record new section/chapter IDs.
- [ ] Compare unaffected section, diagram and narration IDs/payloads; verify shared dependencies appear separately and old accepted history survives.
- [ ] Exercise cancellation/rejection/provider failure with no hidden whole-guide regeneration.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## Codex source delta — 2026-10-15

Codex now shares the transport-independent section context with the API adapters. The generation chooser permits Codex for a single proven-stale section, fixes its declared source selection, binds the accepted chapter/section revisions into the review and rebuilds that context before dispatch. Current source is citable; the old section and old source are reviewed context only, with old/current bytes bounded together. Returned section output enters the existing preservation/citation canonicalizer and saved unaccepted-draft flow. Old source is additionally screened for secret-like content and bounded before/while reading. No runtime, generation, tests or build were executed; preservation, cancellation and packaged acceptance remain pending. Manual handoffs still do not cover section regeneration.
