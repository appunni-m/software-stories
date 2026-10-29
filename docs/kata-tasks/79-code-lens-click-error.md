---
kind: task
key: code-lens-click-error
phase: M1
title: Reproduce and repair the Code Lens click error
priority: 1
labels: acceptance, bug, code-lens
related: design-parity, exploration-history, t08
---

## User outcome
Clicking a captured source file or Code Lens control either opens the exact source/range or presents a recoverable, actionable error. It must not crash the workspace or silently send source to an agent.

## Report to reproduce
The user reported that clicking Code Lens on a file threw an error. The Fontdone snapshot is `main@5249b72371`. The original clicked control and file remain unknown. The user separately supplied this validator message: `Claim in section 'section:4cf1f1eb-3dda-4b65-8af4-d3218b8c09c1' does not name its declared identifier 'record_pipeline_operation_path' in the explanation.` It matches the claim-validation error handled by `src/lib/errorMessage.ts`, but the user did not confirm it came from the Code Lens click. Treat the original click exception as unconfirmed.

## Reproduced Code Lens generation failure — 24 Oct 2026
With the user’s one-request authorization, selected `src/tt/glyf.rs:564–583` and chose **Explain this code**. The app asked “Explain what `glyf.rs:564–583` does and how it works.” Codex completed after about 124 seconds, but none of three proposed sections passed native evidence validation. The saved answer ended as `failed` with `noPublishedSections`; no explanation section was published. The UI showed only “This question could not be completed.” The app stayed open and did not crash. This reproduces an unhelpful failure result in the selected-code flow, not the original click exception. No additional provider request is authorized.

## Authorized Code Lens retry — 24 Oct 2026
The user later authorized one request for the same selected range after the failed attempt. In the open packaged Fontdone workspace at `main@5249b72371`, the approved request is now visible as a saved explanation with six sections: transformed child points, component-point choice, placement, point merging, metrics, and contour-endpoint rebasing. The app remained open. The source pane and section-level source controls are visible; this observation does not establish that every citation opens the intended line or that the original click exception is fixed. Preserve the failed attempt above; do not treat the successful explanation as a retry-free result or as resolution of the original report.

This run exercised a new provider request under the user’s explicit authorization. No further provider request is authorized by this checkpoint. Keep the original clicked control/error unknown and run the local source-navigation reproduction matrix before closing.

## Packaged source-navigation spot-check — 24 Oct 2026

In the open Code Atlas app on Fontdone `main@5249b72371`, a source link in the saved `src/tt/glyf.rs:564–583` explanation navigated to `src/tt/glyf.rs:1037–1050`, showing `component_xy_offset` in the source pane. Selecting captured `src/tt/gasp.rs` from Explorer then displayed that file while the Guide and its saved Code Lens sections remained visible. No error dialog appeared and no provider request was started. The app stayed open.

This verifies one cited-range navigation and one Explorer file-open path in the current packaged workspace. The original report's exact control and exception remain unknown; selected-code generation was not re-triggered. Keep `mhqx` open for the remaining reproduction matrix and the original report.

## v42 restart persistence check — 24 Oct 2026

After installing the v42 app, the saved `src/tt/glyf.rs:564–583` Code Lens
answer and its six sections were restored in Fontdone `main@5249b72371`. The
existing Codex connection reconnected without setup, and no request was sent.
This checks persistence across the app replacement; it does not expand the
source-navigation matrix or identify the original click exception.

## Agent-driven Code Lens control check — 24 Oct 2026

While checking the selection control, I clicked **Explain this code** for the
visible `src/tt/glyf.rs:575–595` selection. The workspace immediately created
“Explain what glyf.rs:575–595 does and how it works” and showed it as building,
without opening a request review. I stopped it as soon as I saw that behavior.
The UI now reports **Stopped** and **No section was published**; the earlier
saved explanations remain. Whether any selected source reached Codex before
Stop is unknown, and quota may have been used. This was my acceptance action,
not a request from the user; do not retry it. The existing snapshot grant may
explain the direct-start behavior, so investigate the permission contract
before changing the one-time workspace-consent flow.

## Implemented response
The native summary now carries only its finite failure category into the workspace. The conversation validator accepts that fixed enum and rejects arbitrary diagnostics. The reader shows a plain next step for `noPublishedSections`: ask about one behavior, or ask how a specific caller or helper changes the result. The source validator remains unchanged; failed drafts still cannot publish. Regression coverage verifies summary serialization, bounded conversation persistence, restored UI state, unknown-code rejection, and the user-facing message.

