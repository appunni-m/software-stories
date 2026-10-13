# Rust direct model API client comparison

**Reviewed:** 2026-10-13  
**Scope:** direct text-generation APIs for Code Atlas. Coding-agent runtimes, MCP, and speech synthesis are separate integration types.

## Recommendation

Prototype **`rig-core` 0.44.0** as the shared direct-API client, depending on `rig-core` directly with only its HTTP transport and TLS features. Do not depend on the `rig` facade or `rig-agent`: this product needs provider request/response adapters, not Rig's agent runtime.

This is a library selection for the implementation spike, not proof that the integration is accepted. Keep the provider capability matrix authoritative. Pin the version, then retain the selection only if mocked tests pass for Gemini, OpenAI, and Anthropic across the product operations, and provider-specific settings, cancellation, request privacy, and packaging costs meet the acceptance contract.

The strongest reasons are cancellable normalized completion streams (by dropping the stream/future), provider-specific clients that make routing visible in code, its broad OpenAI/Anthropic/Gemini support, a same-version direct-core boundary without the agent runtime, and materially stronger public adoption and release signals. Rig 0.44.0 does not expose a separate `cancel()` method on its `Streamed` type; its source documents that dropping the stream cancels the read. The app must therefore own cancellation by dropping the in-flight request future and verify that its transport closes. Its other tradeoffs are a larger package, a more involved typed API, and a fast pre-1.0 release cadence with recent breaking changes. If the prototype cannot keep the transport small or preserve the required provider features, re-evaluate `genai` rather than building adapters ourselves.

## Evidence snapshot

The source pages below were checked on the review date. GitHub stars and forks are an interest signal, not a count of production deployments. Download totals were not used: the crates.io API returned HTTP 403 from this environment, and the registry's client page did not expose its numbers in the retrieved HTML.

| Check | `rig-core` | `genai` | Relevance to Code Atlas |
|---|---|---|---|
| Candidate version | 0.44.0, published 2026-10-07 | 0.6.5 stable, published 2026-06-06; 0.7.0-rc.4 published 2026-10-07 | Pin an exact release; do not depend on a moving prerelease. |
| License | MIT | MIT OR Apache-2.0 | Both are permissive; include the chosen crate's notices in app packaging. |
| Public adoption signal | About 8,818 GitHub stars and 986 forks | About 900 stars and 190 forks | Rig has the larger public interest signal; neither count proves deployed usage. |
| Release activity | 0.42.0 (Aug 17), 0.43.0 (Sep 30), 0.44.0 (Oct 7) | 0.6.5 stable (Jun 6); repeated 0.7 prereleases, latest listed Oct 7 | Both are maintained. Rig has a faster stable cadence; genai's next line is still prerelease. |
| Package/build-size proxy | Docs.rs lists 5.4 MB source and 59 s average successful docs.rs build | Docs.rs lists 1.4 MB source and 35 s average successful docs.rs build | GenAI is lighter by these registry proxies; measure the actual app binary and build after integration. |
| Required providers | First-party OpenAI, Anthropic, Gemini modules | OpenAI (including Responses), Anthropic, Gemini adapters | Both meet the initial provider list on paper. |
| Structured output | Shared typed JSON Schema field, translated by provider wires; model/provider support varies | `JsonSpec`/response-format support, with provider-specific schema normalization and gaps | Every operation/provider/model combination still needs a capability entry and mocked fixture. |
| Streaming and cancellation | Normalized completion stream; in 0.44.0 dropping the stream/future cancels the consumer's read. The pinned API has no separate `cancel()` method; verify the HTTP transport closes on drop. | `ChatStream` yields provider-neutral events; stable public docs do not document an explicit cancellation method | Rig appears usable with the app's task-cancellation boundary, but transport abort behavior remains a prototype check. Never imply cancellation reverses a provider's already accepted/billable request. |
| Provider-specific controls | Provider configuration and per-request additional parameters; clients are constructed for a named provider | Extensive `ChatOptions`, custom endpoint/auth, and provider configuration; includes model-name adapter inference by default | Both preserve vendor controls, with some features unsupported per provider/model. Avoid a generic lowest-common-denominator UI. |
| Exact routing | Construct the selected provider client and model explicitly; no cross-provider catalog router is needed | `ModelIden`/`ServiceTarget` can explicitly bind provider and endpoint, but string model names infer adapters by prefix | Both can route explicitly. GenAI requires avoiding inference helpers and testing that errors never select another adapter. |
| Transport/runtime footprint | `rig-core` has `rig-http` contracts and optional `rig-reqwest`; its `rustls` feature selects TLS. `rig-agent` is a separate crate. | Direct multi-provider client; defaults to Rustls and uses Reqwest 0.13 | The app already uses Reqwest 0.13 and Tokio. Confirm Cargo's resolved graph and macOS bundle size in the spike. |
| Privacy boundary | Library does not authorize a request; app owns exact-context review and user consent. Rig's telemetry docs warn that enabling sensitive content can expose prompt/context. | Library does not authorize a request; app owns exact-context review and user consent. Raw-body capture is an opt-in request option. | Neither is a substitute for the app's request fingerprint, consent, logging redaction, or keychain policy. |

## Integration rules for the spike

