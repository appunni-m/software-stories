# Workspace native spot check — 29 October 2026

This is a live, read-only spot check of the installed Code Atlas app. It adds
current-state evidence to the broader design gap audit; it is not the full
screen-by-screen acceptance required by Kata `x21h` / `13yp`.

## Build and workspace

- App: `/Users/lazytrot/Applications/Code Atlas.app`, version `0.1.0`.
- Foreground process: PID `19502`, launched from the installed app bundle.
- Installed executable SHA-256:
  `58fe9d7778d3c846dfaa2beceea47b500aa9dfd7b7bb14c983d4c128a7cebf83`.
- Source checkout: `main`, UI change commit `f0969c3`; docs-only HEAD at
  inspection: `141fd22`.
- Window capture: 2880 × 1688 pixels.
- Repository/snapshot: Fontdone `main@5249b7237194…`, partial capture.
- No provider request, narration playback, content edit, or repository refresh
  occurred.

## Observed populated states

| Area | Classification | Evidence and remaining uncertainty |
| --- | --- | --- |
| Persistent header | Partial match at this size | Repository, snapshot, **Load latest changes**, Codex connection/model, appearance, notifications and help all fit in one in-flow row. This does not qualify a narrower window or 200% zoom. |
| Explorer and source | Partial | Explorer, selected `src/tt/glyf.rs`, source pane, pinned composer and Guide are visible together. Long source lines extend past the code pane's visible edge; horizontal scrolling and wrapping were not exercised, so source accessibility at those lines remains unverified. |
| Guide | Partial | A saved answer and visual are visible beside source. It has one scene (`Scene 1 of 1`), so this does not exercise previous/next scene controls or a populated multi-scene layout. Narration is ready, but no audio was played. |
| Search | Partial | Existing query `memory_face` shows 47 matches across 22 files, “more matches available,” and the disclosure “Local search may be incomplete.” Search results, source, and current Guide context remain in the same workspace. No-result, regex, keyboard, and complete-index states were not reviewed. |
| Changes | Partial | The screen shows 80 recent commits and a two-file comparison (`7c48b706482b` → `5249b7237194`). `README.md` has one inspectable text hunk and is labeled unsupported; the inventory TSV is labeled oversized with no hunks. The screen and warnings are visible, but this does not qualify alternate refs, empty/error states, or PR Guide behavior. |

The app was inspected through native accessibility state and live screenshots.
The screenshots were not retained as files, so this note does **not** satisfy
the audit's durable screenshot-attachment requirement. First-run, empty and
failure states, Flow, nested restoration, notifications, connection settings,
light appearance, keyboard-only use, VoiceOver, Increase Contrast, Reduce
Motion, the full narrow-window matrix, and 200% zoom remain uninspected here.

## Reduced-window follow-up — 29 October 2026

The installed app was resized through its native window controls and inspected
again. The screenshot output was 2048 × 1365 pixels (the logical CSS viewport
was not exposed by the accessibility API). The header reflowed into two rows;
repository, snapshot, **Load latest changes**, Codex/model, appearance,
notifications, and help controls remained visible without overlap. Explorer,
source, and Guide remained side by side; the long Guide heading wrapped within
its pane. The source viewport still shows only part of long code lines, which
must be read through the source pane's horizontal overflow. The available saved
visual remains one scene, so there were no scene arrows to exercise. The app
window was restored to its prior large size after inspection.

This narrows the header overflow uncertainty for one reduced-size state; it is
not a complete viewport matrix or a retained screenshot. It does not qualify
minimum supported dimensions, 200% zoom, or a multi-scene Guide. The screenshot
was inspected live but not retained as a file.

## Next audit actions

1. Capture and retain the five approved-reference comparisons at matching
   viewport sizes, with the app version and executable identity above.
2. Repeat header, source, section pager and scene pager review at a narrow
   native window and 200% zoom; exercise a saved guide with multiple scenes.
3. Inspect first-run/empty/error states, Flow and navigation restoration, and
   notifications without generating content.
4. Complete keyboard, VoiceOver, contrast, motion, and light-theme checks in
   the packaged app. Leave each result marked unknown until directly observed.

Keep `x21h` and `13yp` open until these required states and retained screenshots
are complete.

## Current native accessibility-tree inventory — 29 October 2026

The running installed app is Code Atlas 0.1.0, PID `63379`, on Fontdone
`main@5249b72371`, partial snapshot. Its executable SHA-256 is
`5c51a28ce421df8b9b8992cf5e84af0b6cf75f7b42f0801392f479b15f58891c`; it
matches the executable in the current local release bundle. The bundle does not
embed a verifiable source commit, so this hash alone does not establish a commit
identity.

The native accessibility tree confirms that this populated workspace exposes
the repository/snapshot header, **Load latest changes**, the saved Codex
connection and model, appearance, Notifications, and keyboard help. Explorer,
Search, Changes, and Explorations share the left activity rail; the pinned chat,
captured source, and Guide / Flow / Chat history views remain in one workspace.
Search results, the selected source, and the current Guide are present together.
The inspected Guide answer has one scene, so it cannot qualify scene arrows or
multi-scene pacing. The same view still includes unavailable earlier guide
material and failed historical attempts; Notifications reports 49 unread
updates. This is a visible recovery and density concern, not evidence that any
stored answer or notification was changed.

No source, snapshot, answer, notification, or connection setting was changed;
no model or narration request was sent. The screen-capture command returned the
desktop rather than the app window, so there is no retained screenshot for this
observation. This inventory supplements live accessibility evidence only and
does not satisfy visual layout, keyboard, VoiceOver, contrast, motion, reflow,
or durable screenshot acceptance. Keep `x21h` and `13yp` open.
