---
kind: task
key: pocket-player-cache
parent: t18
title: Make Pocket TTS the default narration path with distinct cache identity
priority: 1
labels: speech, player, cache
blocked_by: pocket-runtime
---

## User outcome
The reader can review a section script, generate natural local audio and reuse it without configuring Google.

## Native progress — 2026-10-16

Managed Pocket setup now completed inside the release app. Setup option conflict and streaming-worker context failures were reproduced and fixed; full checks pass (98 frontend, 185 Rust passed, two ignored). Local Play now leads to a simple exact-text review, with editing/audio hashes and runtime metadata collapsed. There is no duplicate local request-review button or Google consent requirement. The final app at `~/Applications/Code Atlas.app` generated/played an 8.85-second accepted section, replayed it, paused at 3.4 seconds, reset to zero with Stop and reused Audio cached after switching sections and returning. It did not advance sections on audio completion. Human listening, invalidation, cancellation, concurrency and offline export acceptance remain open; keep this task open.

## Current evidence
The player, commands, narration review/cache and preference parser now have distinct local Pocket and explicitly selected Gemini paths. This is implemented source awaiting acceptance; the previous Google-only description no longer applies.

## Implementation update — 2026-10-15

New section preferences default to Pocket/Alba. Legacy Google preferences and cache keys retain their provider identity, and switching engines preserves delivery settings. Local review does not require a key or Google consent; unsupported style controls are hidden and stored Pocket values normalize to the fixed local contract. Setup/repair uses an operation-specific cancellation ID, with status polling and stale/unmount guards. Generation/cache identity includes the runtime fingerprint, model/voice, final text and segmentation/PCM contract. Only complete joined WAV sections are cached. The existing export scan reads backend-aware preferences, reviews the matching local cache and never synthesizes audio.

Remaining work, with every acceptance criterion still open:

- [ ] Complete the runtime dependency and packaged setup qualification in the runtime child task.
- [ ] Update setup/help and the narration run sheet for local default, repair, runtime prerequisites, limitations and voice attribution; verify every user-facing recovery action.
- [ ] Later exercise legacy Google migration and engine round trips, local review without key/network, cache reuse/invalidation after script/lexicon/runtime changes, and invalid/oversized/corrupt cache omission.
- [ ] Exercise setup/generation cancellation before native registration, section/revision navigation, late results, retry and concurrent windows without accepting stale or partial audio.
- [ ] Verify playback/seek/captions and cached transcript matching in the native app, then export and play the selected backend's matching audio while disconnected.
- [ ] Record a human listening review of real code explanations, identifiers/acronyms, pronunciation substitutions and pacing.

The October 15 implementation was source-only; the October 16 progress above supersedes that historical restriction.

## Contract
Add provider-aware narration review/command contracts and select pocket-tts by default. Local setup states are not-installed/downloading/ready/unavailable/generating; explain recovery in product language. Show exact pronunciation-expanded script and deterministic segments before local generation. Remove Gemini key/billing/remote-consent controls from the local flow. Hide unsupported natural-language delivery style instead of silently ignoring it. Keep transcript, pause/resume/replay/seek and deliberate next-section navigation.

Cache identity includes backend, pinned runtime/model/config/voice asset identity, final text, segmentation/joining policy and PCM format. Old Google cache entries remain distinct. Code-only changes reuse audio if speech inputs are unchanged; changing speech inputs generates a new key. Export resolves the selected backend's matching cache without triggering synthesis.

## Acceptance criteria
- A fresh user chooses local narration and reaches script review without an API key or Google request.
- Only documented Pocket options are selectable; unsupported stored Google style/voice is not silently applied to Pocket.
- Provider/version/voice/text/lexicon/segmentation/format changes invalidate review/cache identity; unchanged inputs reuse valid audio.
- Section/revision navigation cancels or discards obsolete work, does not autoplay late output, and preserves previous audio and transcript.
- Offline export includes matching local audio with manifest hashes; missing/corrupt audio is explicitly omitted.
- Human listening review covers actual behavior explanations, identifiers/acronyms and pacing; do not read raw code by default.
- Update narration setup/help/run sheet to accurately distinguish local synthesis and any retained explicitly selected cloud option.

## Excludes
Removing transcript-only use, automatic whole-guide synthesis, automatic cloud fallback, and silently remapping unsupported controls.

## Done evidence
Packaged-app local narration and export walkthrough with recorded cache/cancellation/listening results after runtime integration; no tests run this pass.


## Connections simplification — October 16
Voice is an application connection, configured once in the header's Connections → Voice. Default Pocket/Alba stays local and free; Google is an explicit alternative with Keychain key, voice and optional style. Answer/player surfaces must not contain an engine selector, runtime download/repair controls, delivery settings or setup metadata. Keep transcript editing and optional pronunciation beside the owning section. Pressing Listen performs native local review, reuses matching audio or generates and plays it; Google still needs exact-text consent before transmission. The visible local transcript reflects pronunciation expansion. No cloud fallback or whole-guide synthesis.

