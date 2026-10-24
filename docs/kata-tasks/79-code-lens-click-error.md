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
