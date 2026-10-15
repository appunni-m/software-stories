---
kind: task
key: static-fix-acceptance
parent: t24
title: Validate the October 15 static bug fixes when testing is authorized
priority: 1
labels: reliability, validation
---

## User outcome
The fixes from the October 15 code review have recorded behavioral evidence before release.

## Current evidence
Source inspection and patches only. The user explicitly requested no tests, builds, or runtime probes during this pass. Keep this task open; prior passing runs do not cover these changes.

## Contract
Exercise the changed paths below when validation is authorized. Record commit, environment, exact trigger, expected/actual outcome, and evidence. Fix any regression before closing; do not run this acceptance merely because the task was created.

## Acceptance criteria
- Snapshot ignore handling: ignored file, negated pattern, re-included directory, escaped literal exclamation mark, and force-tracked ignored file retain correct captured/skipped status.
- While a later file is read, change an earlier already-dirty captured file. Capture reports inconsistent, publishes no snapshot, and preserves history.
- Oversized index (32 MiB), recovery journal (1 MiB), source object (recorded size, at most 2 MiB), and WAV (12 MiB) stop reading at cap plus sentinel. Exact-size, empty, truncated and corrupted inputs fail or load according to their contracts.
- All direct-provider and narration production clients reject 301/302/303/307/308 without a second request, forwarded body/credentials, provider fallback, or history mutation. Ensure any transport fixture uses production redirect settings.
- Resolve Walkthrough history refreshes in reverse order; only newest data/error/loading survives. Switch projects during Git comparison; old success/failure cannot affect the new project.
- Switch providers with a credential read pending; old status cannot enable the new provider. Provider selection/close remain unavailable during Keychain save/remove and controls recover on failure.
- During Chapter Studio save/validate/accept/reject/restore, editing and revision selection cannot discard new input or show results for a different revision. Navigate away: completion must not alter the new route. Saved revisions remain discoverable.
- A file deleted, excluded, or changed to binary in a newer snapshot keeps its chapter history visible with original citations; unavailable current source is not offered for generation.
- While deciding PR draft A, another draft/provider/model/generation cannot replace its context. Abandon an in-progress PR review build: late success is discarded and errors do not update the abandoned UI.
- During narration review/cache/synthesis, navigate away or rapidly use playback controls: no late autoplay/state, abandoned reviews, or adopted cancelled output. Cancel remains reachable; script controls resume after completion/failure. Previous audio and transcript survive.
- Cached playback position survives normal mount and React effect restart. Malformed/truncated/oversized WAV is rejected safely.

## Excludes
Claiming every repository bug is eliminated, live paid calls without separate authorization, or closing other acceptance gates from this checklist alone.

## Done evidence
Attach results for each case above on the patched commit; all currently pending.

## Implementation revision

Static fixes are committed as `0494bd4` (Fix snapshot integrity and async explanation lifecycle bugs). No tests or builds were run locally. The task remains open for the acceptance cases above.

## Additional October 15 integration acceptance — pending

Later source deltas also require the following checks when validation is authorized:

- Manual handoff build/export/import reads successful native responses through the shared `data` envelope. A mismatched envelope must not orphan a review/preview. This static correction is `0d1160b`; the path has not been exercised.
- Codex one-section repair uses the same scoped prompt/schema and declared evidence as the API route after shared-context extraction. Verify old/current source limits and secret-like screening, exact base/section revision binding, changed review rejection, fixed source selection, current-only citations and byte/identity preservation of eligible untouched sections/assets.
- Exercise cancellation, quota, invalid output, save/recovery and backend switching on the Codex section route. Confirm PR remains explicitly unavailable for Codex and section repair remains unavailable for manual handoff; no direct API fallback occurs.
- Interrupt each API/Codex operation. New journal records retain bounded provider/purpose/path/section identity without source or credentials. Restore a Codex file, overview and exact section repair into Codex with a new review; verify section/base changes reject recovery. Legacy scope-less and range/PR records must not reopen as whole-file chapters; range/PR evidence requires deliberate reselection. Navigate away or select newer recovery work during project/history reads: abandoned completion and errors cannot change the new route/dialog. No accepted history may change during recovery lookup.

