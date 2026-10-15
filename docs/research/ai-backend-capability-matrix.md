# AI backend capability matrix

**Reviewed:** 2026-10-15

**Latest source delta:** generation now always presents the backend chooser, except explicit direct API settings. Codex is a native source implementation; Claude Code, Codex, Antigravity and other agents also have a separate **manual handoff** transport for file chapters and selected-source overviews. The manual transport copies an exact task and imports task-bound JSON through the native source validator. It has no app-managed authentication, effective runtime settings, permission enforcement, streamed progress or cancellation. Its agent/model provenance is unverified. Do not treat it as an integrated agent operation in the matrix below or as closing any runtime acceptance gate. See [Agent generation](../AGENT_GENERATION.md) and [the required integrated-agent gate](../kata-tasks/45-agent-first-generation.md). No tests or builds ran for this delta.

**Purpose:** Keep direct model APIs, multi-step coding-agent runtimes, and speech synthesis as separate choices. This matrix distinguishes routes present in source from routes that have passed the full acceptance contract.

**Implementation snapshot (2026-10-14):** `rig-core = 0.44.0` is pinned. Gemini, OpenAI Responses, and Anthropic Messages can be explicitly selected for file chapters, optional repository orientation, selective section regeneration, PR Guides, transient single-hunk PR change explanations, and a distinct range-scoped Code Lens explanation. Their local review binds provider, model, sanitized endpoint, encoded JSON body/hash, selected context, and disclosure version. PR change explanations rebuild one selected local file/hunk from immutable base/head refs, display and send only that hunk, return bounded side-aware claims with exact hunk citations, and do not create PR Guide history or alter Git refs. The Code Lens route accepts only a hash-verified inclusive line range and optional exact UTF-8 byte span from the immutable snapshot, caps the excerpt at 32 KB, screens the selected bytes for secret-like content, and returns only bounded claims with exact in-range citations; it does not create a chapter or affect coverage. A guarded Reqwest transport checks that the method, route, authentication placeholder, content type, and body match review before using the provider-specific sensitive auth header; requests require HTTPS and are unary. Offline Rust fixtures cover exact reviewed provider/model/body dispatch, complete-consent gating, changed-review rejection, and one-call propagation of synthetic quota failures for every currently wired operation/provider pair. They also assert byte-for-byte preservation of accepted chapter history across saved-operation failures; transient PR change failures leave Git refs, the Git index, and PR Guide history unchanged. A separate classifier fixture covers provider-specific status categories from constructed Rig error responses. Local loopback tests through the Reqwest/Rig transport verify that Gemini, OpenAI, and Anthropic send the reviewed body with credentials only in the provider auth header, and that cancellation closes a pending request socket. Both transient explanation routes validate returned citations locally and avoid provider fallback. Live-provider eligibility and behavior, production TLS/proxy behavior, provider-response parsing, and whether a provider received or completed a request before cancellation remain open.


## October 15 implementation decision

[The current agent-backend evaluation](2026-10-15-agent-backends.md) selects Codex app-server with the documented account sign-in route as the first implementation (`z3an`), with separate Claude (`8gqw`) and Antigravity (`m00g`) tasks. Codex now has a source-only experimental adapter for file chapters and selected-source overviews; Claude and Antigravity app adapters remain unimplemented. No agent route has passed packaged/live acceptance. Current official account and tool-control findings in that evaluation supersede earlier feasibility assumptions below; earlier local version/VM/XPC observations remain historical evidence only. Agent implementation no longer waits for the direct API backlog.

## Status legend

- **Source implementation · unvalidated:** a selectable development route exists, but no build, tests or runtime acceptance were performed for this change. This is not accepted support.

- **Implemented · capability gate open:** a selectable route exists in source and local mock-dispatch, consent, invalidation, and failure fixtures cover this operation/provider. One or more schema, model, transport, cancellation, terms, or live-acceptance dimensions remain open, so it is not marked accepted support.
- **Not implemented as a distinct operation:** no operation-specific route exists, even if another workflow can be launched from the same screen.
- **Candidate · unverified:** evaluate this backend; no app-supported route exists yet.
- **Unavailable until verified:** do not show as selectable for that operation until the exact version, auth/terms, capability boundary, and fixture acceptance are recorded.
- **Supported:** may be used only after the corresponding acceptance evidence is linked here. No cell currently has enough evidence to carry this status under the full matrix contract.

