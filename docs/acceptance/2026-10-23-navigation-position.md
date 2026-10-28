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

## Additional packaged same-process check — 28 October 2026

In the running Code Atlas 0.1.0 app on Fontdone `main@5249b72371`, a saved
two-slide Guide was advanced to slide 2 of 2. Guide → Flow → Guide retained
slide 2. From the repository's saved-explorations list, I opened the existing
child “Explain When is_empty returns true in more detail” (level 2), then used
Back one exploration. The parent still showed slide 2 of 2 and restored its
previously selected `src/outline.rs` source; the child had shown `src/font.rs`.
No question, provider request, answer, or narration was started. The app stayed
open. The installed binary's source revision was not verified.

This adds one saved-child/Back path and one Guide/Flow/Guide path. It does not
cover a third exploration level, sibling position independence, exact scroll,
restart, changed/deleted revision fallback, malformed persisted state through
the packaged app, visual-node/edge restoration, or accessibility. Keep Kata
`erzc` open for those acceptance gates.

## Original three-section draft lookup — 28 October 2026

In the running Code Atlas 0.1.0 app, Fontdone `main@5249b72371`, local Chat
history search for `Outline::is_empty` returned two question groups and no
matching saved-explanation library entries. Opening both question results
showed one-section answers, not revision
`revision:d06cc73d-1f1a-4ffa-a6e5-2f0bacf6300d` from the original reproducer.
Searching the artifact title `Outline data model and emptiness check` returned
one level-2 question with an unavailable saved explanation and no section
reader. The required section-two state could not be reached from the current
saved history; no retry, model request, audio, or saved-content mutation was
made. The installed binary revision and durable screenshot were not verified.

This is a blocked reproducer lookup, not evidence that the position bug is
fixed. The original revision must be recovered or a matching preserved copy
made available before its Back/root/branch/restart path can be checked. Keep
Kata `erzc` open; deterministic source tests and other saved-guide checks do
not replace that exact-revision acceptance.

## Code Lens section and Flow round trip — 28 October 2026

In the running Code Atlas 0.1.0 app, Fontdone `main@5249b72371`, executable
SHA-256 `adec1e64e5d1783b5b531256af0c3005743af472e30c167fd4313cde8597af83`,
the saved Code Lens answer for `glyf.rs:564–583` advanced from section 1/3 to
section 2/3. Switching Guide → Flow → Guide retained section 2/3; Flow reported
no verified diagram for that section. Opening an existing level-2 exploration
from the `Outline::is_empty` answer and returning with Back restored the parent
Code Lens answer at section 2/3. No request, narration, answer edit or source
transmission occurred. The original pillow-rs workspace was restored after
the check.

The child path was labeled “Explain When is_empty returns true in more detail,”
but its single Guide question showed the saved `glyf.rs:564–583` answer at
section 4/4. This may be pre-existing child content; it is not accepted as a
matching `is_empty` reproducer. This round trip does not test a verified Flow
node/edge, a third level, sibling isolation, exact scroll, app restart,
revision fallback, malformed state or accessibility. The installed binary's
source revision remains unknown. Keep Kata `erzc` open.
