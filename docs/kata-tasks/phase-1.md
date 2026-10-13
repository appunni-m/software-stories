---
kind: milestone
key: m1
title: M1 — Local repository truth
priority: 4
labels: container, milestone, phase-1, repository
blocked_by: m0
---

## Goal
Make the app useful offline by selecting a local repository, capturing stable source snapshots, accounting for every path, and linking the Atlas to exact code.

## Gate
On a representative local repository, every scanned/skipped path is visible with a reason; parser relationships carry evidence and confidence; every Atlas/Code Lens navigation resolves to bytes in the same snapshot. The selected repository remains unchanged.

## Child issues
`03-project-selection.md`, `04-snapshot-capture.md`, `05-file-inventory.md`, `06-symbol-index.md`, `07-atlas.md`, and `08-code-lens.md`.
