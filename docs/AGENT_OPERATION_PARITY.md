# Required agent generation coverage

This is the implementation contract for multi-agent generation, reviewed October 15, 2026. It describes required behavior and current source gaps. No agent/backend pair has completed native acceptance for this implementation.

## Product requirement

Codex and Claude Agent, powered by Claude Code, must each generate explanations inside Code Atlas through a supported automatic integration. The user selects the agent, reviews source and settings, starts the run, follows progress and reviews the result in the app. They do not have to configure a direct API provider or copy prompts and results between applications.

Direct APIs remain separately selectable. Manual handoff remains separately labeled. Neither completes an automatic agent requirement. A backend that is unavailable for the requested operation must explain the gap and preserve the user's choice; it cannot silently select an API.

## Required operation matrix

**Source** means native dispatch and local validation are present in source, with runtime acceptance still pending. **Missing** means the automatic adapter/operation is not implemented. None of these entries means production support.

Claude now has invocation/stream-consumer, shared chapter/section review helpers, a managed process supervisor and explicit installation/account-metadata inspection in source. An advanced setup action now calls a registered native SDK catalog check, with bounded SDK/host supervision and independent catalog admission. It accepts installation path hints, derives identities natively, verifies the inspected SDK bytes and displays candidate model/effort metadata. It supplies no repository source or user prompt and never enables generation. Packaged runtime installation, qualified discovery, generation dispatch/UI, account/runtime qualification and lifecycle acceptance remain missing, so its automatic operation cells stay **Missing**. Constructing the manager or mounting setup starts no process; no generation command calls it.

| Operation and result | Codex | Claude Agent | Antigravity |
| --- | --- | --- | --- |
| Plan a repository explanation with file coverage, ordering and evidence | Missing; the local static reading planner is separate | Missing | Missing |
| Generate a file chapter from selected captured files | Source | Missing | Missing |
| Generate an optional overview of selected captured files | Source | Missing | Missing |
| Regenerate one stale section while preserving eligible unchanged sections | Source | Missing | Missing |
| Explain the exact selected source range in Files | Source | Missing | Missing |
| Generate a guide for a selected immutable Git comparison | Missing | Missing | Missing |
| Explain one selected base/head diff hunk | Missing | Missing | Missing |

File chapters, overviews, section repair, source-range explanations and both Git-change operations are required for Codex and Claude Agent. Repository planning must also support these agents when the AI narrative planner is implemented; a static call list does not satisfy generated narrative planning. Antigravity remains a separately required M7 integration subject to its exact permitted surface and account route. A direct API result in any row cannot qualify its agent counterpart.

Generation through either agent must retain the same language evidence limits and visual/chapter schema as other transports. Switching agent cannot turn a partial parser into full language support, establish runtime behavior from static evidence, or bypass source citations. Narrative and visual quality remain separate acceptance gates.

## User flow and configuration

1. Choose a repository and capture its source. Browsing and setup require no API key.
2. From any generation action choose a coding agent or direct API. Agent choices remain visible even when an API preference was previously saved.
3. For the chosen agent, show installation, connection and operation availability, with a specific recovery action. Connecting sends no source. Use vendor-supported authentication; keep account usage separate from API billing.
4. Remember non-secret defaults independently for each agent. Offer only its supported model, reasoning/effort, approval policy and run limits. Allow deliberate overrides. Unsupported settings are unavailable with an explanation; a fixed policy is displayed as fixed.
5. Review the exact captured context and effective settings. Changing context, backend, account or effective settings invalidates the review and consent. Action approvals and consent to send source remain separate.
6. Start once, show progress, permit cancellation and retain the selected backend through failures. Do not silently retry using another backend or account route.
7. Validate output locally. Chapter and PR Guide results remain unaccepted drafts; range/hunk explanations remain transient. Preserve the existing operation-specific history rules.
8. Recover the original operation after interruption with fresh review and consent. Missing range/comparison identity requires reselection; recovery cannot widen it into a whole-file request.

