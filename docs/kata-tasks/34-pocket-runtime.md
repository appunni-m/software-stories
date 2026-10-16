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

## Current native evidence — 2026-10-16

The release app's **Set up local narration** action completed Python, package and pinned model/Alba preparation and displayed **Pocket TTS is ready**. The initial native Python failure was caused by passing both `--managed-python` and `UV_PYTHON_PREFERENCE=only-managed`; the installer now uses only the enforced environment preference. The corrected command passed and a regression test covers the exact argument/environment construction. Full `npm run check` passed (98 frontend, 185 Rust passed, two ignored), and unsigned `.app`/DMG packaging passed.

This run used an existing uv installation on Apple Silicon macOS, not a clean-account/bundled installer. The first native synthesis exposed inference-mode tensors being mutated by Pocket’s background no-grad cache worker. The wrapper now uses `torch.no_grad()`; the corrected offline worker produced a valid 4-second synthetic PCM16/24 kHz mono WAV. A source regression guards the context choice. All broader release, boundary, cancellation/concurrency and listening criteria below remain open. See [dated evidence](../acceptance/2026-10-16-codex-validation.md).

## Implementation update — 2026-10-15

`src-tauri/src/pocket_tts.rs` embeds `resources/pocket_tts_worker.py`. Explicit setup/repair downloads Python 3.13.14, pinned core packages and immutable English model/Alba assets into app data, writes readiness only after model initialization, and exposes setup phases. Synthesis uses a finite offline worker per segment, bounded stdin/WAV output, process-group cancellation/reaping and app-exit cleanup, with no listener or cloud fallback. Status reads files without launching Python. The target is Apple Silicon macOS; an existing `uv` executable is required. See [the runtime contract and full pins](../research/pocket-tts-runtime.md).

Remaining broader qualification work:

- [ ] Package and qualify the installer/runtime for the supported Mac target: pin/verify or bundle `uv`, complete the transitive artifact/hash lock, retain licenses, and define signing/notarization/quarantine behavior.
- [ ] Establish minimum macOS, wheel compatibility, disk/download/memory requirements, update/cache cleanup and uninstall behavior on a clean account without developer caches.
- [ ] Define whole-section ownership/queueing. Current exclusivity is per finite segment; the model reloads each invocation and another window can acquire the runtime between segments. Warm reuse is not implemented.
- [ ] When authorized, exercise fresh setup, missing installer, network/low-disk failure, cancelled downloads, retry/repair, changed packages/models and incomplete manifests.
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
