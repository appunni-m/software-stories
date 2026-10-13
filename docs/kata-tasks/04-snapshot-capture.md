---
kind: task
key: t04
parent: m1
title: Capture immutable, bounded repository snapshots
priority: 0
labels: phase-1, repository, storage
blocked_by: t03
---

## User outcome
The user can inspect a stable copy of repository bytes and knows what the capture included, skipped, or could not read.

## Contract
Capture Git HEAD object ID when available, branch, index/worktree state, eligible file bytes, per-file hashes, timestamp, and parent snapshot ID. A snapshot is immutable. The UI shows progress, cancel, completion summary, and partial/error state. Limits and exclusions are versioned and visible. Initial policy never follows symlinks, reads `.git` internals, enters nested repositories, runs repository code, or captures ignored/vendor/generated contents by default.

## Acceptance criteria
- Capture reads are bounded by file size, total bytes, path count, and per-directory entries; reaching a bound records each skipped reason.
- Cancel stops additional reads and leaves the prior snapshot and guide untouched.
- A file changing during capture is detected as inconsistent and reported; mixed bytes are not labeled as one valid snapshot.
- A second capture creates a new snapshot; it does not overwrite blobs or identities from the first.
- Capture performs no writes inside the selected repository.

## Excludes
Language indexing, model requests, Git checkout/commit, and source editing.

## Done evidence
Fixture and temporary-repository run records exact captured hashes, repeat capture ancestry, each configured limit, cancellation, unreadable paths, changed-during-read handling, and unchanged repository status.
