---
kind: task
key: claude-agent-backend
parent: t29
title: Implement automatic Claude Agent generation across required operations
priority: 0
labels: agents, provider
---

## User outcome

The user selects Claude Agent, powered by Claude Code, configures supported settings and completes source-grounded explanation generation inside Code Atlas without mandatory direct API credentials or manual copy/import. This is a priority-zero release requirement. File/overview milestones alone cannot close the full operation requirement.

## Required operations and result handling

Cover every Claude row in the [required operation matrix](../AGENT_OPERATION_PARITY.md): file chapter, selected-source overview, exact one-section repair, selected source range, PR Guide and single Git hunk. Add agent-backed planning with the repository narrative planner. An overview cannot count as explaining all files. An API result, setup check, local preview, disabled option or manual handoff cannot complete an automatic operation.

Reuse shared immutable context/prompt/schema builders and independent validators. Chapters produce recoverable unaccepted previews and require explicit Save/Accept. Guides use separate unaccepted PR history. Range/hunk explanations remain transient. Validate structured output, exact model/review identity, schema, claims/citations and section preservation before publication. Invalid output, cancellation or errors preserve accepted history.

## Supported vendor surface and account decision

Use the maintained official Agent SDK or documented programmatic `claude -p` subprocess; do not implement another agent loop, automate a TUI, extract tokens or invent model-list/control RPCs. The candidate source uses SDK 0.3.300 for discovery and CLI 2.1.300 for generation. [Artifact hashes and interface provenance](../research/snapshots/2026-10-15-claude-sdk-0.3.300.json) record source inspection only, without execution or registry-signature verification.

