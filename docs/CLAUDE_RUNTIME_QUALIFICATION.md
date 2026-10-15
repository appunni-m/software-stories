# Claude native generation admission

Claude generation is still unavailable. The bundled `src-tauri/resources/claude_generation_qualifications.json` has **zero profiles**. Registered send commands and a valid source preview cannot turn an observed installation into an admitted runtime. This document describes source behavior, not accepted runtime support.

## Native command boundary

The five registered commands are `send_claude_chapter_request`, `send_claude_repository_plan_request`, `send_claude_code_lens_request`, `send_claude_pr_guide_request` and `send_claude_pr_change_request`. Each takes one `input` object containing only `reviewId`, `fingerprint` and `consent`. Unknown input fields are rejected. No command accepts executable/account/catalog identities, model settings, qualification flags, endpoint URLs, repository roots or launch arguments.

Before dispatch, native code checks explicit consent, bounded review identity, the stored fingerprint and exact operation contract. It resolves the current unexpired connection from native state, checks its connection identity and consults the release-owned admission policy. An unavailable route returns before review consumption, interruption recording, worker execution or source delivery. It keeps the preview available and never selects an API or another account route.

Only `claude_runtime.rs` can construct `ClaudeGenerationRuntime`. The operation pipeline requires this type rather than an observed `ClaudeReviewRuntime`. Admission retains the same native runtime Arc; the existing operation lease rechecks it after admission, owns revocation/cancellation and guards consumption and publication. A connection replacement between lookup and execution therefore cannot authorize the old request.

## Release-owned profile

A profile must match all of the following native observations and build values:

| Field group | Required meaning |
| --- | --- |
| `id`, `appVersion`, `adapterSha256` | Unique release profile and exact native source/configuration/dependency fingerprint for this app version |
| `platform`, `architecture` | Qualified macOS architecture; release OS support and bundle minimum must be established in the evidence |
| `runtimeVersion`, `cliSha256` | Exact candidate CLI version and executable bytes |
| `nodeVersion`, `nodeSha256` | Exact host used by managed catalog discovery, observed natively |
| `sdkVersion`, `sdkSha256` | Inspected SDK version and bytes; currently SDK 0.3.300 paired with CLI 2.1.300 |
| `accountRoute`, `subscriptionTypes` | Existing native Claude account route and explicit eligible reported plan types; missing/unrecognized plans cannot match |
| Three `*EvidenceSha256` fields | References to reviewed account/product eligibility, runtime boundary/lifecycle evidence and required operation acceptance artifacts |

The registry rejects unknown fields, malformed hashes, duplicate IDs, invalid plan scopes, excessive entries and incompatible versions. Multiple matching profiles are ambiguous and cannot admit a runtime. It is compiled into the app; local preferences, environment switches, connection IDs and WebView data cannot add profiles. No development bypass is provided.

Evidence hashes are references, not proof. A syntactically valid record cannot establish that its reports are true, complete or applicable. Before publishing any profile, the release owner must inspect those artifacts against the complete requirements below. The goal/completion audit must inspect the actual evidence, not treat this registry as evidence of its own correctness.

## Native implementation fingerprint

`build.rs` hashes sorted relative input paths and length-framed bytes from native Rust sources, resources, capabilities, build script, Cargo manifest/lockfile and Tauri configuration. It sets an app-specific compile-time constant. The profile registry itself is excluded to avoid a self-referential receipt. Native changes invalidate old profiles; editing documentation or adding the matching evidence record does not alter that fingerprint. Source inputs must be regular files/directories. No project repository is scanned or agent process started by this build step.

The fingerprint is displayed in the advanced catalog readiness details. It is a reproducible input identity, not a binary signature, immutable-execution guarantee, human review or runtime test result. Qualification still needs a clean controlled build, exact toolchain/platform/package identity, artifact integrity and behavior evidence. The new build path has not been executed or validated in this pass.

## Evidence required before adding any profile

1. Resolve the exact product/account/distribution permission, including eligible plan scope. The [dated official-source evaluation](research/2026-10-15-agent-backends.md) currently leaves that unresolved. Do not infer permission from a logged-in CLI, metadata response or another application's integration.
2. Package or establish trusted provenance for the exact CLI, SDK and host. Record licenses, architecture, supported macOS range, bundle minimum, runtime installation/updates and executable replacement behavior. Source hashing alone is insufficient.
3. Exercise effective model/effort/approval/turn settings, empty tools/MCP/settings sources, hooks/session controls, managed policy and fallback behavior. Discovery and generation request the documented `--safe-mode`, not `--safe` or `--bare`. The discovery worker now checks the inspected SDK's complete launch-argument contract before starting its child, including explicit empty MCP configuration. This is unexecuted source behavior. Safe mode preserves authentication, but managed hooks, status-line commands and file-suggestion commands can still apply; local `disableAllHooks` does not override managed policy. Initialization or later event rejection cannot prove those commands never ran. `--bare` cannot replace these controls for the account route: it skips subscription credentials and the system keychain. Prove reviewed-context scope, command/write/tool denial and the provider-control versus unrelated-egress boundary with the real pinned integration or its qualified OS boundary. See the [CLI flags](https://code.claude.com/docs/en/cli-reference) and [bare-mode authentication contract](https://code.claude.com/docs/en/headless#start-faster-with-bare-mode).
4. Exercise version/account pre/post inspection, raw IO/frame limits, one-run budgets, cancellation/interrupt/escalation, shutdown, pipe EOFs, descendants, reaping, private-storage cleanup and cleanup-failure latching. Prove disconnect/refresh/owner-drop handling through output publication and renderer lifetime.
5. Complete every required Claude operation in the [coverage matrix](AGENT_OPERATION_PARITY.md), with local validators, appropriate unaccepted/transient result handling, fresh review/consent, no API prerequisite/fallback and exact interruption recovery. A registered command, local preview or synthetic dispatch is insufficient.
6. Record the exact source commit, native fingerprint, package/runtime/toolchain versions, real triggers/outcomes and evidence hashes. Inspect reports against every task requirement. Add a release profile only after those requirements actually pass; never fill placeholder hashes to enable a development route.

Chapter/guide validation now prepares candidates outside final publication ownership, uses nonblocking result-store publishers, and rechecks wanted result admission before any successful notice cleanup. Failed/rejected results retain their original recovery scope. See [the shared source lifecycle](AGENT_RESULT_LIFECYCLE.md); these controls have not been exercised.

The registry remains empty while these requirements are unresolved. App generation controls and inference lifetime/late-reply handling now have source wiring but no runtime acceptance. Exact Claude interruption recovery UI now has source wiring; native acceptance remains pending. The deferred acceptance task `xymb`, Claude task `8gqw` and agent release gate `2r6n` remain open. No tests, builds, app launches, agent/SDK/CLI probes, logins or model requests ran for this source change.
