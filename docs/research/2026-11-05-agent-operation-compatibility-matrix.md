# Coding-agent operation compatibility matrix — 2026-11-05

**Evidence revision:** repository `4a25a80` on `main`. This records source and vendor evidence available on 2026-11-05; it is a qualification ledger, not a promise that an agent is available in the app. The operation contract lives in [agent operation parity](../AGENT_OPERATION_PARITY.md). The [2026-11-03 ACP snapshot](https://raw.githubusercontent.com/agentclientprotocol/registry/main/.protocol-matrix/latest.md) is a candidate-protocol probe, not a Code Atlas acceptance run.

## Status key

- **Baseline** — one bounded native synthetic case completed; no broader compatibility claim.
- **Source** — app dispatch and local validation exist, but that operation has not passed runtime acceptance.
- **Gated source** — candidate dispatch/review code exists, but an empty release qualification gate keeps the operation unavailable.
- **Unavailable** — no Code Atlas automatic adapter for this runtime/operation.
- **Recovery only** — an old reviewed request may be recovered; this is not a new operation and does not count as parity.

ACP `initialize` success, advertised session capabilities, a model catalog, offline review, and source code alone never upgrade a status to Baseline or Supported.

## Runtime evidence

### Codex — selected integration

- **Integration:** the installed official Codex executable, started by Code Atlas as `codex app-server --listen stdio://`. This is the direct OpenAI app-server interface. Code Atlas does not use ACP; the registered `codex-acp` wrapper starts the same app-server and adds an adapter layer without improving this product's staging boundary.
- **Version:** Code Atlas records the executable digest and version returned by app-server at connection/review time; the app does not pin one global CLI version. The last packaged synthetic file-chapter acceptance recorded Codex `0.162.0-alpha.17.2`, model `gpt-6-luna`, effort `max` (2026-10-16). Requalify against the version observed by the target build.
- **Authentication and usage:** uses the user's existing ChatGPT account through Codex; no API key or credential copying. The app's review says that reviewed source is sent through the account and uses its plan quota. User consent is per generation. The fixed agent approval policy is **Never**; it is separate from permission to share the reviewed source.
- **Lifecycle:** source implements streamed progress, a stop action, and local recovery that requires a fresh review and consent. Recovery does not resume a provider session or reuse old consent. Permission requests cannot expand the fixed read-only policy.
- **Source boundary:** app-server configuration disables shell, browser/web, computer use, plugins, host skills, inherited MCP servers, writes, and model-tool network access. Repository investigation receives only app-owned retrieval tools over the captured snapshot; fixed-source operations receive the reviewed captured files/ranges. This is source configuration evidence, not an adversarial or destination-restricted egress qualification. The normal documented Codex account-service connection remains necessary.
- **Maintenance evidence:** OpenAI documents app-server as the integration interface for rich clients, including streamed events, authentication, conversation history, and approvals. The upstream ACP registry snapshot lists `codex-acp` 2.5.0, but it is a wrapper around app-server and not the selected integration.

### Claude Code / Claude Agent — deferred, not qualified

- **Integration in source:** candidate official Claude Code programmatic CLI, with native process supervision and a candidate SDK model-catalog helper. The source constants are Claude Code `2.1.300` and `@anthropic-ai/claude-agent-sdk` `0.3.300`; they are candidate versions, not versions observed in a packaged run. The ACP registry separately lists `claude-acp` 0.100.0; Code Atlas does not select that adapter.
- **Account and terms:** Anthropic documents Claude Code access for Pro, Max, and Ultra subscribers using their Claude credentials; usage limits are shared with Claude. This establishes a documented vendor CLI/account route, not Code Atlas account, staging, or embedding qualification. The user deferred Claude until Codex work is complete.
- **Lifecycle and permissions:** source contains progress, cancellation, managed child cleanup, fixed `dontAsk` policy, a bounded turn limit, and review/result validation. It also contains code for the required chapter, section, range, repository-guide, and Git-change pipelines. The native release qualification registry is empty and rejects every inference attempt. Those source paths are unavailable in the app until qualified.
- **Maintenance evidence:** Anthropic documents noninteractive `claude -p` and account login. ACP's snapshot reports `claude-acp` 0.100.0 with `loadSession`, list, fork, and resume capabilities. Neither documentation nor ACP probes prove safe packaging, account entitlement for this embedding, or process/tool isolation.

### Gemini CLI — candidate only

- **Integration evidence:** Google's Gemini CLI supports ACP mode. The 2026-11-03 ACP snapshot reports `gemini` 0.66.0, distributed by npm, with successful `initialize` and `loadSession` advertised. Code Atlas has no Gemini CLI adapter or operation dispatcher.
- **Account, terms, lifecycle and permissions:** not qualified for Code Atlas. The registry probe does not establish the user's applicable account route, exact model/effort forwarding, bounded staged-file access, provider-only network access, cancellation cleanup, or resume correctness. No repository context has been sent through this candidate.
- **Maintenance evidence:** the official Gemini CLI project publishes ACP-mode documentation and the registry tracks the published runtime. This is evidence of an available candidate interface, not app compatibility.

### Google Antigravity — consumer login unavailable to third-party embedding

- **ACP candidate:** the 2026-11-03 registry snapshot reports Google's `antigravity-acp` 1.4.5 binary with successful `initialize` and load/list/resume capabilities. Code Atlas has no adapter. Google states third-party software accessing Antigravity through its consumer login violates its terms and can result in account suspension or termination; wrapping the CLI in a third-party app is not supported under consumer plans. Do not offer that route.
- **Separate official route:** Google's SDK/CLI has separate documented authentication routes such as Gemini API key or Gemini Enterprise/ADC. Those would be a distinct integration and billing/terms choice, not reuse of an Antigravity consumer session. The SDK exposes tool filters and policies, but its built-in tools include filesystem, shell, web, and write capabilities that would need independent restriction and tests. Code Atlas has not integrated or qualified this route.
- **Maintenance evidence:** Google maintains the SDK/CLI documentation and ACP publishes the candidate binary/version. Neither fact grants consumer-account eligibility or proves Code Atlas isolation.

## Operation coverage

| Operation | Codex direct app-server | Claude Code CLI | Gemini CLI ACP | Antigravity |
| --- | --- | --- | --- | --- |
| Plan repository coverage and reading order | **Source** — bounded native planning; repository-scale orchestration, selective refresh and acceptance remain open | **Gated source** — cannot dispatch while release qualification is empty | **Unavailable** — no adapter | **Unavailable** — no permitted consumer-login adapter; SDK route absent |
| Investigate a repository question and stream a progressive Guide | **Source** — dispatcher exists; a reported broad Fontdone Guide exceeded ten minutes and returned no accepted answer, so whole-repository behavior is not qualified | **Gated source** — cannot dispatch while release qualification is empty | **Unavailable** | **Unavailable** |
| Generate a file chapter | **Baseline** — one synthetic packaged native case passed with Codex `0.162.0-alpha.17.2` / GPT-6 Luna max; not a language, quality, or production claim | **Gated source** — candidate pipeline only | **Unavailable** | **Unavailable** |
| Regenerate one stale section while preserving eligible neighboring sections | **Source** — implementation exists; fresh account-backed repair and preservation acceptance remain open | **Gated source** — candidate pipeline only | **Unavailable** | **Unavailable** |
| Explain a selected Code Lens range | **Source** — implementation exists; runtime, exact-scope, and citation acceptance remain open | **Gated source** — candidate pipeline only | **Unavailable** | **Unavailable** |
| Explain a selected Git comparison as a PR Guide | **Source** — implementation exists; comparison-scope recovery and packaged acceptance remain open | **Gated source** — candidate pipeline only | **Unavailable** | **Unavailable** |
| Explain one selected Git diff hunk | **Source** — implementation exists; transient-result, citation, and packaged acceptance remain open | **Gated source** — candidate pipeline only | **Unavailable** | **Unavailable** |

The older selected-source overview is **Recovery only** for an already reviewed request. It is not a substitute for the progressive repository Guide and does not count as another operation. “Gated source” describes implementation present behind a native release gate; it does not mean Claude is selectable or available.

## Acceptance ledger

- **Passed, narrow:** the 2026-10-16 packaged synthetic Codex file-chapter flow reviewed and sent one synthetic file, saved and accepted a draft, opened a verified citation, and advanced between sections manually. Exact evidence and limitations are in [Codex validation](../acceptance/2026-10-16-codex-validation.md).
- **Failed, repository scale:** a broad Fontdone Guide was reported to run past ten minutes, exclude important large files, and publish no accepted answer. The later incremental strategy remains an acceptance requirement; there is no complete-repository Guide pass recorded here.
- **Not accepted:** section repair, Code Lens range, repository plan, PR Guide, diff hunk, cancellation/quota/recovery edge cases, app-server identity races, and destination-restricted egress.
- **Not run:** Claude generation, Gemini CLI, and Antigravity runtime. No candidate runtime was installed or launched and no repository source was sent to them for this review.

## What closes a matrix cell

For each pinned app/runtime version and operation, attach a dated packaged-app record proving the selected account route and user-facing usage disclosure; exact reviewed immutable inputs; model/reasoning forwarding; progress and cancellation/child cleanup; fresh-consent recovery behavior; read-only access to staging with outside-path/write/command/web/network denials; independent result and citation validation; and an accepted, reviewable output. Record unsupported lifecycle capabilities explicitly. Keep any cell unavailable when the documented account/terms route is not eligible. Do not infer support from an ACP handshake, source path, or another operation's test.

No source code was changed, no agent was launched, no account was used, and no tests or builds ran for this matrix update.

## Primary references

- [OpenAI Codex app-server](https://developers.openai.com/codex/app-server) and [Codex as a platform](https://developers.openai.com/blog/codex-as-a-platform)
- [ACP registry snapshot generated 2026-11-03](https://raw.githubusercontent.com/agentclientprotocol/registry/main/.protocol-matrix/latest.md), [Codex ACP adapter](https://github.com/agentclientprotocol/codex-acp), and [Claude ACP adapter](https://github.com/agentclientprotocol/claude-agent-acp)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-usage) and [Claude Pro/Max/Ultra access](https://support.claude.com/en/articles/11145838-use-claude-code-with-your-pro-max-or-ultra-plan)
- [Gemini CLI ACP mode](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md)
- [Antigravity account-use FAQ](https://antigravity.google/docs/faq), [SDK tools](https://antigravity.google/docs/sdk/tools), [SDK policies](https://antigravity.google/docs/sdk/policies/), and [authentication](https://antigravity.google/docs/install)
