---
kind: task
key: bounded-git-lifecycle
parent: t24
title: Bound and cancel repository Git subprocesses
priority: 1
labels: repository, reliability
---

## User outcome
Opening or capturing a repository cannot hang indefinitely or leave Git processes running after cancellation.

## Current evidence
Static inspection: snapshot.rs git helpers and project.rs use blocking Command::output without explicit deadlines/output limits. ignored_paths reads pipes fully; an early stdin error can return without killing/reaping the child and joining readers. Existing cancellation checks are outside these blocking waits. Implementation remains pending.

## Contract
Use a shared bounded child-process runner for project metadata and capture Git phases. Declare per-operation deadlines, stdout/stderr/input caps and a total operation deadline. Check cancellation while waiting/writing/reading. Every success/error/timeout/cancel path terminates or reaps the child and joins readers; errors preserve prior snapshots. Keep existing read-only Git arguments and repository isolation.

## Acceptance criteria
- Opening, scanning, status checks and ignore classification have documented bounds.
- Cancellation at every Git phase promptly releases the UI and reaps all child/readers.
- Output overflow, blocked stdin, hung Git and nonzero exits return actionable bounded errors with no source/secret logging.
- Previous snapshot/catalog/revisions survive; repository refs/index/worktree stay unchanged.
- No new listener or persistent dev server is needed.

## Excludes
Remote fetches, checkout, executing repository hooks/scripts, or changing user Git configuration.

## Done evidence
Code path audit plus later timeout/output/cancel acceptance on a disposable fixture; no tests performed in this pass.