## Direct model API operations

One selected API provider handles one complete request. The adapter must bind the provider, model, and endpoint explicitly. The product must not infer a vendor from a model string or silently retry another provider.

| Product operation | Gemini API | OpenAI API | Anthropic API | Reviewed input and validated output |
|---|---|---|---|---|
| File chapter | Implemented · capability gate open | Implemented · capability gate open | Implemented · capability gate open | Exact snapshot file set and cited source ranges → one file's chapter schema. |
| Optional orientation | Implemented · capability gate open | Implemented · capability gate open | Implemented · capability gate open | Exact selected snapshot excerpts → overview/orientation schema; never substitutes for per-file chapters. |
| Selective section regeneration | Implemented · capability gate open | Implemented · capability gate open | Implemented · capability gate open | One accepted immutable chapter revision, selected section, and required dependency anchors → a new draft revision; unaffected sections keep their prior revisions. |
| On-demand Code Lens explanation | Implemented · capability gate open | Implemented · capability gate open | Implemented · capability gate open | Exact hash-verified snapshot path, inclusive line range, optional half-open UTF-8 byte range, and reviewed provider/model → transient claims citing unique exact quotes in the selected range. Does not create a chapter or alter coverage. Offline fixtures cover all three providers, reviewed endpoint/body and exact excerpt, consent-gated dispatch, citation anchors and absent-quote rejection, and zero dispatch after provider/model/endpoint mutations. For each selected provider, a mapped mock quota failure is returned after one dispatch without fallback, and accepted chapter history remains byte-for-byte unchanged. Local provider-status classification has a fixture; live provider eligibility and response parsing remain open. |
| PR Guide | Implemented · capability gate open | Implemented · capability gate open | Implemented · capability gate open | Reviewed base/head diff and selected bounded text hunks → a separate PR Guide draft with change evidence. |
| PR change explanation | Implemented · capability gate open | Implemented · capability gate open | Implemented · capability gate open | One reviewed local file/hunk from immutable base/head refs → bounded transient claims with exact side-aware source citations. Sends only that hunk; does not write PR Guide history or Git refs. |

For every cell promoted to **Supported**, the catalog must record the pinned adapter/library and provider/model surface; schema, streaming, cancellation, endpoint, authentication, terms, usage disclosure, and data handling; a mocked fixture for the operation; exact-provider routing; consent fingerprint invalidation; and error behavior. An unsupported feature must be shown before send. No key, prompt, source excerpt, or raw response is written to logs, local metadata, exports, or diagnostics.

The source now routes all three direct APIs through Rig's provider-specific unary adapters and the same exact-review boundary. The app catalog contains structured-output and provider streaming flags, but the current transport explicitly rejects streaming; per-operation streaming support is therefore not selectable capability evidence. Offline mock-dispatch coverage spans every currently wired provider/operation pair. Separate loopback Reqwest/Rig transport fixtures verify the reviewed body and credentials' placement for all three APIs and that dropping a pending request closes its socket. These tests do not exercise vendor eligibility, production TLS/proxy behavior, live response parsing, or whether a provider already received a request before cancellation. A local classifier fixture checks constructed Rig response errors against each provider's status rules; it does not exercise live endpoints or their response parsing. No live API key or request was used.

### Cross-cutting API behavior to verify per model

