# Owned progressive section protocol — 17 October 2026

Status: native protocol and synthetic transport checks implemented. The current workspace command and installed app still use the legacy final-chapter path. Kata `3bz5` remains open; no live progressive Fontdone answer is qualified here.

## Native implementation

The internal `Session::new_progressive` constructor selects a separate six-tool catalog. `propose_answer_section` accepts `answerSectionV1` structured arguments: a stable key, zero-based order, optional previous generation identity, and one complete section containing title, claims and visual. The native wrapper derives presentation from the validated anchored claims. The tool saves an unaccepted draft; it does not change repository files, fetch additional source, accept a chapter, synthesize speech or play audio.

Proposals use the same owned event verification, private socket, original 64-call/ten-minute limit and source accounting as the five reading tools. Six unique section keys and eight proposal attempts remain the section limits. The explicit helper flag is `--atlas-answer-mcp`; its envelope is capped at 34,000 bytes and structured proposal arguments at 32,768 bytes. Reading-tool requests retain their 8,192-byte envelope even in this mode. The legacy `--atlas-context-mcp` helper and five-tool catalog retain their original limits.

The native manager installs publication authority only for its active operation. Binding checks include operation, request digest, account/runtime and the exact cancellation owner. Saved records cannot construct that authority. Section commit holds the same native lock as Stop and permission revocation. Live scope, grant and runtime gates run before and after the durable action, and the owner checks the original run deadline again before reporting success. If the durable write succeeds but a later gate fails, the draft remains inspectable; neither a successful reply nor a section notice is emitted. No automatic investigation or tool replay starts.

The source-free notice is delivered only after persistence and includes stable draft identities. Its `runElapsedMs` is observed on the original native run clock after commit; `contextElapsedMs` names the earlier captured-code preparation clock and excludes prior setup. Neither field alone establishes end-to-end time from the reader's submission or explanation usefulness.

## Compatibility

New ledgers use schema 3 and contract `codex-retrieval-v3-answer-sections`; immutable section origins use schema 5. Schema-2 evidence IDs continue to use the fixed `codex-retrieval-v2-native-citations` identity, and schema-1 IDs retain their original project/range identity. Origin versions 1–4 remain readable. A legacy active session cannot gain the sixth tool or publish a progressive receipt. The new final response skips legacy whole-chapter candidate capture; legacy requests keep their sole correction path.

No native command selects this constructor yet. The installed helper must be rebuilt and installed before a live progressive account probe; the currently installed helper does not implement this new flag/catalog. No saved chapter, grant or historical failed candidate is migrated or replayed by this change.

## Verification

- Complete native suite: **326 passed, 11 explicit live probes ignored**, 31.53 seconds. `cargo test --lib`, using `RUSTC_WRAPPER=`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` and `CARGO_BUILD_JOBS=2`. Local log: `/private/tmp/code-atlas-progressive-protocol-native.log`.
- Production native library: `cargo check --lib` passed in 6.61 seconds under the same environment. It reports 24 unused-code warnings, including the not-yet-connected generation path. Local log: `/private/tmp/code-atlas-progressive-protocol-check.log`.
- Frontend regressions: **145 passed**. Production TypeScript/Vite build passed, retaining the existing large-chunk warning. Logs: `/private/tmp/code-atlas-progressive-protocol-frontend.log` and `/private/tmp/code-atlas-progressive-protocol-build.log`.

The synthetic private-socket regression reads captured source, publishes and independently inspects a section before its notice, verifies the delivered reply against the native ledger, repeats it with the same identity, and rejects a later foreign reference without losing earlier work. It then simulates a scope failure after another durable section commit: the section remains inspectable, but no reply or notice escapes. Separate regressions cover catalog isolation, larger-envelope opt-in, historical ID/receipt verification, mismatched runtime/cancellation ownership, stale tokens, Stop ordering, shared permission ordering, shutdown and a deadline crossed during publication. The section-store regressions continue to cover immutable replay, bounds and tampering.

These tests use synthetic native owners and source fixtures, not a real Codex account or model. No new app server, live model request, API fallback, narration request or playback was started.

## Remaining delivery gates

Implement the progressive request/final manifest and background command, terminal complete/partial/stopped/failed outcomes, explicit native draft adoption, Tauri section events, Guide/Flow cards while work continues, reading-position preservation, restart reconciliation and bounded retention controls. A stopped or failed run must retain committed sections without restarting inference. The existing final-chapter schema/instructions cannot serve as the progressive final manifest.

Then build/install the helper and app, qualify frozen Fontdone attached-function and broad questions, independently review actual meaning and diagrams, measure a useful first section from submission within 120 seconds, check nested source navigation and restart, and review the actual Pocket script/audio plus keyboard/VoiceOver behavior. The historical failed Fontdone candidate remains failed. See the [task's full acceptance contract](../kata-tasks/73-progressive-answer-sections.md).
