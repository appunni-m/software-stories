# Packaged reader-position spot check — 23 October 2026

## Environment

- App: running Code Atlas desktop application, bundle ID `com.codeatlas.desktop`, version `0.1.0`.
- Repository snapshot shown in the app: Fontdone `main@5249b72371`.
- Navigation used already-saved answers and saved exploration branches. No question was submitted, no generation was started, and narration remained stopped.
- The installed bundle does not expose its source revision in its visible app metadata; this observation is not bound to a specific Git commit.

## Procedure and result

1. Opened the saved four-section answer “How does Font::memory_face choose which parser to call?”
2. Opened its second section, “The probes look for different kinds of signatures,” using that section’s Guide action. The section showed its one scene.
3. Switched to Flow and back to Guide. The same section remained visible.
4. Opened the saved child exploration “Explain Documented role and point representation in more detail,” then used Back one exploration. The parent returned with its second section, “The probes look for different kinds of signatures,” still visible.
5. A separate existing child (“Explain When `is_empty` returns true in more detail”) was opened and returned from earlier in this same spot check. Parent reading state remained intact.

The check used the app accessibility tree after each interaction. It confirmed the selected parent answer and section title, Guide/Flow selected tabs, and the restored parent after Back. No new source, request, answer, or playback was created.

## Limits

This is one packaged same-process position check against a four-section answer. It does not verify the originally reported three-section draft, a third exploration level, sibling-specific scroll offsets, exact pixel scroll restoration, independent Flow diagram selection/viewport, app restart, changed/deleted revision fallback, malformed persisted state through the app, keyboard/VoiceOver/reflow/contrast/motion, or a binary-to-source commit match. The automated state and persistence regressions remain necessary evidence for the relevant local cases. Keep Kata `erzc` open.
