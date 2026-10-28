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

### Incremental Guide boundary-limit correction — 28 October 2026 (source-only)

The repository Guide planner accepts up to 64 distinct omissions, while the progressive `GuideIntent` validator accepts at most eight `researchBoundaries`. The opening, opening-continuation, and planned-unit builders previously copied the full remaining omission list into each intent. Because the initial opening runs before omission promotion, any valid plan with more than eight omissions could make local request preparation reject its first slide before dispatch. Promotion can append at most the remaining guide capacity (18 units), so a large list could also leave more than eight boundaries for later slide intents.

Each Guide slide intent now receives up to seven exact pending labels and, when more remain, one explicit count stating that the other topics remain uncovered outside this slide. The saved plan, summary, and omission-promotion path retain every exact omission. Evidence is limited to source inspection of `repository_guide::{plan::canonicalize, pending_extension_topics, guide_research_boundaries, unit_intent, run_initial_opening}`, `opening_continuation::intent`, and `workspace_generation::progressive::guide_intent::GuideIntent::validate`. No tests, builds, provider requests, or packaged acceptance were run for this correction; all route-batch qualification in the acceptance section remains required.

## Saved-route compatibility gate

Qualify saved route history across prompt/schema upgrades. The previous saved-plan reader rebuilt only the current planning contract and required its digest to match the recorded contract. A changed instruction or output schema could therefore make an older route unavailable even when its immutable implementation was unchanged. Preserve historical records and chapter lineage, distinguish a contract-version incompatibility from changed source, and support a versioned replay or explicit migration that revalidates all claims. Never renew generation consent or weaken source admission to recover an old route. Include a regression that saves under one prompt version, upgrades the prompt, and reopens the original route and its independent chapters.

### Local compatibility correction — 17 October 2026

The actual saved synthetic route exposed a separate digest regression: shared enum field renames changed typed serialization, preventing its accepted history from loading although its draft and current request were valid. Frozen native files now reproduce that failure. Known schema-1 enum encodings retain their original serialization/digest; new records use schema 2. Exact route/revision/step links and the independently validated saved chapter remain unchanged. Future, mixed, unknown or duplicate fields are rejected; byte/count/publication limits remain in force.

Saved replay also matches the entire recorded request against immutable initial, code-first and current request contracts. Older-contract variants are explicit synthetic upgrade simulations, not additional account runs. Current generation keeps the current request bytes and cannot use a historical policy. Every read still independently reopens source/index, resolves claims and compares the full saved plan. Unknown contracts receive a specific error, with no inference, consent renewal or record rewriting.

Regressions cover persisted draft/accepted/step/chapter replay, full chapter source validation, new schema-2 explicit decisions, both schema-1 encodings, exact digest preservation, invalid evidence/claims and unknown formats. Read success/failure and a new explicit acceptance preserve older bytes. [Qualification and remaining gates](../acceptance/2026-10-17-saved-route-replay.md). Packaged upgrade, route batching/cancellation/recovery, native reading, semantic and voice acceptance remain required; this task remains open.
