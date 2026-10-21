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

## Focused prewarm checks — 2026-10-21

Current-source verification:

- `node --experimental-strip-types --test tests/workspace-narration-prewarm.test.mjs tests/narrationPrewarm.test.mjs`: **6 passed**. The workspace publishes an answer before a delayed local TTS request resolves, warms newly adopted progressive-guide sections, avoids cloud work, and derives the same authored scene scripts and saved pronunciation edits as the player.
- `cargo test --lib narration::tests::`: **28 passed**. Includes delayed synthesis followed by a cache hit and replay with one worker run, generation-time matching-cache reuse without queueing, text/voice/style/pronunciation invalidation, bounded/coalesced queue behavior, cancellation, shutdown and worker failure recovery.

These are source-level fixtures. The prior installed Fontdone observation saw a newly published section ready to play, but did not match the WAV to that section's canonical fingerprint or time the exact preparing-to-ready transition. Packaged status/latency, restart/offline reuse and cancellation/failure under load remain open.

## Packaged first-section timing — 2026-10-21

One approved request against the packaged app and Fontdone `main@5249b72371`
used GPT-6 Luna/max and produced a single Guide section. The saved native run
record (`d456b9ba-12ac-4633-83c2-62aa51d49fa8`) reports publication at
174,204 ms (2m54s), exceeding the 120-second first-section target. The recorded
source reads totaled 10,395 bytes; this was a focused function/helper question,
not a broad repository explanation.

The saved revision appeared at 06:44:38 IST and a 679,724-byte local WAV appeared
at 06:44:44 IST. Connections showed **Pocket TTS — on this Mac** with Alba. The
Guide already showed **Ready to play** before Play; the first Play click went
directly to **Playing**, and the clip reached 14.15 seconds. This is packaged
evidence consistent with post-publication local prewarming and immediate manual
playback. It does not identify the WAV by the section's canonical cache key,
capture the exact preparing-to-ready transition, or prove offline reuse after
restart. No listening-quality, worker-failure, cancellation, or shutdown check
was performed. Keep this task open.

## Additional fresh-section prewarm check — 2026-10-21

With explicit user approval, one focused Codex request on the packaged app and
Fontdone `main@5249b72371` generated a fresh Guide section about
`parse_table_directory_at` in `src/tt/mod.rs`. The request started at
01:39:27 UTC and was still building at 01:39:46 UTC. By the first completed
observation at about 01:40:46 UTC, the section was visible and scene 1 already
showed **Ready to play** at 00:00. The four authored visual scenes had audio
durations of 9, 7, 8, and 12 seconds. By about 01:41:37 UTC, every scene showed
**Ready to play** at 00:00 without having been played. There was no autoplay.

Pocket TTS was selected in Connections as **Pocket TTS — on this Mac, free**
(Alba). Clicking Play on scene 4 immediately changed the player to **Playing**
at 00:00/00:12; playback was then paused. This is packaged evidence that the
fresh section's scene audio was ready before manual playback. No audio or
generated narration was added to the repository.

This run did not record the exact preparing-to-ready transition, inspect the
canonical cache key or worker cache-hit counter, test offline/restart reuse, or
exercise failure, cancellation, shutdown, and subjective listening quality.
The section remains in the app for review; keep the issue open.