The subsequent source changes add Codex account-backed chapters, Pocket narration and lexical index v2. They were not built or exercised. In addition to the owning task contracts (`z3an`, `q6g3`, `dbmd`, `129x`), record these integration cases when authorized:

- Cancel before a native command registers: the unique operation stays cancelled; it cannot start later after the originating view closes. Cover Pocket setup, narration synthesis and Codex generation; cancellation records remain bounded.
- Switch backend or close during model/status/login/review completion. No late result populates a different view, starts unexpected generation or leaves an unsaved accepted draft. A browser preview never invokes native agent commands.
- Codex runtime/executable/model/effort/source changes invalidate the reviewed request. A missing or weaker runtime fails clearly without API fallback. Confirm exact supported runtime version and account eligibility separately.
- A completed Codex preview remains accessible if interruption-journal cleanup fails. Cancellation, timeout, quota, unsupported model and runtime diagnostics remain distinct; no raw RPC or model output is shown as an error.
- A lost save reply discovers the immutable revision already written rather than generating again. Normal close discards an unsaved preview; app termination/restart exposes persisted recovery previews.
- New narration preferences choose Pocket; legacy complete Google preferences still identify their original cloud cache. Switching engines preserves each engine's delivery settings. Invalid preferences must not silently choose a cloud engine.
- Pocket runtime fingerprint or final spoken text changes invalidate audio. Cached playback/export does not install a runtime or call a provider. Multi-segment failure accepts no partial section audio.
- Check normal exit, cancellation during preparation, malformed protocol frames, stalled writers/readers and killed workers for owned-child cleanup. Verify packaged permissions and runtime boundaries under the separate acceptance contracts.

No issue can be closed from the source implementation or formatting alone.

## Claude protocol preparation and shared agent review — pending

- Recheck Codex file/overview/section contracts after shared review extraction: identical effective controls, schema, evidence and deterministic fingerprint inputs; invalid scope/source still fails before dispatch.
- Claude offline review/revalidation must reject changed executable/account digests, candidate version, model, effort, turn limit, source, base/section revision, schema and expiry. Capability/catalog data must originate from bounded native runtime discovery, never WebView input or a fabricated bundled model list.
- Invocation preparation must reject unsupported model-specific effort/policy/limits and keep source off argv. Command preparation must reject a mismatched runtime/account/stage and clear inherited API, endpoint, cloud-provider, proxy and Node options. Verify no default, managed-policy or internal fallback can silently change the account/billing/model route.
- Feed malformed, oversized, out-of-order, duplicate-init, wrong-session, changed-model, nested-agent, permission, hook and external-tool events. Rejected streams cannot be recovered by appending a valid result. Check total bytes, frames, events and structured-result limits separately.
- Permit only the runtime's internal `StructuredOutput` formatter; its tool results must reference observed formatter calls. An external MCP lookalike, injected user message or free-form final text cannot become a draft. A valid final field still requires successful process exit/EOF and independent schema/citation/section-preservation validation.
- Keep the candidate helpers unavailable for automatic generation until the separately owned account/runtime/isolation, supervisor/cancel/reap, dispatch/UI and recovery requirements in `8gqw` are implemented and accepted. No source implementation or offline review constitutes a qualified Claude integration.

All cases are deferred. No tests, build or runtime request was run for this source change.

## Claude process supervisor — pending

These cases cover `claude_worker.rs` source changes. They do not authorize agent execution; use an explicitly authorized native acceptance run later.

