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

Schema/configuration checks require restricted reads, structured output and ephemeral threads. Shell/web/plugin-related controls are explicitly configured and unexpected tool events are rejected; this is not proof of a universal `tools: none` boundary. Runtime `model/list` provides capability choices, not account-entitlement or remaining-quota proof. Threads are ephemeral; persistent thread resume is absent. Exact Code Lens ranges, PR Guides and single-hunk explanations are now connected in source; full runtime and native workflow acceptance remain open.

The source now also connects **one-section stale repairs** to Codex. The common chapter context builder owns freshness, accepted-base/latest-revision checks, exact declared current source, bounded old/current source, section-only prompt/schema and dependency filtering; API and agent transports use that same context. The Codex v2 operation review binds the exact chapter/section revision metadata, prompt, schema, model, effort and executable. Revalidation rebuilds the repair before dispatch. The UI freezes the section's declared source selection and discloses that old source/section text appear in the prompt as context only. Successful output uses the existing section canonicalizer and unaccepted-draft flow, preserving unaffected identities/assets according to that canonicalizer. Additional secret-like scanning and early size bounds now cover old section source. These are static source changes, without generation or acceptance evidence.

Interruption records now retain provider, purpose, selected paths and exact section revision identity without source bytes or consent. A recovered Codex file/overview/section operation returns to Codex with the recorded source scope and a fresh review; section recovery reloads the exact accepted base. A different route or newer recovery selection abandons pending lookups. Legacy records without scope, and range/PR records without their full original evidence targets, do not reopen as a whole-file request. They require deliberate reselection of the original evidence. This is scope-safe source recovery, not persistent runtime thread resume or qualified interruption acceptance.

The source also connects **exact Code Lens selections**. The backend chooser keeps Codex available for range requests without an API prerequisite. The existing Codex dialog shows a fixed path/line/optional UTF-8-byte selection, native model/effort controls and a fresh exact review/consent. A shared API/agent range builder performs snapshot, full-file hash, UTF-8 bounds, 32 KiB excerpt and secret-like checks before transport encoding. The Codex review binds the exact target, prompt, schema and effective controls, and revalidation rebuilds that same range. One native execution helper owns the chapter/range lifecycle and refuses cross-operation review IDs. Only the excerpt is staged; its staged-content hash is distinct from the captured-file hash in the review. Returned range claims pass the existing independent exact/unique-quote and identifier validator, open their exact citations, and remain transient. No chapter draft or accepted history is written. Early cancel, stale dialogs and backend switching discard late range responses. Runtime qualification and packaged range acceptance remain pending; interruption recovery still requires deliberate reselection of the original range.

Remaining work, with every acceptance criterion still open:

- [ ] Pin and package a supported runtime/version, record its license/distribution inventory and confirm the exact native-login route is eligible for this app; separately decide whether a direct SIWC client is needed for distribution.
- [ ] Qualify account availability and model selection independently of the runtime catalog; bind/revalidate account and runtime identity in reviews and exercise expiry, refresh, logout and executable changes.
- [ ] Exercise actual filesystem, command, write, `.git`, instruction/plugin/MCP and tool-egress restrictions; account/provider control traffic must remain possible without granting arbitrary agent egress. Add an OS boundary or keep the route unqualified if runtime controls cannot satisfy the contract.
- [ ] Record real file/overview generation, structured-output rejection, local citation validation, draft save/accept, quota/error recovery, cancellation/crash/restart and process cleanup on the packaged app.
- [ ] Implement or explicitly scope persistent resume, complete exact range/Git-scope recovery and qualify every implemented chapter/section/range/PR route. Current ephemeral runs and source wiring do not satisfy native lifecycle/workflow acceptance.

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

## Source scope correction — no implicit truncation

Codex chapter initialization and context reset now retain the complete supplied source selection instead of silently taking its first 12 paths. A selection above the supported limit remains visible and cannot prepare a review until supporting files are deliberately removed. A fixed section-repair scope cannot be truncated or edited to bypass preservation checks; an oversized declared scope requires correction at its owner. Matching silent truncation was removed from manual handoff initialization and direct API file-chapter initialization. Native source/byte/path limits and independent validation remain authoritative. This is a source correction, not accepted workflow evidence; no tests, builds or runtime requests ran.

## Git comparison and hunk source dispatch

The PR Guide screen now defaults to Codex, keeps Claude Agent visibly pending and exposes direct APIs as an optional choice. Agent selection never requires API settings or credentials. Selecting changed files then **Review with Codex**, or **Explain hunk**, opens the existing Codex setup/model/reasoning/review flow. Approval stays fixed to `never`. These are source changes, not an exercised native workflow.

- `build_codex_pr_guide_review` and `build_codex_pr_change_review` use transport-independent immutable Git context. API admission/encoding remain in the separate API builders; an agent does not need an API request to obtain a prompt.
- Reviews bind the comparison ID, refs/resolved commits, exact path pairs or hunk, patch bytes/hashes, schema, model, effort, executable and staged patch mapping. Native send rebuilds and compares the reviewed contract before consumption. Source remains bounded by 12 changes/250,000 bytes and encoded requests by 512 KB.
- `send_codex_pr_guide_request` and `send_codex_pr_change_request` use the common managed Codex lifecycle and operation guards. Temporary patch names are neutral aliases; original Git path/side/hunk identities remain authoritative for citations. Rename labels cannot become staged filesystem paths.
- Valid guides enter independent unaccepted PR Guide history through the existing citation validator. Done returns to claim review/accept/reject. Hunk claims use the existing exact base/head quote validator and open their selected hunk; no chapter/guide history is written.
- Progress, cancellation and late-result handling reuse the Codex dialog. A late cancellation or lost reply may follow a saved guide; the UI reloads PR Guide history and tells the user to inspect it before regenerating. A worker panic preserves recovery state and shuts down the manager rather than clearing the notice.

Remaining: automatic repository narrative planning, permitted pinned runtime/account/distribution qualification, effective boundary checks, persistent resume or its explicit scope, exact Git/range interruption recovery, packaged acceptance and all deferred cases in `xymb`. The journal still requires deliberate comparison/hunk reselection. No tests, builds, launches, logins, runtime probes or model calls ran.
