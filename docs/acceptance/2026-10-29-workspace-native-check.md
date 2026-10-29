# Native workspace check — 29 October 2026

This is a focused packaged-app check for Kata `13yp` and the gap ledger in
`docs/kata-tasks/64-workspace-ui-audit.md`. It does not qualify every page,
screen size, accessibility mode, or reader state.

## Build and workspace

- Installed app: `/Users/lazytrot/Applications/Code Atlas.app`, Code Atlas 0.1.0.
- App executable SHA-256:
  `0430c271caf7c4703a561b4bc8ca166cdbc102e92ebf3224aa94e73010226f80`.
- App code was built from `9df03b9`; current checkout is `df7e5d2`, which adds
  only the preceding UI audit note.
- CUA capture: 2880 × 1680 device pixels, corresponding to a 1440 × 840 logical
  Mac window.
- Repository and snapshot: Fontdone `main@5249b72371`, partial snapshot.
- No source, snapshot, answer, receipt, or audio content was changed. No model
  or narration request was sent.

## Observed journeys

| Area | Result | Observation |
| --- | --- | --- |
| Header | Pass at this window size | Repository, snapshot, **Load latest changes**, Codex connection/model, appearance, notifications, and keyboard help fit within the header. No overflow was visible. |
| Workspace shell | Pass for this state | VS Code style Explorer/Search/Changes/Explorations activity rail; persistent file tree and source pane; composer above content; Guide/Flow/Chat history inside the exploration. |
| Guide | Partial | The active `Outline::is_empty` answer shows a visual decision with an OR condition and separate true/false outcomes, a 13-second narration, collapsed narration/details, and source-backed controls. Its visual has one scene, so previous/next arrows and the multi-scene dot strip could not be inspected. |
| Flow | Partial | Three source-linked points for the same `Outline::is_empty` behavior cite `src/outline.rs:49–51`. This view is a static source relationship, not a runtime trace. |
| Nested navigation | Pass for one branch | Opened **Explain When `is_empty` returns true in more detail** and returned to the root. Back restored the root Guide, `src/outline.rs`, and the existing answer without generating anything. The child exploration contains further saved questions, including unavailable answer parts; no saved work was edited. |
| Search | Partial | `⌘⇧F` opened repository search and focused the query field. `memory_face` showed 47 matches across 22 files with an incomplete-coverage disclosure. Quick Open (`⌘P`) focused the file filter. No inference was sent. |
| Changes | Partial | On open, HEAD `5249b72371` automatically compared with parent `7c48b70648` (two changed paths). Selecting `7c48b70648` switched to its first-parent diff from `f3096b4e4c91` (four paths). An explicit local `f3096b4e4c91` → `5249b72371` comparison showed five paths and survived leaving and reopening Changes. Selecting HEAD restored the default comparison, then Back restored the same Guide and `src/outline.rs`. README's unsupported parser status still allowed a textual diff; the oversized retention inventory had no hunk; partial static-relationship coverage was disclosed. No repository or provider action occurred. Empty repositories and initial commits were not exercised. |
| Keyboard | Partial | A short Tab sequence moved through the repository breadcrumb, Guide and Flow actions, and composer. This is not a keyboard-only acceptance pass. |
| Notifications | Partial | The bell opened a bounded panel with grouped **Needs attention** and **Recent updates** entries, a visible close action, and per-question open/read/dismiss controls. The app displayed 27 attention items and grouped repeated attempts. No notification action was activated. |
| Connections | Partial | The centered dialog says choices apply across repositories and answers. Explanations showed the connected Codex model/reasoning; Voice showed Pocket TTS, ready status, Alba/English, local processing, and the one-time local storage estimate. No connection or voice setting was changed. |
| Light appearance | Spot check | The active Guide and source panes remained legible in Light mode; returning the selector to System restored the prior appearance. This is a visual spot check, not a contrast/accessibility measurement. |

## Deviations and remaining acceptance

- The selected Guide has one scene. Populate a multi-scene answer to verify
  scene arrows, dot-strip containment, and navigation actions.
- Only one wide logical window size was inspected. Narrow width, window resize,
  200% zoom, light theme, Increase Contrast, Reduce Motion, VoiceOver, and full
  keyboard-only navigation remain uninspected.
- Search, Changes, notification center, connection details, first run, and
  error/recovery states need durable screenshots and a complete interaction
  matrix. This run's CUA screenshots were inspected live and were not retained
  as files. The notifications and connection panels were inspected, but their
  settings/actions were not exercised.
- The saved Fontdone workspace includes prior failed or unavailable turns.
  Some remain visible because they own nested explorations; the stored answers
  were left untouched. This check does not determine whether every retained
  card is the best default presentation.
- The source/answer pair and one nested branch were restored correctly. This
  does not establish deeper branch, sibling, exact scroll, restart, or
  revision-fallback acceptance.
- Notifications, connection settings, and Light mode received spot checks;
  notification actions, model/voice changes, first-run and error recovery
  remain unqualified. Contrast, motion, zoom, narrow-window, and VoiceOver
  checks remain open.

**Conclusion:** the specific header overflow reported by the user is corrected
in the installed bundle at this window size. The packaged design acceptance
remains open under Kata `13yp`; no broader screen or accessibility gate is
closed by this check.

## Latest header and scene-row correction — 29 October 2026

After the user clarified that the scene navigation and top menu still looked
misplaced, the scene count was moved back to its own centered row below the
scene title at all Guide pane widths. The desktop header now gives bounded
shares to repository and account controls, allowing each group to wrap instead
of using an intrinsic-width account track.

- Focused layout checks: 3 passed.
- `npm run build`: passed.
- Native Tauri app bundle: built and installed over the existing app.
- Built and installed executable SHA-256:
  `07fa3b6708751ac249c9c350fe780f08195d0bad7f91f7fce152c72fcba429cc`.
- At 1440 × 840 logical pixels, the header controls fit on one line. The
  active Guide displays the scene count below the title, without overlaying it.
- The active saved visual has one scene; previous/next buttons and dots are
  absent, so multi-scene navigation is not visually qualified. Narrow-window,
  high-zoom, and accessibility checks remain open.

No source or saved repository data, answers, receipts, or narration changed.
