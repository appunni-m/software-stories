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
The user reported that clicking Code Lens on a file threw an error. The Fontdone snapshot is `main@5249b72371`. Exact original file path, clicked control, and error text remain unknown, so the original report is not yet confirmed.

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
