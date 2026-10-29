---
kind: task
key: default-changes-comparison
phase: M6
title: Open Changes on a useful default commit comparison
priority: 2
labels: acceptance, changes, git
related: pr-diff-evidence, design-parity
---

## Purpose
Make the VS Code-style Changes activity useful as soon as it opens, while keeping every comparison local and read-only.

## Observed gap
In the packaged Fontdone workspace, entering Changes showed the 80-commit history in the activity sidebar but left the main pane empty with “Select a commit on the left to inspect its changes.” The latest commit was already visible. Selecting it produced a first-parent comparison with changed-file and diff details. The extra click left the main work area empty on first entry.

## Required behavior
- If the user opens Changes without explicit starting/ending refs, select the current local HEAD commit and compare its first parent to HEAD.
- Start only when the commit list is loaded and a first parent exists. If no comparable commit exists, show a useful empty state with a direct next action.
- Preserve explicit branch/tag/commit comparisons across navigation and reopening Changes; never overwrite a user-selected range with the default.
- Keep the commit list and changed-file/diff view visible together. Clearly identify the selected commit and comparison direction.
- Do not modify the working tree, run repository code, or invoke AI/provider work.

## Acceptance
1. Opening Changes in a repository with a comparable HEAD shows the HEAD-versus-parent diff without requiring a commit click.
2. Selecting another commit still compares that commit to its first parent.
3. A previously chosen custom comparison remains selected when returning to Changes.
4. Empty repositories and initial commits show a clear explanation and next action without starting a failed comparison.
5. Unsupported, oversized, and partial comparisons remain labeled truthfully.

## Scope boundary
This improves Changes entry only. It does not add remote GitHub data, full DAG editing, merge resolution, PR generation, or AI review.

## Source checkpoint — 29 October 2026

The default comparison now waits for a successful, completed history read and
uses the explicitly identified checked-out `HEAD`, not the first parented row
from `git log --all` (which can belong to another branch). The read-only
history command includes detached `HEAD` when it is outside the first 80
reachable entries, while retaining the all-ref commit rail. The rail marks
`HEAD`; empty repositories, a missing current commit, and an initial `HEAD`
receive specific guidance. Explicit comparison refs still bypass the default.

This is source-level evidence only. The native Changes screen has not been
reopened for this correction; do not close the task until its packaged
acceptance cases are checked.
