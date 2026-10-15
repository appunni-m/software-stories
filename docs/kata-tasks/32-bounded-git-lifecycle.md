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
The project, snapshot-capture and local-revision-comparison paths now use the shared `bounded_process` runner. It caps retained stdin/stdout/stderr, continues draining output after the storage cap, applies per-command and operation deadlines, polls snapshot cancellation, terminates/reaps the child, and joins pipe workers. The earlier direct `ignored_paths` writer/readers are removed. The local comparison path remains deadline-bounded but has no user-facing cancellation token.

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

## Implementation update — 2026-10-15

- Project inspection: five-second Git command deadline, twenty-second inspection deadline, and 16 KiB stdout/stderr limits.
- Snapshot capture: fifteen-second Git command deadline, five-minute capture deadline, 32 MiB stdout, 64 KiB stderr, and 2 MiB stdin caps. Cancellation is checked in filesystem scan/read loops and during Git execution.
- Local Git comparison: all comparison Git commands now use the shared runner under its existing thirty-second deadline, with caller-specific stdout limits, a 16 KiB stderr cap, and a 1 MiB stdin cap.
- `bounded_process::tests` covers output overflow, input overflow, timeout and running-child cancellation. `snapshot::tests`, `project::tests` and `git_evidence::tests` pass, including cancellation/history preservation and repository non-mutation cases.

Remaining acceptance: add a blocked-stdin fixture and explicit timeout/output-limit error assertions at the app-facing boundary; inspect cancellation/status propagation through the native command lifecycle; evaluate the remaining Git subprocess call sites; and record packaged-app cancellation/no-orphan evidence. Do not close this task until that evidence exists.
