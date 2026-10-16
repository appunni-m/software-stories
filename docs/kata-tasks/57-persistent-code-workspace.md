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
Interactive sample preview and reusable notification popover exist. Production shell migration is pending. No native layout parity is claimed.

## 16 October workspace migration

Production migration now consumes real immutable inventory/source in a shared activity-bar workspace; packaged acceptance is being recorded in the new UI audit task na54.