One versioned global preference controls all answers and offline cache scans. Preserve saved section text/lexicon edits and legacy engine preferences without silently selecting their cloud backend; do not rewrite saved audio identities. Changing delivery invalidates current reviews, stops obsolete playback, cancels/discards an in-flight result and causes export to rescan only matching cached audio. Global Google delivery choices survive a round trip to Pocket. Failed preference storage must report failure instead of claiming persistence.

Required final native qualification: open Voice from the header; verify ready state and repair tucked away; reopen Connections and restart the app; press Listen on two different answers without backend/voice selection; verify play/pause/seek, edited pronunciation, cancellation and audio cache reuse. Setup should recover a pending Listen intent after the Connections window closes. Cloud selection must expose consent before any outbound text, and legacy sections must default to the global local connection until the user explicitly changes it. Keep listening quality and broader cancellation/export acceptance open until observed.


Native observation in the Connections package: header → Voice showed Pocket/Alba ready, with troubleshooting collapsed. Google settings were inspected without entering a key or making a Google request, then Pocket was restored. Listen generated/played two sections (~8s/~10s) without any engine/voice choice; a different answer reused ~3s cached audio. Pause was observed at 5.45s and Stop reset zero; Next stayed manual. See docs/acceptance/2026-10-16-connections.md. Restart and broad cancellation/export/listening quality remain separately qualified; do not close this task from a playback smoke check.


Final package restart observed: Pocket remained selected and ready; Codex GPT-6 Luna/max resumed without sign-in; root and child cached recordings played directly. Child pause at 2.7s and existing source/answer records survived. The new header visibly names Connections; Stop is disabled during synthesis preparation while Cancel audio remains available. Fresh setup recovery, all cancellation races, voice-change invalidation/export and human listening remain open acceptance work.

## Whole-script worker follow-up — 17 October

The local narration layer now submits all reviewed segments to one finite worker
and receives one complete validated WAV. Model/voice load once, ordering and
200 ms joining are unchanged, and no intermediate segment can be cached or
returned after later failure. The existing worker fingerprint separates new
reviews/audio from old cache files; Google cache identity and all Voice controls
remain unchanged. Six deterministic Python tests, native packet/queue/cancellation
checks and a real isolated 13.08-second worker sample pass. [Scope and exact
identities](../acceptance/2026-10-17-pocket-whole-script.md).

- [ ] In the replacement app, verify changed-worker review/cache invalidation,
  unchanged-script reuse, queued and active cancellation, voice switches and
  whole-output preservation across navigation/window close and restart.
- [ ] Requalify scene/section playback and selected matching export; record a
  human review of real code narration. A valid synthetic WAV does not pass these.

That checkpoint left the task open pending packaged navigation, cache invalidation, cancellation and offline export qualification.

## Fresh-section prewarm observation — 21 October

In the running native app, one approved Codex request against Fontdone
`main@5249b72371` created one new narrated section. On first inspection after
publication, its Pocket player showed **Ready to play** at `00:00 / 00:10`.
A newly written 506,924-byte WAV appeared in the app's local narration cache at
01:21 IST, within roughly two minutes of the request. This is consistent with
the post-publication Pocket prewarm path populating the cache before the user
presses Play. The section citation opened the selected snapshot at
`src/font.rs:4053–4078`; no PFR parser internals were needed.

Pressing Play switched directly to **Playing**; the first observed position was
`00:00.2`, without a generating state. I paused it at `00:03.55`. This supports
the cached playback path for the new answer, but is not a precise
click-to-first-sample measurement or a human listening review. The cache file
was not matched to the section fingerprint, so the observation does not
distinguish a previous matching cache hit from newly synthesized audio. Keep
listening quality, invalidation, cancellation, offline export and restart
qualification open.

## Backend cache namespace correction — 25 October 2026

Code inspection found that `cache_path` placed both Pocket and Gemini WAVs under
`gemini-narration-cache-v1`, even though their hashed cache contracts were
already different. New writes now select `pocket-narration-cache-v1` for Pocket
and keep Gemini under `gemini-narration-cache-v1`. Pocket reads fall back to
the old shared location only for the exact Pocket cache key, so existing local
clips remain available and are not synthesized again. The focused native
narration suite passes **31 tests**; a synthetic regression covers the new
namespace and old-location reuse. This is source-level evidence; fresh native
packaging, backend round trips, cache invalidation, offline export and the
human listening review remain open.

The v45 bundle has since been installed and reopened on Fontdone
`main@5249b72371`. Voice settings still show Pocket/Alba ready, and the saved
18-second Code Lens narration shows **Ready to play** after the restart. The
cache tree still has only the old shared directory, and no TTS operation ran;
this confirms the new app can reuse the existing Pocket cache entry there. It
does not test new Pocket writes under the distinct namespace, offline restart,
engine round trips, invalidation, export, cancellation or human listening.
