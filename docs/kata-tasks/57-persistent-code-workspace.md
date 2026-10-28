---
kind: task
key: persistent-code-workspace
parent: design-parity
title: Build the shared developer workspace with standard components
priority: 1
labels: product, design, ux
---

## Outcome
The reader has one persistent developer workspace: explorer left, top chat composer, resizable source and explanation. Guide, Flow and history belong to the current exploration. Settings never take a permanent pane.

## Work
Use the current design contract in docs/design/WORKSPACE_EXPLORATION.md and the real-component workspace preview. Reuse Radix buttons/tabs/inputs/popovers, CodeMirror source selection, React Flow and keyboard-accessible resizable panels. Extract shared components; do not copy the preview's illustrative data into production. Adapt the existing native immutable inventory/source commands to the shell. Remove the separate Files destination from the main learning journey while preserving source routes, chapter editing, changes and recovery as contextual tools. Preserve existing deep links and source/reader return behavior through an adapter.

## Acceptance
The explorer stays mounted in Guide, Flow and history. Chat stays visible during independent code/content scrolling. Opening/closing a source pane does not reset explanation state. Full-width content is possible by closing source. Settings only open on demand. At 1280x800 the majority of space is real code/content; no workflow hero, phase checklist or permanent metadata column. Light/dark themes share readable tokens and a 4/8/12/16/24/32 spacing scale. Verify keyboard split resizing, 200% zoom/reflow and large explorer responsiveness. Record packaged screenshots against the same preview; relabeled navigation alone does not pass.

## Current status
The production app now uses a persistent activity-bar workspace with captured Explorer source, Search, Changes and nested Explorations; Guide, Flow and chat history are views within each exploration. The interactive preview remains illustrative. Shared controls are only partly migrated, and packaged layout parity, focus/zoom behavior and the full screen matrix remain unqualified.

## 16 October workspace migration

Production migration now consumes real immutable inventory/source in a shared activity-bar workspace; packaged acceptance is being recorded in the new UI audit task na54.

## Shared action primitive — 28 October 2026

Added `src/design/AppButton.tsx`, a forward-ref Radix Themes button wrapper with a safe `type="button"` default and shared primary, secondary, quiet and icon intents. The connection and notification header triggers use its custom style path; repository-discovery and planning actions use the primary/secondary intents, notification actions use quiet, and the planning close control and chapter-recovery reload use icon/secondary. Existing labels, handlers, discovery consent boundaries, notification focus behavior and spacing classes remain intact.

The rest of the app still contains native and one-off button implementations. This source change does not establish packaged visual or accessibility parity; the broader component inventory, keyboard/split behavior, zoom/reflow and app screenshots remain open.

## Offline export action migration — 28 October 2026

The Offline Export trigger, close action, select/clear control, and footer actions now use the shared `AppButton` wrapper. Existing style classes, labels, handlers and disabled states are preserved; the trigger now also explicitly uses `type="button"`. This is a component-consistency increment only. No test/build or packaged visual/accessibility acceptance ran, and the wider migration remains open.

## Flow and validation action migration — 28 October 2026

The Flow relationship controls and Chapter Studio validation actions also use `AppButton`; their flow-specific classes, quiet-action styling, labels, disabled states, and source-navigation handlers are retained. The shared wrapper now covers actions across the reader, Flow, validation review, discovery/planning, connection/notification, recovery, and offline export surfaces. The remaining native controls and one-off styles have not been inventoried exhaustively. No tests/build or packaged visual/accessibility acceptance ran; task remains open.

## Agent handoff action migration — 28 October 2026

The manual Agent Handoff dialog's close, prepare, copy, import-validation, backend-change, and save-draft actions now use the shared wrapper with their existing intent classes and handlers. This standardizes controls but does not make manual handoff equivalent to a connected coding-agent backend. No tests/build or packaged acceptance ran.

## Connection settings action migration — 28 October 2026

The Connections dialog's close and footer actions, plus Pocket setup/repair/cancel and Google key actions, now use `AppButton`. The Google key Save action explicitly remains a form submit; Remove remains a non-submit action. Existing credential storage, voice selection, handlers and state are unchanged. The navigation categories remain their current controls pending a broader interaction/accessibility review. No tests/build or packaged acceptance ran.

## Connection category tabs — 28 October 2026

Replaced the Connections dialog's `aria-pressed` navigation buttons with Radix Tabs, matching the main exploration's Guide/Flow/history tab implementation. Active styling now follows the tab's `data-state`, and the library supplies tab semantics and arrow-key navigation. Category state, busy disabling, and the existing Connections/Voice contents remain the same. No tests/build or packaged keyboard/focus check ran.

## Initial connection chooser actions — 28 October 2026

The one-time AI connection chooser and unsupported-connection notice now use `AppButton` for close, Open Connections, Codex, direct API, Claude and Antigravity actions. Existing visibility/availability checks and callbacks are unchanged. This is a shared-control migration only; it does not claim that manual Claude/Antigravity handoff is an integrated agent backend. No tests/build or packaged screen acceptance ran.

## Notification panel header actions — 28 October 2026

