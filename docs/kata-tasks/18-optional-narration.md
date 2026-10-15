---
kind: task
key: t18
parent: m3
title: Add local Pocket TTS narration and distinct cached audio
priority: 2
labels: phase-3, speech, provider
blocked_by: t10, t12, t16
---

## User outcome
The reader can listen to an explanation without hearing punctuation, file paths, or code identifiers read aloud unnaturally.

## Contract
Keep editable `spoken_text` separate from visible explanation, code, path labels, and citations. The user previews/edits speech text and explicitly requests synthesis. Pocket TTS is the selected local default to implement; any retained cloud option requires its own explicit selection and provider disclosure. The current app adapter is still Google-only. Default speech describes behavior in natural language; raw code is not sent to TTS. Provide a pronunciation lexicon for selected names when the user chooses to include them. Cache audio by spoken-text hash, provider/model, voice/style, and format. Accept up to 16,000 UTF-8 bytes per section; split requests into sequential chunks of at most 4,000 bytes, preferring sentence boundaries and falling back to whole-word boundaries when a sentence is longer. Show the exact synthesis chunks before generation (and before consent for a selected cloud provider) and assemble their audio into one cached WAV only after every request succeeds.

## Acceptance criteria
- A file chapter can be read and completed with audio disabled or unavailable.
- TTS request review identifies the exact narration text and current backend, pinned model/voice identity and applicable licenses/terms; no source excerpt is included by default.
- Long narration review displays each exact request segment; audio is joined in order with a short pause and cached only after all segments succeed.
- While audio synthesis is in flight, the reader can explicitly confirm cancellation; cancellation stops/reaps the local worker operation or drops the explicitly selected cloud HTTPS future, treats any late result as discarded, and preserves the transcript and previous audio.
- Changing code with unchanged spoken text reuses audio; changing text or voice settings creates a new cache key.
- Pause, resume, replay, captions, and transcript stay synchronized with section state.
- Quota/network/provider errors preserve transcript and previous audio; no paid fallback is attempted.

## Excludes
Voice cloning, required narration, synthesizing code verbatim by default, and generating audio for an entire guide without review.

## Done evidence
Complete the [narration acceptance run sheet](../NARRATION_VALIDATION.md) on the packaged macOS app. Listen to natural and opt-in identifier scripts; verify the reviewed speech text is sent, raw source is absent by default, local synthesis uses no key/network after setup and any explicitly selected cloud cost/tier is checked before consent, cache reuse follows fingerprints, playback stays synchronized, and cached transcript/audio work offline. Do not close the task on a source inspection or generated fixture alone.

## Pending work — 2026-10-15

- [ ] Current user choice is Pocket TTS. The app integration remains Google-only; the earlier local CLI sample is not completion.
- [ ] Implement the Pocket runtime child, then the Pocket player/cache child. Make local synthesis the default with no API key and no cloud fallback.
- [ ] Complete later narration lifecycle/cache/redirect acceptance on this static patch and human listening/pacing review on actual code explanations.
- [ ] Update packaged setup, voice attribution, offline export and narration run sheet to match the local path.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.
