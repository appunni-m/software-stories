---
kind: task
key: claude-agent-backend
parent: t29
title: Implement Claude Agent chapter adapter and resolve account eligibility
priority: 0
labels: agents, provider
---

## User outcome
The user can choose the Claude Code-powered agent loop, configure its supported model/effort/approvals, and generate a source-grounded chapter through an explicitly chosen account route.

## Research decision — 2026-10-15
Prefer the official Claude Agent SDK behind one managed worker; a documented claude -p stream-json subprocess is an alternative. The October 7 Help Center explicitly permits subscription usage by SDK, -p and third-party apps, while the SDK overview still requires prior approval for third-party claude.ai login/subscription limits. Preserve both sources and resolve custom-product distribution eligibility before advertising approved subscription support. Do not state that subscription usage is technically impossible.

### Eligibility recheck — 2026-10-15

The official pages remain distinct in their wording: the Help Center says third-party apps may use subscription limits, while the SDK overview's product-integration guidance says a third-party developer needs prior approval to offer claude.ai login or subscription limits. Because Code Atlas would expose that login/plan route from its own UI, keep **Claude plan login unavailable until Anthropic confirms this product is approved**. The Help Center confirms that Agent SDK calls signed in with a Claude plan consume plan limits; it does not state that a third-party product is exempt from the SDK overview's approval condition. API-key/Console billing is a separate explicit backend choice and must never be selected as a fallback. [Help Center plan usage](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan), [Agent SDK integration requirements](https://code.claude.com/docs/en/agent-sdk/overview).

## Contract

Implement automatic dispatch first for file chapters and selected-source overviews, then the section/range/PR operations in the [required parity matrix](../AGENT_OPERATION_PARITY.md). Reuse transport-independent captured context and the existing local validators. The same task owns agent installation/connection state, supported model/effort/approval defaults and overrides, exact review, streaming progress, bounded cancellation/process cleanup and scope-preserving interruption recovery. Claude must be selectable before any direct API setup. An API-key-only Agent SDK route cannot close the user's no-mandatory-API requirement.

This is a required automatic agent integration, not an optional API adapter. The separate manual Claude Code handoff now present in source provides reviewed task copy/import and local evidence validation without an API key in Code Atlas. It does not manage a Claude runtime or account, verify model/reasoning settings, enforce permissions, or provide cancellation. It does not satisfy this task. See the [agent-first release gate](45-agent-first-generation.md).
Pin SDK/binary versions and use the documented auth store/flow with isolated app configuration. Separate account-backed usage from optional API-key usage. Strip inherited credential/provider/endpoint environment that could select API billing; ANTHROPIC_API_KEY overrides account auth in noninteractive mode. Do not assume --bare supports OAuth token behavior. Reuse chapter JSON schema, independent citation validation and explicit draft acceptance.

Expose model-dependent effort, approval behavior and max turns; unsupported values are unavailable. Explicitly configure available tools, empty settingSources and strict app-owned MCP settings. allowedTools is an auto-approval list, not a tool-removal boundary. Remove write/command/web/subagent capabilities and enforce staged-file scope with per-call checks and appropriate sandboxing. Stream progress; interrupt then terminate/reap after a bounded grace period. Never retry through a direct API automatically.

## Acceptance criteria

- All required Claude operation rows in the parity matrix complete through an automatic supported agent adapter; file/overview milestones alone do not close the full task. The API path, manual handoff and a disabled chooser entry are not acceptance evidence.
- Dated resolution records which exact account/distribution path is permitted; login UI follows it.
- Account mode cannot silently pick inherited API credentials or incur API billing; quota/eligibility failures stay on selected backend.
- A real chapter returns structured_output and passes existing local schema/evidence checks; success without structured output is a failure.
- Effective model/effort/permission settings reach the pinned runtime unchanged or by documented mapping.
- Progress, cancellation, invalid output, crash and shutdown preserve history and release process resources.
- Runtime cannot read outside reviewed staging, execute repository instructions, or widen tools via inherited settings/MCP.
- Record packaged acceptance later; no tests/logins/agent execution occurred during this evaluation.

## Excludes
Reimplementing the agent loop, token extraction, invented universal effort options and treating a cost budget as guaranteed zero spending.

## Done evidence
Auth eligibility decision, pinned worker adapter, configuration UI and later account/permission/cancel/chapter acceptance.

## Source progress — October 15, protocol and process supervision; unqualified

`src-tauri/src/claude_agent.rs` now prepares the documented programmatic CLI invocation and an environment-cleared command without spawning it. It validates native-owned runtime/account digests and model-specific effort capabilities, fixes approval to `dontAsk`, bounds turns, puts source on stdin, requests safe/restricted mode, removes external tools/MCP/customization and disables session persistence. Version `2.1.300` is a candidate from the official changelog, not an accepted runtime pin.

The bounded output consumer checks initialization, session/model identity, permission/tool/customization scope and final structured output. The runtime's internal `StructuredOutput` formatter is distinct from external tools; its intermediate arguments/results cannot become chapters. Wrong/oversized/malformed/late output poisons the stream. A final result requires EOF and a successful process exit before it may reach the existing independent validators.

`provider.rs` now shares the chapter/section review builder between Codex and the offline Claude contract. Claude review/revalidation binds captured evidence, selected section/base identity, executable hash/version, account digest and effective requested settings. No direct API request is built to obtain this context. The Claude contract explicitly remains pending qualification.

`src-tauri/src/claude_worker.rs` adds a native worker manager with a single active operation, bounded early-cancel records, a ten-minute cooperative deadline, executable SHA-256 reinspection, private per-run staging, nonblocking stdin/stdout/stderr and application-authored progress. It rejects oversized streams, incomplete input, missing EOF, nonzero exit and invalid protocol. Cancellation interrupts the owned process group, then escalates and reaps; shutdown signals that same owned group. Successful output is withheld until process completion and staging cleanup. Cleanup uncertainty disables further runs for the app session, including failures during preparation or unwinding. The manager is registered only for state and shutdown; no generation command calls it.

The Unix implementation observes child exit using `waitid(..., WNOWAIT)` before group cleanup and reaping, with signaling and PID publication under one mutex. `ECHILD` drops the stale identity and rejects the run instead of signaling a possibly reused PID. Pipe flag and wait-record FFI contracts were inspected against locked libc `0.2.190` and local macOS SDK headers. This is source review, not native lifecycle evidence. Hard app termination, child escape, platform behavior, blocking OS cleanup and executable replacement between hash inspection and launch remain unqualified.

**Not delivered:** qualified model catalog, account reinspection for generation, automatic generation dispatch/UI, persisted recovery, range/PR operations or permitted account-route evidence. Automatic Claude generation is still unavailable. Native code must supply runtime identity; the WebView cannot deserialize it. Environment clearing and staged working directories do not prove managed-policy behavior, OS confinement, provider egress restrictions or absence of internal model fallback. Qualify those before enabling a route. No tests, builds, agent processes, logins or model requests ran. Process supervision and storage cleanup also require the deferred acceptance in `xymb`; source implementation does not close this task.

## Source progress — explicit installation/account inspection; generation remains missing

The backend chooser now offers **Check Claude installation** independently of direct API setup. `ClaudeConnectionDialog.tsx` invokes a native inspection only after an explicit click in the macOS app. It remembers only the canonical executable path; account observations remain in memory. Browser preview and mounting the dialog start no process. Closing, returning to the chooser or cancelling abandons metadata and requests cancellation; the shared native manager handles early cancellation and one active operation.

`claude_commands.rs` registers inspection and cancellation, not generation or login. The worker hashes the selected executable, runs only `--version`, and, for the candidate `2.1.300`, the documented `auth status` command. It uses the existing owned-process cleanup, bounded nonblocking pipes and private storage, with a 45-second cooperative inspection deadline. Version stdout is limited to 128 bytes; account JSON to 64 KiB; stderr to 256 KiB and discarded. It rechecks executable identity between commands and before returning. The shared cleared environment keeps real HOME for the vendor account store, does not set `CLAUDE_CONFIG_DIR`, and disables the vendor background updater with `DISABLE_AUTOUPDATER=1`. This does not prevent executable replacement by another process or qualify immutable execution.

`claude_connection.rs` accepts a strict semantic version envelope and typed documented account fields, rejects duplicate known fields, malformed/oversized JSON and inconsistent login/exit metadata, and discards unknown fields. It reports account, API-key/helper, alternate-provider and unknown routes separately. A bounded hash of reported account/configuration metadata is neither a credential nor entitlement proof. Raw stdout, stderr and future credential fields never become UI messages, logs or saved preferences. Matching the candidate or reporting a login leaves `automaticGenerationAvailable` false; no model turn is started by inspection.

### Remaining work in order

1. Qualify a vendor-maintained model-discovery surface and its exact version/provenance. The current documented CLI command list does not establish a model-list command. Evaluate the maintained SDK's initialization/server-information capabilities before choosing an adapter; do not invent an undocumented control RPC, fabricate a catalog or call the direct model API as a hidden prerequisite. Resolve aliases to the exact requested/observed model and model-specific effort capabilities. A metadata list alone is not account entitlement.
2. Resolve the exact permitted product/account route, then bind fresh native account/catalog/executable observations to review and recheck before dispatch. An API-backed route stays an explicit separate choice and cannot close the no-mandatory-API requirement.
3. Connect file/overview generation through the existing shared review, worker, independent evidence validation, unaccepted draft and explicit acceptance flow. Add effective model/effort/approval controls, app-authored progress, cancellation and scope-preserving recovery. A successful setup check is not generation support.
4. Complete section, exact range and both Git-change operations in the required parity matrix, then record the deferred native/runtime acceptance in `xymb`. Keep this task, `2r6n` and M7 open until their actual criteria are met.

This delta also restricts the manual handoff dialog's mode type after Codex range support: it accepts chapters/overviews only. Shared modal focus handling includes the native disclosure's summary control. No tests, builds, app launches, CLI probes, logins or model requests ran.

## References
- https://code.claude.com/docs/en/agent-sdk/overview
- https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan
- https://support.claude.com/en/articles/17154008-monthly-api-credits-for-max-and-team-plans
- https://code.claude.com/docs/en/authentication
- https://code.claude.com/docs/en/agent-sdk/permissions
- https://code.claude.com/docs/en/agent-sdk/python
- https://code.claude.com/docs/en/agent-sdk/structured-outputs
- https://code.claude.com/docs/en/cli-reference
- https://code.claude.com/docs/en/setup#disable-auto-updates