## Verification — 24 Oct 2026
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: 745 passed, 18 ignored.
- `npm run quality:build` passed; this is required before the offline quality tests.
- `npm test`: 646 passed.
- `npm run build` passed.
- `npm run bundle:macos` passed and produced a fresh Code Atlas app bundle and DMG.
- Installed that bundle as `/Users/lazytrot/Applications/Code Atlas.app`, reopened the existing Fontdone workspace, and verified the retained failed question now displays: “I couldn’t verify an explanation from the selected code, so nothing was saved. Try asking about one behavior at a time, or ask how a specific caller or helper changes the result.” The app remained open. The earlier question/failure survived the app restart, and the new message appeared without another provider request.

## Reproduction matrix
On the packaged app, distinguish these actions and record the exact result for each: open a captured file from Explorer; open a cited range from Guide; select code and choose **Explain this code**; open the legacy **Explain cited range** review if that screen is reachable. For each, record project/snapshot, relative path, inclusive line range, visible error, whether the app stays open, and whether a provider review was created. Start with Fontdone `main@5249b72371` and a captured text file.

## Contract
- Validate snapshot identity, captured-file membership, source hash, and range before opening or creating a provider review.
- A local review failure must stay in the app and explain what the user can do next; no uncaught error or unexplained dead click.
- Opening source/citations is local and read-only. Do not send code to Codex or another provider until the user explicitly accepts the reviewed request.
- Preserve the current exploration, Guide section/scene, source selection, and focus when opening and returning.
- Cover both supported Code Lens entry points, if both remain in the current product, without adding a legacy path solely for this test.

## Completion evidence
Run the focused source-navigation and selected-code controls in the packaged app and record exact results. The selected-code generation failure above is confirmed; the original click exception is still unknown. The actionable no-published-section message is implemented, covered by frontend/native regressions, and confirmed in the rebuilt installed app after reopening the retained failed question. This check did not re-run Explorer/citation navigation after restart. Keep the original user report unconfirmed until its exact control and exception are identified. Source navigation requires no model invocation; any provider generation requires its own authorization.

## One authorized request after the v44 rebuild — 25 October 2026

The user authorized one Code Lens request for `src/tt/glyf.rs:564–583` in
Fontdone `main@5249b72371`. The app saved one section,
“Preparing each composite component’s points,” with a scene titled
“Transforming component points.” Its evidence links cite
`src/tt/glyf.rs:554–574` and `:575–595`. A source hotspot opened its cited range,
and returning preserved the explanation. The app remained open. During the v44
replacement the multi-detail operation was interrupted after this section had
published; the saved section survived. No retry or second provider request was
sent. This proves one selected-code explanation and source-return path, not the
original reported click exception or the full Code Lens reproduction matrix.
Keep the issue open; no more provider request is authorized by this run.

## Fresh authorized Code Lens attempt — 25 October 2026

The user explicitly authorized one new Code Lens request for
`src/tt/glyf.rs:564–583` on Fontdone `main@5249b72371`. The selection was
verified in the packaged app before clicking **Explain this code**. The app
created the matching question and remained open, but the attempt ended with:
“I couldn’t verify an explanation from the selected code, so nothing was
saved. Try asking about one behavior at a time, or ask how a specific caller
or helper changes the result.” The UI reported that no section was published;
earlier saved explanations remained. No narration was synthesized. This
confirms a recoverable failed outcome for this exact request, but it does not
reproduce the original click exception or prove that selected-code generation
works reliably. Keep the issue open; this authorization covered one request.

## Read-only source and notice-cap check — 25 October 2026

In the same installed Fontdone workspace, clicking the operation cue in the
saved point-transform scene opened captured `src/tt/glyf.rs:554–574`. The
source pane showed that range; **Show explanation** returned to the same
visual and scene. No error, provider request, or narration start occurred.

The current app-notice journal contains 100 of its 1,000 permitted entries;
the workspace's 176-unread badge also includes question outcomes and other
update categories. This read-only snapshot does not reproduce a full-history
condition and does not explain the earlier private Codex-file creation error.
No saved notices, receipts or explanations were dismissed or changed. The
original clicked control and exception remain unknown. Keep the issue open.

## Latest one-shot Code Lens run — 25 October 2026

After the user authorized one request, selected exactly
`src/tt/glyf.rs:564–583` in Fontdone `main@5249b72371` and clicked **Explain
this code** once. The app created the matching question and remained open, but
ended with: “I couldn’t verify an explanation from the selected code, so
nothing was saved. Try asking about one behavior at a time, or ask how a
specific caller or helper changes the result.” No section was published and no
narration was synthesized. Earlier saved sections remain available. No retry
was started. This repeats the recoverable no-published-section outcome; it
does not reproduce the original click exception. Keep the issue open.

## v30 packaged source and return check — 25 October 2026

After the user authorized one request for `src/tt/glyf.rs:564–583`, Code Atlas
published four Code Lens sections with distinct scene compositions and
narration ready. No second request was sent. I then opened captured
`src/tt/gasp.rs` from Explorer in the rebuilt v30 app. The source pane changed
to that file while the saved Guide sections remained visible; **Show
explanation** returned to the Guide view without an error. The replacement
app reopened the existing Fontdone snapshot and restored the prior answers.

