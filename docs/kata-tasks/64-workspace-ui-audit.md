---
kind: task
key: workspace-ui-audit
parent: design-parity
title: Replace inconsistent pages with the conversational developer workspace
priority: 1
labels: design, ux, product
---

## Outcome
Replace the installed Guide/Files workflow with the conversational developer workspace shown in workspace-preview.html. Explorer, Search and Changes live in a narrow VS Code style activity bar; source and explanation receive the remaining space. Stacked conversations and nested explorations must be visible without discovering hidden controls.

## Audited inconsistencies and required fixes
1. Files: the compact-heading selector applies .has-snapshot and .view-atlas to the same element even though they are ancestor/descendant. The huge hero is therefore still visible. Hide the contextual heading visually while preserving the focus/landmark target.
2. Guide: two competing starting points (question form and tour dashboard), progress tables and metadata dominate the reading journey. Replace with a single pinned composer and real answer cards.
3. Files/source: three simultaneous empty instructions, assistant configuration sorted ahead of application code, expanded folders, no sensible initial source. Show one quiet prompt, collapse folders and choose a likely implementation entry point without hiding searchable files.
4. Navigation: Guide/Files/Changes across the header compete with local Explanation/Flow/Player tabs. Use the left activity bar for repository tools; Guide/Flow/Chat history belong to each exploration.
5. Conversation: preview follow-ups are below a large unrelated visual and branches appear only in breadcrumbs. Render question/answer pairs as a stack, real branch counts attached to originating turns, a hierarchical exploration list and parent/root controls.
6. Shared controls: legacy 8–11px typography, inconsistent button sizes, blue paragraph text, different pane headers, dense card nesting, duplicate status labels. Use shared Radix controls and neutral semantic surfaces; keep readable text and a consistent spacing scale.
7. Generation: source discovery and chapter generation own separate modals and discard intent. Give one question-specific background operation a lifetime independent of navigation, an explicit stop action, persisted result references and native evidence validation. A revocable snapshot permission replaces repeated source-review steps for the new workspace only; old explicit review/recovery remains available.
8. Saved work/edit/player: do not remove revision editing, source citations, independent section changes or Pocket TTS. Expose them on the answer that owns them.
9. Changes: comparisons require typed refs and show internal IDs. Provide a real bounded local commit list/ancestry with one-click comparison; retain exact diff/PR explanation functionality.
10. Search: current header search opens another workflow. Search captured paths/code beside the composer, with Explore with AI even for no results; do not run inference on typing.
11. Notifications/settings: keep background outcomes in the existing bell, one connection in the header, settings only on demand. Remove duplicate navigation and unused controls from the normal path.
12. First-run, capture/error, connection/recovery, chapter editor, player and Git details: review shared typography/spacing/focus; preserve actionable errors and existing records. Do not claim these flows passed merely because a detached preview looks correct.

## Acceptance
Build, run frontend and native regression checks, and inspect the packaged app. Record screenshots of real source + Guide, Flow, stacked follow-ups, at least two branch depths, Search, Changes/commit history, settings and first-run where feasible. Verify evidence remains snapshot-bound, cancellation preserves prior drafts, navigation does not cancel generation, re-opening restores branches and last view, selection explains at the current level, and ordinary typing never sends source. Native permissions must bind snapshot/account/runtime and be checked again at dispatch, including after revocation. Use the existing single port 4173. Install the verified replacement bundle and open it; clearly record any incomplete qualification instead of closing unrelated gates.

## October 16 migration checkpoint
The production workspace replaces normal Guide/Files routes and reuses native source, chapters, permission validation and generation. Automatic local indexing is shared between reading and questions so there is no extra Build index step in the normal path. Short natural follow-ups are accepted and retain recent question intent. Shared source controls correctly identify file languages; JavaScript/TSX variants use the existing syntax highlighter and selection shortcuts use inclusive end lines. Captured-source search and real local commit history are implemented. Browser proof images show stacked follow-ups, originating-turn child links and branch counts; they remain labelled sample design evidence. Frontend 126 tests / native 198 tests pass. Packaged acceptance remains blocked by the locked Mac, not considered passed. Root-commit comparison, complete reading-position recovery and durable scheduling remain follow-up work in the workspace task set.
