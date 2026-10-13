# AI backend capability matrix

**Reviewed:** 2026-10-13  
**Purpose:** Keep direct model APIs, multi-step coding-agent runtimes, and speech synthesis as separate choices. This matrix distinguishes routes present in source from routes that have passed the full acceptance contract.

**Implementation snapshot (2026-10-13):** `rig-core = 0.44.0` is pinned. Gemini, OpenAI Responses, and Anthropic Messages can be explicitly selected for file chapters, optional repository orientation, selective section regeneration, and PR Guides. Their local review binds provider, model, sanitized endpoint, encoded JSON body/hash, selected context, and disclosure version. A guarded Reqwest transport checks that the method, route, authentication placeholder, content type, and body match review before using the provider-specific sensitive auth header; requests require HTTPS and are unary. These are implemented routes, not accepted support: provider-by-operation fixtures, capability evidence, and cancellation/transport-close evidence remain open. The Code Lens action currently reuses file-chapter generation; a distinct range-level explanation and a separate PR change-explanation operation are not implemented.

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
| OpenAI Codex programmatic surface / ACP adapter | Repository investigation, chapters/orientation, section update, Code Lens, PR guide/change explanation | Unavailable until verified | Record exact supported SDK/app-server/headless or ACP adapter/version, provenance, auth/terms, events, usage behavior, and isolated read-only fixture run. |
| Claude Agent (Claude Code-powered; official Agent SDK / ACP wrapper) | Same candidate operation set | Unavailable until verified | Prefer the official SDK if it fits; identify an ACP wrapper by its exact registry name and provenance. Use allowed Anthropic product branding and API-key auth unless separately approved. |
| Gemini CLI ACP mode | Same candidate operation set | Unavailable until verified | Pin and test the exact ACP launch mode/version, auth and terms; verify filesystem/process/network restrictions independently of Plan mode. |
| Google Antigravity SDK | Same candidate operation set | Unavailable until verified | Evaluate the official Python SDK separately; pin SDK and native runtime, supported macOS architecture, account terms, documented controls, lifecycle events, and staged-snapshot boundary. Never reuse a consumer Antigravity login; API-key or Enterprise auth must be permitted for embedding. |
| Google Antigravity ACP binary | Same candidate operation set | Unavailable until verified | Pin and evaluate the exact Google-distributed registry binary separately from the SDK; registry presence does not prove third-party auth/terms eligibility or read-only behavior. |
| Google Antigravity CLI | Same candidate operation set | Unavailable until verified | Evaluate interactive TUI and headless scripting as separate surfaces. Never automate the TUI; headless wrapping is unavailable on consumer plans and requires explicit terms/account eligibility. |

For each workflow/runtime pair, the versioned compatibility record must state **supported** or **unavailable with a reason**. An agent receives only reviewed immutable staged files, no `.git`, live checkout, shell/build execution, unrelated MCP tools, or outside paths. If documented permissions cannot prove that boundary, the runtime stays unavailable. Output is untrusted and must pass local schema and source-anchor validation before the user can accept it. Runtime failure, cancellation, or invalid output never switches to a direct API.

MCP may be evaluated as a shared read-only tool/context protocol only if needed. ACP and MCP are communication protocols; neither is by itself an OS security boundary.

### Reuse decisions and current first-party surfaces (reviewed 2026-10-13)

Prefer a maintained, vendor-supported integration surface when it meets the product contract. Use a shared protocol such as ACP only where the exact runtime/adapter supports the needed lifecycle and permissions. Do not drive a vendor's interactive terminal UI, and do not create a new agent loop merely to normalize providers. Compare release activity, license, reproducible adoption signals, platform support, and capability evidence; stars or protocol participation alone do not establish safety or fit.

