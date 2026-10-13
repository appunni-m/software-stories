# AI backend capability matrix

**Reviewed:** 2026-10-14

**Purpose:** Keep direct model APIs, multi-step coding-agent runtimes, and speech synthesis as separate choices. This matrix distinguishes routes present in source from routes that have passed the full acceptance contract.

**Implementation snapshot (2026-10-14):** `rig-core = 0.44.0` is pinned. Gemini, OpenAI Responses, and Anthropic Messages can be explicitly selected for file chapters, optional repository orientation, selective section regeneration, and PR Guides. Their local review binds provider, model, sanitized endpoint, encoded JSON body/hash, selected context, and disclosure version. A guarded Reqwest transport checks that the method, route, authentication placeholder, content type, and body match review before using the provider-specific sensitive auth header; requests require HTTPS and are unary. New offline Rust fixtures verify selected-provider review routes and encoded bodies for all three adapters across those four implemented operations. They also prove zero mock dispatch before complete consent, exact model/body dispatch afterward, and one-dispatch quota failure behavior for file chapters and orientation. These are fixture proofs, not accepted provider support: live eligibility, the production HTTP transport, full per-operation capability behavior, and cancellation/connection-close evidence remain open. The Code Lens action currently reuses file-chapter generation; a distinct range-level explanation and a separate PR change-explanation operation are not implemented.

## Status legend

- **Implemented · fixture gate open:** a selectable route exists in source for this operation/provider, but required mocked routing and capability evidence is incomplete, so it is not marked accepted support.
- **Not implemented as a distinct operation:** no operation-specific route exists, even if another workflow can be launched from the same screen.
- **Candidate · unverified:** evaluate this backend; no app-supported route exists yet.
- **Unavailable until verified:** do not show as selectable for that operation until the exact version, auth/terms, capability boundary, and fixture acceptance are recorded.
- **Supported:** may be used only after the corresponding acceptance evidence is linked here. No cell currently has enough evidence to carry this status under the full matrix contract.

## Direct model API operations

One selected API provider handles one complete request. The adapter must bind the provider, model, and endpoint explicitly. The product must not infer a vendor from a model string or silently retry another provider.

| Product operation | Gemini API | OpenAI API | Anthropic API | Reviewed input and validated output |
|---|---|---|---|---|
| File chapter | Implemented · fixture gate open | Implemented · fixture gate open | Implemented · fixture gate open | Exact snapshot file set and cited source ranges → one file's chapter schema. |
| Optional orientation | Implemented · fixture gate open | Implemented · fixture gate open | Implemented · fixture gate open | Exact selected snapshot excerpts → overview/orientation schema; never substitutes for per-file chapters. |
| Selective section regeneration | Implemented · fixture gate open | Implemented · fixture gate open | Implemented · fixture gate open | One accepted immutable chapter revision, selected section, and required dependency anchors → a new draft revision; unaffected sections keep their prior revisions. |
| On-demand Code Lens explanation | Not implemented as a distinct operation | Not implemented as a distinct operation | Not implemented as a distinct operation | The current Code Lens action can launch file-chapter generation; a range-scoped transient explanation with citations is not wired. |
| PR Guide | Implemented · fixture gate open | Implemented · fixture gate open | Implemented · fixture gate open | Reviewed base/head diff and selected bounded text hunks → a separate PR Guide draft with change evidence. |
| PR change explanation | Not implemented as a distinct operation | Not implemented as a distinct operation | Not implemented as a distinct operation | A reviewed file/hunk → one change explanation; does not write or comment on the PR. |

For every cell promoted to **Supported**, the catalog must record the pinned adapter/library and provider/model surface; schema, streaming, cancellation, endpoint, authentication, terms, usage disclosure, and data handling; a mocked fixture for the operation; exact-provider routing; consent fingerprint invalidation; and error behavior. An unsupported feature must be shown before send. No key, prompt, source excerpt, or raw response is written to logs, local metadata, exports, or diagnostics.

