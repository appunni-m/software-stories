# Pocket narration acceptance run sheet

Use this run sheet on the packaged Mac app after narration changes. The first
release uses one local voice path: runtime setup is tracked in `q6g3` and
playback/cache/listening in `dbmd`. Cloud speech, generation-time prewarming,
and standalone audio export are outside first-release scope.

## One-time setup

- Prepare Pocket TTS once in Connections. Record the app and macOS versions,
  runtime status, and selected section. The first setup downloads the pinned
  runtime, model, and Alba voice into app storage and needs internet, disk space,
  and memory. Playback afterward uses local files; no API key or listening
  server is required.
- Confirm the saved voice connection is shown in the header and reused for
  other answers. The user should not choose an engine or configure a model for
  each section.
- If setup fails, confirm the app gives a clear recovery action. Repair and
  retry must preserve the explanation, transcript, and earlier valid audio.
- Pocket currently offers one English voice, Alba. Keep its attribution and
  license visible in Voice details.

## Example narration

Use a behavior-focused sample without source code:

> A request reaches the route resolver. Only a GET request for the users path
> returns the user list. Other method and path combinations return not found.

Then check selected identifier pronunciation:

| Written identifier | Spoken form |
| --- | --- |
| `isRetryable` | is retryable |
| `retryDelayMs` | retry delay milliseconds |

The speech should explain what the check changes and why it matters. It should
not read punctuation, paths, or code aloud. The visible source and explanation
keep their original identifiers.

## Packaged-app review

1. Open a generated answer and confirm its explanation, visual, source links,
   and transcript are usable while audio is preparing or unavailable.
   Preparation must not block reading or start playback.
2. Play one section. Listen for natural phrasing, clear pacing, pronunciation,
   and agreement with the visible explanation. The narration should describe
   the behavior as a short story through its inputs, decisions, and outcomes.
   It must not simply read definitions or a documentation paragraph aloud.
3. Pause, resume, seek, stop, replay, and move between sections. Confirm the
   transcript and selected visual stay paired with the audio, and section
   progression remains deliberate.
4. Replay the unchanged section and confirm its cached audio is reused.
   Change the spoken text and confirm the new audio has a distinct cache entry.
   Change only source code while the spoken text stays the same and confirm
   matching audio can be reused.
5. Navigate away during synthesis. Confirm cancellation or late
   results cannot replace the selected section's audio, and the transcript and
   previous valid clip remain intact.
6. After setup, disconnect the network and replay cached audio. Confirm local
   playback works and no cloud fallback is attempted. Missing or corrupt audio
   must leave the transcript readable.
7. Restart the app and reopen the same answer. Confirm its section and matching
   cached audio remain available.

## Record

```text
Date/time and listener:
macOS version / app version+commit:
Answer, section, and source snapshot:
Pocket runtime/voice:
Natural narration and pronunciation: pass | fail — notes:
Visual, source, and narration agree: pass | fail — notes:
Playback controls and section selection: pass | fail — notes:
Unchanged replay uses cache: pass | fail — notes:
Changed spoken text invalidates cache: pass | fail — notes:
Cancellation/failure preserves transcript and old audio: pass | fail — notes:
Offline replay after setup: pass | fail — notes:
Unresolved issues:
```
