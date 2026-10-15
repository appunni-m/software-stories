---
kind: task
key: agent-first-generation
parent: t29
title: Make Codex and Claude Agent complete generation choices without mandatory API keys
priority: 0
labels: agents, product-quality, release-gate
blocked_by: codex-account-backend, claude-agent-backend
---

## User outcome

A user who already uses Codex or Claude Code can explain a repository from inside Code Atlas through a supported agent integration. A direct API key is optional. Agent support is a release requirement and must be delivered before visual polish or narrative quality is described as a complete product experience.

## Concrete current gaps — 2026-10-15

- A remembered direct API preference previously bypassed the backend chooser, hiding agent alternatives. The source now opens the chooser for generation and lets Codex return to it using Change backend.
- Codex has source wiring for file chapters, selected-source overviews and one-section stale repairs, but no accepted pinned-runtime/native workflow evidence. Code Lens and PR operations remain unwired. Section repair freezes declared source, binds base/section revision identity in review, and shares the existing independent preservation/citation validator; native execution and lifecycle evidence are still required.
- Claude Agent has protocol/review helpers and an unqualified process supervisor in source, but no automatic native generation route or connection UI. A manual Claude Code task handoff exists: review bounded immutable source, copy the task, operate the agent independently, import its complete JSON, validate locally, then explicitly save an unaccepted draft. This is interim manual transport, not automatic Claude integration.
- Antigravity and other agents can use that same manual text exchange. Their account/runtime integrations remain unimplemented or unqualified. A handoff does not prove which agent/model ran or enforce its permissions, cost, progress or cancellation.
- UI entries, API provider choices, manual handoffs and this source implementation do not satisfy integrated multi-agent acceptance.

Codex section repair now has scope-aware interruption recovery in source: the journal retains provider, purpose, selected paths and exact section revision metadata. It reopens the original Codex operation with a fresh review rather than widening to a file chapter. Legacy/range/PR records missing full evidence identity require reselection. Runtime thread resume, full PR/range recovery and native lifecycle acceptance remain pending.

## Required product flow

The [required operation matrix and user flow](../AGENT_OPERATION_PARITY.md) define exact coverage. File chapters, selected-source overviews, one-section repair, source-range explanation, PR Guides and single-hunk explanations must each work through both Codex and Claude Agent. Agent-backed repository planning is required alongside the future AI narrative planner. An API implementation cannot satisfy an agent cell; the static reading planner cannot satisfy AI narrative planning. Exact current gaps and source reuse points are recorded there.

1. From Explain file, Create repository tour, journey step, Regenerate section, Explain selection, or Explain PR, choose Coding agent or Direct API. Never require an API credential before the agent choice.
2. Choose Codex or Claude Agent. Show installation, connection and supported-operation status with one concrete recovery action. Evaluate Antigravity separately; clearly explain an unavailable operation.
3. Configure supported model, reasoning and action-approval settings per agent. Remember non-secret defaults, allow deliberate run overrides and display actual effective settings. Unsupported settings are unavailable. A manual handoff delegates configuration to the agent and cannot claim these controls.
4. Review the exact immutable context and settings, then explicitly start. Context consent is separate from action approvals. Use maintained official app-server/SDK/ACP surfaces appropriate to each vendor; do not reimplement an agent loop or credential system.
5. Observe progress, handle any supported in-scope approval, cancel, recover from failure and review the resulting explanation. A rejected, crashed, expired or cancelled operation preserves accepted history and never falls back to a direct API.
6. Validate returned claims and visuals against local evidence. Save an unaccepted draft, then explicitly accept it. Keep the selected agent in the operation/recovery metadata; label any unverified provenance accurately.

## Acceptance checklist

- [ ] Codex and Claude Agent each complete a real native file chapter and selected-source overview without requiring a direct API key in Code Atlas, through a documented permitted account route.
- [ ] Every required Codex/Claude pair in the linked operation matrix has automatic dispatch and dated acceptance evidence, including file, overview, section, range and both PR operations. Repository planning receives both agents when that feature is implemented. No direct API credential is a prerequisite; unsupported pairs cannot silently route to an API.
- [ ] Per-agent model/reasoning/approval defaults and overrides reach the runtime and appear in the review. Unsupported controls cannot be selected or silently ignored.
- [ ] Connection, quota, expired login, unavailable model, runtime crash, invalid evidence, cancellation and interrupted-save recovery remain on the chosen backend and preserve history.
- [ ] Exact supported versions, distribution/account eligibility, immutable source scope and runtime permission boundaries have recorded acceptance evidence.
- [ ] The native interface keeps agent choices visible without an API setup gate; backend switching invalidates reviews/consent and handles unsaved previews explicitly.
- [ ] Manual handoff separately handles malformed/oversized/cross-task output, changed/expired source, repeated import, failed save, clipboard refusal and restart. It stays labeled manual with unverified runtime settings/provenance.
- [ ] A real user completes the integrated Codex and Claude flows without developer intervention. Manual copy/import, a disabled option or another vendor's API cannot close this requirement.

## Owning tasks and order

Codex: `z3an`; automatic Claude Agent: `8gqw`; Antigravity: `m00g`; parent: `tpta`; backend release milestone: `mfz2`. Work on Codex and Claude integration before further visual/narrative polish. Account eligibility must be resolved honestly; if either required integration remains blocked, keep this task and the backend milestone open.

## Done evidence

Record the exact commit, native package/runtime versions, permitted account routes, operation/configuration matrix, real generation/cancel/recovery results and user workflow review. No tests, builds, logins or runtime requests were executed during this source implementation. All acceptance remains pending.
