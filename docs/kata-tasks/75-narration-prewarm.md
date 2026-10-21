---
kind: task
key: narration-prewarm
title: Prewarm local narration while an explanation is generated
priority: 1
labels: cache, latency, speech
parent: dbmd
---

## User outcome

The explanation is ready to follow while local Pocket TTS prepares narration in the background, so the first Play does not hold up the reader.

## Requirements

- Start preparation after each validated narration unit is checkpointed. Audio must not delay or fail answer publication.
- Use the saved global Voice connection and prewarm only the explicitly selected Pocket path. Keep cloud speech behind its exact-text review and consent.
- Reuse the canonical audio fingerprint and cache. Skip matching audio, separate changed scripts or runtime settings, and preserve earlier revision audio.
- Reuse the single bounded Pocket worker. Coalesce duplicate fingerprints; bound the queue; handle cancellation, shutdown, navigation and worker failure without partial audio or a stranded queue.
- Keep playback manual and source exploration available. Show a plain preparing, ready, or unavailable state; retain on-demand Play when a clip is not ready.

## Acceptance

- A delayed-worker fixture shows the answer before synthesis completes and verifies a cache hit after completion without a second synthesis.
- Script, pronunciation, voice/model/runtime, segmentation or output-format changes cannot reuse stale audio.
- Queue bounds, duplicate coalescing, cancellation, shutdown and failure preserve the transcript, citations, earlier audio and future queue progress.
- Cloud selection causes no background request and does not weaken consent.
- In the packaged app, record the status transition and first-section latency while reading; verify manual playback and offline cache reuse.

Do not treat after-first-play caching as generation-time prewarming. The 21 October Fontdone observation in [the Pocket player/cache task](35-pocket-player-cache.md) is evidence of a ready cached section, but does not measure the exact transition, offline restart, or cancellation/failure under load. Keep this child issue open until the remaining criteria are observed.