- Reject a second active run, invalid operation IDs and runs cancelled before registration. Bound/expire cancellation records; shutdown prevents later starts. A progress callback that cancels must prevent the next source write, and cancellation during final cleanup must prevent result acceptance.
- Reinspect an absolute canonical executable and its reviewed SHA-256. Reject symlinks, missing/nonexecutables, changed contents and files above 256 MiB. A file growing during hashing is read only to cap plus one byte. Qualify immutable execution separately: hashing before spawn does not eliminate path replacement races.
- Restrict private stage/tmp/instruction permissions and reject symlinked data paths. Source remains on stdin; raw output/diagnostics do not reach progress, logs or user errors. Verify the actual cleared environment, account route and effective CLI flags in the qualified runtime; a private working directory is not an OS sandbox.
- Stall stdin while stdout/stderr fill, split JSON frames across reads, omit the final newline, and close a pipe early. Nonblocking I/O must continue cancellation/deadline checks without stranded I/O threads. Reject a frame above 2 MiB, output above 32 MiB, diagnostics above 256 KiB and the independent parser event/result limits. Never accept a partial source delivery.
- A success-shaped final result is withheld until stdout/stderr EOF, successful child exit and staging cleanup. Trailing malformed events, nonzero exit, missing result and inherited pipes staying open after leader exit must reject without saving history. Independent chapter/evidence validators remain mandatory after the worker returns.
- Cancel or expire during preparation, input delivery, generation and drain. Check SIGINT, then SIGTERM/SIGKILL grace escalation; invalid protocol may kill immediately. Exercise app exit and child descendants, and record process ownership/no-orphan evidence. Hard app termination and children escaping the group require separate qualification, not an assumption from signal delivery.
- Observe exit without reaping, signal remaining owned-group members, then reap under the PID registry lock. Concurrent shutdown cannot signal a stale identity. On `ECHILD`, clear the identity, reject cleanup and disable further session runs; do not signal a possibly reused PID.
- Inject failures in pipe setup, child status inspection/reaping and private-storage cleanup, including preparation errors/unwinding. Cleanup failure takes precedence over a concurrent cancel; no response is accepted and further runs stay disabled for the session. Confirm fresh-launch recovery handles leftover staging before enabling generation.
- Qualify the locked libc/macOS `fcntl`, `waitid` and `kill` assumptions on supported packaged targets. Source review and SAFETY comments are not runtime evidence; non-Unix execution remains unavailable.

No test, build, app launch, agent login or model request ran for these changes. Automatic Claude dispatch, account/catalog discovery, UI, isolation and persisted recovery remain owned by `8gqw`.

## Codex exact source range and shared dispatch — pending

- From a captured code selection, choose Codex before any API setup. Keep the path, full-file SHA-256, line range and optional byte range fixed; only the reviewed excerpt reaches prompt/staging. A range in a file above the chapter per-file limit remains eligible if the excerpt satisfies the range limit. No supporting-file selection can widen it.
- Recheck direct API range review after extracting immutable context: the same selected bytes, prompt, schema, source bounds, secret-like checks and provider-specific request encoding must remain intact. Recheck Codex file/overview/section generation after extracting shared dispatch; chapter preview, journal cleanup notes and save/recovery behavior must be preserved.
- Change range, file identity, model, effort, executable, schema, expiry or purpose between review and send. Revalidation must reject; chapter and range commands cannot consume each other’s review. Never route a Codex operation to a direct API.
- Verify staged excerpt hashes against the exact delivered bytes independently of the original captured-file digest. No source outside the selection is staged or added to the range prompt. Existing runtime read/tool isolation still requires separate qualification.
- Validate exact unique quote, whole identifier, UTF-8 byte and line citations against the original immutable file; wrong/outside/ambiguous quotes, duplicate claims, schema violations and model mismatches reject. Citation actions open the corresponding range. Valid responses remain transient: no pending chapter draft, accepted revision or chapter-history mutation.
- Cancel before registration, during stream and after native completion; close, switch backend or change selection while local review or generation resolves. No late range result appears in a new view. Discard/Done and reopening leave no saved explanation. Listener cleanup and cancellation resources remain bounded.
- Interrupt/crash a range operation. Its current recovery record requires deliberate reselection and fresh review/consent; it must never reopen as a whole-file chapter. Journal write failure prevents dispatch; cleanup failure returns a local-storage error without accepting the transient result.

All cases remain deferred. Source inspection, Rust formatting and diff whitespace checks do not establish compiled or native behavior. No tests, builds, app launches, agent calls or logins ran for this source change.
