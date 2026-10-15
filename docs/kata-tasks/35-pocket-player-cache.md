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
NarrationPlayer, narration command types, disclosure and cache constants remain Google-specific. Integration is pending; existing lifecycle/redirect/read-bound fixes do not change the TTS provider.

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
