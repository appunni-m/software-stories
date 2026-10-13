---
kind: task
key: t24
parent: m5
title: Meet accessibility, responsiveness, and repository safety requirements
priority: 1
labels: phase-5, accessibility, reliability
blocked_by: t02, t07, t08, t13, t16, t17
---

## User outcome
Developers can use core flows with keyboard, VoiceOver, zoom, and reduced motion, and large repositories do not freeze or execute untrusted code.

## Contract
Apply semantic roles, accessible names, focus order, announcements for async state, keyboard shortcuts, text zoom, contrast, and reduced-motion behavior across all three workspaces and review flows. Scans/parsing are bounded, cancellable background operations. Never execute repository files or generated visual code.

## Acceptance criteria
- All core journeys work without pointer input and have visible focus.
- VoiceOver announces current project/snapshot, selected file/section, validation errors, and async progress/results.
- Text zoom and reduced motion do not hide evidence or controls.
- Large fixtures remain responsive; scan/index limits and cancellation preserve app usability.
- Path traversal, symlink, malformed package, oversized provider response, and untrusted HTML/script inputs are rejected at the Rust boundary.

## Excludes
Claiming universal language support or replacing security review with UI validation.

## Done evidence
Keyboard/VoiceOver/reduced-motion pass on repository-to-walkthrough journey, performance report on a representative large fixture, and documented adversarial path/content checks.