## Reuse the existing implementation

Use vendor-maintained agent loops: the official Codex app-server, the official Claude Agent SDK or its documented programmatic CLI, and a qualified official Antigravity surface. Evaluate maintained ACP adapters where they fit. Code Atlas owns context review, dispatch, lifecycle and validation; it does not implement a new agent loop, credential system or universal model/effort catalog.

Keep operation context and validation independent of transport. Extend the existing shared chapter context rather than building an API request to obtain an agent prompt. Apply the same separation to the existing range and PR context builders when wiring their agent operations.

| Existing source | Responsibility to reuse or extend |
| --- | --- |
| [provider.rs](../src-tauri/src/provider.rs) | Immutable chapter/section context, review fingerprints and operation-specific range/PR validators |
| [codex_commands.rs](../src-tauri/src/codex_commands.rs), [codex_agent.rs](../src-tauri/src/codex_agent.rs) | Existing native Codex dispatch and managed runtime lifecycle |
| [claude_agent.rs](../src-tauri/src/claude_agent.rs), [claude_worker.rs](../src-tauri/src/claude_worker.rs), [claude_connection.rs](../src-tauri/src/claude_connection.rs), [claude_commands.rs](../src-tauri/src/claude_commands.rs) | Candidate Claude protocol/process supervision and explicit sanitized version/account inspection; model discovery, generation dispatch/UI and qualification remain open |
| [claude_model_discovery.mjs](../src-tauri/resources/claude_model_discovery.mjs), [claude_discovery_worker.mjs](../src-tauri/resources/claude_discovery_worker.mjs), [claude_discovery.rs](../src-tauri/src/claude_discovery.rs), [claude_models.rs](../src-tauri/src/claude_models.rs) | Candidate SDK initialization/projection, native supervision and catalog admission, called by the explicit setup check; qualified installation, generation dispatch and acceptance remain open |
| [chapter_generation.rs](../src-tauri/src/chapter_generation.rs), [lib.rs](../src-tauri/src/lib.rs) | Chapter/section preservation and draft preparation; separate PR Guide preparation |
| [recovery.rs](../src-tauri/src/recovery.rs) | Bounded interruption metadata without source, credentials or reusable consent |
| [App.tsx](../src/App.tsx), [CodexChapterDialog.tsx](../src/CodexChapterDialog.tsx) | Current backend chooser and supported Codex operation/settings flow |

## Implementation order and owners

1. **Automatic Claude Agent (`8gqw`, priority 0):** implement the maintained SDK/CLI adapter, effective settings, managed lifecycle and shared file/overview validation. Resolve the exact product/account eligibility in the same task; do not replace it with an API requirement or call manual handoff complete. Current official pages disagree on third-party subscription integration, so account-route qualification remains open. See [the recorded sources and decision](kata-tasks/41-claude-agent-backend.md).
2. **Codex completion (`z3an`, priority 0):** qualify the implemented routes and extend the shared context/dispatch to range and Git-change operations. Retain exact section identity and preservation semantics.
3. **Coverage and UX gate (`2r6n`, priority 0):** require both agents for every required row above, real effective controls, cancellation and scope-preserving recovery. Update this matrix with exact runtime/adapter versions and dated acceptance evidence before promoting any cell to supported.
4. **Antigravity (`m00g`):** qualify its supported integration and account route independently. An unresolved requested backend keeps M7 open.

Codex range dispatch now shares immutable range context with direct APIs and uses the existing managed runtime and independent citation validator. It stages only the excerpt, returns a transient explanation and never prepares a chapter draft. The full captured-file identity remains in the review; staged-content integrity uses the excerpt digest. Native acceptance, runtime qualification and scope-preserving interruption recovery remain pending.

Runtime qualification, login, real generation and native acceptance remain pending. This contract update ran no tests, builds, agent processes or provider requests.
