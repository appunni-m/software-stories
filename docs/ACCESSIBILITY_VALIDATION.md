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
  and smooth scrolling. A source-level regression check now verifies the global
  CSS override and the instant-scroll preference helper in
  [accessibility.test.mjs](../tests/accessibility.test.mjs); rendered behavior
  on the packaged app remains unverified.
- The default dim token now has at least 4.5:1 contrast against the four main
  application surfaces. When the WebView reports `prefers-contrast: more`, the
  app raises text, border, separator, and keyboard-focus contrast. CSS-token
  checks are in [accessibility.test.mjs](../tests/accessibility.test.mjs).
- The Tauri main WebView enables [Tauri's page-zoom hotkeys](https://v2.tauri.app/reference/config/#zoomhotkeysenabled)
  on macOS: `⌘=` / `⌘−` in 20% steps. The narrow-window and enlarged-text
  result still needs a packaged-app check.
- Named source, citation, transcript, provider-review, and relationship-map
  containers now use roles that accept accessible names. The narration seek
  slider uses its visible “Audio position” label. A TypeScript-AST regression
  check rejects `aria-label`/`aria-labelledby` on generic containers without a
  nameable role. The [W3C naming guidance](https://www.w3.org/WAI/ARIA/apg/practices/names-and-descriptions/)
  identifies `generic` as prohibited from naming. This source check does not
  prove the native WebView's VoiceOver output.
- PR Guide citation buttons remain keyboard-operable, expand the cited file,
  scroll the exact hunk into view, and highlight it. Citation scrolling uses
  non-smooth behavior when reduced motion is requested.

## Evidence available in source

- Workspace shortcuts and menu keyboard handling are in
  [App.tsx](../src/App.tsx).
- Modal focus containment and the recovered-preview focus return are implemented
  in [App.tsx](../src/App.tsx).
- Walkthrough section navigation exposes aria-current, Previous/Continue
  buttons, keyboard section movement, and focus restoration. The focused section
  heading includes its position and title in its accessible name in
  [Walkthrough.tsx](../src/Walkthrough.tsx).
- Notices, interrupted jobs, captions, and narration errors use status or alert
  semantics in [App.tsx](../src/App.tsx) and
  [NarrationPlayer.tsx](../src/NarrationPlayer.tsx).
- Repository and snapshot route changes update a persistent polite live region
  with the repository name and selected snapshot branch, commit, completeness,
  and creation date. Native VoiceOver confirmation remains pending.
- Focus restoration in the navigation model is covered by
  [navigation.test.mjs](../tests/navigation.test.mjs); that test does not drive
  the rendered UI.
- Reduced-motion styles and focus/reflow fixes are in
  [App.css](../src/App.css).
- Increase Contrast is implemented with the standard
  [`prefers-contrast: more`](https://www.w3.org/TR/mediaqueries-5/#prefers-contrast)
  media feature: it strengthens border tokens, applies them to component
  boundaries, and gives focused controls a 3 px white outline. The Node fixture
  checks declared palette ratios and rule presence; it does not prove the
  packaged WebView receives the macOS preference or that the rendered app
  remains legible at every size.
- Chapter Studio section navigation and Code Lens citation navigation now
  select non-animated scrolling when macOS Reduce Motion is enabled. The
  reduced-motion media query also disables CSS animations and transitions
  throughout the app. The preference helper and CSS contract are covered by a
  source-level test; rendered behavior still requires the native acceptance
  pass below.
- Sidebar help text color was raised to meet the 4.5:1 normal-text contrast
  threshold on its current background (computed ratio: 5.92:1). Native Increase
  Contrast review and the remaining rendered color-pair audit are still pending.
- The Atlas inventory tree filters with retained-path sets, renders a virtual
  row window, and exposes a single roving tree-item focus stop, Left/Right
  expand/collapse and parent/child navigation, and hierarchy/position metadata
  in [App.tsx](../src/App.tsx). Selecting a tree row with a pointer moves focus
  to that tree item so its selected state is announced. A 25,000-row helper
  fixture in [atlas.test.mjs](../tests/atlas.test.mjs) checks that a 510
  CSS-pixel viewport calculates a maximum 39-row window at the default row
  height. This checks the bounded window calculation, not rendered
  responsiveness or screen-reader acceptance.
- `accessibility.test.mjs` parses the TSX AST and checks that labeled generic
  containers use a role that supports naming. This catches markup regressions;
  it does not test the native accessibility tree.
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

On 2026-10-14, the same preview server rendered the synthetic accepted-guide
fixture at `/tests/visual-demo.html`. Keyboard navigation reached each source
link and section control. Enter on the source link opened its exact cited range,
focused the destination heading, and retained the fixture's 18-second playback
position and 243-pixel transcript position; Back restored focus to the source
link. Enter on the section control moved to section 2 and focused its heading;
Previous returned focus to the original section control. The reader remained
manual-pace with no auto-advance. This exercises the static synthetic fixture,
not a populated repository or the packaged/native walkthrough.

On 2026-10-14, the same synthetic fixture was rendered at a 640 by 900 CSS-pixel
viewport as a narrow-width check. The document width remained 640 px with no
page-level horizontal overflow. The left-to-right diagram used its own
horizontal scroller (520 px viewport, 748 px content), keeping the surrounding
reader in bounds. This viewport override does not enlarge text and is not a
200% zoom or packaged WebView acceptance pass.

## Manual checks still required

The following are **not verified**:

- VoiceOver announcements for project, snapshot, file, section, validation
  errors, and asynchronous progress/results.
- Text zoom at 200%, narrow effective viewport reflow, and rendered contrast
  behavior (including whether macOS Increase Contrast reaches the packaged
  WebView).
- Reduced-motion behavior in the rendered app.
- Keyboard focus through a populated repository-to-walkthrough journey,
  including PR Guide citation-to-hunk links and the provider/recovery dialogs.
  The synthetic visual fixture covers section-heading focus and source-link
  return only.

Run and record those checks on the native app before treating the accessibility
acceptance criteria as complete. The Rust and frontend tests and the preview
pass remain implementation evidence, but cannot substitute for VoiceOver or
native zoom testing.

## Native acceptance run sheet

Use the packaged macOS app built from the commit being reviewed. Record the
macOS version, app version/commit, repository, snapshot ID, and VoiceOver state.
Use a populated snapshot with an accepted multi-section chapter. If a provider
review or PR Guide is needed, open its review surface but do not send a request
as part of this accessibility pass.

1. **VoiceOver.** Enable VoiceOver with `⌘F5`. Navigate without the pointer
   through the project and snapshot picker, Atlas file tree, Code Lens source
   range, Guide Plan, Walkthrough section rail and reader, Chapter Studio
   validation, and any provider or recovery dialog used in the journey. Record
   whether VoiceOver announces the selected repository/snapshot, file path and
   state, exact section and source destination, validation result, and async
   progress/result. Check that dialog entry announces its title, Tab remains
   inside it while modal, Escape/cancel restores focus to the invoker, and no
   error or selection state is conveyed by color alone.
2. **Keyboard-only journey.** With VoiceOver off, repeat the populated journey
   using Tab, Shift+Tab, arrow keys, Return, and Escape. Check the repository
   tree, section Previous/Continue controls, source-citation return, PR
   citation-to-hunk link, and provider/recovery dialogs. Record any pointer-only
   action, lost focus, invisible focus, unexpected section/playback change, or
   focus that does not return to its origin.
3. **Zoom and reflow.** Set the app's supported page/text zoom to 200% if
   available. On the packaged Tauri app, press `⌘=` five times from 100%; each
   press increases page zoom by 20%. Use `⌘−` five times to return to 100%.
   Resize the app to the resulting narrow effective content width, and inspect
   all controls and evidence in Atlas, Code Lens, Walkthrough, Chapter Studio,
   and the open dialog. Record the actual zoom and window size. The macOS screen
   magnifier by itself is not a reflow pass. Confirm that text, citations,
   validation messages, and primary controls remain visible and operable.
4. **Contrast and motion.** Enable macOS Increase Contrast and Reduce Motion,
   then repeat section navigation, citation navigation, menu/dialog open/close,
   and status/error review. Confirm focus remains visible, information is not
   conveyed by color alone, and the same destinations work without smooth
   scrolling or decorative transitions.

Record results before closing issue `bcn2`:

```text
Date/time and tester:
macOS version / app version+commit:
Repository / snapshot ID:
VoiceOver: pass | fail — notes:
Keyboard-only journey: pass | fail — notes:
200% zoom / effective width: pass | fail | unsupported — notes:
Increase Contrast: pass | fail — notes:
Reduce Motion: pass | fail — notes:
Unresolved controls or announcements:
```
