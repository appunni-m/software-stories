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

`src-tauri/src/claude_worker.rs` adds a native worker manager with a single active operation, bounded early-cancel records, a ten-minute cooperative deadline, executable SHA-256 reinspection, private per-run staging, nonblocking stdin/stdout/stderr and application-authored progress. It rejects oversized streams, incomplete input, missing EOF, nonzero exit and invalid protocol. Cancellation interrupts the owned process group, then escalates and reaps; shutdown signals that same owned group. Successful output is withheld until process completion and staging cleanup. Cleanup uncertainty disables further runs for the app session, including failures during preparation or unwinding. The manager is registered for state, shutdown and explicit installation inspection; no generation command calls it.

The Unix implementation observes child exit using `waitid(..., WNOWAIT)` before group cleanup and reaping, with signaling and PID publication under one mutex. `ECHILD` drops the stale identity and rejects the run instead of signaling a possibly reused PID. Pipe flag and wait-record FFI contracts were inspected against locked libc `0.2.190` and local macOS SDK headers. This is source review, not native lifecycle evidence. Hard app termination, child escape, platform behavior, blocking OS cleanup and executable replacement between hash inspection and launch remain unqualified.

**Not delivered:** qualified model catalog, account reinspection for generation, automatic generation dispatch/UI, persisted recovery, range/PR operations or permitted account-route evidence. Automatic Claude generation is still unavailable. Native code must supply runtime identity; the WebView cannot deserialize it. Environment clearing and staged working directories do not prove managed-policy behavior, OS confinement, provider egress restrictions or absence of internal model fallback. Qualify those before enabling a route. No tests, builds, agent processes, logins or model requests ran. Process supervision and storage cleanup also require the deferred acceptance in `xymb`; source implementation does not close this task.

## Source progress — explicit installation/account inspection; generation remains missing

The backend chooser now offers **Check Claude installation** independently of direct API setup. `ClaudeConnectionDialog.tsx` invokes a native inspection only after an explicit click in the macOS app. It remembers only the canonical executable path; account observations remain in memory. Browser preview and mounting the dialog start no process. Closing, returning to the chooser or cancelling abandons metadata and requests cancellation; the shared native manager handles early cancellation and one active operation.

`claude_commands.rs` registers inspection and cancellation, not generation or login. The worker hashes the selected executable, runs only `--version`, and, for the candidate `2.1.300`, the documented `auth status` command. It uses the existing owned-process cleanup, bounded nonblocking pipes and private storage, with a 45-second cooperative inspection deadline. Version stdout is limited to 128 bytes; account JSON to 64 KiB; stderr to 256 KiB and discarded. It rechecks executable identity between commands and before returning. The shared cleared environment keeps real HOME for the vendor account store, does not set `CLAUDE_CONFIG_DIR`, and disables the vendor background updater with `DISABLE_AUTOUPDATER=1`. This does not prevent executable replacement by another process or qualify immutable execution.

`claude_connection.rs` accepts a strict semantic version envelope and typed documented account fields, rejects duplicate known fields, malformed/oversized JSON and inconsistent login/exit metadata, and discards unknown fields. It reports account, API-key/helper, alternate-provider and unknown routes separately. A bounded hash of reported account/configuration metadata is neither a credential nor entitlement proof. Raw stdout, stderr and future credential fields never become UI messages, logs or saved preferences. Matching the candidate or reporting a login leaves `automaticGenerationAvailable` false; no model turn is started by inspection.

### Remaining work in order

1. Qualify the app-wired candidate catalog check below, and replace its advanced installation path hints with a packaged runtime installation. SDK hashing/staging, environment clearing, raw vendor limits and independent child completion are implemented in source. Pin/package/verify the host runtime and complete actual lifecycle/isolation acceptance before treating discovery as supported. The advanced metadata action does not establish a trusted installation or enable generation. The documented CLI command list does not establish a model-list command; use the maintained SDK methods instead of an invented control RPC or a hidden direct model API prerequisite. Resolve aliases to the exact requested/observed model and model-specific effort capabilities. A metadata list alone is not account entitlement.
2. Resolve the exact permitted product/account route, then bind fresh native account/catalog/executable observations to review and recheck before dispatch. An API-backed route stays an explicit separate choice and cannot close the no-mandatory-API requirement.
3. Connect file/overview generation through the existing shared review, worker, independent evidence validation, unaccepted draft and explicit acceptance flow. Add effective model/effort/approval controls, app-authored progress, cancellation and scope-preserving recovery. A successful setup check is not generation support.
4. Complete section, exact range and both Git-change operations in the required parity matrix, then record the deferred native/runtime acceptance in `xymb`. Keep this task, `2r6n` and M7 open until their actual criteria are met.

