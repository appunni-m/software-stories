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
