# Narration acceptance run sheet

Use this run sheet on the packaged macOS app after the narration feature or its
provider/model settings change. It records the human listening checks for
`70x5`; it does not replace automated cache, segmentation, cancellation, or
offline-export checks.

## Before generation

- Record app version/commit, macOS version, selected chapter/section, model ID,
  voice and delivery style from Connections → Voice. Only for explicitly selected Google speech, record the key's project tier; the app cannot inspect tier, quota or remaining usage.
- Keep narration optional. The chapter prose, captured source, path names, and
  citations must remain visible/readable with audio unavailable.
- Newly generated behavior sections should already contain a concise natural
  speech script; this script is still editable, and no audio is synthesized
  until the reader explicitly requests it. App-generated evidence-limit
  sections may remain silent.
- Use only the synthetic speech text below for this pass. Do not put source
  code, private repository details, or credentials in the script.
- Review the current model terms, quota, and pricing in the request dialog.
  This requirement applies only to selected Google speech. Do not enable Cloud Billing or proceed on a paid tier for this zero-cost
  acceptance run. If the selected project/model is not confirmed Free Tier,
  stop before sending.

## Sample scripts

First use natural language without identifiers:

> A request reaches the route resolver. Only a GET request for the users path
> returns the user list. Other method and path combinations return not found.

Then test selected identifier pronunciation by adding this optional
pronunciation lexicon and reviewing its resolved speech text:

| Written identifier | Spoken form |
| --- | --- |
| `isRetryable` | is retryable |
| `retryDelayMs` | retry delay milliseconds |

> The isRetryable check allows a failed job to retry while its attempt count is
> below three. The retryDelayMs setting describes the configured wait.

Check that the spoken form is natural, while the visible explanation and
source identifiers remain unchanged.

## Run and record

1. Edit the section's **spoken text**. Confirm the visible explanation, code,
   path, and citations remain separate and unchanged.
2. For local Pocket, choose Listen and check the resolved transcript after pronunciation substitutions. The native review/cache lookup runs in the background. Engine/setup/model/voice controls must be absent from the answer. For Google, inspect the exact text, provider terms and optional segment details in its send review before consent. Confirm no source excerpt, repository path or whole-chapter content is included by default.
3. Edit a script word beside the section or change delivery in Connections → Voice. Confirm old review/cache matching is invalidated and obsolete audio cannot play. Google consent must reset and a fresh exact-text review is required before sending.
4. Listen once, then hear the complete result. Local narration must not require another Generate click. Assess pronunciation,
   punctuation, pacing, voice quality, and whether sentences are easy to
   follow. Compare the audio against the exact displayed script; do not accept
   audio that omits, adds, or garbles a substantive phrase.
5. Verify captions/transcript and section playback stay synchronized during
   play, pause, replay, seek, and section navigation. Playback must not
   auto-advance the walkthrough.
6. Reopen the same section to confirm its verified cached audio plays. If
   practical, disable network access before replaying it to verify offline
   playback. Edit only chapter prose/code and confirm unchanged spoken text can
   reuse its cache; edit spoken text or voice/style and confirm the cache key
   changes.
7. If a provider/quota/network error occurs, confirm the transcript and any
   prior valid audio remain usable, with no automatic paid fallback. Do not
   deliberately trigger a paid request to test this condition.

Record the result before closing `70x5`:

```text
Date/time and listener:
macOS version / app version+commit:
Chapter/section and source snapshot:
Provider/model ID and returned model version (if shown):
Free Tier and current terms/pricing reviewed: yes | no:
Voice / delivery style:
Natural-script pronunciation, pacing, clarity: pass | fail — notes:
Opt-in identifier pronunciation: pass | fail — notes:
Exact request excludes source/path/citations: pass | fail — notes:
Review invalidates on script/settings changes: pass | fail — notes:
Captions, transcript, playback controls: pass | fail — notes:
Cached offline replay and cache invalidation: pass | fail | not run — notes:
Provider-error recovery / no paid fallback: pass | fail | not observed — notes:
Unresolved issues:
```



## Current Connections workflow (October 16 workspace)
Open the header's connection button, then **Voice**. Pocket/Alba is the default. Prepare the voice once if needed; repair and license attribution are in Voice details and troubleshooting. Google is optional and explicitly selected here, with its key saved in Keychain. Voice and style apply across answers and repositories, including export cache matching. Existing per-section text and pronunciation edits remain local; legacy cloud settings do not override the global choice.

On an answer choose **Listen**. Local review/cache lookup and synthesis run behind playback controls. Captions show the final pronunciation-expanded script. Pause, replay, seek and Next stay deliberate. The engine, runtime setup, voice and style no longer appear on each section. Optional transcript/pronunciation editing remains beside the section. For Google, inspect the exact text and provider terms and consent before sending; choosing the engine alone never authorizes a request. No cloud fallback is used.

Validate across two answers and app restart. Changing Connections during a request must invalidate its review and prevent a stale result from playing. Export must scan the selected connection's matching audio without generating speech. Setup/key updates must refresh the open player when Connections closes. Human speech quality and code-name pronunciation still require listening review.

## Complete progressive scripts — October 17

New progressive answer checkpoints use presentation version 2. Reading and
saved speech contain all distinct grounded claim sentences in order, including
conditions, later error cases and visible inference labels. The complete copy
must fit 20–4,000 UTF-8 bytes. An oversized proposal is rejected as a whole with
`presentationLimit`; the agent must split it rather than receiving a silently
shortened script. Historical version-1 checkpoints retain their original copy
when reopened; they are not upgraded or regenerated on read.

For a section with no saved narration edits, the player expands underscores,
camel-case boundaries and Rust namespace separators into words, and removes
backticks around simple names. This is a narrow pronunciation rule; it does not
rewrite facts, interpret expressions or establish semantic accuracy. Dotted
names, paths and URLs remain unchanged. User-edited transcripts and explicit
pronunciations remain authoritative. The cache scan for offline export uses the
same default script as the player; the exact prepared text remains visible.

Regressions cover the earlier Fontdone sentence-loss case and immutable legacy
replay. Fresh generated content, actual Pocket output, pacing and comprehension
still require the review above. Existing incomplete drafts remain incomplete.
