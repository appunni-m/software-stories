# Accessibility validation status

This is a source and test evidence ledger for the repository-to-walkthrough
journey. A source inspection or model test does not count as a VoiceOver,
contrast, zoom, or native-app usability pass.

## Changes made

- Walkthrough section changes programmatically focus the section heading. The
  heading now keeps a visible outline while focused.
- Provider and recovered-preview dialogs now trap Tab navigation, receive
  initial focus, and restore focus to the calling control when they close.
- The page no longer enforces a 760 CSS-pixel minimum width. Existing
  responsive layouts can take effect between 761 and 900 pixels as well as at
  the narrow-window breakpoint.
- Reduced-motion rules remain active for application animations, transitions,
  and smooth scrolling.

## Evidence available in source

- Workspace shortcuts and menu keyboard handling are in
  [App.tsx](../src/App.tsx).
- Modal focus containment and the recovered-preview focus return are implemented
  in [App.tsx](../src/App.tsx).
- Walkthrough section navigation exposes aria-current, Previous/Continue
  buttons, keyboard section movement, and focus restoration in
  [Walkthrough.tsx](../src/Walkthrough.tsx).
- Notices, interrupted jobs, captions, and narration errors use status or alert
  semantics in [App.tsx](../src/App.tsx) and
  [NarrationPlayer.tsx](../src/NarrationPlayer.tsx).
- Focus restoration in the navigation model is covered by
  [navigation.test.mjs](../tests/navigation.test.mjs); that test does not drive
  the rendered UI.
- Reduced-motion styles and focus/reflow fixes are in
  [App.css](../src/App.css).

## Native manual checks still required

No macOS app or browser preview was started for this record. Consequently, the
following are **not verified**:

- A keyboard-only repository-to-walkthrough journey in the rendered app,
  including focus visibility at every transition.
- VoiceOver announcements for project, snapshot, file, section, validation
  errors, and asynchronous progress/results.
- Text zoom at 200%, narrow effective viewport reflow, and contrast legibility.
- Reduced-motion behavior in the rendered app.

Run and record those checks on the packaged or native app before treating the
accessibility acceptance criteria as complete. The Rust and frontend tests
remain useful implementation evidence, but cannot substitute for those manual
checks.
