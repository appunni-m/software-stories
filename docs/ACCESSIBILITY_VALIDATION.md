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
- PR Guide citation buttons remain keyboard-operable, expand the cited file,
  scroll the exact hunk into view, and highlight it. Citation scrolling uses
  non-smooth behavior when reduced motion is requested.

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
- The Atlas inventory tree filters with retained-path sets, renders a virtual
  row window, and exposes tree hierarchy/position metadata plus full-list arrow
  navigation in [App.tsx](../src/App.tsx). The 25,000-row helper fixture in
  [atlas.test.mjs](../tests/atlas.test.mjs) checks that a 510 CSS-pixel viewport
  calculates a maximum 39-row window at the default row height. This is a
  bounded-window calculation, not a rendered responsiveness or screen-reader
  acceptance test.
- PR Guide citations connect to selected hunks in
  [GitChanges.tsx](../src/GitChanges.tsx) and
  [PrGuideStudio.tsx](../src/PrGuideStudio.tsx). Source/build evidence is not a
  rendered interaction or native-app accessibility pass.

## Rendered keyboard preview pass

On 2026-10-13, the Vite preview at 127.0.0.1:4173 was used at its normal
1280-pixel viewport. From the empty-project screen, keyboard Tab showed a
visible focus outline on the repository picker and workspace navigation.
Return opened Walkthrough; the workspace heading received focus and showed a
visible outline. Return opened Project Tools, Arrow Down focused its first menu
item, Return opened Guide Plan, and Escape closed the menu and restored focus
to the Tools button. The accessibility tree exposed the resulting workspace
headings. This checks a rendered browser preview, not the packaged Tauri app.

## Manual checks still required

The following are **not verified**:

- VoiceOver announcements for project, snapshot, file, section, validation
  errors, and asynchronous progress/results.
- Text zoom at 200%, narrow effective viewport reflow, and contrast legibility.
- Reduced-motion behavior in the rendered app.
- Keyboard focus through a populated repository-to-walkthrough journey,
  including a focused chapter section heading, PR Guide citation-to-hunk links,
  and the provider/recovery dialogs.

Run and record those checks on the native app before treating the accessibility
acceptance criteria as complete. The Rust and frontend tests and the preview
pass remain implementation evidence, but cannot substitute for VoiceOver or
native zoom testing.
