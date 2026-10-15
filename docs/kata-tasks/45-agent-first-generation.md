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
**Priority decision:** automatic Codex and Claude Agent generation is a prerequisite for this product. Complete their app flows before further API expansion or visual polish. A settings screen, connection check, offline preview, manual handoff or disabled option cannot count as delivery. The user must be able to choose either installed agent and finish the explanation in the app without configuring a direct API key.


A user who already uses Codex or Claude Code can explain a repository from inside Code Atlas through a supported agent integration. A direct API key is optional. Agent support is a release requirement and must be delivered before visual polish or narrative quality is described as a complete product experience.

## Concrete current gaps — 2026-10-15

- A remembered direct API preference previously bypassed the backend chooser, hiding agent alternatives. The source now opens the chooser for generation and lets Codex return to it using Change backend.
- Codex has source wiring for file chapters, selected-source overviews, one-section stale repairs and exact Code Lens selections, but no accepted pinned-runtime/native workflow evidence. PR Guide and hunk operations are now wired in source through a Codex-first backend chooser, shared immutable Git context/revalidation and the existing validators. Guides remain separate unaccepted drafts; hunk explanations stay transient. Newer Codex range/Git notices now have exact scope recovery in source; legacy records require explicit reselection. Native runtime/recovery acceptance remains pending. Code Lens uses the shared immutable range builder and exact citation validator, returns only a transient explanation, and requires deliberate source reselection after interruption. Section repair freezes declared source, binds base/section revision identity in review, and shares the existing independent preservation/citation validator; native execution and lifecycle evidence are still required.
- Claude Agent has protocol/review helpers, an unqualified process supervisor and an explicit native installation/account-metadata check with setup UI in source. That check sends no prompt and always leaves generation unavailable. An advanced native SDK catalog check and metadata UI are now wired in source; qualified discovery, account/runtime qualification and automatic native generation/UI remain missing. A manual Claude Code task handoff exists: review bounded immutable source, copy the task, operate the agent independently, import its complete JSON, validate locally, then explicitly save an unaccepted draft. Neither the metadata check nor this manual transport satisfies automatic Claude integration.
- Antigravity and other agents can use that same manual text exchange. Their account/runtime integrations remain unimplemented or unqualified. A handoff does not prove which agent/model ran or enforce its permissions, cost, progress or cancellation.
- UI entries, API provider choices, manual handoffs and this source implementation do not satisfy integrated multi-agent acceptance.

Codex section repair now has scope-aware interruption recovery in source: the journal retains provider, purpose, selected paths and exact section revision metadata. It reopens the original Codex operation with a fresh review rather than widening to a file chapter. Legacy/range/PR records missing full evidence identity require reselection. Runtime thread resume and native lifecycle/recovery acceptance remain pending. Newer Codex range/Git scope restoration is described below.

Claude catalog groundwork now also uses the maintained SDK's public model/init contract, an independent native normalizer and a bounded native SDK discovery worker; the inspected candidate artifact is `0.3.300` paired with CLI `2.1.300`. It resolves aliases to full selectors and preserves per-model effort limits without a direct API dependency. The source supervisor checks SDK/executable identities and reported account metadata, stages verified scripts, bounds raw pipes and requires host/child completion. A registered app command and advanced catalog UI now call it using explicitly selected installation paths and native-derived identities. The SDK/host runtime is not packaged or qualified, and this metadata check does not enable generation. An internal native chapter pipeline now connects review consumption, worker execution and shared draft validation, with reported-account pre/post checks and locked preflight revalidation. It now has four registered native send commands and a release-owned admission factory, but the profile registry is empty and app generation controls remain missing; every Claude automatic operation remains **Missing** in the matrix.

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

### Native review ownership delta

Claude now has a native five-minute observed catalog connection and a registered offline file/overview/section review command. Requested settings/source enter from the renderer; account/runtime/capability observations come from native state only. Refresh/cancel/path change/close invalidate the connection, late catalog completion cannot republish it, and review fingerprints/expiry bind to the current record. The app now calls offline review through the Claude settings/source/request panel for file/overview/section/range/guide/hunk scopes. Native inference commands are registered behind an empty qualification registry; no app generation UI or eligible runtime was delivered. The account/runtime/isolation qualification and every automatic Claude operation remain open, along with disconnect-aware active-run/result handling. Metadata and offline reviews still cannot satisfy this release gate. No tests, builds, agent probes or model calls ran.

### Claude operation pipeline delta

The internal candidate Claude pipeline now includes exact range, PR Guide and single-hunk operations through shared immutable context, strict per-operation contracts, one worker lifecycle and independent result validators. New registered commands are offline review only; their typed helpers now have explicit app callers for local request preview. App generation UI, an eligible release profile and actual runtime/account/isolation qualification, disconnect-aware inference results and scope-preserving recovery remain absent. Source-range and Git preview targets stay fixed and are checked independently against the returned review; preview does not grant consent or start generation. This does not complete any automatic Claude cell or weaken this release gate. No tests, builds or runtime/model calls ran.


### Native lifecycle delta; automatic app delivery remains open

The internal Claude pipeline now binds every operation to a revocable native owner shared with worker cancellation. Refresh/disconnect/cancel prevent subsequent publication; ordinary five-minute review expiry cannot invalidate the same already-consented ten-minute run. An async owner drop revokes worker and blocking validator access. A preview or guide whose publication already won remains unaccepted and may need recovery/history lookup after a late disconnect. Native send commands now connect this pipeline through release-owned admission, but the registry is empty and generation UI is absent; this does not qualify its account/runtime boundary or establish renderer cancellation/late-reply disposal. Keep the priority-0 gate and all Claude automatic cells open until the actual integrated flows, exact settings and scope-preserving recovery are delivered and accepted. No tests, builds or agent/model calls ran.


### Exact range/Git recovery source delta

Newer Codex interruption notices now restore exact immutable ranges, selected Git changes and original hunks after journal-owned native local validation. Guide recovery checks saved history first. Backend choice, fresh model controls, exact review and consent remain explicit; there is no automatic request or API fallback. Original Codex notices persist through review-only abandonment and failed/cancelled results, then clear on validated result adoption. The metadata/native validator are reusable by Claude, whose automatic sender/recovery UI remains missing. This improves Codex source coverage and does not complete either agent's packaged acceptance or this multi-agent gate. Deferred evidence cases are in `xymb`.
