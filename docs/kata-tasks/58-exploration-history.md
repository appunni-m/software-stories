---
kind: task
key: exploration-history
parent: design-parity
title: Persist exploration branches, answers and last-view state
priority: 1
labels: product, design, ux
---

## Outcome
A reader can follow questions at any depth, return one level or root, and reopen the same place without regenerating.

## Record contract
Versioned flat records keyed by exploration ID: project, immutable snapshot, parent ID, originating turn/card, title, ordered messages and artifact references. Independent presentation state: last Guide/Flow/history view, current file/range, code pane visibility, per-view scroll, graph viewport/expanded nodes, player section/time. Do not recursively embed children or duplicate chapter content per tab. Each card lists its real child references/count; depth does not create nested UI component trees.

## Transitions
Submitting chat or Explain this code appends a turn at the CURRENT level. Only explicit Explore further creates/opens a child. Guide/Flow/history switch presentation of the same exploration. Back restores parent state; root breadcrumb returns directly. Reopening a known branch uses saved state, never starts inference. Follow a citation without changing exploration; source identity is project/snapshot/path/hash/range. Switching project cannot mingle history or private context. Corrupt/missing/outdated records recover safely, preserving original content.

## Acceptance
Exercise at least five levels and sibling branches, repeated navigation, restart on each view, mixed source snapshots, removed source, deleted/corrupt preferences and content-version migration. Verify no duplicate generation, unchanged parent transcript, real counts, last tab restored and no chapter data loss. Keep revisioned answers separate so selective section edits preserve unrelated artifacts.
