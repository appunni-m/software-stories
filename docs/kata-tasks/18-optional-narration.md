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
Keep editable `spoken_text` separate from visible explanation, code, path labels, and citations. The user previews/edits speech text and explicitly requests synthesis. Pocket TTS is the selected local default; any retained cloud option requires its own explicit selection and provider disclosure. The source now contains distinct Pocket and Gemini adapters, awaiting local-runtime/player acceptance. Default speech describes behavior in natural language; raw code is not sent to TTS by default. Provide a pronunciation lexicon for selected names when the user chooses to include them. Cache audio by spoken-text hash, backend/runtime/model/voice identity, applicable style, segmentation/joining policy and format. Accept up to 16,000 UTF-8 bytes per section; split requests into sequential chunks of at most 4,000 bytes, preferring sentence boundaries and falling back to whole-word boundaries when a sentence is longer. Show the exact synthesis chunks before generation (and before consent for a selected cloud provider) and assemble their audio into one cached WAV only after every request succeeds.

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

## Implementation update — 2026-10-15

New section preferences now default to Pocket/Alba; legacy Google settings and cache identity remain intact. App-managed setup/status/repair, local review without a key or Google consent, worker cancellation, runtime-aware cache identity and backend-aware export cache lookup are implemented in source. The worker is finite and reloads per segment, with no listener; the current target is Apple Silicon with an external `uv` prerequisite. This is not a self-contained runtime bundle or completed narration acceptance. Read the [runtime child](34-pocket-runtime.md), [player/cache child](35-pocket-player-cache.md) and [runtime contract](../research/pocket-tts-runtime.md).

Every acceptance criterion remains open. No setup, synthesis, listening, packaged playback/export or lifecycle checks ran for these changes.

## Pending work — 2026-10-15

- [ ] Finish runtime packaging/version/hash/license and clean-account qualification; earlier standalone CLI audio does not validate the app-managed runtime.
- [ ] Complete whole-section scheduling and qualify setup/repair/cancellation/exit behavior, including concurrent windows and no partial-audio acceptance.
- [ ] Later exercise narration lifecycle/cache/legacy Google separation, no-network local synthesis, backend-aware offline export, and retained cloud redirect/consent behavior.
- [ ] Update setup/help and the narration run sheet, then record human listening/pacing review of real code explanations and opt-in identifier pronunciation.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## October 17 complete scripts and visual scenes

New progressive checkpoints keep every grounded claim sentence instead of selecting three short sentences; historical checkpoints retain their original script. Default code-name pronunciation expands underscores, camel-case boundaries and Rust namespaces, with paths/URLs/expressions and saved edits preserved. The installed Fontdone partial answer loaded Pocket audio (observed paused at 20.2 / 72 seconds); no listening-quality acceptance is claimed.

Guide now presents scene-scoped narration with collapsed text. Actual clip completion advances the highlighted evidence-backed scene, never the next section; manual scene/source selection pauses it. Exact clip text/settings determine cache reuse, global Connections remains authoritative, and a pre-existing whole-section script edit stays whole. Require real play/pause/seek/replay, scene/source/return, independent cache invalidation, restart, human pronunciation/pacing and no simultaneous narration checks. Full offline export of newly cached scene clips remains pending: existing whole-section export must report transcript-only when matching whole-section audio is absent. [Interaction contract](61-guide-flow-detail.md). Keep open.

## October 17 HTML scene alignment

The same narrated claim scenes can render as AI-authored HTML compositions or graphs. Audio completion advances the corresponding visual scene, never the section. Restoring/choosing a paused clip at its saved end must not synthesize another clip or advance scenes; completion must follow actual playback. Keep manual scene changes, source clicks and seek interruptible, with only one audio active. Validate Pocket against HTML cues as well as graphs. Scoped scene audio export and human voice/story quality remain open acceptance gates.

## October 17 HTML scene qualification

Pocket completed the narrow Fontdone HTML scene’s 17.25-second clip, and paused-end restoration was observed without advancing. This is mechanical evidence, not a listening pass. The app continues to pin Pocket 3.3.0, english_2026-09 and Alba. Upstream also documents english_2026-09_2 and english_drifting_26-09; no code-specialized model or comparative code-pronunciation evidence was found. Any model evaluation must use the same reviewed technical narration samples, measure pronunciation/pacing/latency, retain exact model/cache identity, and keep voice configuration in Connections. No model was changed from this research. Sources: [official model options](https://github.com/kyutai-labs/pocket-tts), [CLI default model](https://github.com/kyutai-labs/pocket-tts/blob/main/pocket_tts/main.py). See [results, preserved failures and remaining gates](../acceptance/2026-10-17-visual-story-guide.md). This task remains open.

## October 17 offline scene-audio checkpoint

Player/export now share exact narration scopes and inherited delivery settings;
the exporter assembles matching cached scene clips, reviewed transcripts and
independent hashes, retaining custom whole scripts and explicit missing/partial
coverage. Two fresh Pocket/Alba fixture clips and all four source/audio package
combinations passed mechanical checks. The model is unchanged. This used the
existing worker with isolated outputs, not native cache injection or an actual
accepted chapter. Native cache-preview matching, actual browser playback and
human technical-pronunciation/pacing review remain pending. See [scoped evidence
and remaining export checks](../acceptance/2026-10-17-offline-visual-scenes.md).
Keep this task open.