This delta also restricts the manual handoff dialog's mode type after Codex range support: it accepts chapters/overviews only. Shared modal focus handling includes the native disclosure's summary control. No tests, builds, app launches, CLI probes, logins or model requests ran.

## Source progress — candidate SDK catalog adapter; qualified generation remains missing

Inspected the official published `@anthropic-ai/claude-agent-sdk@0.3.300` artifact, whose package metadata pairs it with Claude Code `2.1.300`. The downloaded archive matched the registry's SHA-512 integrity value; registry signatures were not verified. [The provenance record](../research/snapshots/2026-10-15-claude-sdk-0.3.300.json) records source file hashes and the exact interface. No dependency was installed or executed, and no vendor implementation was copied into the app.

The maintained public `Query.supportedModels()` returns `ModelInfo` with `value`, optional `resolvedModel`, display name and optional per-model effort metadata. `Query.initializationResult()` includes optional `claude_code_version`. In this inspected implementation, both methods read initialization metadata. These are candidate SDK contracts, not a fabricated CLI model-list command or an account-entitlement guarantee. The SDK package requires Node 18 or later; the production worker/runtime distribution remains unresolved.

`resources/claude_model_discovery.mjs` is a library with no import-time work. The explicit advanced setup action now reaches it through a registered native metadata command. The candidate native worker described below injects the verified SDK and native-owned path observations. The helper passes an async iterable that yields **zero user messages**, uses the maintained initialization/model methods, projects only known model fields and emits a bounded versioned envelope. It requests empty tools/settings sources/MCP, safe/restricted mode, `dontAsk`, no prompts, hooks or session persistence. Discovery has a 30-second cooperative deadline and a separate 5-second SDK cleanup limit; cancellation discards metadata and cleanup uncertainty wins. SDK return/cleanup can finish after its own bounded exit wait, so it is **not** proof that the child exited or was reaped. The native owner must establish that before accepting observations. The SDK's env option is additive; the host must clear inherited credentials/endpoints before loading it. Import-time work, hidden initialization activity, effective settings and process behavior remain unqualified.

`claude_models.rs` independently admits only the candidate SDK/runtime pair and bounded catalog JSON (128 KiB, at most 256 rows). It rejects duplicate known JSON fields/selection values, unknown bridge fields, malformed/inconsistent effort metadata and contradictory capability rows. It uses SDK canonical resolution when present; unresolved named/latest aliases are omitted rather than mapped by the app. Aliases resolving to the same full ID coalesce only when effort capabilities agree; display/capability ordering is deterministic. A boolean without enumerated effort levels exposes no invented explicit levels. An empty usable catalog rejects. The same full-ID predicate now protects generation settings, including rejection of bare `claude-` and unresolved `-latest` values.

The native catalog type cannot be deserialized from WebView input. Its capabilities remain candidate metadata; no constructor here establishes account eligibility or a qualified runtime. The explicit Tauri catalog check calls the worker and native parser; qualified app discovery and automatic generation remain unavailable. Every Claude automatic operation cell remains Missing. No tests, builds, SDK/CLI execution, logins or model turns ran; deferred cases are in `xymb`.

## Source progress — native discovery supervision; app integration remains pending

`claude_discovery.rs` defines native-only host/CLI executable identities and an inspected SDK module path. `ClaudeWorkerManager::inspect_model_catalog` uses the existing single-operation lock, early cancellation, shutdown and cleanup-failure latch. It checks canonical executable hashes, requires bounded reported first-party account metadata, and runs version/auth inspection before and after SDK discovery under **one shared 45-second cooperative deadline**. A changed reported account or executable rejects metadata; these observations do not prove the SDK's effective account, entitlement or product eligibility.