The source now routes all three direct APIs through Rig's provider-specific unary adapters and the same exact-review boundary. The app catalog contains structured-output and provider streaming flags, but the current transport explicitly rejects streaming; per-operation streaming/cancellation support is therefore not selectable capability evidence. Rust formatting and a library compile check were completed for this snapshot. They do not prove endpoint behavior, provider eligibility, provider-by-operation mock routing, credential non-disclosure, cancellation closing the HTTP transport, or the unimplemented Code Lens and PR change operations. No live API key or request was used.

### Cross-cutting API behavior to verify per model

| Capability | Required product behavior |
|---|---|
| Structured output | Rig is given the catalogued JSON schema and local chapter/PR validators run afterward. Per-provider, per-operation fixtures remain open. |
| Streaming | Provider catalog flags describe model/provider capability; the app transport currently rejects streaming requests for every route. Current operations use unary responses. |
| Cancellation | Tauri cancellation drops the in-flight local request future and discards late output. The provider may already have accepted work; transport-close behavior still needs a mocked fixture. |
| Exact routing | Review and dispatch bind explicit provider, model, endpoint, request body, and disclosure. Code constructs that provider's Rig adapter and has no fallback path; mocked route assertions remain open. |
| Consent | The request review fingerprints provider/model/endpoint/body/context/disclosure and requires fresh consent after changes. This source-level behavior still needs operation-by-provider fixtures. |
| Failure | Gemini has provider-specific error classification; OpenAI/Anthropic map status classes into shared failures. Per-provider error fixtures and proof that accepted guide history remains untouched are still required. |

## Coding-agent runtime workflows

Coding-agent sessions are distinct from direct API calls. Candidate names do not imply account eligibility, official programmatic access, or a safe read-only boundary.

| Runtime surface | Candidate operations | Status | Gate before exposure |
|---|---|---|---|
| OpenAI Codex app-server / ACP adapter | Repository investigation, chapters/orientation, section update, Code Lens, PR guide/change explanation | Unavailable until verified | The documented app-server has stdio JSONL, `readOnly` sandbox policy, and restricted readable roots; an OAuth token path for ChatGPT plan usage is documented for open-source clients. Still prove command/build tools are unavailable, verify distribution eligibility and app attribution, then run the staged-snapshot fixture. Evaluate ACP separately. |
| Claude Agent (Claude Code-powered; official Agent SDK / ACP wrapper) | Same candidate operation set | Unavailable until verified | Prefer the official SDK if it fits; identify an ACP wrapper by exact registry name and provenance. Use “Claude Agent” branding. API-key/provider auth is documented. Do not assume subscription login is approved: the SDK overview and the newer Help Center currently give conflicting guidance for third-party apps; resolve that before exposing plan-based auth. |
| Gemini CLI ACP mode | Same candidate operation set | Unavailable until verified | Pin and test the exact ACP launch mode/version and auth. `--approval-mode=plan` is documented as under development and not fully functional. Treat it as no security guarantee; prove the boundary with an OS sandbox and a read-only staged snapshot. |
| Google Antigravity SDK | Same candidate operation set | Unavailable until verified | Evaluate the official Python SDK separately; pin SDK/native runtime, supported macOS architecture, account terms, documented controls, lifecycle events, and staged-snapshot boundary. `BuiltinTools.read_only()` is a tool preset, not isolation. Never reuse a consumer Antigravity login; Google's third-party Gemini API-key recommendation does not itself approve embedding Antigravity SDK/ACP/headless surfaces. |
| Google Antigravity ACP binary | Same candidate operation set | Unavailable until verified | Pin and evaluate the exact Google-distributed registry binary separately from the SDK; registry presence does not prove third-party auth/terms eligibility or read-only behavior. |
| Google Antigravity CLI | Same candidate operation set | Unavailable until verified | Evaluate interactive TUI and headless scripting as separate surfaces. Never automate the TUI; headless wrapping is unavailable on consumer plans and requires explicit terms/account eligibility. |

