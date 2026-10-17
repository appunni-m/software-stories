# Progressive answer storage checkpoint — 17 October 2026

Status: native foundation implemented; progressive generation and reader integration remain open in Kata `3bz5`. This checkpoint does not replace the installed app or qualify an end-to-end partial answer.

This report records commit `00325e6`. The subsequent [native protocol checkpoint](2026-10-17-progressive-section-protocol.md) adds versioned tool dispatch and shared native publication ownership; its later verification supersedes the storage-only integration status below.

## What this change establishes

`evidence_ledger::sections` admits one complete draft section using the existing native citation resolver, claim alignment checks, chapter canonicalizer, exact delivered-range verifier and full chapter/diagram validator. A rejected claim or visual rejects the section. The caller's original proposal is retained with bounded fixed findings; the implementation does not delete an offending claim, rewrite identifiers or substitute source.

The private `answer-sections/<operation>.json` record contains the native operation, immutable snapshot, grant, account/runtime and GPT-6 Luna/max binding; exact structured proposal bytes and digests; stable keys and ordinals; and each published draft's content origin and immutable evidence checkpoint. Later ledger appends cannot change that checkpoint. Inspection replays the proposal and validates captured source, claims, diagrams and the content digest locally, without acquiring a current grant or starting inference. A saved record cannot reconstruct an active publisher.

Admission is bounded to six unique section keys, eight proposal attempts and eight saved answer records, each capped at 4 MiB. A repeated identical proposal returns the same generation identity. Changed content under an existing key requires the previous generation identity and appends an immutable revision. Capacity, malformed records and stale prepared writes preserve existing bytes. The eight-answer retention interface remains to be implemented before enabling this path for readers.

Whole-section preparation is separate from commit. Commit rechecks the live grant, cancellation, deadline, runtime and manifest, then compares the exact previous memory and disk digest before replacing the private file. The existing private-file helper now cleans up only files it created on a failed write, syncs a replacement before rename, and syncs the parent directory before confirming durable replacement. A failure after rename preserves the destination for inspection and must be treated as uncertain, not replayed.

## Regression evidence

The complete native library suite passed **318 tests**, with **11 explicit live probes ignored**, in 29.83 seconds. This includes **14 new section-storage regressions**. Command: `cargo test --lib` under `src-tauri`, with `RUSTC_WRAPPER=`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` and `CARGO_BUILD_JOBS=2`. Local log: `/private/tmp/code-atlas-progressive-sections-native.log`.

The production library also passes `cargo check --lib` under the same environment (38.75 seconds). It reports 46 unused-code warnings, including the section foundation that has no production caller yet; this is not a warning-free build. Local log: `/private/tmp/code-atlas-progressive-sections-check.log`. The documentation audit reports zero errors and six existing review findings.

The new synthetic regressions cover:

- Exact source and visual replay after subsequent reads, removal of the mutable ledger, and grant revocation.
- A later invalid claim, foreign citation, malformed proposal, invalid visual or ordinal without changing an earlier section.
- Exact duplicate identity, explicit immutable revisions and key conflicts.
- Stop, revoked permission, changed account/runtime, changed snapshot and expired deadline between preparation and commit.
- Six-section/eight-attempt/eight-record limits, oversized input/record and excessive directory entries.
- Corrupt storage, symlinks, stale preparations and uncertain source delivery.
- Proposal, checkpoint, delivery state, content, edge, origin and binding tampering.
- A failed final outcome that cannot erase an earlier section; terminal records cannot publish again.

No live Codex call, model fallback, narration request, automatic playback or additional app server was used for this checkpoint.

## Remaining integration gates

The proposal tool and its versioned six-tool catalog are not enabled. The native operation lock shared with Stop must cover section commit before an event or model reply can be sent. The storage primitive's rechecks alone do not qualify that concurrent cancellation gate.

The agent loop still returns one final chapter. Background section events, Guide/Flow cards during investigation, durable draft adoption, restart reconciliation, retention controls and incomplete-topic presentation remain pending. The existing five-tool receipts and saved chapters remain unchanged. A new progressive protocol and receipt version must preserve historical evidence-ID and origin verification.

The failed Fontdone native-citation run remains failed. Fresh frozen Fontdone attached-function and broad questions, useful first-section timing within 120 seconds, independent semantic/diagram review, actual Pocket script/audio, nested reading position, keyboard and VoiceOver checks remain required. This checkpoint establishes storage behavior only; exact source admission does not prove explanation quality.
