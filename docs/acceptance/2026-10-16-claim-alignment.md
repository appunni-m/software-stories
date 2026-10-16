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

## Real Codex correction

The opt-in native test `installed_codex_corrects_identifier_mismatch_in_same_operation` passed in 17.91 seconds using the existing official installed Codex executable, GPT-6 Luna and max reasoning. Only an isolated synthetic greeting fixture was shared. The test deliberately requested an inconsistent initial claim, observed exactly one correction turn, and validated the corrected full chapter against the captured source and the independent revision validator. It did not use an API key, another model, a replacement Codex installation or the user's repository. This exercises the actual same-thread transport; it does not establish a production success rate or semantic quality benchmark.

Code commit: `a696b69`. Local logs: `/private/tmp/code-atlas-claim-repair-rust.log`, `/private/tmp/code-atlas-claim-repair-live.log`, `/private/tmp/code-atlas-claim-repair-ui-tests.log`, `/private/tmp/code-atlas-claim-repair-build.log`.

## Packaged app observation

`npm run bundle:macos` succeeded, including bundle metadata and DMG checksum verification. The unsigned app was installed at `/Users/lazytrot/Applications/Code Atlas.app` from code commit `a696b69` with executable SHA-256 `ac73ec0eed68e5e3d9897c36048b7f87d95211459274fbff11d21c8faccb0a53`. The previous build remains at `/Users/lazytrot/Applications/.Code Atlas.before-claim-repair-35aa23f.app`. Build log: `/private/tmp/code-atlas-claim-repair-native.log`; local install identity: `/private/tmp/code-atlas-workspace-installed.json`.

Reopening restored the selected repository/snapshot, all three questions and selected source context. Codex returned to connected state without a sign-in or setup selection. Both existing identifier failures now show readable copy directly on their answer cards. Chat history uses the same readable copy. The reported section UUID was absent from both views. The native app is left on Guide with the retry action available. No question in the user's repository was resent and no existing chapter was accepted or rewritten.

This completes the narrow correction task `tmck`. Parent quality `q0sm` and workspace `na54` remain open. The live result establishes a synthetic two-turn correction path and source admission, not a real-repository success rate, semantic truth, narrative/visual/language quality, full accessibility or signed distribution.