For each workflow/runtime pair, the versioned compatibility record must state **supported** or **unavailable with a reason**. An agent receives only reviewed immutable staged files, no `.git`, live checkout, shell/build execution, unrelated MCP tools, or outside paths. If documented permissions cannot prove that boundary, the runtime stays unavailable. Output is untrusted and must pass local schema and source-anchor validation before the user can accept it. Runtime failure, cancellation, or invalid output never switches to a direct API.

MCP may be evaluated as a shared read-only tool/context protocol only if needed. ACP and MCP are communication protocols; neither is by itself an OS security boundary.

### Reuse decisions and current first-party surfaces (reviewed 2026-10-13)

Prefer a maintained, vendor-supported integration surface when it meets the product contract. Use a shared protocol such as ACP only where the exact runtime/adapter supports the needed lifecycle and permissions. Do not drive a vendor's interactive terminal UI, and do not create a new agent loop merely to normalize providers. Compare release activity, license, reproducible adoption signals, platform support, and capability evidence; stars or protocol participation alone do not establish safety or fit.

| Backend | Existing programmatic surface found | What it means for Code Atlas |
|---|---|---|
| OpenAI Codex | OpenAI publishes the Codex SDK and app-server for application clients. App-server uses stdio JSONL, streams lifecycle events, and documents `readOnly` plus restricted readable roots. OpenAI also documents an OAuth access-token path for ChatGPT plan usage by open-source app-server clients. | The app-server is the first surface to prototype; ACP remains a separate adapter candidate. Keep it distinct from the OpenAI direct model API. Verify OAuth registration/attribution and distribution eligibility, then prove no shell/build execution and no reads outside the staged snapshot; `readOnly` alone does not demonstrate those product-specific requirements. |
| Anthropic Claude Code | Anthropic publishes the Claude Agent SDK for Python and TypeScript. The SDK starts the Claude Code runtime and includes built-in file, command, web, and MCP capabilities. | Prefer the official SDK over its CLI or ACP wrapper if it fits. In Code Atlas UI, use “Claude Agent” (preferred) or “Claude” in an Agents menu; do not label or brand this integration “Claude Code.” API-key and documented cloud-provider authentication are available. The SDK overview says third-party apps may not offer `claude.ai` login or subscription limits unless approved, while the Help Center updated 2026-10-07 says third-party apps can use plan limits and Max/Team include monthly API credits. Treat subscription-based app auth as unresolved until Anthropic reconciles or approves it; do not silently pick either interpretation. |
| Google Antigravity | Google publishes an official Python SDK (`google-antigravity`) with a local runtime, tool controls, hooks, sessions, and subagents. Its `BuiltinTools.read_only()` preset omits write/command tools from the built-in tool set. The CLI has an interactive TUI and a separate headless mode. | Evaluate SDK and registry-listed ACP binary separately; do not automate the TUI. The preset does not isolate filesystem paths or establish terms eligibility. Pin the runtime, verify supported macOS architectures and bundle cost, and test tool removal, OS path/process isolation, and denied actions. |
| Google Antigravity auth/terms | Google documents SDK auth using Gemini API keys or Gemini Enterprise/Vertex credentials. Its FAQ says consumer Antigravity login must not be used by third-party software, recommends Gemini Enterprise or Google AI Studio API keys for third-party coding agents using Gemini, and says wrapping `agy -p` in third-party applications is unsupported on consumer plans. | Never reuse Antigravity login/session credentials. Google's recommendation for a third-party agent using Gemini is not a blanket approval to embed the Antigravity SDK, ACP binary, or headless CLI. Verify terms for the exact SDK/account combination. Keep it distinct from the separately supported Gemini direct API option. If no permitted Antigravity runtime route is verified, M7 remains incomplete. |

### ACP registry snapshot

