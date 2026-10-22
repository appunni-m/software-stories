# Native UI state spot check — 22 October 2026

This is one partial observation for Kata `x21h`, not a completed screen audit or
visual/accessibility acceptance.

## App and workspace

- The running app is Code Atlas, bundle version `0.1.0`; installed executable
  SHA-256: `b8add4a7d5f74f883b29506622045c74622de8a12438ca9e7e899cd8eaad7ea8`.
  The bundle does not identify its source commit, so this hash is not attributed
  to current `main` (`3216da7`).
- Native accessibility inspection reports Fontdone, selected snapshot
  `main@5249b72371`, captured 16 October 2026, marked partial. Window bounds are
  1440×844 points.
- The selected source is `src/tt/mod.rs`; its captured code pane is open beside
  the Guide. No generation or narration was started during this inspection.

## Contract evidence from the accessibility tree

The active workspace exposes Explorer, Search, Changes, and Explorations in its
left repository tools; a pinned composer has `Guide me`, `Show the flow`, and
`Ask about this codebase`; the explanation pane has Guide, Flow, and Chat history
views. The Codex connection and Notifications controls are in the header. This
confirms that those controls exist in the native accessibility tree, but does
not establish their visual spacing, colors, focus order, or behavior at zoom.

The Guide tree contains many earlier question cards in the selected
exploration, while Chat history separately lists those questions. This may make
the active answer harder to find in a long-lived repository; verify the rendered
layout and compare it with the approved card/nesting design before treating it
as a deviation. The tree also exposes “Some updates could not be loaded” as an
announced status; the current source places that text in an `sr-only` live region,
so it is not evidence of a visible banner.

## Why the visual state is uninspectable here

The display capture returned only the desktop wallpaper. A window-specific
capture of the onscreen Code Atlas window failed; the window server reports
sharing disabled for that window. System Events also rejected accessibility
inspection from AppleScript. The CUA accessibility tree remains available, but
there is no native screenshot to include. No screenshot in this note should be
used as visual evidence.

## Still uninspected

First run and project selection; empty and partial capture; active guide
generation and slide refill; diagram, graph, no-visual, citation and source
return states; failed/interrupted notifications and their navigation; connection
and sharing dialogs; light/dark themes; keyboard-only focus; 200% reflow;
VoiceOver, Increase Contrast, and Reduce Motion.

Next audit step: obtain a native window screenshot through an enabled
computer-use capture path, then inspect these states on the installed app and
record build identity for each. No product change or Kata closure is justified
by this partial inspection.
