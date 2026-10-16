# Identifier correction — 16 October 2026

## Reported failure

The installed app showed a persisted failure whose claim declared `record_pipeline_operation_path` without naming it in its prose. The exact message was observed in the native workspace under Details. The failed raw model response is not persisted, so this establishes a lexical contract failure, not which sentence or model rationale caused it. No user repository was sent again during this repair.

## Implementation

Workspace Codex questions include a fingerprinted `identifier-alignment-v1` policy with one maximum correction. Native preflight uses the existing whole-identifier predicate to detect prose/quote mismatches in section claims and evidence limits. Diagnostics contain positions and inconsistent names, not new source excerpts. One correction runs in the same ephemeral thread with the same staged context, model/reasoning/schema/permissions, cancellation owner and original ten-minute deadline. It can consume additional account quota. A bad correction or transport failure stops; exact source, relationships, visuals, independent storage validation and draft acceptance remain unchanged. Discovery, planning, PR and legacy/API requests do not enable the policy.

Current and persisted legacy mismatch errors receive plain workspace copy without generated section IDs. Failed questions and saved chapter bytes remain unchanged; the app does not resend them after restart.

## Automated validation

- Frontend suite: 134 passed; TypeScript/Vite production build passed (existing chunk-size warning).
- Native release suite: 210 passed, four opt-in tests ignored. Regressions cover the reported snake_case name, whole-identifier boundaries/qualified names/Unicode, evidence limits and section updates, a valid draft with zero correction calls, one-attempt failure/transport propagation, malformed/oversized candidates, exact fabricated-quote rejection after alignment, and independent revision validation.
- Native review rebuild rejects a modified correction policy even when the copied fingerprint/body still match the original review.
- Open Source documentation audit: zero errors, four existing review notes.

## Remaining focused validation

Opt-in synthetic transport, packaging and installed-app observations will be recorded here before delivery. Broader narrative/visual/language quality and release qualification remain open.
