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
The project, snapshot-capture and local-revision-comparison paths use the shared `bounded_process` runner. On Unix it caps stdin/stdout/stderr, supervises all three nonblocking pipes on one loop, applies the same deadline through child exit and both output EOFs, polls cancellation and owns child cleanup. It has no pipe worker threads to join or detach. Non-Unix hosts fail before launch rather than use unbounded blocking IO. The earlier direct `ignored_paths` writer/readers are removed. Local revision comparison also has a user-facing Stop action, a request-scoped native cancellation token, and suppression of late results. Packaged cancellation and timeout behavior have now been observed; see the closure evidence below.

## Contract
Use a shared bounded child-process runner for project metadata and capture Git phases. Declare per-operation deadlines, stdout/stderr/input caps and a total operation deadline. Check cancellation while waiting/writing/reading. Every success/error/timeout/cancel path owns child termination/reaping and releases all parent pipe handles without leaving workers; errors preserve prior snapshots. Keep existing read-only Git arguments and repository isolation.

## Acceptance criteria
- Opening, scanning, status checks and ignore classification have documented bounds.
- Cancellation at every Git phase promptly releases the UI, reaps the owned child and releases all pipe resources.
- Output overflow, blocked stdin, hung Git and nonzero exits return actionable bounded errors with no source/secret logging.
- Previous snapshot/catalog/revisions survive; repository refs/index/worktree stay unchanged.
- No new listener or persistent dev server is needed.

## Excludes
Remote fetches, checkout, executing repository hooks/scripts, or changing user Git configuration.

## Done evidence
Current production call-site audit, bounded-process and caller-boundary regression suites, and packaged disposable-repository Stop/timeout checks with fixture restoration and no-orphan verification.

## Implementation update — 2026-10-15

- Project inspection: five-second Git command deadline, twenty-second inspection deadline, and 16 KiB stdout/stderr limits.
- Snapshot capture: fifteen-second Git command deadline, five-minute capture deadline, 32 MiB stdout, 64 KiB stderr, and 2 MiB stdin caps. Cancellation is checked in filesystem scan/read loops and during Git execution.
- Local Git comparison: all comparison Git commands now use the shared runner under its existing thirty-second deadline, with caller-specific stdout limits, a 16 KiB stderr cap, and a 1 MiB stdin cap.
- `bounded_process::tests` covers output overflow, input overflow, timeout and running-child cancellation. `snapshot::tests`, `project::tests` and `git_evidence::tests` pass, including cancellation/history preservation and repository non-mutation cases.

Remaining acceptance: rerun the existing blocked-stdin fixture on the current implementation and add explicit timeout/output-limit error assertions at the app-facing boundary; inspect cancellation/status propagation through the native command lifecycle; evaluate the remaining Git subprocess call sites; and record packaged-app cancellation/no-orphan evidence. Do not close this task until that evidence exists.

## Static follow-up — 2026-10-16

Review found that `bounded_process::run` used `?` while joining the stdout reader. A pipe read error or reader panic could return before the stderr reader and stdin writer were joined, detaching those workers. The runner now collects all worker results first, joins every worker, and only then returns the selected error. This is a source-level fix only; no test, build, or runtime acceptance was run. The remaining acceptance above still applies.


## Static follow-up — pipe lifetime after child exit

The threaded runner still left its bounded supervision loop as soon as `try_wait` observed child exit, then performed blocking pipe-worker joins. A descendant retaining stdout/stderr or stdin could therefore outlive every declared deadline; cancellation also ceased being checked during those joins.

The Unix implementation now owns stdin/stdout/stderr directly, sets each borrowed descriptor nonblocking while preserving its existing flags, and performs at most one 16 KiB operation on each pipe per supervision iteration. It handles partial writes and WouldBlock/Interrupted, closes completed stdin immediately and requires child exit plus both output EOFs before success. Cancellation and the original command/operation deadlines remain checked throughout draining. Output overflow returns an error without storing bytes beyond the cap. Setup/IO/control errors unwind through an owned-child cleanup guard; an observed ECHILD prevents signaling an identity another waiter may have released. No naked PID or process-group identity survives reaping.

The two fcntl calls were inspected against locked libc 0.2.190 and local macOS SDK declarations for F_GETFL/F_SETFL/O_NONBLOCK and `fcntl(int, int, ...)`. Rust formatting and source review were the only new evidence at that checkpoint. No tests, builds or runtime probes ran then. Existing test bodies were retained; only their Arc import moved from production scope. This runner now explicitly rejects non-Unix hosts before launch. Startup/OS cleanup blocking, cleanup failures, inherited/escaped descendants and packaged no-orphan behavior remained unqualified. The sentence about a missing comparison cancel token was true only at the time of that checkpoint; cancellation has since been wired and tested in source.

Pending regression evidence: inherited output pipes after parent exit, inherited stdin with an incomplete write, blocked stdin, exact/overflow caps on both outputs, concurrent saturated pipes, partial writes, EOF/nonzero exit, Interrupted/WouldBlock, setup/IO/status errors, cancellation during post-exit draining, original total-deadline preservation, no detached workers and unchanged repository/snapshot/history state. Do not close this task from formatting or historical passing tests.

## Call-site audit and focused regressions — 2026-10-21

