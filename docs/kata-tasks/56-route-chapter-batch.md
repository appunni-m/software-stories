---
kind: task
key: route-chapter-batch
parent: agent-first-generation
title: Qualify automatic chapter building for an accepted learning route
priority: 1
labels: agents, chapters, recovery, acceptance
---

## User outcome
Choose Build all chapters on an accepted route. The saved connection generates its remaining chapters in order without repeated backend, model or reference selection. Each chapter remains independently editable and unaccepted until reviewed.

## Implemented contract
- Prepare exact native step scopes from the immutable accepted plan revision and record digest. Show all remaining source, prompts and settings in one local batch review before dispatch.
- Rebuild each step's request immediately before dispatch. Compare its fingerprint, request-body digest, project, snapshot and model with the reviewed request; native account/runtime/source admission remains mandatory. Never silently widen scope or change backend.
- Run sequentially through Codex with the saved model/reasoning. Save each validated chapter as an independent unaccepted revision with the original plan-step lineage. Do not autoaccept chapters or mark them read.
- Skip only saved draft/accepted chapters from the exact project, snapshot, plan revision, record digest and step; a same-file chapter or rejected chapter does not qualify.
- Stop on failure or cancellation; retain completed drafts, reconcile lost save replies by exact revision ID and never automatically repeat inference. Reopening determines remaining steps from saved history.
- Closing cancels the active run and stops admission of further steps. A local save already underway can finish. Pending previews and native recovery remain visible if a result/save cannot be confirmed.

## Acceptance still required
Record a packaged multi-step route run, saved draft origins, per-step review/acceptance and manual reading. Exercise close during preparation, active inference and save; quota/provider failure at step two; changed account/runtime/source; save success with a lost reply; restart before and after a save; rejected chapter regeneration; and two same-file steps with different questions. Verify no repeated connection picker or reference checklist. Confirm no automatic queue resume after restart.

Automated orchestration regressions cover sequencing, exact lineage skipping, changed requests, provider failure, cancellation, late results and lost save replies. They do not replace native runtime and semantic review. Claude support, durable background jobs and whole-repository multi-window orchestration are later work.