| Capability | Required product behavior |
|---|---|
| Structured output | Rig is given the catalogued JSON schema and local chapter/PR validators run afterward. Local fixtures cover operation routing; provider/model schema compatibility and live response parsing remain unverified. |
| Streaming | Provider catalog flags describe model/provider capability; the app transport currently rejects streaming requests for every route. Current operations use unary responses. |
| Cancellation | Tauri cancellation drops the in-flight local request future and discards late output. A loopback Reqwest/Rig fixture verifies that dropping a pending request closes its socket. The provider may already have accepted work; the fixture cannot prove whether a vendor received or completed it. |
| Exact routing | Review and dispatch bind explicit provider, model, endpoint, request body, and disclosure. Code constructs that provider's Rig adapter and has no fallback path. Offline fixtures assert reviewed provider/model/body dispatch and complete-consent gating for every currently wired operation/provider pair. Provider/model/endpoint mutations are rejected before mock transport across those pairs; PR change fixtures also revalidate changed refs/hunks against the local comparison. Separate loopback Reqwest/Rig fixtures inspect each API's route, exact reviewed body, and sensitive credential header; they do not exercise vendor endpoints. |
| Consent | The request review fingerprints provider/model/endpoint/body/context/disclosure and requires fresh consent after changes. Each currently wired operation/provider pair rejects incomplete consent before mock transport. Provider/model/endpoint changes also reject dispatch before transport; PR change explanation's changed-hunk/ref check is performed through its local Git revalidation path. |
| Failure | Synthetic quota-failure fixtures return the selected operation/provider failure after exactly one mock transport call across the current direct API matrix, exercising app-level no-retry/no-fallback behavior. They also compare accepted chapter history byte-for-byte across failures for file chapters, orientation, selective updates, PR Guides, Code Lens, and PR change explanations; PR change explanation fixtures additionally verify unchanged Git refs/index and no PR Guide history. Classifier fixtures check constructed Rig response errors: Gemini maps authentication, prepaid-credit, quota/rate, region, and model-not-found statuses; OpenAI separates invalid-key, region, permission, credit/spend, rate-limit, and organization-usage-limit cases; Anthropic distinguishes authentication, billing, permission, rate-limit, and overload statuses. These fixtures check local mapping only; they do not exercise live provider response parsing. Live vendor eligibility/TLS/proxy behavior and whether a vendor completes a request before cancellation remain open; local loopback transport tests cover credential placement and socket closure. |

