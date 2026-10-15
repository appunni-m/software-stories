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

No tests, builds, app launches, synthesis or runtime probes ran for this integration.

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