| Backend | Existing programmatic surface found | What it means for Code Atlas |
|---|---|---|
| OpenAI Codex | OpenAI publishes the Codex SDK for integrating Codex in an application and the app-server for custom clients needing auth, history, approvals, streaming, and sandbox controls. | Evaluate the official SDK/app-server before wrapping a CLI or ACP adapter. Keep this distinct from the OpenAI direct model API. Verify the user's plan/API path, app attribution, sandbox roots, and process controls for the exact version. |
| Anthropic Claude Code | Anthropic publishes the Claude Agent SDK for building an app around Claude Code's agent loop. | Prefer the official SDK over its CLI or ACP wrapper if it fits. In Code Atlas UI, use “Claude Agent” (preferred) or “Claude” in an Agents menu; do not label or brand this integration “Claude Code.” Use API-key auth unless Anthropic has approved another path; observe Anthropic's commercial terms. |
| Google Antigravity | Google publishes an official Python SDK (`google-antigravity`) with a local runtime, tool controls, hooks, sessions, and subagents. The Antigravity CLI has an interactive TUI and a separate headless mode. | Evaluate the SDK and the registry-listed ACP binary separately; do not automate the TUI. The SDK exposes filesystem, command, and web tools, so a read-only preset is only a starting point. Pin the runtime, verify supported macOS architectures and bundle cost, and test tool removal, path isolation, and denied actions. |
| Google Antigravity auth/terms | The SDK documents Gemini API-key and Gemini Enterprise/Vertex auth. Google says third-party software must not use a consumer Antigravity login and recommends Gemini Enterprise or Google AI Studio API keys for third-party agents; wrapping `agy -p` in third-party applications is unsupported under consumer plans. | Never reuse Antigravity login/session credentials. The API-key or Enterprise SDK route is the candidate; verify its applicable terms and test it. Keep the SDK/agent-runtime option distinct from the separate Gemini direct API option. If no permitted route is verified, M7 remains incomplete rather than counting “unavailable” as support for the requested backend. |

### ACP registry snapshot

The registry is a useful distribution index, not a safety, terms, or product-compatibility certification. The exact registry entries below were checked on 2026-10-13; pin a version and re-check the registry before implementation.

| Registry entry | Snapshot | Provenance and decision |
|---|---|---|
| Codex | `codex-acp` 2.2.2, `@agentclientprotocol/codex-acp`, Apache-2.0 | Registry lists OpenAI, JetBrains, and Zed authors. This is an adapter, not the Codex SDK/app-server. Evaluate it against the official SDK/app-server; use only if its lifecycle, auth, terms, and permission boundary are better verified for this product. |
| Claude Agent | `claude-acp` 0.89.1, `@agentclientprotocol/claude-agent-acp`, proprietary | Registry labels it a wrapper and lists Anthropic, JetBrains, and Zed authors. Evaluate the exact wrapper separately from the official Agent SDK; use the required “Claude Agent” UI label and approved API-key auth. |
| Gemini CLI | `gemini` 0.63.0, `@google/gemini-cli@0.63.0 --acp`, Apache-2.0 | Registry identifies Google's official CLI. Pin the CLI and ACP mode, then verify events, permissions, auth, and cancellation; its ACP support alone does not establish the required read-only boundary. |
| Google Antigravity | `antigravity-acp` 1.3.0, Google-authored proprietary binary distributed from `dl.google.com` | Registry identifies a Google binary but gives no proof that its auth mode is allowed for a third-party host app. Evaluate this exact adapter against the official SDK and current auth/terms rules; do not use consumer Antigravity login. |

At this milestone's completion, Code Atlas must have a **selectable, tested route** for Codex; Claude Agent built on Claude Code; and Google Antigravity through a permitted integration/auth path. If a named option cannot pass its terms, isolation, lifecycle, or fixture gate, keep M7 open and explain why; do not count an unavailable entry or another vendor's API as satisfying the requested backend. Gemini CLI remains an additional candidate. Sources: [ACP Registry](https://agentclientprotocol.com/get-started/registry), [versioned registry feed](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json), and linked official surfaces below.

Sources: [Codex SDK](https://developers.openai.com/codex/codex-sdk), [Codex app-server and sandbox controls](https://developers.openai.com/codex/app-server), [Claude Agent SDK overview and branding/auth requirements](https://code.claude.com/docs/en/agent-sdk/overview), [Antigravity SDK overview](https://www.antigravity.google/docs/sdk/overview), [Antigravity SDK tools](https://www.antigravity.google/docs/sdk/tools), [Antigravity CLI headless mode](https://www.antigravity.google/docs/cli/headless/), [Antigravity FAQ](https://www.antigravity.google/docs/faq/), [Antigravity Additional Terms](https://www.antigravity.google/terms), and [Antigravity SDK releases](https://github.com/google-antigravity/antigravity-sdk-python/blob/main/google/antigravity/CHANGELOG.md). These are candidate-surface findings; none promotes a runtime to Supported.

## Speech is a separate modality

TTS remains under [t18 optional narration](../kata-tasks/18-optional-narration.md). The speech catalog has its own provider/model capability record, request review, voice/language limits, key/terms/usage disclosure, cancellation, local cache, and audio validation. A text adapter's support for Gemini/OpenAI/Anthropic does not imply that it can synthesize speech.

## Implementation references

- [Direct API client comparison and prototype recommendation](2026-10-13-rust-model-client-comparison.md)
- [Direct model API provider task](../kata-tasks/28-model-api-providers.md)
- [Coding-agent runtime task](../kata-tasks/29-agent-runtime-backends.md)
- [Speech synthesis task](../kata-tasks/18-optional-narration.md)
