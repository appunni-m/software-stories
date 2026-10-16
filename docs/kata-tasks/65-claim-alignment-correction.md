---
kind: task
key: t65
parent: t14
title: Correct identifier and explanation mismatches before publishing a workspace answer
priority: 1
labels: bug, chapters, reliability
---

## User outcome
An internally inconsistent AI draft is corrected in the background when possible. The developer sees an explanation or a clear recovery action, without generated section IDs.

## Contract
For workspace Codex learning questions only, fingerprint policy `identifier-alignment-v1` with one maximum correction into the request review. Audit complete identifiers in claim prose and cited quotes using the source validator's existing boundary rules. On mismatch, allow one follow-up in the same ephemeral thread, with bounded positional diagnostics and the original evidence, model, reasoning, schema, permissions, cancellation owner and deadline. Never add prose or remove evidence locally to make validation pass. No new files, model/provider fallback, restart retry or automatic acceptance. The candidate must still pass the full exact-source and independent storage validators. Preserve failed questions and existing revisions. Map both current and persisted legacy mismatch errors to useful workspace copy.

## Acceptance criteria
- A synthetic draft with `record_pipeline_operation_path` omitted from its prose exercises exactly one correction callback.
- Whole-identifier boundaries, qualified names, evidence limits, one-section updates and missing quote links use the same lexical rules.
- Valid, unenabled and malformed drafts do not trigger correction; malformed candidates still require the full validator and cannot be admitted by this helper.
- A bad second draft or transport/cancellation failure stops; there is no third turn or fallback.
- Exact source fabrication is rejected after a successful lexical correction; independent storage validation still passes for a valid correction.
- The policy is fingerprinted and tampering fails review rebuild.
- An opt-in GPT-6 Luna/max synthetic fault injection exercises the actual second turn and passes normal source/storage validation.
- Frontend build/tests and native tests pass; package and install the updated app, then inspect the original persisted failure without resending the user's repository.

## Scope limits
This task does not establish semantic truth, universal generation success, broader narrative/visual/language quality, accessibility or release readiness. Parent quality and workspace tasks remain open.

## Evidence
See [identifier correction acceptance](../acceptance/2026-10-16-claim-alignment.md).
