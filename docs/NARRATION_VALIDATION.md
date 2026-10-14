# Narration acceptance run sheet

Use this run sheet on the packaged macOS app after the narration feature or its
provider/model settings change. It records the human listening checks for
`70x5`; it does not replace automated cache, segmentation, cancellation, or
offline-export checks.

## Before generation

- Record app version/commit, macOS version, selected chapter/section, model ID,
  voice, delivery style, and whether the key's project is on Google's Free
  Tier. Code Atlas cannot inspect project tier, quota, or remaining usage.
- Keep narration optional. The chapter prose, captured source, path names, and
  citations must remain visible/readable with audio unavailable.
- Newly generated behavior sections should already contain a concise natural
  speech script; this script is still editable, and no audio is synthesized
  until the reader explicitly requests it. App-generated evidence-limit
  sections may remain silent.
- Use only the synthetic speech text below for this pass. Do not put source
  code, private repository details, or credentials in the script.
- Review the current model terms, quota, and pricing in the request dialog.
  Do not enable Cloud Billing or proceed on a paid tier for this zero-cost
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
2. Open the TTS review. Check the exact submitted text after the selected
   pronunciation substitutions, provider/model, voice/style, current terms,
   and whether the text is split into sequential request segments. Confirm no
   source excerpt, repository path, or whole-chapter content is included by
   default.
3. Change a word or voice setting while reviewing. Confirm the previous
   review/consent is invalidated and the displayed request is rebuilt. Consent
   only after reviewing the new text and the applicable usage disclosure.
4. Generate once, then listen to the complete result. Assess pronunciation,
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

