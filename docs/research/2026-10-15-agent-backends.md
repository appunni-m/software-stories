# Coding-agent backends: practical implementation decision

Reviewed October 15, 2026 from current official documentation and repository source. No agent runtime was launched, no login was initiated and no tests were run. These are implementation recommendations, not accepted app capabilities.

## Current product gap

Chapter generation exposes direct APIs. The runtime options requested by the user are documented in `tpta`, but there is no usable Codex, Claude Agent or Antigravity chapter adapter in the app. Model API coverage does not fulfill that requirement. Agent work no longer waits for completion of the direct API backlog.

Provide two visible backend categories: **Coding agent** and **Direct API**. A connected agent stores its supported non-secret defaults: model, reasoning effort, approval behavior and run limits. Each run shows the effective settings and reviewed source scope. Unsupported settings are unavailable, never silently ignored. Every backend returns the same validated chapter draft to the existing revision workflow. Pocket TTS stays a separate local narration component.

| Backend | Recommended integration | Account usage | Next task |
| --- | --- | --- | --- |
| Codex | Official app-server over Rust-managed stdio | Documented ChatGPT plan route for eligible open-source applications; no direct API key needed for that route | `z3an`: implement first |
| Claude Agent, powered by Claude Code | Official Agent SDK worker; documented headless subprocess is an alternative | Subscription usage is documented, but current third-party product-login guidance conflicts between official pages | `8gqw`: implement adapter and resolve distribution/auth eligibility |
| Antigravity | Official ACP for account-route evaluation; official Python SDK for explicitly selected API/Cloud route | Personal-account editor integrations exist; custom app eligibility is unconfirmed. Consumer headless wrapping is explicitly unsupported | `m00g`: qualify custom ACP eligibility; do not promise subscription support yet |

## Codex: clearest first implementation

OpenAI documents Sign in with ChatGPT for eligible open-source applications, including eligible Plus/Pro usage. Public-client registration uses PKCE without a client secret or partner API key. Confirm this project's distribution/license eligibility, implement the supported native OAuth flow and store credentials in the host. Plan quotas still apply. [Quickstart](https://developers.openai.com/siwc/quickstart), [open-source registration](https://developers.openai.com/siwc/token-sharing-open-source/sign-in).

Use one app-server child process over stdio, with app attribution and the documented account-backed Responses configuration. The host manages token refresh and process restart/resume. Obtain account-entitled models through the account API; a bundled runtime model list alone is not entitlement evidence. [Account-backed app-server](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server), [models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference).

The protocol exposes model/effort, approval/sandbox policies, structured output, progress events, interruption and thread resume. Pin a version supporting restricted readable roots; the older installed-version investigation is historical evidence, not a current capability limit. Disable command execution separately from read-only filesystem access, plus web and unrelated tools/configuration. Begin with bounded reviewed evidence and add app-owned search/read tools when needed. No ACP wrapper or additional Node server is required. [App-server](https://developers.openai.com/codex/app-server), [configuration](https://learn.chatgpt.com/docs/config-file/config-basic).

Account-route preview restrictions include unsupported hosted connector/tool-search paths; narration is outside this integration. Keep those controls unavailable and handle quota errors without switching to paid API credentials. [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations).

## Claude Agent: feasible, with an unresolved account-policy conflict

The official SDK embeds the Claude Code agent loop and supports a structured chapter result, streamed progress and interruption. Use the supported SDK or documented `claude -p` subprocess; do not write another agent loop. The SDK's structured output still needs independent local citation validation. [SDK overview](https://code.claude.com/docs/en/agent-sdk/overview), [structured output](https://code.claude.com/docs/en/agent-sdk/structured-outputs), [programmatic CLI](https://code.claude.com/docs/en/headless).

The Help Center updated October 7 explicitly describes subscription usage for SDK, headless and third-party applications. Another official page distinguishes account usage from API-key usage and credits. However, the SDK overview still requires prior approval for third-party products offering claude.ai login/subscription limits. Neither page establishes that it supersedes the other. Subscription operation is a documented technical candidate; obtain a clear answer for this product's distribution/login path before advertising approved support. [Plan usage](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan), [credits](https://support.claude.com/en/articles/17154008-monthly-api-credits-for-max-and-team-plans), [SDK requirements](https://code.claude.com/docs/en/agent-sdk/overview).

Keep account mode explicit: an inherited `ANTHROPIC_API_KEY` can select API billing in noninteractive operation. Isolate settings and credentials without extracting tokens. Expose only model-supported effort and permission options. Tool auto-approval lists do not remove other tools; configure the actual tool set, strict app-owned MCP and per-call scope checks. Cancel with the supported SDK interruption then bounded process cleanup. [Authentication](https://code.claude.com/docs/en/authentication), [permissions](https://code.claude.com/docs/en/agent-sdk/permissions), [Python SDK](https://code.claude.com/docs/en/agent-sdk/python), [effort and limits](https://code.claude.com/docs/en/agent-sdk/agent-loop).

## Antigravity: separate SDK, ACP and CLI decisions

Google's official Python SDK supports agent tools, policies, structured output, streaming and cancellation, with API-key or Cloud credentials. That is a viable custom-app surface but does not meet the user's preferred subscription-login route by itself. Do not invent a reasoning-effort field where its SDK contract has not been established. [SDK overview](https://antigravity.google/docs/sdk/overview/), [tools](https://antigravity.google/docs/sdk/tools/), [policies](https://antigravity.google/docs/sdk/policies/), [lifecycle](https://antigravity.google/docs/sdk/lifecycle/).

Google also distributes an official ACP binary. Documented Zed and Xcode integrations support personal Google accounts, including plan usage. Those editor instructions do not establish eligibility for an independent Code Atlas ACP client. Qualify that route directly; do not describe all account integration as impossible. [Official registry entry](https://raw.githubusercontent.com/agentclientprotocol/registry/main/antigravity-acp/agent.json), [Zed](https://antigravity.google/docs/ide/extensions/zed), [Xcode](https://antigravity.google/docs/ide/extensions/xcode).

The current FAQ expressly excludes consumer-plan headless wrapping inside third-party applications. Headless streaming is also not an interactive approval bridge: documented control-request responses are unsupported, workspace reads/writes are allowed automatically, and EOF waits for the current turn. Use the appropriate supported SDK/ACP surface, not a wrapped terminal UI. Antigravity and Gemini CLI are distinct products and adapters. [FAQ](https://antigravity.google/docs/faq/), [headless contract](https://antigravity.google/docs/cli/headless/).

## Delivery order

1. Implement Codex connection, effective model/reasoning settings and one complete file-chapter operation.
2. Carry it through progress/cancel, schema/citation validation, saved draft and explicit acceptance. A provider dropdown is insufficient.
3. Reuse the same operation contract for section regeneration, Code Lens and PR explanations; keep each operation's reviewed source scope.
4. Add Claude and Antigravity through their independently qualified auth/runtime paths. Retain direct APIs as optional alternatives, never automatic fallbacks.

Runtime controls must be checked on the pinned packaged integration when validation is authorized. Start from documented runtime capabilities; do not require a VM merely because an earlier generic isolation experiment failed.
