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

## Packaged default and selection restoration — 30 October 2026

In the installed Code Atlas 0.1.0 app (executable SHA-256
`659ad188436b1a6f0818c71c1e85fb763481ced55808b1072f92e625fe4668c8`),
Fontdone `main@5249b72371`, opening Changes without choosing a comparison
selected `HEAD` and immediately displayed its first-parent diff from
`7c48b706482b` to `5249b7237194`: two changed files, with the README hunk
visible. Selecting the `7c48b706482b` commit then displayed its first-parent
comparison from `f3096b4e4c91` to `7c48b706482b`, with four changed files.
After switching to Explorer and returning to Changes, that selected commit
and four-file comparison were retained. I restored the default HEAD comparison
and returned to the original Guide/Explorer view.

This verifies the default HEAD comparison, first-parent selection for another
commit, and selection retention across activity navigation in the installed
app. The check was read-only: no repository refs/files changed and no AI work
ran. A custom branch/tag range, empty history, initial commit, errors, and
partial/oversized comparison states remain unqualified; keep this task open.

## Custom commit range survives activity navigation — 30 October 2026

In the same installed Fontdone workspace, I entered the explicit commit range
`f3096b4e4c91fa1b8c9ff2e2c48b2dce548f2046` →
`5249b723719445813b8771a602ea5b96891d4450`. **Compare versions** displayed
the five-file diff and the selected README hunk. Switching to Explorer and
back to Changes retained both the exact refs and five-file comparison. I
restored the default HEAD/first-parent comparison and the original Guide view.
No repository files or refs changed and no AI operation ran.

This covers explicit commit-range retention across activity navigation. A
branch-name or tag-name range, custom comparison after restart, empty history,
initial commit, failure messages, and partial/oversized states remain open.

## Rechecked on the current packaged build — 31 October 2026

On the installed app built from `d91e3f2759a065bea8e44a5ccdaab8a550898591`
(executable SHA-256
`e8c761717da958e3ef9efd8b7d9e621ec6659672f95bad61790da18d2c0fd93b`), Fontdone
`main@5249b72371` opened Changes directly on `HEAD` versus its first parent:
`7c48b706482b` → `5249b7237194`. The two changed files were shown with truthful
`Unsupported` and `Oversized` labels; the README's textual hunk remained
inspectable with its exact added line. Selecting `7c48b706482b` changed the
comparison to its parent, `f3096b4e4c91` → `7c48b706482b`, and showed four
changed files. Returning to the `HEAD` row restored the default comparison.
I then restored the original Explorer, Fontdone exploration, Guide, selected
`src/api.rs:710–728`, and scene 2 of 3. No repository ref or file, explanation,
or provider state changed.

This reconfirms acceptance cases 1–2 on the current installed bundle. The
branch/tag range, restart persistence of a custom range, empty/initial history,
and distinct error states remain open; keep Kata `0vd8` open.

## Custom branch/tag refs survive app restart — 31 October 2026

The Changes screen now stores the last submitted explicit local ref pair in
per-repository local storage. It restores that pair when Changes opens without
route refs, opens the range editor so the values are visible, and leaves the
automatic HEAD-versus-parent default unstored. Malformed, oversized, and
unavailable preferences are ignored. Source coverage is in
`tests/gitComparisonPreferences.test.mjs`.

Packaged acceptance on the rebuilt app (executable SHA-256
`45f9d1a5bd4056b4603938b5b6215169d80e1daf732447b81c1be6f6a3c97feb`):
compared tag `v2.14.3` to branch `main` in Fontdone, quit and relaunched Code
Atlas, then reopened Changes. The range editor showed the same `v2.14.3` and
`main` refs with the prompt to compare them. The comparison is not silently
rerun after restart; the user can activate **Compare versions**. I returned to
the original Guide and `src/api.rs:710–728`. No AI request, repository write,
or saved-answer change occurred.

The pair's persistence and visible restoration are now qualified. Empty and
initial history, distinct failure states, and partial/oversized comparisons
remain to be checked; keep Kata `0vd8` open.

## Missing-ref feedback in the packaged app — 31 October 2026

With the same Fontdone snapshot, submitted `missing-ref-codeatlas-acceptance`
as the starting ref and `main` as the ending ref. The Changes view reported
that no matching local branch, tag, remote-tracking ref, or full commit ID was
found, identified the missing starting ref, and asked for another ref. This
was a local read-only lookup with no repository changes or AI request. I then
restored the valid `v2.14.3 → main` comparison and returned to the original
Guide/source selection.

The missing-ref case is qualified. Empty/initial history, native bridge
failure, and partial/oversized comparison behavior remain separate open cases.