The native reader caps SDK module reads at 2 MiB plus sentinel, rejects nonregular/symlink-resolved/mutated paths, and requires the inspected `sdk.mjs` SHA-256 from the recorded artifact. It copies those verified bytes and app-owned wrapper/library scripts into private read-only staging. The host command clears its environment before script execution and receives only a bounded native-path configuration over nonblocking stdin, with no repository source or vendor RPC. The shared metadata pump still handles version/auth commands with empty input. It requires complete input, bounded stdout/stderr, host EOF/success, owned-group cleanup/reaping and staging cleanup before returning a candidate catalog. Native/OS blocking operations and executable replacement races remain unqualified.

`claude_discovery_worker.mjs` clears unexpected environment before SDK import and injects the SDK's public custom-spawn hook. It permits one exact CLI child with shell disabled and no detachment, constructs an explicit account-store environment, and bounds raw vendor stdout (2 MiB frames, 32 MiB total, valid complete UTF-8) and discarded stderr (256 KiB). Raw-output checks run upstream of the readable given to the SDK, so they remain active when SDK cleanup stops consuming it. After SDK cleanup, the wrapper drains through those checks and independently requires child close, both raw EOFs and a successful exit before emitting catalog bytes. It bounds child termination after SDK cleanup; Rust separately supervises the retained host group. Cleanup uncertainty takes priority over cancellation and disables further session operations.

The scripts are embedded through Rust source inclusion. No SDK module or Node binary is vendored, downloaded or executed by these changes. A qualified native installation owner must still supply a genuine pinned host executable and licensed SDK module. The advanced metadata check accepts path hints and derives hashes natively; neither a renderer hint nor an observed hash establishes trusted installation provenance. Host version, architecture, signatures/licenses, module-resolution behavior, effective settings/account route, hard-kill/descendant behavior and packaged acceptance remain open. No new server or port was added.

**Still missing:** a qualified runtime installer, accepted app discovery, account/product-route resolution, generation dispatch/UI, persisted recovery and section/range/PR completion. The explicit native catalog command now calls this method for metadata only. It returns metadata with generation unavailable and never constructs a qualified review runtime or a chapter. All automatic Claude operation cells stay **Missing**. No tests, builds, app launches, Node/SDK/CLI probes, logins or model turns ran. `xymb` records deferred supervision and shared-inspection regression cases.


## Source progress — explicit app catalog inspection; no generation qualification

`inspect_claude_agent_model_catalog` is now registered alongside installation inspection and cancellation. The advanced setup form starts it only on an explicit click in the native app. It accepts CLI, Node and installed `sdk.mjs` path hints only; no catalog, executable digest, account attestation or arbitrary runtime options come from the WebView. The native operation validates/canonicalizes paths, reads executable identities and constructs the private runtime record under the same operation lock and shared 45-second deadline. Before SDK import, it requires the inspected module bytes and a bounded successful `node --version` observation compatible with the SDK's declared Node >=18 engine range. That engine check is not a production Node pin, provenance or architecture/signature acceptance.

The form displays canonical model IDs and advertised explicit efforts, omitted unresolved-row count, and sanitized host/SDK observations. It has no generation-settings selector because generation is still unavailable. Only successful canonical installation path hints persist; account/catalog data and fingerprints remain in memory. Editing a path invalidates observations. Installation checks and catalog checks share busy/cancel handling; abandoned/cancelled metadata is discarded, cleanup uncertainty retains priority and closing/returning to the chooser requests cancellation. Mounting setup and browser preview start no process.

This is an advanced candidate setup path, not the final installation experience. It requires an externally installed SDK/Node host and offers no installer, login, automatic fallback or generation action. The app supplies zero user messages; actual vendor initialization/network/configuration behavior still needs qualification. SDK/host packaging, permitted account/product integration, effective generation settings, review/dispatch, output validation/draft flow, recovery and operation parity remain open. All automatic Claude cells stay Missing. No tests, builds, app launches, SDK/CLI/Node probes, logins or model turns ran; the new command/UI cases are deferred in `xymb`.

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
