# Rust direct model API client comparison

**Reviewed:** 2026-10-13  
**Scope:** direct text-generation APIs for Code Atlas. Coding-agent runtimes, MCP, and speech synthesis are separate integration types.

## Recommendation

Prototype **`rig-core` 0.44.0** as the shared direct-API client, depending on `rig-core` directly with only its HTTP transport and TLS features. Do not depend on the `rig` facade or `rig-agent`: this product needs provider request/response adapters, not Rig's agent runtime.

This is a library selection for the implementation spike, not proof that the integration is accepted. Keep the provider capability matrix authoritative. Pin the version, then retain the selection only if mocked tests pass for Gemini, OpenAI, and Anthropic across the product operations, and provider-specific settings, cancellation, request privacy, and packaging costs meet the acceptance contract.

The strongest reasons are explicit cancellation on Rig's normalized completion stream, provider-specific clients that make routing visible in code, its broad OpenAI/Anthropic/Gemini support, a same-version direct-core boundary without the agent runtime, and materially stronger public adoption and release signals. Its tradeoffs are a larger package, a more involved typed API, and a fast pre-1.0 release cadence with recent breaking changes. If the prototype cannot keep the transport small or preserve the required provider features, re-evaluate `genai` rather than building adapters ourselves.

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
| Streaming | Normalized completion stream; explicit `cancel()` aborts the provider stream | `ChatStream` yields provider-neutral events; stable public docs do not document an explicit cancellation method | Rig better matches the product's cancel control. Do not infer transport cancellation from dropping a stream; verify it for any alternative. |
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
6. Use a mocked transport for each provider and operation. Test zero transport activity before consent, exact provider routing, endpoint/model change invalidation, schema/capability handling, stream cancellation, errors, and no fallback.
7. Measure release binary size and compile impact after resolving the exact dependency graph. The docs.rs metrics above are only comparative signals, not app measurements.

## Alternatives and unresolved evidence

`genai` remains the fallback candidate if the Rig core prototype cannot satisfy the adapter contract with a small dependency set. Its stable release exposes direct chat/streaming for the three vendors and explicit provider identities; the main concern for this product is the stable API's lack of a documented explicit stream-cancel method, plus the need to opt out of model-prefix inference. Re-check the 0.7 line only after a non-prerelease release and rerun the same contract.

The current workspace's provider implementation is Gemini-specific. No provider library has yet been added to the shipped app, no API key or live request was used for this comparison, and no real-provider billing, data handling, or capability behavior was tested here. Those remain acceptance work, not assumptions.

## Primary sources

- [`rig-core` 0.44.0 crate metadata, dependency list, size, and release history](https://docs.rs/crate/rig-core/0.44.0)
- [Rig provider modules](https://docs.rs/rig-core/0.44.0/rig_core/providers/)
- [Rig streaming API, including explicit cancellation](https://docs.rs/rig-core/0.44.0/rig_core/streaming/struct.StreamingCompletionResponse.html)
- [Rig 0.44.0 source manifest and transport feature boundaries](https://raw.githubusercontent.com/0xPlaygrounds/rig/main/crates/rig-core/Cargo.toml)
- [Rig release history](https://github.com/0xplaygrounds/rig/releases)
- [`genai` 0.6.5 crate metadata, dependency list, size, and release history](https://docs.rs/crate/genai/0.6.5)
- [`genai` chat options and response-format capabilities](https://docs.rs/genai/0.6.5/genai/chat/struct.ChatOptions.html)
- [`genai` stable streaming API](https://docs.rs/genai/0.6.5/genai/chat/struct.ChatStream.html)
- [`genai` adapter routing and explicit model identities](https://docs.rs/genai/0.6.5/genai/struct.Client.html)
- [`genai` manifest, license, and Reqwest/Tokio dependencies](https://github.com/jeremychone/rust-genai/blob/main/Cargo.toml)
- [Rig GitHub adoption/maintenance signals](https://github.com/0xplaygrounds/rig)
- [GenAI GitHub adoption/maintenance signals](https://github.com/jeremychone/rust-genai)