The registry is a useful distribution index, not a safety, terms, or product-compatibility certification. The exact registry entries below were checked on 2026-10-13; pin a version and re-check the registry before implementation.

| Registry entry | Snapshot | Provenance and decision |
|---|---|---|
| Codex | `codex-acp` 2.2.2, `@agentclientprotocol/codex-acp`, Apache-2.0 | Registry lists OpenAI, JetBrains, and Zed authors. This is an adapter, not the Codex SDK/app-server. Evaluate it against the official SDK/app-server; use only if its lifecycle, auth, terms, and permission boundary are better verified for this product. |
| Claude Agent | `claude-acp` 0.89.1, `@agentclientprotocol/claude-agent-acp`, proprietary | Registry labels it a wrapper and lists Anthropic, JetBrains, and Zed authors. Evaluate the exact wrapper separately from the official Agent SDK; use the required “Claude Agent” UI label and approved API-key auth. |
| Gemini CLI | `gemini` 0.63.0, `@google/gemini-cli@0.63.0 --acp`, Apache-2.0 | Registry identifies Google's official CLI. Pin the CLI and ACP mode, then verify events, permissions, auth, and cancellation; its ACP support alone does not establish the required read-only boundary. |
| Google Antigravity | `antigravity-acp` 1.3.0, Google-authored proprietary binary distributed from `dl.google.com` | Registry identifies a Google binary but gives no proof that its auth mode is allowed for a third-party host app. Evaluate this exact adapter against the official SDK and current auth/terms rules; do not use consumer Antigravity login. |

At this milestone's completion, Code Atlas must have a **selectable, tested route** for Codex; Claude Agent built on Claude Code; and Google Antigravity through a permitted integration/auth path. If a named option cannot pass its terms, isolation, lifecycle, or fixture gate, keep M7 open and explain why; do not count an unavailable entry or another vendor's API as satisfying the requested backend. Gemini CLI remains an additional candidate. Sources: [ACP Registry](https://agentclientprotocol.com/get-started/registry), [versioned registry feed](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json), and linked official surfaces below.

Sources: [Codex SDK](https://developers.openai.com/codex/codex-sdk), [Codex app-server and sandbox controls](https://developers.openai.com/codex/app-server), [Codex app-server with Sign in with ChatGPT for open-source clients](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server), [Claude Agent SDK overview and branding/auth requirements](https://code.claude.com/docs/en/agent-sdk/overview), [Claude Agent SDK permissions](https://code.claude.com/docs/en/agent-sdk/permissions), [Claude plan usage for the Agent SDK](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan), [Gemini CLI configuration](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/configuration.md), [Gemini CLI sandboxing](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/sandbox.md), [Antigravity SDK overview](https://www.antigravity.google/docs/sdk/overview), [Antigravity SDK tools](https://www.antigravity.google/docs/sdk/tools), [Antigravity CLI headless mode](https://www.antigravity.google/docs/cli/headless/), [Antigravity FAQ](https://www.antigravity.google/docs/faq/), [Antigravity Additional Terms](https://www.antigravity.google/terms), and [Antigravity SDK releases](https://github.com/google-antigravity/antigravity-sdk-python/blob/main/google/antigravity/CHANGELOG.md). These are candidate-surface findings; none promotes a runtime to Supported.

## Speech is a separate modality

TTS remains under [t18 optional narration](../kata-tasks/18-optional-narration.md). The speech catalog has its own provider/model capability record, request review, voice/language limits, key/terms/usage disclosure, cancellation, local cache, and audio validation. A text adapter's support for Gemini/OpenAI/Anthropic does not imply that it can synthesize speech.

## Implementation references

- [Direct API client comparison and prototype recommendation](2026-10-13-rust-model-client-comparison.md)
- [Direct model API provider task](../kata-tasks/28-model-api-providers.md)
- [Coding-agent runtime task](../kata-tasks/29-agent-runtime-backends.md)
- [Speech synthesis task](../kata-tasks/18-optional-narration.md)