The current production Git call sites found by source search are routed through the shared runner: project inspection (`project.rs`), snapshot capture (`snapshot.rs`), commit history (`workspace/git_history.rs`), and local revision comparison (`git_evidence.rs`). The remaining direct `Command::new("git")` calls found by the audit are test fixture helpers. This is a source audit, not a packaged runtime observation.

`cargo test --lib bounded_process::` passes **27 tests, 0 failed, 1 ignored** (651 unrelated tests filtered out). Coverage includes exact independent stdout/stderr caps, overflow without retaining extra bytes, simultaneous saturated stdin/stdout/stderr, partial and transient reads/writes, blocked stdin, inherited output-pipe holders after parent exit, cancellation of the process group, nonzero parent exit, timeout, cleanup failures, and direct-child reaping. The test binary required a clean dependency compile; it emitted five existing dead-code warnings.

The 16 October list above described what was still pending at that checkpoint; the inherited-pipe, blocked-stdin, output-cap, concurrent-stream, cancellation, and EOF/nonzero fixtures have since been added and pass. Local revision comparison now exposes **Stop comparison** and sends a request-scoped cancellation to the native runner; controller and native cancellation tests exist. Remaining acceptance is the packaged app-facing cancellation/status and snapshot/result-preservation matrix, plus timeout/cancellation with no orphan process or pipe. Keep this issue open until those boundaries are observed.

## Optional repository metadata exit handling — 23 October 2026

Project inspection and snapshot capture now accept exit code 1 only for the optional branch/HEAD probes, where it represents a detached HEAD or an unborn repository. Any other nonzero exit becomes a bounded Git error; it can no longer be mistaken for missing metadata and reported as an opened project or clean snapshot state. `rev-parse --verify --quiet HEAD` suppresses expected unborn-HEAD diagnostics. Permission failures receive a stable, non-sensitive message.

**Source verification:** `cargo test --manifest-path src-tauri/Cargo.toml --lib project::tests` passes **14/14**; `cargo test --manifest-path src-tauri/Cargo.toml --lib snapshot::tests` passes **21/21**. The new caller-boundary regressions corrupt a disposable loose branch ref and verify that project opening returns an error without changing Recents, and that snapshot capture fails without changing saved history or rewriting the repository metadata. Separate regressions confirm detached and unborn states remain valid. `docs/SECURITY_VALIDATION.md` now includes the actual history-command bounds and current comparison Stop behavior.

The combined locked native library suite then passes **720 tests, 0 failed, 18 explicitly ignored**.

**Still open:** packaged cancellation/status and preservation matrix, including no-orphan behavior across all Git phases. These native source tests do not close `kghr`.

## Cancellation-path verification — 2026-10-21

Reran the focused local-comparison cancellation boundaries on the current source:

- `node --experimental-strip-types --test tests/gitComparisonController.test.mjs`: **9 passed**. Covers Stop, superseded/late results, repository scoping, external abort, and project navigation cleanup.
- `cargo test --lib git_evidence::cancellation_tests::`: **3 passed**. Covers cancellation before evidence recovery, propagation through ref validation/blob/syntax work, and interruption of a real Git child blocked on config IO.

These runs confirm frontend/native source behavior, not packaged comparison UI behavior. Packaged Stop/status behavior and preservation/no-orphan checks for comparison remain open.

## App-facing inspection timeout boundary — 2026-10-21

Added `project::tests::running_git_timeout_maps_to_a_bounded_app_result_and_preserves_saved_state`. It creates a disposable Git repository, replaces its config with an include of a named pipe with no writer, and runs a real Git inspection through the project-facing bounded runner with a 250 ms deadline. The result is the stable `GitError` timeout message, contains no fixture marker, and returns within three seconds. The test also verifies the saved recent-project record and repository index, HEAD, and fixture config bytes are unchanged. The existing output-limit boundary test asserts its stable app-facing message, hides fixture source text, and preserves recent-project metadata, index, refs, config, and worktree content.

Verification on the current source: the focused timeout test passed (1 passed, 0 failed); `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib project::tests:: -- --nocapture` passed (12 passed, 0 failed); targeted `rustfmt --edition 2024 --check src-tauri/src/project.rs` and `git diff --check` passed. Cargo emitted five existing dead-code warnings. At this checkpoint native source-boundary evidence was complete, while packaged cancellation/status and the no-orphan matrix remained open; the later packaged acceptance is recorded below.

## Packaged acceptance and closure — 2026-10-23

The packaged Code Atlas app was exercised with disposable repositories whose
`.git/config` included a held FIFO. Stop during blocked project metadata
inspection returned to the existing Fontdone workspace with its snapshot and
source retained; a separate wait beyond the five-second Git command deadline
returned the bounded “Could not inspect Git metadata” result. Both probes left
no fixture-related Git process and preserved HEAD, index and worktree state.
The fixture configuration was restored and the fixture removed. Earlier
packaged observations covered Stop/no-orphan behavior during snapshot capture
and Git-history loading.

The current production audit found application Git launches only in project
inspection, snapshot capture, local comparison and commit history, all through
the shared bounded runner. Direct `Command::new("git")` calls outside that
runner are test fixtures. Native regressions cover bounded pipes, blocked
stdin, timeout and cancellation, caller error mapping, snapshot history
preservation, and comparison/history cancellation. The Kata close event
`71459` records the reviewed paths and eight test evidence items. This closes
the bounded-Git lifecycle task; it does not establish unrelated product or
release acceptance.