1. Add `rig-core = 0.44.0` with `default-features = false` and only the required HTTP/TLS features; do not enable `rig-agent`, agent tools, vector stores, MCP, audio, PDF, or embeddings for these direct text requests.
2. Build an adapter catalog entry that contains an explicit provider, model, endpoint, and typed capability record. Do not route from an untrusted model-name prefix or a library default.
3. Convert each approved request fingerprint to exactly one selected provider request. A provider error, unsupported option, timeout, or cancellation must not retry through another provider or model.
4. Keep the local consent and request-review UI in front of transport creation/use. The review covers the exact snapshot files/excerpts, provider/model/endpoint, prompt/schema/settings, disclosure, and cost uncertainty.
5. Disable sensitive request/response telemetry and avoid storing provider payloads in diagnostics, local application metadata, exports, or accepted guide history.
6. Use a mocked transport for each provider and operation. Test zero transport activity before consent, exact provider routing, endpoint/model change invalidation, schema/capability handling, drop-based stream cancellation, errors, and no fallback.
7. Measure release binary size and compile impact after resolving the exact dependency graph. The docs.rs metrics above are only comparative signals, not app measurements.

## Version-specific integration risk found in the compile spike

Rig's 0.44.0 Gemini GenerateContent encoder puts its API key in the request URI query. Google's current REST reference says Gemini API requests must send the key in the `x-goog-api-key` header. Do not connect Rig's default Gemini transport unchanged: it would regress the app's current header-only credential handling.

The current compile spike uses Rig to encode the reviewed Gemini JSON body locally. Request review shows the sanitized provider endpoint and the exact encoded JSON body/hash; the consent fingerprint binds that endpoint and body hash. The existing Gemini Reqwest sender still authenticates with the documented header and keeps its response-size, safety-finish, and error classification checks. It does not yet use Rig's HTTP transport or response decoder. OpenAI and Anthropic encoders are present in the prototype layer, but they are not yet exposed in the app or selectable end-to-end. The next integration step must either add a narrow adapter that moves Rig's Gemini query key to the required header and verifies the body/route immediately before transport, or keep the existing Gemini sender/decoder as a documented exception while Rig handles OpenAI and Anthropic. Never weaken credential handling for library symmetry.

The 0.44.0 `Streamed` API documents drop-based cancellation; it does not expose the older `cancel()` method shown by older Rig source pages. Code Atlas can own the cancellation by dropping the in-flight call/stream future, but must validate transport closure with a mocked transport before describing cancellation as supported.

## Alternatives and unresolved evidence

`genai` remains the fallback candidate if the Rig core prototype cannot satisfy the adapter contract with a small dependency set. Its stable release exposes direct chat/streaming for the three vendors and explicit provider identities; the main concern for this product is the stable API's lack of a documented explicit stream-cancel method, plus the need to opt out of model-prefix inference. Re-check the 0.7 line only after a non-prerelease release and rerun the same contract.

The workspace now pins `rig-core` 0.44.0 and uses its provider wires to encode exact reviewed JSON payloads for Gemini, OpenAI, and Anthropic without making a network request. Only the Gemini payload encoder is connected to the app's review flow; Gemini still owns the only selectable generation path and its HTTP sender/response parser remain provider-specific. No API key or live request was used for this comparison, and no real-provider billing, data handling, or capability behavior was tested here. OpenAI/Anthropic end-to-end routes, all unimplemented operations, mock routing across providers, and real-provider behavior remain acceptance work.

## Primary sources

- [`rig-core` 0.44.0 crate metadata, dependency list, size, and release history](https://docs.rs/crate/rig-core/0.44.0)
- [Rig provider modules](https://docs.rs/rig-core/0.44.0/rig_core/providers/)
- [Rig 0.44.0 stream drop/cancellation behavior](https://docs.rs/crate/rig-core/0.44.0/source/src/streaming/mod.rs)
- [Rig 0.44.0 Gemini GenerateContent encoder](https://docs.rs/crate/rig-core/0.44.0/source/src/providers/gemini/completion.rs)
- [Gemini API authentication and REST reference](https://ai.google.dev/api)
- [Gemini API key security guidance](https://ai.google.dev/gemini-api/docs/api-key)
- [Rig 0.44.0 source manifest and transport feature boundaries](https://raw.githubusercontent.com/0xPlaygrounds/rig/main/crates/rig-core/Cargo.toml)
- [Rig release history](https://github.com/0xplaygrounds/rig/releases)
- [`genai` 0.6.5 crate metadata, dependency list, size, and release history](https://docs.rs/crate/genai/0.6.5)
- [`genai` chat options and response-format capabilities](https://docs.rs/genai/0.6.5/genai/chat/struct.ChatOptions.html)
- [`genai` stable streaming API](https://docs.rs/genai/0.6.5/genai/chat/struct.ChatStream.html)
- [`genai` adapter routing and explicit model identities](https://docs.rs/genai/0.6.5/genai/struct.Client.html)
- [`genai` manifest, license, and Reqwest/Tokio dependencies](https://github.com/jeremychone/rust-genai/blob/main/Cargo.toml)
- [Rig GitHub adoption/maintenance signals](https://github.com/0xplaygrounds/rig)
- [GenAI GitHub adoption/maintenance signals](https://github.com/jeremychone/rust-genai)
