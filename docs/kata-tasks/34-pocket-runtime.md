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

## Current native evidence — 2026-10-16

The release app's **Set up local narration** action completed Python, package and pinned model/Alba preparation and displayed **Pocket TTS is ready**. The initial native Python failure was caused by passing both `--managed-python` and `UV_PYTHON_PREFERENCE=only-managed`; the installer now uses only the enforced environment preference. The corrected command passed and a regression test covers the exact argument/environment construction. Full `npm run check` passed (98 frontend, 185 Rust passed, two ignored), and unsigned `.app`/DMG packaging passed.

This run used an existing uv installation on Apple Silicon macOS, not a clean-account/bundled installer. The first native synthesis exposed inference-mode tensors being mutated by Pocket’s background no-grad cache worker. The wrapper now uses `torch.no_grad()`; the corrected offline worker produced a valid 4-second synthetic PCM16/24 kHz mono WAV. A source regression guards the context choice. The final app at `~/Applications/Code Atlas.app` generated and played an 8.85-second accepted section, replayed/paused it and reused its local cache after section navigation. These are synthetic functional baselines. Broader release, boundary, cancellation/concurrency and listening criteria below remain open. See [dated evidence](../acceptance/2026-10-16-codex-validation.md).

## Implementation update — 2026-10-15

`src-tauri/src/pocket_tts.rs` embeds `resources/pocket_tts_worker.py`. Explicit setup/repair downloads Python 3.13.14, pinned core packages and immutable English model/Alba assets into app data, writes readiness only after model initialization, and exposes setup phases. Synthesis uses a finite offline worker per segment, bounded stdin/WAV output, process-group cancellation/reaping and app-exit cleanup, with no listener or cloud fallback. Status reads files without launching Python. The target is Apple Silicon macOS; an existing `uv` executable is required. See [the runtime contract and full pins](../research/pocket-tts-runtime.md).

Remaining broader qualification work:

- [x] Pin and verify the app-managed uv setup utility and retain its MIT/Apache notices in bundle resources. Source/unit tests and a local extracted-binary smoke check pass.
- [ ] Complete the transitive Python artifact/hash lock, inspect packaged notices, and qualify signing/notarization/quarantine behavior for the supported Mac target.
- [ ] Establish minimum macOS, wheel compatibility, disk/download/memory requirements, update/cache cleanup and uninstall behavior on a clean account without developer caches.
- [x] Define and implement whole-script ownership with a bounded queue and warm model/voice reuse within that script. Isolated packet/queue/cancellation and real-worker checks are recorded above; installed cross-window/setup interleaving, app exit and player/cache behavior remain qualification work.
- [ ] When authorized, exercise fresh packaged setup, missing system curl/tar, network/low-disk failure, cancelled downloads, retry/repair, changed packages/models and incomplete manifests.
- [ ] Record offline synthesis, bounded output, repeated cancellation, timeout, window navigation and actual app-exit child cleanup; accept only complete validated WAV output.

The original October 15 implementation was source-only. October 16 native setup and automated checks above supersede that earlier checkpoint; they do not close this task.

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
