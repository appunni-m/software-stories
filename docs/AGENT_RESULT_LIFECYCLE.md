# Agent result ownership and recovery

This is the source contract as of October 16, 2026. It has not been compiled or exercised in this pass. It does not qualify account eligibility, the runtime permission boundary, package behavior or any automatic operation. Claude generation remains unavailable behind its empty release registry.

## Why ownership extends past generation

A completed model turn is not yet a usable explanation. Native code must rebuild the original operation, independently validate its evidence and publish the correct result. Cancellation or expiry before publication must still reject that result. The managed run must not end when raw text arrives and then create a draft outside its owner.

The registered Codex senders now use `run_with_result_and_publish` for planning, file chapters, selected-source overviews, section repair, exact source ranges, PR Guides and individual hunks. The existing Claude pipeline retains its revocable operation lease across those stages. Both reuse vendor agent runtimes and local validators; this adds no agent loop, API credential requirement, model catalog or fallback.

## Stages

| Stage | Native requirement | Local result state |
| --- | --- | --- |
| Prepare | Exact open project, immutable evidence, supported agent settings, stored review and fresh source consent | No result; no restored consent |
| Admit | Consume the exact reviewed request and record its original recovery scope before worker dispatch | Interrupted operation is discoverable |
| Run | Own the queued/running worker and original admission clock; support cancellation and owner drop | Raw response stays inside native processing |
| Rebuild and validate | Check the same owner before/after rebuilding the entire original contract; validate exact source and operation-specific output | In-memory candidate only |
| Publish | Recheck cancellation/deadline/owner and serialize final publication with cancellation or connection revocation | Unaccepted preview/plan/guide, or admitted transient explanation |
| Finish | Clear ordinary operation notices only after a successful result is admitted; planning has explicit separate resolution | Errors and unconfirmed results retain recovery scope |

New request reviews keep their ordinary expiry. A result can finish after that expiry only inside the same already admitted run's ten-minute budget. Rebuilding retains the original review ID and expiry; it cannot renew consent, establish a new connection or start another run. The clock includes queue/setup, generation and validation after native admission. A publication already admitted under the owner can finish; a later cancellation cannot retract its saved result.

### Codex

`CodexCommandOwner` cancels the exact run if its async command disappears. The original `started_at` reaches the manager instead of being discarded or restarted on the blocking worker. `CodexResultAccess` exists only inside the owned callback and checks run/review/fingerprint/executable, cancellation and the deadline.

`rebuild_active_codex_review` rebuilds chapter/overview/section/range/Git contracts from native immutable evidence, retains only the original ID/expiry and compares every other review field. Ordinary preflight still rejects expired reviews. Exact hunk validation uses the active rebuilder rather than the ordinary admission-expiry check. Planning retains its distinct active rebuilder and contract. No raw response leaves the owned worker for later renderer/native validation.

Pure validation retains process/run ownership while leaving cancellation available. Final publication uses the same active-run lock as cancel. Worker panic shuts down the manager and retains original recovery scope. Failure, cancellation, timeout, changed evidence, invalid output or failed publication no longer count as a reason to clear the journal. These are unexecuted source invariants.

### Claude

The existing native lease checks original connection/runtime/review and cancellation. Refresh, forget, disconnect and owner drop remain revocations. Chapter and PR Guide validation now prepare their candidate outside the final publication lock. Publication alone holds that lock, so local validation does not prevent revocation from taking effect before a save.

The pipeline rechecks wanted result ownership before successful notice cleanup. Validation/provider/storage failure, uncertain cleanup and rejected late results retain their exact notice. A result published before a later revocation may already have a pending preview or guide on disk; its error directs the user to existing results before another generation. All inference commands still require a genuinely qualified runtime; this source correction does not enable one.

## Separate validation from publication

The shared chapter and PR Guide helpers now have private validated candidate types. Validating a candidate reads evidence and canonicalizes a result but creates no pending preview, saved guide or recovery copy. Existing API/manual callers retain their ordinary wrapper behavior. Native agent callbacks separately publish the candidate under their existing owner.

Agent publication uses nonblocking local store entry points. For chapters, `PendingChapterDrafts::try_publish` acquires the memory-store guard, creates a generation identity and calls the nonblocking recovery-copy publisher. Only after that succeeds does it register the new preview and expire/evict old memory entries. A failed or busy publication cannot evict an earlier preview. A file may exist after uncertain filesystem publication even if no memory preview was registered; recovery discovery remains required.

PR Guides use their existing immutable draft store through `try_save_draft`. Busy history readers cause an explicit failure instead of waiting for a store lock under cancellation ownership. Planning already uses its separate nonblocking publisher. No publisher may call its agent manager or connection owner. No failed save may secretly retry or regenerate. These changes do not prove filesystem durability, cross-process transactions, hostile-filesystem isolation or bounded OS I/O latency.

Chapter drafts remain unaccepted pending previews. Guides remain separate unaccepted PR revisions. Range/hunk explanations remain transient. Plans retain their separate draft/accepted history and explicit notice-resolution flow. None of these stages accepts chapters, marks files explained/read or completes repository coverage.

## Discovery and remaining delivery limits

Both app agent panels now refresh recovery after non-planning operations, including range/hunk errors and abandoned replies. Planning retains its own saved-plan/recovery event. The shell versions recovery lookups so a superseded response or initial startup read cannot replace a newer result; local notice removal starts a fresh authoritative lookup. This event only reads local state and cannot launch an agent or restore consent.

Successful native result admission is not proof of IPC delivery or renderer adoption. Transient result delivery, post-publication cancellation, lost replies, uncertain storage, journal cleanup and all packaged focus/accessibility paths still need acceptance. The [deferred cases](kata-tasks/30-static-fix-acceptance.md) cover the complete operation matrix. See [agent coverage](AGENT_OPERATION_PARITY.md), [planning](REPOSITORY_PLANNING.md) and [Claude admission](CLAUDE_RUNTIME_QUALIFICATION.md). No test, build, app launch, CLI/SDK probe, login or model request ran. Source review and formatting close no release gate.
