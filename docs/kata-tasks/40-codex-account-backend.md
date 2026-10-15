---
kind: task
key: codex-account-backend
parent: t29
title: Implement Codex app-server chapter generation with account sign-in
priority: 0
labels: agents, provider
---

## User outcome
The user connects an eligible ChatGPT account once and generates chapters through Codex without configuring a direct API key for each workflow.

## Research decision — 2026-10-15
Use official Codex app-server over Rust-managed stdio. The initial local-app implementation uses the runtime's native managed ChatGPT login. OpenAI's separate direct Sign in with ChatGPT documentation remains relevant to a possible qualified distribution; Code Atlas has not implemented direct client registration, PKCE or token sharing. Establish this project's license, exact account/distribution eligibility and pinned runtime contract. An older installed runtime is not evidence that current app-server lacks required controls.

## Implementation update — 2026-10-15

Source now includes the managed runtime in `codex_agent.rs`, native commands in `codex_commands.rs`, shared chapter context/schema/evidence validation, `CodexChapterDialog`, and the app backend picker. The route covers one file chapter with selected supporting files or an overview of 1–12 selected captured files. It includes executable setup, native browser login/cancel/logout, explicit runtime `model/list`, model/effort preferences, fixed `never` approval policy, exact review/consent, staged context, progress/cancel, unaccepted draft save/review and child shutdown.

Schema/configuration checks require restricted reads, structured output and ephemeral threads. Shell/web/plugin-related controls are explicitly configured and unexpected tool events are rejected; this is not proof of a universal `tools: none` boundary. Runtime `model/list` provides capability choices, not account-entitlement or remaining-quota proof. Threads are ephemeral; persistent thread resume is absent. Section updates, Code Lens, PR Guide and hunk explanations are not connected to this agent path.

Remaining work, with every acceptance criterion still open:

- [ ] Pin and package a supported runtime/version, record its license/distribution inventory and confirm the exact native-login route is eligible for this app; separately decide whether a direct SIWC client is needed for distribution.
- [ ] Qualify account availability and model selection independently of the runtime catalog; bind/revalidate account and runtime identity in reviews and exercise expiry, refresh, logout and executable changes.
- [ ] Exercise actual filesystem, command, write, `.git`, instruction/plugin/MCP and tool-egress restrictions; account/provider control traffic must remain possible without granting arbitrary agent egress. Add an OS boundary or keep the route unqualified if runtime controls cannot satisfy the contract.
- [ ] Record real file/overview generation, structured-output rejection, local citation validation, draft save/accept, quota/error recovery, cancellation/crash/restart and process cleanup on the packaged app.
- [ ] Implement or explicitly scope persistent resume and additional operation adapters (section, Code Lens and PR); current ephemeral runs do not satisfy those lifecycle/workflow requirements.

No tests, builds, login, app-server launch, model request or runtime probes ran for this implementation. The selectable source UI does not establish supported-backend acceptance.

## Contract
Add an Agent runtime category alongside Direct API. Connect/disconnect with the documented browser OAuth flow, app attribution and native token storage/refresh. Read account-entitled models; combine with pinned runtime capability metadata for supported reasoning values. Show effective model, effort, approval policy and immutable source manifest before a run. Launch one managed app-server child, with app-owned configuration and no additional listener/Node daemon. Request the existing chapter output schema and route successful output through local source/citation validation to an unaccepted draft.

Use restricted read-only staging, explicitly disable command execution and web/unrelated tools, and do not load repository instructions/plugins. Read-only access alone is not a no-command policy. Begin with bounded reviewed evidence passed directly; expand investigation with app-owned read/search tools. Implement progress, turn/interrupt, exit/reap, refresh/restart/resume and late-result guards. No automatic direct API fallback.

## Acceptance criteria
- Eligible account login and one real chapter run require no API key; plan limits/eligibility and account route are clear.
- Account-entitled model discovery controls choices; unsupported effort/approval values cannot be silently ignored.
- OAuth secrets stay outside WebView, repository files, exports and logs; expiry/logout are recoverable.
- Exact selected context is staged; supported runtime tool controls and sandbox enforce the declared boundary.
- Structured output is independently validated; invalid claims cannot become accepted chapters.
- Progress/cancel/quota/refresh/crash states preserve prior history and no orphan process remains.
- Update matrix from candidate to supported only with pinned-version packaged evidence when testing is authorized.

## Excludes
ACP wrappers without need, token scraping, paid fallback, speech synthesis through this account route and an extra Node server.

## Done evidence
Documented auth integration, versioned adapter/configuration, complete source-grounded chapter workflow and later account/cancel/permission acceptance.

## References
- https://developers.openai.com/siwc/quickstart
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
- https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
- https://developers.openai.com/codex/app-server
