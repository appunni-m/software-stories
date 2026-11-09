---
kind: task
key: workspace-design-acceptance
parent: design-parity
title: Qualify the packaged conversation and code workspace against its design
priority: 1
labels: product, design, ux
---

> **Superseded 8 November 2026:** Kata `3rcd` was soft-deleted after
> duplicate packaged workspace qualification was consolidated into the single first-release workspace gate
> `13yp`. This specification is retained as dated history; formal accessibility
> audits are not release requirements.

## Outcome
The shipped Mac app looks and behaves like the reviewed workspace, with a novice developer able to understand real code without learning our pipeline.

## Journey
Open repository and connect once; ask a question, read/play a source-backed Guide, inspect a citation, switch to Flow, select a graph node, select code and Explain this code at that SAME level, drag a supporting file into chat, ask a follow-up, open a nested card branch five levels deep, return parent/root, restart and resume last view. Search with matching/no results and Explore with AI. Finish a job while reading elsewhere; open its result from the bell. No per-stage workflow modal, mandatory manual reference selection or repeated settings choice.

## Evidence
Use the current workspace design contract and preview, then record actual packaged screenshots for Guide, Flow with inline code, nested exploration, search and notification center in both themes. Compare proportions, spacing, density and action count. Clearly label preview/sample data versus actual generated content. Test a populated large Rust repo and TypeScript repo after explicit code-sharing authorization. Do not use schema/test totals as proof of narrative quality.

## Core interaction and regressions
Verify ordinary explorer, search, tabs, chat, selection, graph, and splitter workflows, including keyboard shortcuts where provided. No content is trapped in tiny panes at supported window sizes. Validate native job cancellation/recovery, source immutability, revisions/selective regeneration and changed snapshots. Formal screen-reader, zoom, contrast-preference, and motion-preference audits are out of scope. Existing parser, Codex account, Pocket, quality and release work remains separately tracked; close this task only for its recorded real-app journeys, never for the sample preview.
