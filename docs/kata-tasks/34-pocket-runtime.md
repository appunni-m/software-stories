---
kind: task
key: pocket-runtime
parent: t18
title: Integrate one managed local Pocket TTS runtime
priority: 1
labels: speech, local, runtime
---

## User outcome
The Mac app generates narration locally with Pocket TTS, without an API key or a separate listening port.

## Whole-script ownership checkpoint — 2026-10-17

One finite offline worker now owns a complete reviewed script (1–64 segments,
4,000 UTF-8 bytes per segment / 16,000 total), loading model/voice once and
preserving exact order with 200 ms pauses. Setup and synthesis share one owner
with four waiting slots. Queued cancellation starts no process; active work
retains ownership through child reaping. Nonblocking stdin observes the same
five-minute deadline as generation. Duplicate/future/oversized packets, empty
or nonfinite segments and partial WAV output are rejected. Worker fingerprint
changes invalidate new local review/cache lookup without rewriting old files.

Six deterministic Python contract tests and focused native queue/packet/stalled
pipe cancellation/reaping tests pass. A real pinned worker separately generated
a valid 13.08-second sample from two explicitly synthetic segments in 12.679 s,
using isolated output/config and existing interpreter/assets read-only. This is
not installed playback, clean setup, cache/export or human listening evidence.
[Exact changes, evidence and remaining gates](../acceptance/2026-10-17-pocket-whole-script.md).
Keep this task open. The earlier per-segment description below is historical.

## App-managed setup tool — 2026-10-21

Explicit setup now downloads Astral uv 0.13.4 for Apple Silicon into app data.
The fixed release archive is capped and SHA-256 verified before extracting only
the expected `uv` executable. A local manifest records the release and binary
hash; later setup reuses it only if the executable still matches. Curl user
configuration is disabled, redirects stay on HTTPS, and cancellation reaps the
download/extraction processes. The Tauri bundle includes both upstream MIT and
Apache notices. The official archive was checksum-verified and the extracted
binary reported `uv 0.13.4` in a local smoke check. Four focused Rust tests
passed for binary tampering, release mismatch, symlink rejection and setup
availability without a global `uv` install. This is source and local-tool
evidence, not packaged clean-account setup.

## Reproducible Python dependencies — 2026-10-21

The runtime now includes a generated lock for all 49 dependencies resolved from
the five direct pins in
[`pocket-tts-requirements-macos-aarch64-py313.in`](../../src-tauri/resources/pocket-tts-requirements-macos-aarch64-py313.in).
The adjacent lock includes SHA-256 hashes and is installed with `uv pip install
--require-hashes --only-binary :all:`. Rust writes that exact lock into app
storage and checks its SHA-256 against both the readiness manifest and the
compiled lock before reporting the runtime ready. The worker receives the same
digest and rejects synthesis if the manifest differs. Lock changes also change
the runtime fingerprint used for narration-cache identity.

The lock was resolved with uv 0.11.29 for Python 3.13.14 and macOS using
`MACOSX_DEPLOYMENT_TARGET=14.0` and the `aarch64-apple-darwin` target. The selected PyTorch wheel is tagged for
macOS 14 on Apple Silicon; the app bundle now declares macOS 14 as its minimum.
Source/unit evidence does not qualify a clean-account packaged setup or an
offline reinstall, so this task remains open.

Focused verification on 2026-10-21: all 14 `pocket_tts::tests` pass. The uv
hash-enforced dry run resolves all 49 locked packages for Python 3.13.14 on
Apple Silicon macOS, and both worker files pass Python syntax compilation. The
worker contract suite could not run under the available Python because PyTorch
is not installed in that interpreter. `cargo fmt --check` also reports existing
formatting differences across unrelated Rust modules; the changed Pocket module
passes standalone rustfmt. No packaged setup or fresh account was exercised.

## Current native evidence — 2026-10-16

The release app's **Set up local narration** action completed Python, package and pinned model/Alba preparation and displayed **Pocket TTS is ready**. The initial native Python failure was caused by passing both `--managed-python` and `UV_PYTHON_PREFERENCE=only-managed`; the installer now uses only the enforced environment preference. The corrected command passed and a regression test covers the exact argument/environment construction. Full `npm run check` passed (98 frontend, 185 Rust passed, two ignored), and unsigned `.app`/DMG packaging passed.

This run used an existing uv installation on Apple Silicon macOS, not a clean-account/bundled installer. The first native synthesis exposed inference-mode tensors being mutated by Pocket’s background no-grad cache worker. The wrapper now uses `torch.no_grad()`; the corrected offline worker produced a valid 4-second synthetic PCM16/24 kHz mono WAV. A source regression guards the context choice. The final app at `~/Applications/Code Atlas.app` generated and played an 8.85-second accepted section, replayed/paused it and reused its local cache after section navigation. These are synthetic functional baselines. Broader release, boundary, cancellation/concurrency and listening criteria below remain open. See [dated evidence](../acceptance/2026-10-16-codex-validation.md).

## Implementation update — 2026-10-15

