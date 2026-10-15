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

## Current evidence
An isolated Pocket TTS 3.3.0 CLI sample and a cached offline CLI sample were generated before this static-review pass. The app still calls Google TTS; there is no Pocket adapter or packaged runtime yet. Those samples are not integration acceptance.

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
