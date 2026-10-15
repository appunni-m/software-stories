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
