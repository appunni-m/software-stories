---
kind: task
key: t03
parent: m1
title: Open a local repository and create a Code Atlas project
priority: 0
labels: phase-1, repository, onboarding
blocked_by: t02
---

## User outcome
The user can add a local Git working copy and understand which folder will be read before any source capture begins.

## Contract
Use the native macOS folder picker. After selection, show the canonical path, detected Git root, branch/HEAD when available, access status, and a clear **Create snapshot** action. Cancel leaves the current workspace unchanged. Non-repository folders and inaccessible folders show a recoverable explanation and a way to choose again. Store recent project paths locally and let users remove a recent entry without deleting its repository.

## Acceptance criteria
- Selecting a folder does not recursively read source; capture starts only after an explicit action.
- A nested folder resolves to its containing Git root, which is shown for confirmation.
- Cancel, invalid directory, missing Git metadata, and permission denial have distinct outcomes.
- Project identity is stable across relaunch and does not depend only on display name.
- Removing a recent project removes only Code Atlas metadata, never repository files.

## Excludes
Remote Git URLs, GitHub login, repository writes, and snapshot contents.

## Done evidence
Demonstrate add, cancel, non-repo recovery, permission recovery, relaunch, and remove-recent on temporary repositories while verifying their files remain unchanged.
