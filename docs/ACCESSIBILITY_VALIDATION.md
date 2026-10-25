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
- Code Lens source wraps long lines when the effective viewport is narrow.
  The line-number gutter stays separate and continuation text aligns with the
  code column. A source regression check covers the layout; the packaged 200%
  Code Lens result is recorded below.
- Reduced-motion rules remain active for application animations, transitions,
  and smooth scrolling. A source-level regression check now verifies the global
  CSS override and the instant-scroll preference helper in
  [accessibility.test.mjs](../tests/accessibility.test.mjs); rendered behavior
  on the packaged app remains unverified.
- The default dim token now has at least 4.5:1 contrast against the four main
  application surfaces. When the WebView reports `prefers-contrast: more`, the
  app raises text, border, separator, and keyboard-focus contrast. In addition
  to CSS-token checks, [accessibility.test.mjs](../tests/accessibility.test.mjs)
  checks detectable solid foreground/background pairs declared together in a
  rule under default and Increase Contrast tokens. That source scan does not
  resolve inherited backgrounds, gradients, transparency, or final rendered
  native WebView colors.
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
  threshold on its current background (computed ratio: 5.92:1). The source scan
  found and corrected the inventory and coverage table headers from 4.29:1 to
  5.21:1. Native Increase Contrast review and rendered color-pair validation
  are still pending.
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

The following acceptance areas remain **incomplete** after the partial native
run recorded below:

- Spoken VoiceOver announcements for project, snapshot, file, section,
  validation errors, and asynchronous progress/results.
- Full 200% acceptance across the app and actual window resizing. The packaged
  Code Lens source now wraps long lines and aligns continuation text at 200%;
  Atlas, Walkthrough, Chapter Studio, dialogs, and a separately resized app
  window still need review.
- Reduced-motion animation and scrolling behavior across a populated chapter
  and its citations.
- Keyboard focus through a populated repository-to-walkthrough journey,
  including section navigation, citation return, PR Guide citation-to-hunk
  links, and provider/recovery dialogs. The native run reached a selected file
  and its read-only Code Lens source, but this snapshot has no accepted
  chapters. The synthetic visual fixture covers section-heading focus and
  source-link return only.

Complete the remaining checks on the native app before treating the
accessibility acceptance criteria as complete. The Rust and frontend tests and
the preview pass remain implementation evidence, but cannot substitute for
spoken VoiceOver announcements or a populated native journey.

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

### Packaged native run — 2026-10-15

```text
Date/time and tester: 2026-10-15 06:45 IST; Codex via native computer use
macOS version / app version+commit: macOS 15.7.7 (24G720); Code Atlas 0.1.0,
  unsigned packaged debug app, commit e44f002
Repository / snapshot ID: fontdone / snap-1792025678232575000-74046-0
VoiceOver: inconclusive — enabled from System Settings. The accessibility
  tree exposed the selected heading and the CUA could inspect the VoiceOver
  setting, but it did not expose spoken announcements. The attempted
  Control-Option-Right traversal did not produce a verifiable focus change.
Keyboard-only journey: partial pass — visible focus was observed on Chapter
  Studio. Tab/Return opened Atlas inventory, Tab reached the selected
  src/lib.rs row, and Return opened Code Lens. The packaged app showed the
  verified, read-only snapshot source and hash. No accepted guide exists, so
  section, citation-return, validation, and dialog flows were unavailable.
200% zoom / effective width: incomplete — ⌘= five times enlarged the packaged
  app and ⌘− five times restored it. Navigation wrapped and project context
  reflowed; long source lines extended past the visible viewport. The app
  window was not resized and horizontal source navigation was not verified.
Increase Contrast: partial pass — baseline was off; enabling it reached the
  packaged WebView and made its boundaries/focus treatment visibly stronger.
  A populated guide and status/dialog states were unavailable. Restored off.
Reduce Motion: partial pass — baseline was off; with it enabled, navigation
  reached the Walkthrough empty state. There were no accepted sections or
  citations with which to verify section movement or scroll animation.
  Restored off.
Unresolved controls or announcements: no accepted chapters; no spoken
  announcement stream; no populated citation, validation, provider, or
  recovery flow; no narrow-window 200% check; source-line horizontal navigation
  not verified. VoiceOver, Increase Contrast, Reduce Motion, Reduce
  Transparency, and page zoom were returned to their original off/100% state.
```

### Packaged Code Lens reflow follow-up — 2026-10-15

```text
Tester: Codex via native computer use; unsigned Code Atlas 0.1.0 package built
  from the working tree based on commit f28652a.
Repository / snapshot ID: fontdone / snap-1792025678232575000-74046-0
Zoom: started at 100%; pressed ⌘= five times to reach the app's 200% setting.
Result: pass for Code Lens source reflow. The packaged app displayed long Rust
  source lines wrapped inside the code panel. Line numbers remained in their
  own gutter, and continuation text aligned with the code column. The lines
  were fully visible without horizontal source scrolling. Pressed ⌘− five
  times and visually confirmed the normal 100% source and relationship layout.
Scope: this verifies Code Lens source reflow at the 200% page-zoom setting only.
  The app window was not resized, and Atlas, Walkthrough, Chapter Studio,
  dialogs, VoiceOver, Increase Contrast, and Reduce Motion were not retested.
```

### Packaged native follow-up — 2026-10-25

```text
Date/time and tester: 2026-10-25 05:28 IST; Codex via native computer use
macOS version / app version+commit: macOS 15.7.7 (24G720); Code Atlas 0.1.0.
  The installed bundle does not expose its source commit; workspace HEAD was
  bc8f552 during this check.
Repository / snapshot ID: fontdone / main@5249b72371
VoiceOver: incomplete — baseline was off. I temporarily enabled it and checked
  the caption-panel preference, but no spoken transcript was visible through
  the app observation. No announcement claims are made; VoiceOver was restored
  off. The earlier native attempt remains inconclusive for the same reason.
Keyboard-only journey: partial — Tab moved visible focus from Explain this
  code to Add selected lines to your question and then the source/Guide splitter.
  No activation was used on provider actions. This confirms focus visibility
  across those controls, not the full repository-to-guide journey.
200% zoom / effective width: fail at the tested window size — five ⌘= presses
  enlarged the page from the 100% baseline. The Guide was no longer visible
  beside the source in the viewport, and long source text was clipped at the
  code pane boundary. The app window was not resized, so the narrow-window
  combination remains untested. Five ⌘− presses restored the original 100% view.
Increase Contrast: partial pass — baseline was off. With the macOS preference
  enabled, the app showed stronger white component boundaries and visible focus
  outlines. No validation dialog or status/error state was reviewed. Restored off.
Reduce Motion: partial — baseline was off. With it enabled, saved visual
  navigation moved from scene 1/3 to scene 2/3 and the app remained usable.
  Playback stayed stopped. This static observation does not verify animation or
  smooth-scroll suppression; a citation destination was not established in
  this pass. Restored off.
Unresolved controls or announcements: spoken VoiceOver output; a complete
  keyboard-only file-to-section and citation-return journey; narrow-window
  200% reflow; rendered status/dialog contrast; and reduced-motion behavior
  during an actual scroll or transition.
```

The accessibility settings and page zoom were restored to their original
off/100% values. The saved scene was returned to scene 1; no narration played,
and no answer, notice, or receipt was modified.