`src-tauri/src/pocket_tts.rs` embeds `resources/pocket_tts_worker.py`. Explicit setup/repair downloads Python 3.13.14, pinned core packages and immutable English model/Alba assets into app data, writes readiness only after model initialization, and exposes setup phases. Synthesis uses a finite offline worker per segment, bounded stdin/WAV output, process-group cancellation/reaping and app-exit cleanup, with no listener or cloud fallback. Status reads files without launching Python. This October 15 entry is historical: the current app-managed `uv` bootstrap, complete hash lock and macOS minimum are recorded above. See [the runtime contract and full pins](../research/pocket-tts-runtime.md).

Remaining broader qualification work:

- [x] Pin and verify the app-managed uv setup utility and retain its MIT/Apache notices in bundle resources. Source/unit tests and a local extracted-binary smoke check pass.
- [x] Commit a transitive Python artifact/hash lock, require hashes and binary wheels during install, and bind runtime readiness/cache identity to its digest. The lock has source/unit evidence; clean-account install remains unqualified.
- [x] Declare macOS 14 as the supported minimum for the selected Apple Silicon PyTorch wheel.
- [ ] Qualify actual wheel installation, disk/download/memory requirements, update/cache cleanup and uninstall behavior on a clean account without developer caches.
- [ ] Inspect the packaged dependency notices and qualify signing, notarization and quarantine behavior for the supported Mac target.
- [x] Define and implement whole-script ownership with a bounded queue and warm model/voice reuse within that script. Isolated packet/queue/cancellation and real-worker checks are recorded above; installed cross-window/setup interleaving, app exit and player/cache behavior remain qualification work.
- [ ] When authorized, exercise fresh packaged setup, missing system curl/tar, network/low-disk failure, cancelled downloads, retry/repair, changed packages/models and incomplete manifests.
- [ ] Record offline synthesis, bounded output, repeated cancellation, timeout, window navigation and actual app-exit child cleanup; accept only complete validated WAV output.

The original October 15 implementation was source-only. October 16 native setup and automated checks above supersede that earlier checkpoint; they do not close this task.

## Installed-size estimate — 29 October 2026

The completed app-managed runtime on this Apple Silicon Mac occupies
1,116,380 KiB (about 1.1 GB): 796,564 KiB for the virtual environment,
220,188 KiB for model/voice assets, and 27,236 KiB for the pinned setup tool.
The Voice connection screen tells the user the size needed for setup, that the
prepared voice works offline, and that local synthesis has no provider usage
fee. This is one post-setup measurement, not a clean-install peak or a
cross-machine measurement. Temporary disk/download requirements and memory
remain unqualified.

## Source implementation — runtime removal — 30 October 2026

Connections → Voice now offers **Remove local voice** for a verified Pocket
runtime, behind an explicit confirmation that states the approximate storage
reclaimed and what happens to saved audio. The Tauri operation shares the
exclusive Pocket worker queue with setup and synthesis. It deletes only
`pocket-tts/runtime-3.3.0-v1`; the separate
`pocket-narration-cache-v1` audio cache and other app data remain. Missing
runtime paths are treated as already removed. The removal helper rejects a
symlinked app-storage/runtime parent and unlinks a symlink at the runtime leaf
without following it. Errors tell the user to check permissions and retry.

Focused source evidence on 30 October: `npm run build` passed;
`cargo test --manifest-path src-tauri/Cargo.toml --lib
pocket_tts::tests::removing_runtime -- --nocapture` passed all three matching
tests for cache preservation/idempotence and symlink handling; and
`rustfmt --edition 2024 --config skip_children=true --check
src-tauri/src/pocket_tts.rs` passed. This does not verify the packaged dialog or
remove the runtime from a user's Mac. Clean-account setup, update/recovery,
offline synthesis, and packaged uninstall/lifecycle acceptance remain open;
keep the task open.

## Contract
Pin Pocket TTS 3.3.0, a compatible Python runtime, exact model/config identity and a vetted built-in voice. Provide app-managed installation/download progress, cancellation, retry and actionable missing-runtime/disk errors. Use one managed worker with bounded input/output/concurrency and no network listener. Reuse loaded resources where feasible; cancel active work and reap children on cancellation/app exit. Never execute repository-provided scripts/config or accept arbitrary voice/model URLs. Keep runtime and model storage separate from repository data.

## Acceptance criteria
- Setup shows versions, download/storage location and expected requirements; interruption can recover without treating partial assets as installed.
- The packaged app finds its own runtime/assets without relying on developer caches, shell activation or a globally installed Python.
- After setup, synthesis works with network disabled and makes no cloud request or API-key lookup. Missing assets never trigger a silent paid fallback.
- One worker and bounded queue serve all sections; failures/cancellation leave transcript and previous audio intact and no orphan process/listening port.
- Output is bounded, validated PCM WAV compatible with the existing player/exporter.
- Record code/model/voice license and required attribution. Verify Alba asset attribution if selected; no arbitrary voice cloning.
- Document supported Mac architectures, initial disk/download requirements, runtime update identity, cache cleanup and uninstall behavior.

## Excludes
Implementing a new TTS model, voice cloning, automatic Google fallback and counting a CLI-only sample as app support.

## Done evidence
Packaged-app setup, offline synthesis, failure/cancel/process-cleanup evidence and licenses when testing is authorized.