Official wording rechecked October 15 remains unresolved: the [Help Center](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan) documents SDK/headless/third-party subscription usage, while the [SDK integration guidance](https://code.claude.com/docs/en/agent-sdk/overview) requires prior approval for third-party products offering claude.ai login/subscription limits. Keep both sources and resolve this exact product/distribution/account route before advertising approved support or adding a qualification profile. Subscription use is not technically impossible. API-key/Console billing is a separate explicit choice and cannot substitute for the requested route. Use vendor-supported account storage/authentication; no Code Atlas Claude login is implemented. Use the preferred embedded-product label Claude Agent.

## Settings and source contract

- Select only full model IDs and advertised model-dependent effort from current native catalog metadata. Runtime default effort is an explicit omitted override; unavailable saved settings require a deliberate valid selection. Approval is currently fixed to `dontAsk`; turn limit is 1–8. Remember non-secret defaults separately for Claude and allow deliberate run overrides. Do not invent universal controls or silently ignore selections.
- Review exact context, model/effort/approval/turns, account usage, prompt/schema, source hashes and fingerprint before explicit send consent. Source consent is separate from action approvals. Any source/settings/backend/connection/account/runtime change invalidates the review.
- Use bounded immutable captured files, never the live checkout. Range review freezes complete-file SHA, lines and optional paired half-open UTF-8 bounds; include only the bounded excerpt. Git review freezes requested refs, resolved comparison, selected base/head path pairs, patch hashes and original hunk index. Reject moved refs or mixed/cross-purpose contracts; do not widen missing range/Git identity to a file.
- Configure the actual tool set, empty settings sources, strict app-owned empty MCP, disabled hooks/session persistence and supported safe/restricted controls. `allowedTools` is an auto-approval list, not tool removal. Deny commands, writes, web, unrelated MCP/subagents and reads outside approved context with verified runtime/OS controls. Keep provider-control traffic distinct from arbitrary tool egress.
- Strip inherited credentials/endpoints/provider/proxy/startup environment that could select API billing or change behavior. Keep the documented native account store without reading/copying its tokens. Safe flags, staged cwd and cleared environment alone do not establish confinement, effective policy, entitlement or absence of fallback.

## Current source — incomplete automatic backend

| Layer | Source present | Required next evidence or work |
| --- | --- | --- |
| Version/account inspection | Explicit managed native command; sanitized bounded observations, no repository source or user prompt; native account digest is not a credential | Trusted installation, account/product eligibility and actual metadata behavior |
| Catalog | Public SDK initialization/model methods with zero user messages; pinned module bytes, native Node/CLI identity checks, bounded host/child streams and independent canonical model/effort admission | Packaged/trusted host and SDK, actual init/config/network/lifecycle acceptance; metadata is not entitlement |
| Connection/review | One native five-minute observed connection; native ticket/expiry checks; all six local operation review builders and app review callers | Native/keyboard/accessibility acceptance and final simplified installation experience |
| Generation boundary | Four registered native send commands; strict review ID/fingerprint/consent input; release-owned admission type and native installation provenance | Registry is empty, so every invocation remains unavailable before consumption or execution; this does not complete an operation |
| Operation pipeline | Shared worker, exact context/settings revalidation, local validators, guarded unaccepted/transient results; automatic app controls for all six scopes | Actual effective runtime/result behavior and app workflow acceptance |
| Lifecycle | One revocable native owner, shared worker cancellation, monotonic run budget, publication guards, renderer inference lifetime handling and recovery metadata | Accepted exact Claude recovery and renderer/native/package lifecycle/no-orphan evidence |

The native boundary and profile format are defined in [Claude runtime qualification](../CLAUDE_RUNTIME_QUALIFICATION.md). Only the native factory can construct `ClaudeGenerationRuntime`; renderer data cannot grant it. Profiles bind build/native source/configuration/dependency fingerprint, platform/architecture, CLI and discovery host/SDK identities, eligible plan scope and reviewed evidence references. **The bundled profile list is empty.** No metadata or source preview enables inference. A hash-shaped evidence reference is not proof; adding a record requires the full release audit.

Send commands resolve the stored exact operation and current connection, check consent and identity, then consult native admission. An unavailable route consumes no review, journals no run and starts no worker/model request. After admission, the existing lease rechecks the same native runtime Arc before consumption. No per-send model/account/argv override or API fallback exists.

## Native lifecycle and result invariants

The candidate supervisor owns one active operation, bounded early-cancel records, private staging and nonblocking pipes. A ten-minute monotonic budget starts at review consumption and includes queue time, account pre/post checks, generation and final result admission. Metadata checks are constrained by that same remaining budget. Require complete input, bounded frames/streams, output EOFs, clean child exit and storage cleanup before accepting output. Cancellation interrupts then escalates/reaps the owned group; cleanup uncertainty wins over concurrent cancellation and disables later operations for the session. Actual OS/startup/cleanup/hard-termination/descendant and executable-replacement behavior remains unqualified.

Refresh, forget, cancellation and async-owner drop revoke blocking work. Ordinary five-minute admission expiry cannot invalidate the same already-consented bounded run; it also cannot renew a review or authorize a new run. Wrong IDs and old-owner drop cannot revoke a newer operation. Guard consumption and publication against disconnect/replacement. Already-admitted publication may finish before revocation obtains its lock; an unaccepted preview or saved guide must remain discoverable after a late/lost reply.

Chapter and range/Git validation now run on the blocking pool. Cloned chapter-preview storage shares the same synchronized pending map. A panicked validation/preparation retains its interruption notice and provides operation-specific recovery guidance; do not clear uncertain persistence or regenerate automatically. Accepted chapter/guide history remains untouched. Native publication admission cannot prove a later IPC reply is still wanted. The app now filters replies by mounted view/run identity, cancels on close, discards abandoned chapter previews, rejects unwanted transient replies and refreshes independent pending/guide history. These source paths still need runtime acceptance.

## Remaining delivery order

1. Finish trusted runtime/host/SDK installation and exact permitted account/product route. Qualify effective tools/settings, OS/filesystem/process/egress boundary, supported macOS range and packaged lifecycle. Keep the release registry empty until real applicable evidence exists; do not add a development bypass.
2. Qualify the automatic app generation controls now wired to native send helpers for all six scopes: exact review and consent, progress, cancel, operation-specific preview/save or transient result, citation navigation and actionable failure handling. The compiled empty registry still prevents every send. No API prerequisite or fallback is added. Save remains separate from acceptance.
3. Accept renderer lifetime behavior now in source. Close/backend/context changes cancel/revoke; late chapter previews are discarded, saved guides are preserved, unwanted transient replies are ignored and Claude remains selected after failure. Once a send attempt starts, settings/source stay fixed until explicit reset. Admission expiry cannot erase an in-flight result or prevent saving it. Verify lost-save, late-result disposal and cleanup-failure recovery before another request.
4. Accept Claude interruption restoration now wired for file/overview/section/range/guide/hunk through the shared journal-owned native evidence validator. It restores exact backend/scope, inspects saved/pending results first, requires fresh settings/review/consent and keeps the old notice until valid wanted result adoption. Legacy incomplete records require explicit reselection, never an API. Exercise notice discard/replacement and abandoned lookup/result races with both agents.
5. When validation is authorized, exercise all deferred cases in `xymb`, every matrix cell and a real native user workflow. Record package/runtime/model/configuration/account evidence and source-grounded draft results. Keep `8gqw`, `2r6n`, the parent and M7 open until their actual requirements pass.

## Acceptance criteria

- Every required automatic Claude operation works through a documented permitted account route without a mandatory direct API key or manual handoff.
- Effective supported settings reach the pinned runtime, appear in exact review and cannot be silently changed, ignored or widened.
- Valid locally grounded results enter only the correct unaccepted/transient workflow; invalid/cancelled/crashed/quota-failed operations preserve accepted history and remain on Claude.
- Source/tool/network/process boundaries, progress/cancellation/shutdown and exact-scope recovery have applicable native evidence, including abandoned/lost replies and cleanup failures.
- The user completes the app flow without developer intervention. Registry rows, metadata checks, synthetic dispatch, local previews and formatting do not satisfy this criterion.

## Done evidence and exclusions

Attach exact commit/build fingerprint, native package/platform/toolchain/runtime versions, account/product permission, configuration/operation matrix, real outcomes, source/visual/narrative validation limits and user workflow review. No tests, builds, app launches, Node/SDK/CLI probes, logins or model requests ran for this source change. Automatic Claude cells remain Missing. Do not close this task from source alone, reimplement an agent loop, extract credentials or present a cost limit as guaranteed free usage.


## Exact recovery source checkpoint

The app now accepts Claude journal records and restores the native canonical scope into Claude, rather than rejecting it or opening an API. File/overview/section recovery also runs through the native pure context builder. Section history lookup uses the project-wide history API so an accepted base from an older snapshot is actually discoverable. A shared backend-tagged Git recovery type preserves guide/path-pair/hunk selections for Codex and Claude. Guide recovery inspects saved history before another review.

Both agents now have a local chapter recovery panel listing possible pending previews and saved revisions before a fresh request. The legacy pending format lacks request/agent provenance; matching project/snapshot/target is explicitly only a candidate match. The user can open a saved preview, or inspect saved chapters in Edit. Failed/abandoned history reads cannot authorize another request. Claude completion clears the old notice only after a validated wanted result is adopted; review creation, errors, cancellation, closing and unavailable runtime preserve it. Deliberate API preview creation cannot clear an agent notice.

All of this is unexecuted source. The qualification registry remains empty and automatic Claude cells stay Missing. No tests, builds, launches, runtime probes, logins or model requests ran.