This verifies a current packaged Explorer-open and Guide-return path. It does
not reproduce the original click exception or qualify the selected-code
request review/consent matrix. Keep `mhqx` open.

In the same v30 run, I clicked the saved visual's “Invalid attachment index”
supporting-code control. It opened captured `src/tt/glyf.rs:596–605`; the
Guide stayed on “Choose the component displacement,” scene 2/2, with that
failure outcome selected. **Show explanation** returned to the same scene and
selection. No error or provider request occurred. This verifies one exact
visual-to-source-and-back path after installation; it does not identify the
original reported click failure.

## Authorized exact-range Code Lens check — 25 October 2026

After the user authorized one request, I selected exactly
`src/tt/glyf.rs:564–583` in Fontdone `main@5249b72371` and clicked **Explain this
code** once. The app saved one section and one visual scene,
“Turning a child glyph into parent points.” Narration was ready to play; I did
not play it. The scene's source details cite `554–574`, `575–595`, and
`761–773` (plus repeated overlapping references), covering the selected
transformation and its helper. Clicking the `575–595` source link opened that
exact captured range; **Show explanation** returned to the same scene. The app
stayed open with no visible error. This confirms one successful Code Lens
generation and source-return path. The original user-reported click exception
and the full reproduction matrix remain unresolved; keep `mhqx` open. No further
provider request is authorized by this run.

## Require a Code Lens review before dispatch — 27 October 2026

The Code Lens action now opens a per-question review even when the repository
already has a workspace sharing grant. It shows the exact captured path and
selected range, a bounded source preview, and the generated question. The
review is revalidated against the currently displayed snapshot/source before
the user can choose **Ask about this code**; that action still requires the
saved Codex connection, then follows the existing workspace-grant flow. This
prevents an existing broad grant from silently dispatching the selected code.
No test, build, package run or provider request was made. The original click
exception remains unreproduced, so keep the task open.

## Packaged Code Lens review click — 28 October 2026

In the open packaged Code Atlas app on Fontdone `main@5249b72371`, selected
`src/tt/glyf.rs:554–574` and clicked **Explain this code**. The app opened
“Review your code question,” displayed the exact path and range, and stated
that nothing is sent until **Ask about this code** is chosen. I canceled the
review without dispatching a provider request; the Guide, source pane, and
saved explanations remained open. No exception or error appeared. This
confirms the current click opens the explicit review, but the original clicked
control and exception are still unknown; keep the task open.

## Current packaged source-navigation matrix — 28 October 2026

Still in Fontdone `main@5249b72371`, I opened `src/tt/gasp.rs` from Explorer;
the source pane showed that captured file and the saved Guide stayed in place.
I then opened the scene's cited `src/tt/glyf.rs:554–574` range; Code Atlas
showed those lines, and **Show source** returned to the same Guide section and
selected file. Both local navigation paths completed without an error, crash,
or provider request. Together with the review click above, this covers Explorer
open, a visual/source citation, return, and the per-question review entry point
in the current packaged app. The original click location/error text and any
legacy “Explain cited range” control remain unidentified; keep the issue open.

## Rebuilt app file-selection check — 28 October 2026

Installed the current `main@0f9247b8c438` bundle (version `0.1.0`; executable
SHA-256 `57027250d6309a4c59ef87ae5fe9da82efed6756d6a6e39f2255cb047ef9dbf3`)
and reopened Fontdone `main@5249b72371`. Selecting captured `src/tt/glyf.rs`
from Explorer switched the Code pane to that file; the app remained open and
the saved Guide stayed available. No error, crash, provider review, or model
request occurred. This is a successful Explorer-selection check, not a
reproduction of the original Code Lens click exception or the selected-code
review flow. Keep the original report unconfirmed and the issue open.

## Current packaged source and review check — 28 October 2026

On the installed `main` build (version `0.1.0`, executable SHA-256
`969c7414504f7cb1398c127bd6be3bc64c703efd144cdb6b7685bf7a835568e4`), opened
the saved Fontdone answer and clicked its captured `src/font.rs:4053–4078`
citation. The source pane showed the captured range while Guide and the saved
answer remained available. **Explain this code** opened the local “Review your
code question” dialog with the same path and range; I canceled before dispatch.
The app returned to the same source and Guide state. Selecting
`src/outline.rs` from Explorer also opened captured source normally. No error,
crash, source transmission, or provider request occurred. This verifies local
navigation and the review boundary only; it does not reproduce the original
click exception. The user-reported control/error is still unknown, so keep
`mhqx` open.

## Selected-code review on a different captured file — 28 October 2026