The notification panel's Mark all read, Dismiss read and Close controls now use the shared `AppButton` wrapper while retaining their existing classes, accessible labels, title text, callbacks and Radix popover close behavior. No notification data or storage behavior changed. No tests/build or packaged visual/accessibility acceptance ran.

## Chapter draft recovery actions — 28 October 2026

The conflict-resolution Back, bulk choice, and continue actions now use `AppButton` quiet/primary intents; the saved-preview review action in Chapter Recovery uses the quiet intent. Existing labels, enablement, choices, callbacks and chapter revision behavior are unchanged. No tests/build or packaged acceptance ran.

## Stale draft rebase action — 28 October 2026

The stale-draft comparison's Rebase action now uses the shared secondary intent. Its visibility, callback and revision context remain unchanged. No tests/build or packaged acceptance ran.

## Claude generation review actions — 28 October 2026

Claude generation's Generate, Cancel, Save draft, citation navigation and Review another request actions now use shared primary, secondary and quiet intents. Existing disabled conditions, generation/recovery callbacks, labels, citations and saved-draft lifecycle are unchanged. This is shared UI control work only; it adds no Claude runtime capability. No tests/build or packaged acceptance ran.

## Claude request preparation actions — 28 October 2026

Claude's fixed file-scope Remove action, exact-request preview/refresh action, and request-preview discard action now use shared quiet/secondary intents. File selection limits, request preview fingerprinting, consent boundaries and callbacks are unchanged. No tests/build or packaged acceptance ran.

## Codex account and generation actions — 28 October 2026

Codex dialog controls now use shared icon, primary, secondary and quiet intents for close, account setup, model refresh, source review, citations and draft/generation lifecycle actions. The executable form's Save action explicitly remains a submit button; all other actions retain button behavior. Existing auth, review, consent, selection, and generation handlers are unchanged. No tests/build or packaged acceptance ran.

## Changes view keyboard tabs — 28 October 2026

The compact Changes surface now uses Radix Tabs for Diff and PR Guide, including roving arrow-key navigation and tab/panel semantics. Both panels remain mounted while one is hidden so switching views retains in-progress PR Guide state. Active styling follows the Radix `data-state` attribute. No tests/build or packaged keyboard/accessibility review ran.

## PR Guide action migration — 28 October 2026

PR Guide recovery, hunk review, provider request, history refresh, saved-draft, and accept/reject actions now use the shared `AppButton` wrapper while retaining their existing classes, labels, disabled states, callbacks, and explicit non-submit behavior. Rich citation links remain native buttons so their multi-line cited content and source-navigation semantics stay intact. No tests/build or packaged interaction review ran; the full UI and accessibility gates remain open.

## Git Changes actions — 28 October 2026

The repository-open, ref-swap, compare/stop, and captured-source actions now use `AppButton` while preserving their existing classes and callbacks; the compare action remains the form submit, and every other action remains a non-submit button. Specialized changed-file selectors and hunk disclosure controls remain native elements. No tests/build or packaged Changes review ran.

## Agent setup and discovery actions — 28 October 2026

Claude setup, source discovery, and repository planning now use `AppButton` for their close, setup, catalog, back, retry, and cancellation actions. Submit buttons remain explicit form submits; other actions keep the shared non-submit default. Existing status, consent, enablement, and callbacks are unchanged. No tests/build or packaged dialog review ran.

## Direct API settings and review actions — 28 October 2026

Direct API settings, request review, citation navigation, and cancellation actions now use `AppButton`. Added a shared danger intent for the existing cancel-confirm action. Key saving and catalog-style forms retain explicit submit semantics; other controls stay non-submit. The selected provider, disclosures, consent, source scope, callbacks and state transitions are unchanged. No tests/build or packaged dialog review ran.

## Narration player actions — 28 October 2026

Narration editing, playback, review, cached playback, connection shortcuts, and cancellation controls now use `AppButton` while retaining the player-specific classes, labels, disable conditions and callbacks. Playback/lexicon controls keep their tailored styling; cancel confirmation uses the shared danger intent. Audio caching, synthesis, consent and playback behavior are unchanged. No tests/build or packaged player review ran.

## Guide scene navigation controls — 29 October 2026

The visual Guide's previous/next scene buttons and scene dots now use Radix
`IconButton`; **Full map** uses the shared `AppButton`. Their accessible names,
disabled state, active-scene indicator, sizing classes, and existing actions
remain in place. This extends the shared-control migration to the visible
scene toolbar without changing navigation or saved reader progress.

The first packaged check caught that **Full map** stretched into the left
column after moving to `AppButton`. A dedicated grid slot now keeps the shared
button right-aligned. `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run bundle:macos`
passed frontend compilation, release build, metadata validation,
app/DMG bundling, and checksum verification. The installed `pillow-rs` Guide
shows centered scene navigation and **Full map** at the right edge; the
selected `canvas.rs` answer and narration remain in place. No question or
provider request was sent, and no test suite ran. Packaged keyboard/focus
review and broader workspace acceptance remain open.

Installed executable SHA-256:
`31bd63e308eae89b0990b269368a97e1766f5372e9fb02346b84acbf506ecaa7`.
DMG SHA-256:
`c28ea0867735d3ac9324cea9c4d85b98c08d2f0f0ad264cf6020e39c7d512226`.
