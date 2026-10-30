---
kind: task
key: single-section-guide-player
phase: M3
title: Present multi-section answers one section at a time in Guide
priority: 1
labels: acceptance, guide, reader
related: progressive-answer-sections, design-parity, code-lens-click-error
---

## Purpose
Keep multi-part code explanations readable and paced in the normal Guide view. Users should see the active visual section with its narration and source, then move to the next section when ready.

## Observed gap
In the packaged Fontdone workspace, a Code Lens answer for `src/tt/glyf.rs:564–583` published multiple visual sections. Guide rendered every section as a full-height card in one long stack, leaving the reader partway down with earlier sections and the active title out of view. Chat history already provides access to older questions; duplicating every full visual answer in Guide obscured the active explanation.

## Required behavior
- Render a multi-section ordinary question or Code Lens answer as one focused visual section at a time, using the existing scene/player and section data.
- Provide Previous/Next navigation and a clear `Section n of m` position. Keep the exact selected section stable as new sections arrive; never jump the reader because the list grew.
- If a job is still building and no next section exists, keep Next disabled and say another section is being prepared. Enable it when a validated section is published.
- Keep narration, visual hotspots, source citations, section notes, nested exploration actions, and partial/failure status attached to the selected section and reachable without expanding unrelated sections.
- Keep every question, section, and citation in saved history. Older questions remain reachable from Chat history and the exploration tree; do not delete, rewrite, or silently replace saved work.
- Keep a single-section answer simple and repository Guide slide navigation unchanged.
- At narrow pane widths and app zoom, navigation and content must reflow without clipping; use the shared UI components and workspace spacing tokens.

## Acceptance
1. A multi-section Code Lens result opens on one section and does not render other full visual sections below it.
2. Previous/Next changes the selected section and its source together; each citation opens the correct captured file and range.
3. When a section is added during generation, the current selection stays on the same revision. Next becomes available without reloading or restarting generation.
4. Stopped, partial, interrupted, and unavailable sections remain explicitly recoverable and do not hide readable saved sections.
5. Ordinary one-section answers and repository-guide slides retain their current behavior.
6. The result preserves all saved turns and remains usable at the supported narrow layout and high zoom.

## Scope boundary
This is a Guide presentation change. Do not change provider selection, generation prompts, retrieval limits, permission grants, saved answer contents, or chapter revision identity. Evidence-based screen audit remains in `x21h`; this task owns this specific reader correction under `13yp`.

## Source implementation — 30 October 2026

`AnswerSection` now resolves one active section from the saved reading position
for the exact revision, renders that section's `VisualStory` and source links,
and shows Previous / Next with a `Section n of m` position only when the answer
has multiple sections. Navigation writes the selected section ID back to the
workspace reading state; newly published sections do not change the selected
ID. Scene audio advances within that section, while section changes remain
manual. The old `src/lib/walkthrough.ts` contract is no longer part of the
greenfield reader; its orphaned test was replaced with the current Guide-reader
contract in `tests/guide-reader.test.mjs`.

Source-level Guide-reader checks pass 2/2. `tests/workspace.test.mjs` and
`tests/reader-progress.test.mjs` also cover revision-scoped selection,
restoration and nested exploration positions. Packaged progressive generation,
source-return, keyboard, narrow-pane and high-zoom acceptance remain open; keep
`tt1s` and `13yp` open.