In the open Fontdone workspace on the selected `main@5249b72371` snapshot,
opening the saved citation for `src/api.rs:685–709` showed the captured range
in the Code pane. **Explain this code** opened “Review your code question,”
displayed that exact path and range, and stated that nothing would be sent
unless **Ask about this code** was chosen. I canceled; the app returned to the
same source and saved Guide without an error. No provider request or narration
started.

This adds a second captured-file range to the local review-click check. It does
not identify the original clicked control or exception, exercise every source
entry point, or verify the installed executable's source revision. Keep the
original report unconfirmed and `mhqx` open.

## Static current-path trace — 28 October 2026

The current source path was inspected without an app run. In `src/design/CodePane.tsx`, selection state is built from CodeMirror's selected line bounds and source hash; **Explain this code** invokes the workspace callback only when that button is clicked. `Workspace.explain` binds the selection to the displayed captured source, checks snapshot, path, inventory membership, hash and range, builds a bounded preview, and opens the local review. Its handler does not dispatch a provider request. The separate **Ask about this code** action revalidates the review, checks the saved Codex connection, and then enters the ordinary question/grant flow. Optional syntax-loader errors are caught and leave source readable as plain text.

No source change is justified from this trace: the reported click target and exception are still unknown, and the inspected code has explicit guards for the captured-source review route. No tests, build, app launch or provider request ran. The original failure remains unconfirmed; retain the packaged route matrix and ask for the exact error only if a new reproduction is needed. Keep `mhqx` open.

## Current packaged selected-code review — 28 October 2026

In the installed Code Atlas 0.1.0 app (executable SHA-256
`adec1e64e5d1783b5b531256af0c3005743af472e30c167fd4313cde8597af83`), Fontdone
`main@5249b72371`, opening the saved `glyf.rs:564–583` answer citation selected
captured `src/tt/glyf.rs:554–574`. **Explain this code** opened the review with
the same path/range; its preview showed captured source and the dialog stated
that nothing is sent until **Ask about this code** is chosen. I canceled before
dispatch. The app returned to the same source and Guide, stayed open, and the
question count remained 27. No source was sent, no provider request was made,
and narration remained off. The app's bundle metadata does not identify its
source revision. See [the native acceptance record](../acceptance/2026-10-28-codelens-source-navigation.md).

This verifies the local selected-code review boundary and one saved citation
path. It still does not identify the original clicked control/error, reproduce
the exception, or exhaust the source-navigation matrix. Keep `mhqx` open.

## Current installed-build Code Lens selection click — 29 October 2026

In the installed app built from `eeb2c11` (executable SHA-256
`4059e01548a752d6005bd2d465457f8af988f6d03a4875cd218059a2814a293c`),
`pillow-rs` `main@e843f864fc` had `canvas.rs` selected at captured lines
28–51. Clicking **Explain this code** opened the local review with the exact
question and captured path/range, and stated “Nothing is sent until you choose
Ask about this code.” I canceled; no provider request was made, the app stayed
open, and the Guide/source selection remained intact. No error appeared.

This current-build check confirms that specific click opens its review safely.
It does not reproduce the user's original exception because the original
control and error text remain unknown. Keep `mhqx` open; do not infer that this
one path proves every Code Lens entry point.

## Current installed-build saved-visual citation — 29 October 2026

On the installed Code Atlas 0.1.0 app (executable SHA-256
`58fe9d7778d3c846dfaa2beceea47b500aa9dfd7b7bb14c983d4c128a7cebf83`), Fontdone
`main@5249b72371`, I opened the saved `Outline::is_empty` Guide visual's local
code details and clicked its `src/outline.rs:49–51` citation. The source pane
opened `src/outline.rs`, highlighted lines 49–51, and kept the same Guide
section visible beside it. No error dialog or crash appeared and no provider
review/request was created. The click was local and read-only.

This confirms one saved-visual citation path in the current installed build.
The screenshot was inspected live but not retained. The user's original
clicked control and exception remain unknown, so the task and broader source
navigation matrix stay open.

## Selected-code review after a visual citation — 30 October 2026

In the installed Code Atlas 0.1.0 app (executable SHA-256
`07fa3b6708751ac249c9c350fe780f08195d0bad7f91f7fce152c72fcba429cc`), Fontdone
`main@5249b72371`, the visual citation had opened `src/outline.rs:49–51`.
Clicking **Explain this code** opened the local review for exactly that captured
path/range and stated that nothing is sent until the separate **Ask about this
code** action. I canceled; the app stayed open and returned to the same Guide,
selected visual cue, and source range. No provider request, narration, or
source transmission occurred.

This checks the current selected-code review/cancel path only. It does not
reproduce the user's original clicked control or error, test the request path,
or exhaust all citation/selection entry points; keep `mhqx` open.
