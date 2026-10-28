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

## Cache-status and namespace correction — 25 October 2026

The packaged v44 reader briefly showed **Audio not prepared** after reopening a
saved section, then resolved to **Ready to play** with its 18-second Pocket clip.
The player waits 100 ms before asking native storage for the canonical cache
status, but had initialized that interval as `idle`, which misreported
“not prepared.” The UI now has a local `checking` state and shows
**Checking audio** until the status request resolves; `idle` remains the actual
no-cache result. Pocket audio now writes under
`pocket-narration-cache-v1`, while Gemini remains under its own cache version.
Reads also check the previous shared Gemini-named directory for a matching
Pocket fingerprint, preserving clips already created by older app builds.

Verification on this source:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib narration::tests::` —
  **31 passed**, including separation of the two new cache namespaces and reuse
  of a Pocket WAV from the previous shared location.
- `node --experimental-strip-types --test tests/narration.test.mjs
  tests/visualStory.test.mjs` — **60 passed**.
- `npm run build` — passed; Vite reports the existing large main-chunk warning.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and
  `git diff --check` — passed.

## v45 packaged cache-reuse recheck — 25 October 2026

The v45 release app and DMG built successfully; license notices, embedded
metadata and DMG checksum all passed. The installed executable matches the
bundle at SHA-256
`d1eb8c3a1c66ccc89095ea82d48f0da1394f913afa86936aa0375c8600d1b44d`.
After relaunch, Code Atlas restored Fontdone `main@5249b72371`, its existing
Code Lens section, and the Codex connection. Connections → Voice showed
**Pocket TTS — on this Mac, free**, **Ready to listen**, Alba, with text staying
on this Mac. The saved 18-second section showed **Ready to play** after the
restart. The on-disk narration tree still contains only the old shared
`gemini-narration-cache-v1` directory; no Pocket synthesis ran during this
check. Thus the installed build successfully found the saved section's matching
Pocket audio through the compatibility read, without generating another clip.

This did not capture the brief **Checking audio** state before the lookup
resolved, test offline restart, inspect a file-to-section hash independently,
exercise worker cancellation/failure, or perform a listening review. Keep those
acceptance gates open.

## Runtime-invalidation source checkpoint — 27 October 2026

When Pocket setup or repair changes the managed runtime identity, the client
queue now drops prewarm jobs that have not yet entered the native worker and
releases their deduplication slots. Already submitted work has no exposed cancel
command and remains governed by the native queue; replacing its runtime is not
claimed to interrupt active synthesis. No test/build or packaged runtime check
ran. Keep queue race, offline restart, latency and listening acceptance open.

## Saved voice-switch source checkpoint — 27 October 2026

Saving Gemini now broadcasts the updated global voice preference to the local
prewarm queue, including changes received through another app window's storage
event. Pocket jobs still waiting in the client queue are dropped and release
their deduplication slots. Requests already submitted to the native worker are
allowed to finish and remain cacheable; switching to Gemini does not start any
cloud narration request. No test/build or packaged transition check ran. Keep
the queue race and remaining packaged acceptance open.

## Invalid saved-voice transition — 27 October 2026

The cross-window voice-change listener now also drops queued Pocket work when
the stored preference cannot be parsed. The preference store exposes Pocket as
a display fallback while separately reporting the parse issue; treating that
fallback as an explicit Pocket choice could otherwise keep old queue entries
alive. Already submitted native work remains unaffected. No tests, build, or
packaged transition check ran; keep the queue race and remaining acceptance
open.

## Best-effort queue admission — 27 October 2026

Saved-answer adoption now catches synchronous errors from optional narration
prewarm admission. Native answer storage and workspace reconciliation remain
the success boundary; malformed local narration preferences or a queue
admission error cannot turn an already-saved section into a failed adoption.
Synthesis still runs asynchronously through the existing bounded worker. No
tests or build ran; keep audio failure, cancellation, packaged readiness and
restart acceptance open.

## Skip silent sections — 27 October 2026

Prewarm input construction now omits empty or whitespace-only narration scopes.
The Guide's intentionally silent evidence-boundary section therefore uses no
Pocket worker slot; nonempty scripts are passed unchanged so their audio key
continues to match player synthesis. This source-only refinement has no test or
packaged evidence. Queue bounds, failure recovery, latency and playback remain
open.

## Clear queued clips when any saved voice setting changes — 27 October 2026

The process-wide prewarm queue now drops all Pocket clips that have not yet
entered the native worker when the global Voice preference changes. This also
covers Pocket-to-Pocket edits: queued jobs retain their original script and
voice settings, so continuing them after a saved edit wastes worker capacity
even though their fingerprints would prevent stale playback reuse. Synthesis
already submitted to the native worker is allowed to finish and remains
cacheable under its original fingerprint. No tests, build, or packaged
transition check ran; queue race and remaining acceptance gates stay open.

## Align renderer and native queue bounds — 27 October 2026

The renderer previously retained up to 512 queued scripts even though the
native cache admits 48 prewarm jobs. The renderer cap now matches 48, which is
the documented maximum for five ready/ahead Guide slides plus one refill at
eight scenes per slide. Extra scripts remain unprepared until ordinary
on-demand playback requests them. This bounds duplicate/stale queued work in
both layers without changing cache fingerprints or the single native worker.
Source-only change; no tests, build or packaged queue-pressure check ran. Keep
capacity, cancellation and first-play latency acceptance open.

## Release playback reviews after prewarmed audio — 27 October 2026

The native playback command now removes its request review when it returns
from the prewarmed-audio path. That path can return a cached WAV (or a terminal
wait error) without passing through normal synthesis, which consumes reviews;
the player then drops its review reference. Repeated ready replays could
therefore leave entries in the 16-slot native review store until expiry and
block new reviews. This cleanup changes no audio or cache data. No tests or
build ran. Acceptance still needs a source regression check for review-slot
reuse and packaged verification of the preparing-to-ready transition,
first-section latency, canonical cache-key match, offline restart, queue
cancellation/shutdown/failure under load, manual playback, and listening
quality.

## Fast-path review-slot regression — 28 October 2026

Prewarmed playback and its per-playback review cleanup now share one
`load_prewarmed_audio_for_playback` path. If a prepared clip or terminal
prewarm result completes playback, the helper releases the review slot; if no
prewarmed result is available, the normal synthesis path still owns and
consumes the review. A focused native regression fills all 16 review slots,
plays a cached Pocket clip through the fast path, then verifies a new review is
admitted. The test has been added but not run in this checkpoint; a full Rust
build was deferred to avoid recreating the 5.1 GB local target cache just
removed for disk recovery. Packaged timing, offline restart, queue cancellation
and failure under load, and listening-quality acceptance remain open.

## Disabled pronunciation queue deduplication — 28 October 2026

Prewarm input construction now removes pronunciation rows marked `include: false` before client queue admission. Native fingerprinting ignores those rows, but the renderer's validation could classify a disabled empty row as an invalid lexicon and assign an otherwise identical script a separate queue key. That wasted one of the bounded prewarm slots. A regression compares inputs with and without a disabled empty row and verifies that the queue admits only one copy of each authored scene. The initial direct test launch exposed a Node 24 resolution failure for an extensionless runtime import; that import now names `narration.ts` explicitly. The paired workspace tests then exposed a stale harness stub that swallowed injected prewarm callbacks; it now records those callbacks without loading or starting the real native worker. The focused prewarm and workspace files pass together: 9/9. No synthesis or packaged check ran. Keep the issue open for queue cancellation/failure, offline restart, latency, and listening acceptance.
