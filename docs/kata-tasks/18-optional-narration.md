---
kind: task
key: t18
parent: m3
title: Add optional natural-speech narration and cached TTS
priority: 2
labels: phase-3, speech, provider
blocked_by: t10, t12, t16
---

## User outcome
The reader can listen to an explanation without hearing punctuation, file paths, or code identifiers read aloud unnaturally.

## Contract
Keep editable `spoken_text` separate from visible explanation, code, path labels, and citations. The user previews/edits speech text and explicitly requests TTS under the provider disclosure. Default speech describes behavior in natural language; raw code is not sent to TTS. Provide a pronunciation lexicon for selected names when the user chooses to include them. Cache audio by spoken-text hash, provider/model, voice/style, and format. Accept up to 16,000 UTF-8 bytes per section; split requests into sequential chunks of at most 4,000 bytes, preferring sentence boundaries and falling back to whole-word boundaries when a sentence is longer. Show the exact outbound chunks before consent and assemble their audio into one cached WAV only after every request succeeds.

## Acceptance criteria
- A file chapter can be read and completed with audio disabled or unavailable.
- TTS request review identifies the exact narration text and current model/terms; no source excerpt is included by default.
- Long narration review displays each exact request segment; audio is joined in order with a short pause and cached only after all segments succeed.
- While audio synthesis is in flight, the reader can explicitly confirm cancellation; cancellation drops the local HTTPS future, treats any late result as discarded, and preserves the transcript and previous audio.
- Changing code with unchanged spoken text reuses audio; changing text or voice settings creates a new cache key.
- Pause, resume, replay, captions, and transcript stay synchronized with section state.
- Quota/network/provider errors preserve transcript and previous audio; no paid fallback is attempted.

## Excludes
Voice cloning, required narration, synthesizing code verbatim by default, and generating audio for an entire guide without review.

## Done evidence
Listen to sample scripts containing identifiers and punctuation; verify edited natural speech is sent, raw source is absent by default, cache reuse follows fingerprints, and transcript works offline.