Provider status classification was reviewed against current first-party references on 2026-10-14: [Gemini API errors](https://ai.google.dev/gemini-api/docs/api-errors), [OpenAI API error codes](https://developers.openai.com/api/docs/guides/error-codes), and [Claude API errors](https://platform.claude.com/docs/en/api/errors). Gemini documents depleted prepay credits as 402 and quota/rate limits as 429; OpenAI assigns distinct 429 codes to credit balance, spend limits, rate limits, and organization usage limits; Anthropic documents 402 billing errors, 429 rate/spend-cap errors, and 529 overload. The fixtures verify local category mapping only, not account eligibility or live service behavior.

## Coding-agent runtime workflows

Coding-agent sessions are distinct from direct API calls. Candidate names do not imply account eligibility, official programmatic access, or a safe read-only boundary.

| Runtime surface | Candidate operations | Status | Gate before exposure |
|---|---|---|---|
| OpenAI Codex app-server | File chapters and selected-source overviews implemented; repository investigation, section update, Code Lens and PR operations pending | Source implementation · unvalidated | Managed ChatGPT login for local development, runtime model/effort catalog, restricted read-only staging, documented command/web feature controls, schema/citation validation and progress/cancel are in source. Confirm pinned compatible runtime, account entitlement, tool boundary, distribution/auth eligibility and packaged behavior. No ACP wrapper is used. |
| Claude Agent (Claude Code-powered; official Agent SDK / ACP wrapper) | Same candidate operation set | Unavailable until verified | Prefer the official SDK if it fits; identify an ACP wrapper by exact registry name and provenance. Use “Claude Agent” branding. API-key/provider auth is documented. Do not assume subscription login is approved: the SDK overview and the newer Help Center currently give conflicting guidance for third-party apps; resolve that before exposing plan-based auth. |
| Gemini CLI ACP mode | Same candidate operation set | Unavailable until verified | Pin and test the exact ACP launch mode/version and auth. `--approval-mode=plan` is documented as under development and not fully functional. Treat it as no security guarantee; prove the boundary with an OS sandbox and a read-only staged snapshot. |
| Google Antigravity SDK | Same candidate operation set | Unavailable until verified | Evaluate the official Python SDK separately; pin SDK/native runtime, supported macOS architecture, account terms, documented controls, lifecycle events, and staged-snapshot boundary. `BuiltinTools.read_only()` is a tool preset, not isolation. Never reuse a consumer Antigravity login; Google's third-party Gemini API-key recommendation does not itself approve embedding Antigravity SDK/ACP/headless surfaces. |
| Google Antigravity ACP binary | Same candidate operation set | Unavailable until verified | Pin and evaluate the exact Google-distributed registry binary separately from the SDK; registry presence does not prove third-party auth/terms eligibility or read-only behavior. |
| Google Antigravity CLI | Same candidate operation set | Unavailable until verified | Evaluate interactive TUI and headless scripting as separate surfaces. Never automate the TUI; headless wrapping is unavailable on consumer plans and requires explicit terms/account eligibility. |

For each released workflow/runtime pair, the versioned compatibility record must state **supported** or **unavailable with a reason**. The current selectable Codex development route is explicitly source-only and has not met that release contract. An agent receives only reviewed immutable staged files, no `.git`, live checkout, shell/build execution, unrelated MCP tools, or outside paths. If documented permissions cannot prove that boundary, the runtime stays unavailable. Output is untrusted and must pass local schema and source-anchor validation before the user can accept it. Runtime failure, cancellation, or invalid output never switches to a direct API.

MCP may be evaluated as a shared read-only tool/context protocol only if needed. ACP and MCP are communication protocols; neither is by itself an OS security boundary.

Provider authentication/model traffic is a separate control path from agent-tool egress. A runtime may use only the selected adapter's documented authentication and model endpoints through an app-managed broker or an OS-enforced, destination-restricted route. Agent-controlled arbitrary destinations, browser/web/search/fetch tools, and unrelated MCP remain denied; approvals cannot widen that boundary. Authentication/setup must send no repository context, and model requests require explicit consent for the reviewed staged context. Each supported runtime needs a positive fixture for its provider path and negative fixtures for unapproved destinations and tools; if the runtime cannot separate them, it remains unavailable.

### macOS agent isolation research (reviewed 2026-10-14)

No current runtime has a verified Code Atlas boundary. Codex's app-server documents restricted readable roots for `readOnly`, but the same surface also exposes `command/exec`; that mode therefore does not establish the task's no-shell/no-build requirement. ACP provides terminal methods and file operations as protocol capabilities, but withholding client-side methods does not prove that an agent's own tools cannot access the host. Keep all candidates unavailable until both filesystem confinement and command/tool restrictions are exercised against the selected version. See [Codex sandbox read access and command execution](https://developers.openai.com/codex/app-server), [ACP terminals](https://agentclientprotocol.com/protocol/v1/terminals), and [ACP tool calls and permissions](https://agentclientprotocol.com/protocol/v1/tool-calls).

One macOS boundary candidate is a per-run Linux VM that exposes only a newly materialized reviewed snapshot directory through a read-only `VZSharedDirectory`. This can omit the live checkout and `.git` from guest-visible mounts, but it does not itself prevent a guest process from running shell/build commands or reaching the network. The runtime must still have its command, web, and unapproved MCP tools removed and those restrictions tested; egress, cancellation, crash cleanup, and stage deletion also need explicit checks. Apple's Virtualization framework requires the virtualization entitlement and architecture-matched Linux guest assets, so startup time, bundle size, updates, and supported Mac architectures need a separate packaging decision. See [VZSharedDirectory](https://developer.apple.com/documentation/virtualization/vzshareddirectory), [running Linux in a VM](https://developer.apple.com/documentation/virtualization/running-linux-in-a-virtual-machine), and [adding the Virtualization entitlement](https://developer.apple.com/documentation/virtualization/adding-the-virtualization-entitlement-to-your-project).

**Next evidence:** run a no-agent boundary spike with a synthetic stage using either the sandboxed XPC candidate below or a Linux VM when it can boot under the intended signing/runtime setup. Prove writes fail, `.git` and unrelated host paths are absent, arbitrary shell/build execution and arbitrary network destinations are denied by the chosen runtime configuration, and cancel/crash cleanup removes per-run state. Then separately prove the selected adapter can use its narrowly reviewed provider-control route without enabling other egress. Keep the user-requested Codex, Claude Agent, and Antigravity routes unavailable until each also passes its own version, terms, lifecycle, provider-route, and runtime-tool checks.

**No-agent VM startup probe (2026-10-14):** On the development Mac
(macOS 15.7.7, arm64), a temporary Swift probe used the Fedora 44 ARM64
PXE kernel and initrd from the path Apple documents for its Linux VM sample.
The read-only `VZSharedDirectory` configuration validated with one `stage`
share and zero network devices. Starting that guest returned
`VZErrorDomain` code 1 (`Internal Virtualization error. The virtual machine
failed to start.`) before any serial output. A minimal no-share VM using the
same boot assets failed the same way, both with a custom serial VM queue and
the framework's default main queue. Therefore this does not isolate the
failure to virtiofs and does not establish an Apple Virtualization limitation.
The guest never ran, so host-path absence and write-denial checks were not
reached; the synthetic stage and sentinel were removed. The only available
probe signature was ad hoc (`security find-identity -v -p codesigning`
reported zero valid identities), so a normally signed application launch also
remains unverified. Keep the VM boundary candidate unproven and diagnose a
guest boot under the intended app-signing/runtime environment before relying
on it. Sources: [Apple's Linux VM sample and Fedora PXE image guidance](https://developer.apple.com/documentation/virtualization/running-linux-in-a-virtual-machine), [Virtualization entitlement setup](https://developer.apple.com/documentation/virtualization/adding-the-virtualization-entitlement-to-your-project).

### Alternate macOS process boundary: sandboxed XPC service (reviewed 2026-10-14)

Apple documents XPC services as separate processes with their own sandbox;
by default they run with minimal filesystem and network access. Apple also
documents passing a security-scoped bookmark to another process and creating
one with read-only access. These APIs make an app-embedded XPC service a
promising next boundary experiment: create the immutable stage in Code Atlas,
grant the service only a read-only scoped bookmark for that stage, and keep
the service's own container free of repository content and credentials. This
is a candidate architecture, not a demonstrated boundary. The existing
Code Atlas app is not yet an App Sandbox/XPC product, and these sources do
not prove what a specific agent SDK or its subprocesses can access.

Do not replace XPC with a plain `NSTask`/`posix_spawn` helper and call that
privilege separation: Apple's XPC guidance says those child-process APIs do
not create a distinct sandbox. A separately embedded command-line helper can
inherit its containing app's sandbox, but that alone does not prove a
stage-only read root or deny shell/build execution. For any XPC prototype,
record the signed service entitlements and test, using synthetic sentinels,
that it can read only the staged files, cannot write the stage or repository,
cannot read the live checkout, `.git`, or unrelated user files, and cannot
launch a shell/build tool or access unrelated network/MCP services. Also test
that cancellation and service crash remove the stage and invalidate the run.
The service still needs an explicit, narrowly reviewed route for the selected
agent's model traffic; an XPC boundary is not evidence that outbound traffic
is restricted to that provider. Verify the positive provider route separately
from denials for arbitrary destinations, web tools, and unapproved MCP; the
agent's own approval settings must not expand the route.

**Developer-host toolchain check (2026-10-14):** On macOS 15.7.7 arm64,
`/usr/bin/sandbox-exec` is present, but the installed `sandbox-exec(1)` manual
marks it deprecated and directs app developers to App Sandbox. Do not treat a
profile-wrapped CLI as the release isolation design. This machine has Command
Line Tools but not full Xcode, and `security find-identity -v -p codesigning`
reports zero valid identities. That leaves a signed XPC integration and clean
release-package verification unproven here; an ad hoc local probe would not
satisfy the distribution gate. Apple's current App Sandbox documentation
describes kernel-enforced file, network, and process restrictions, but the
Code Atlas host app still needs a product-specific permission and bookmark
design before adopting it.

Current sources: [XPC services and their sandbox](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingXPCServices.html), [read-only security-scoped file access and sharing bookmarks between processes](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [current App Sandbox protections](https://developer.apple.com/documentation/security/protecting-user-data-with-app-sandbox), and [embedding a sandboxed helper tool](https://developer.apple.com/documentation/xcode/embedding-a-helper-tool-in-a-sandboxed-app). The XPC service overview is archived Apple documentation; verify its lifecycle and entitlement behavior on the supported macOS version before implementation.

### Enhanced Security helper extension candidate (reviewed 2026-10-14)

Apple's Xcode 26 release notes introduce an Enhanced Security extension target
for handling untrusted data in a separate process. Apple's setup guide says the
system runs the extension in a sandbox that restricts most system services and
frameworks, disallows extension UI, and expects the host app to provide UI and
system services over XPC. This could offer a stronger starting boundary than a
plain child process, but it does **not** by itself prove the exact Code Atlas
requirements: the implementation must still show that the extension reads only
the reviewed staging directory, cannot modify it, cannot access the live
checkout or `.git`, cannot launch shell/build tools, and cannot make
unreviewed network or MCP requests. A service that needs model traffic may
require an explicitly reviewed host-mediated request path; verify that it
allows only the selected provider endpoints and cannot be expanded by the
agent's tools or approval settings.

Do not infer the feature's minimum runtime OS from the older
`AppExtensionProcess` API availability. Apple's app-extension guide says
programmatic `.appext` generation from extension-point definitions is supported
starting with macOS 26; the exact deployment floor for a manually supplied
extension point and the Enhanced Security runtime still needs a target-OS test.
The project's supported macOS floor is also undecided, so this candidate cannot
be selected until that decision and compatibility are verified.

**Developer-host check (2026-10-14):** This Mac runs macOS 15.7.7 arm64 and has
Command Line Tools with SDK 15.5 and Swift 6.1.2, but no full Xcode. The SDK can
import `ExtensionFoundation` and declares `AppExtensionProcess`, but it does
not declare `AppExtensionPoint`; a direct Swift compile probe for
`AppExtensionPoint` fails with “cannot find in scope.” `xcodebuild -version`
confirms full Xcode is required. Therefore the documented Xcode 26 extension
template and its Enhanced Security behavior cannot be built or run on this
host. This is a toolchain limitation, not evidence that the candidate is
unsafe or safe. Keep it unproven until a supported Xcode/macOS environment can
run the no-agent boundary checks and verify the actual process entitlements.

Current sources: [Xcode 26 release notes](https://developer.apple.com/documentation/xcode-release-notes/xcode-26-release-notes), [Creating Enhanced Security helper extensions](https://developer.apple.com/documentation/xcode/creating-enhanced-security-helper-extensions), [AppExtensionPoint](https://developer.apple.com/documentation/extensionfoundation/appextensionpoint), and [AppExtensionProcess](https://developer.apple.com/documentation/extensionfoundation/appextensionprocess).

### Local Codex app-server schema probe (reviewed 2026-10-14)

On the development Mac (macOS 15.7.7, arm64), `codex --version` reports
`codex-cli 0.149.1`. The installed binary generated both its stable and
experimental JSON Schema bundles with `codex app-server generate-json-schema`;
no app-server session, model request, or repository context was sent. The
generated `ModelListResponse` declares model-specific `supportedReasoningEfforts`,
and `TurnStartParams` declares per-turn `model`, `effort`, `approvalPolicy`, and
`sandboxPolicy` fields. These shapes are useful for a future adapter, but the
schema alone does not prove how the runtime enforces them.

Neither generated bundle contains `readOnlyAccess`, `readableRoots`, or
`includePlatformDefaults`. In this exact installed version, the `readOnly`
`sandboxPolicy` shape contains only `type` and `networkAccess` (default false),
so the app-server schema does not provide a way to restrict readable paths to
the reviewed staging directory. The protocol bundle also declares command
execution request/response methods. Do not expose this installed version for
repository explanation. A future pinned version must be probed again and pass
the staging-path, command-denial, network, cancellation, and cleanup fixture;
otherwise the product must provide and verify an outer OS boundary first.

Reproduce the local schema inspection without starting an agent turn:

```sh
codex --version
schema_dir="$(mktemp -d)"
mkdir -p "$schema_dir/stable" "$schema_dir/experimental"
codex app-server generate-json-schema --out "$schema_dir/stable"
codex app-server generate-json-schema --experimental --out "$schema_dir/experimental"
```

These bundles are generated from the local executable and are not committed.
The checked values therefore describe only this development host's
`0.149.1` installation. The live app-server documentation may describe a
newer protocol surface; it does not upgrade the local binary or prove that a
different pinned runtime behaves the same way.

### Reuse decisions and current first-party surfaces (reviewed 2026-10-13)

Prefer a maintained, vendor-supported integration surface when it meets the product contract. Use a shared protocol such as ACP only where the exact runtime/adapter supports the needed lifecycle and permissions. Do not drive a vendor's interactive terminal UI, and do not create a new agent loop merely to normalize providers. Compare release activity, license, reproducible adoption signals, platform support, and capability evidence; stars or protocol participation alone do not establish safety or fit.

| Backend | Existing programmatic surface found | What it means for Code Atlas |
|---|---|---|
| OpenAI Codex | OpenAI publishes the Codex SDK and app-server for application clients. App-server uses stdio JSONL, streams lifecycle events, and documents `readOnly` plus restricted readable roots. OpenAI also documents an OAuth access-token path for ChatGPT plan usage by open-source app-server clients. | The app-server is the first surface to prototype; ACP remains a separate adapter candidate. Keep it distinct from the OpenAI direct model API. Verify OAuth registration/attribution and distribution eligibility, then prove no shell/build execution and no reads outside the staged snapshot; `readOnly` alone does not demonstrate those product-specific requirements. |
| Anthropic Claude Code | Anthropic publishes the Claude Agent SDK for Python and TypeScript. The SDK starts the Claude Code runtime and includes built-in file, command, web, and MCP capabilities. | Prefer the official SDK over its CLI or ACP wrapper if it fits. In Code Atlas UI, use “Claude Agent” (preferred) or “Claude” in an Agents menu; do not label or brand this integration “Claude Code.” API-key and documented cloud-provider authentication are available. The SDK overview says third-party apps may not offer `claude.ai` login or subscription limits unless approved, while the Help Center updated 2026-10-07 says third-party apps can use plan limits and Max/Team include monthly API credits. Treat subscription-based app auth as unresolved until Anthropic reconciles or approves it; do not silently pick either interpretation. |
| Google Antigravity | Google publishes an official Python SDK (`google-antigravity`) with a local runtime, tool controls, hooks, sessions, and subagents. Its `BuiltinTools.read_only()` preset omits write/command tools from the built-in tool set. The CLI has an interactive TUI and a separate headless mode. | Evaluate SDK and registry-listed ACP binary separately; do not automate the TUI. The preset does not isolate filesystem paths or establish terms eligibility. Pin the runtime, verify supported macOS architectures and bundle cost, and test tool removal, OS path/process isolation, and denied actions. |
| Google Antigravity auth/terms | Google documents SDK auth using Gemini API keys or Gemini Enterprise/Vertex credentials. Its FAQ says consumer Antigravity login must not be used by third-party software, recommends Gemini Enterprise or Google AI Studio API keys for third-party coding agents using Gemini, and says wrapping `agy -p` in third-party applications is unsupported on consumer plans. | Never reuse Antigravity login/session credentials. Google's recommendation for a third-party agent using Gemini is not a blanket approval to embed the Antigravity SDK, ACP binary, or headless CLI. Verify terms for the exact SDK/account combination. Keep it distinct from the separately supported Gemini direct API option. If no permitted Antigravity runtime route is verified, M7 remains incomplete. |

### ACP registry snapshot

The registry is a useful distribution index, not a safety, terms, or product-compatibility certification. The exact registry entries below were checked on 2026-10-14 against registry version `1.0.0`. A filtered copy of the four candidate entries and the SHA-256 of the complete downloaded feed are recorded in [`snapshots/2026-10-14-acp-registry-v1.0.0.json`](snapshots/2026-10-14-acp-registry-v1.0.0.json). Pin a version and re-check the registry before implementation.

| Registry entry | Snapshot | Provenance and decision |
|---|---|---|
| Codex | `codex-acp` 2.2.2, `@agentclientprotocol/codex-acp@2.2.2`, Apache-2.0 | Registry lists OpenAI, JetBrains, and Zed authors. This is an adapter, not the Codex SDK/app-server. Evaluate it against the official SDK/app-server; use only if its lifecycle, auth, terms, and permission boundary are better verified for this product. |
| Claude Agent | `claude-acp` 0.90.0, `@agentclientprotocol/claude-agent-acp@0.90.0`, proprietary | Registry labels it a wrapper and lists Anthropic, JetBrains, and Zed authors. Evaluate the exact wrapper separately from the official Agent SDK; use the required “Claude Agent” UI label and approved API-key auth. |
| Gemini CLI | `gemini` 0.64.0, `@google/gemini-cli@0.64.0 --acp`, Apache-2.0 | Registry identifies Google's official CLI. Pin the CLI and ACP mode, then verify events, permissions, auth, and cancellation; its ACP support alone does not establish the required read-only boundary. |
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
