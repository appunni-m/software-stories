# Visual stories and AI-authored HTML scenes — 17 October 2026

## Delivered behavior

Guide and the standalone player share a visual stage with narration controls
above it. Script, speech editing, section notes and evidence details start
collapsed. The existing workspace, code pane and nested explorations remain.

For new progressive Codex answers, the agent can compose a responsive HTML scene
from bounded groups, cards, code, tokens, byte strips, pixel matrices and tables.
A scene can instead select the existing graph renderer when relationships or
paths are the useful representation. This is an app-rendered composition
contract, not execution of generated HTML, CSS or JavaScript. Each content
element resolves to the same scene's captured claim evidence; invented example
values carry an illustration label. See the [complete contract](../VISUAL_SCENES.md).

Complete claim narration advances authored scenes on actual audio completion.
Sections stay manually paced. Selecting a scene or source interrupts the prior
clip, with only one narration active. Scene and audio positions persist by
immutable revision. Older summarized and custom whole-section scripts remain
intact, rather than receiving guessed timestamps or rewritten content.

## Real generation evidence

Both operations used the existing installed account-backed Codex and
**GPT-6 Luna/max**, with no direct API key or new agent installation.

| Operation | Result | Evidence |
|---|---|---|
| Synthetic greeting transformation | Complete, one HTML scene; native total 22.351 s, first publication 19.950 s | [Unmodified result and ledger](artifacts/2026-10-17-progressive-html-scene-synthetic.json) |
| Installed Fontdone PFR0 dispatch branch | Complete, one HTML scene; publication at run elapsed 75.442 s; fourth proposal accepted | [Result, rejected attempts and observations](artifacts/2026-10-17-fontdone-html-scene.json) |

The synthetic scene shows an explicitly illustrative name becoming a returned
greeting. The installed Fontdone scene shows the source-backed PFR0 prefix and
the dispatch to `pfr_face`. Computer use observed the HTML composition, collapsed
text, and a source click opening captured `src/font.rs:4053–4078`. Pocket loaded
and completed its 17.25-second clip; the section remained paused at the end.
This establishes mechanical playback, not a human listening or comprehension
pass.

The installed generation/playback artifact identifies executable SHA-256
`9c661e10408510dcd950e959e241c38e03cc8fac386840e2f2f4ad7aa0616af5`.
Later source refinements must be identified separately from that inference run.

Three Fontdone proposals omitted the required visual title and received only
generic section-validation feedback. The agent corrected the fourth itself.
The follow-up adds fixed `visualField` positions for missing or wrongly typed
visual fields and explicitly names required fields in the prompt. Its unit
check prevents source/model text from entering diagnostics. This refinement
does not retroactively change the original attempts or establish a new account
qualification.

The accepted Fontdone script still reads call syntax and spends a sentence on
an illustration disclaimer. New-generation instructions now put expressions
and argument lists in visual code elements, retain required bare identifiers,
and narrate the meaning and consequence in ordinary language. They introduce
examples naturally while retaining actual conditions, errors and uncertainty.
This prompt improvement still needs fresh narration-quality qualification.

## Preserved failures and partial results

- The earlier synthetic graph/story run published two sections but ended
  **partial**, and its complete-outcome assertion failed. The
  [original failure](artifacts/2026-10-17-progressive-visual-story-synthetic.json)
  remains separate from the later successful HTML operation.
- The broader Fontdone complete-script run published one four-claim section and
  ended **partial**. Its last read was rejected, but the original native cause
  was not retained. Its wording about “all control bytes” also exceeds the
  actual below-ASCII-space guard. The
  [partial result](artifacts/2026-10-17-fontdone-complete-script-partial.json)
  is not a quality oracle or a complete-repository pass.
- Restoring a paused clip at its saved end originally advanced scenes and could
  start the next clip. A playback-completion guard fixes this; native computer
  use observed scene 2 remain paused at 12.6/12.6 seconds after restoration.

## Remaining acceptance

Representation choice, semantic accuracy, narrative depth, language coverage,
human pacing/pronunciation/comprehension, keyboard/VoiceOver/reflow and broad
Fontdone completion remain open. Exact source attachment validates provenance,
not the semantic truth of every illustration. Other generation operations
still use their existing graph prompts.

Offline export retains typed presentation data and source links, but displays a
source outline instead of reproducing the HTML layout. Both export preview and
the generated guide disclose that limitation. Scene-scoped audio is not yet
assembled into a faithful offline story. The app remains an unsigned debug
build, without signed release or clean-account installation acceptance.

Kata tasks `j53m`, `mrpj`, `q0sm`, `70x5` and `3bz5` remain open. No task is closed
by these mechanical checks.

## Final replacement verification

Implementation commit: `fdaf379` (on top of complete-narration commit `7d6e335`).

- Frontend: **171 passed**, 21 suites, zero failures
  (`/private/tmp/code-atlas-html-scenes-final-frontend-tests.log`).
- Native: **346 passed**, zero failures, 12 explicitly ignored live probes,
  38.36 s (`/private/tmp/code-atlas-html-scenes-final-native-tests.log`).
- TypeScript/Vite production build and native debug app bundle passed
  (`/private/tmp/code-atlas-html-scenes-final-app-build.log`). Existing chunk-size
  and 13 native unused-code warnings remain.
- Built and installed executable integrity checks passed. Final installed
  executable SHA-256:
  `3dfb4899085278066bc8f844c3bdd8b0ba51c2112189f1062003def23dc24c0b`.
- Computer use observed the final replacement restore the saved Fontdone HTML
  scene, source pane and collapsed disclosures. Clicking its `pfr_face` element
  reopened captured lines 4053–4078. Cached narration remained paused at
  17.25 seconds; the Codex connection restored without another setup action.
  No new account-generation request was made for this replacement check.
- Documentation audit: zero structural errors, six review findings. Five are
  scoped or explicitly denied broad claims in existing records; the other is
  the existing missing root license. No license or release permission was
  invented (`/private/tmp/code-atlas-html-scenes-documentation.json`).
- Live Kata readback confirmed all five updated task bodies and their open
  status. The project still has 73 records: 50 open and 23 closed.

The prior installed app is retained at
`/Users/lazytrot/Applications/Code Atlas.pre-visual-feedback-9c661e1.app`.
The final fingerprint above qualifies the replacement/readback, not another
inference or human listening run.
