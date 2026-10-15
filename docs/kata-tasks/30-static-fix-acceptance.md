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
