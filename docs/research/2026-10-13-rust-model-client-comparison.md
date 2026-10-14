# Rust direct model API client comparison

**Reviewed:** 2026-10-13  
**Scope:** direct text-generation APIs for Code Atlas. Coding-agent runtimes, MCP, and speech synthesis are separate integration types.

## Recommendation

Retain **`rig-core` 0.44.0** as the shared direct-API adapter, depending on `rig-core` directly with only its HTTP and TLS features. The current implementation uses Rig's provider wires and normalized completion response, wrapped by a narrow app-owned Reqwest transport that enforces the reviewed request boundary. Do not depend on the `rig` facade or `rig-agent`: this product needs provider request/response adapters, not Rig's agent runtime.

The library choice is implemented but is not proof that the provider integration is accepted. Keep the provider capability matrix authoritative. The 2026-10-14 Rust test suite now covers mocked routing, consent, invalidation, quota failure, and byte-for-byte history-preservation checks for each wired provider/operation pair, plus loopback request-body, credential-placement, and local cancellation/socket-close behavior. These fixtures do not establish live vendor behavior, account eligibility, production TLS/proxy behavior, or whether a provider received a request before cancellation.

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
| Streaming and cancellation | Normalized completion stream; in 0.44.0 dropping the stream/future cancels the consumer's read. The pinned API has no separate `cancel()` method. A later loopback test verifies that dropping the app's pending Reqwest/Rig future closes the local socket. | `ChatStream` yields provider-neutral events; stable public docs do not document an explicit cancellation method | Local transport-close behavior is tested for Rig; whether a live provider received or completed work before cancellation remains unknown. Never imply local cancellation reverses provider-side work or billing. |
| Provider-specific controls | Provider configuration and per-request additional parameters; clients are constructed for a named provider | Extensive `ChatOptions`, custom endpoint/auth, and provider configuration; includes model-name adapter inference by default | Both preserve vendor controls, with some features unsupported per provider/model. Avoid a generic lowest-common-denominator UI. |
| Exact routing | Construct the selected provider client and model explicitly; no cross-provider catalog router is needed | `ModelIden`/`ServiceTarget` can explicitly bind provider and endpoint, but string model names infer adapters by prefix | Both can route explicitly. GenAI requires avoiding inference helpers and testing that errors never select another adapter. |
| Transport/runtime footprint | `rig-core` has `rig-http` contracts and optional `rig-reqwest`; its `rustls` feature selects TLS. `rig-agent` is a separate crate. | Direct multi-provider client; defaults to Rustls and uses Reqwest 0.13 | The app already uses Reqwest 0.13 and Tokio. Confirm Cargo's resolved graph and macOS bundle size in the spike. |
| Privacy boundary | Library does not authorize a request; app owns exact-context review and user consent. Rig's telemetry docs warn that enabling sensitive content can expose prompt/context. | Library does not authorize a request; app owns exact-context review and user consent. Raw-body capture is an opt-in request option. | Neither is a substitute for the app's request fingerprint, consent, logging redaction, or keychain policy. |

## Implementation contract

1. Pin `rig-core = 0.44.0` with `default-features = false` and the required HTTP/TLS features; do not enable `rig-agent`, agent tools, vector stores, MCP, audio, PDF, or embeddings for these direct text requests.
2. Keep the provider/model/endpoint explicit in the catalog. Do not route from an untrusted model-name prefix or library default.
3. The app-owned `SafeRigHttpClient` may replace the Rig placeholder credential only after method, exact endpoint, JSON content type, and serialized body match the reviewed request. It must keep Gemini authentication in `x-goog-api-key`; OpenAI and Anthropic also use their documented sensitive headers.
4. Convert each approved request fingerprint to exactly one selected provider request. A provider error, unsupported option, timeout, or cancellation must not retry through another provider or model.
5. Keep the local consent and request-review UI in front of transport use. Review covers the exact snapshot files/excerpts, provider/model/endpoint, prompt/schema/settings, disclosure, and cost uncertainty.
6. Keep request/response telemetry disabled and avoid storing provider payloads in diagnostics, local application metadata, exports, or accepted guide history.
7. The current fixture suite covers mocked routing, consent, endpoint/model change invalidation, synthetic quota errors, no fallback, and byte-for-byte history preservation on failed requests for each wired provider/operation pair. Loopback tests exercise actual local request encoding and credential placement and verify socket closure on cancellation; the capability matrix lists the remaining live-vendor and production-transport gaps.
8. Measure release binary size and compile impact after resolving the exact dependency graph. The docs.rs metrics above are only comparative signals, not app measurements.

## Version-specific integration risk found in the compile spike

Rig's 0.44.0 Gemini GenerateContent encoder puts its API key in the request URI query. Google's current REST reference says Gemini API requests must send the key in the `x-goog-api-key` header. Do not connect Rig's default Gemini transport unchanged: it would regress the app's current header-only credential handling.

The current adapter handles Rig's Gemini wire placing its placeholder key in the query by checking the exact reviewed request, removing the placeholder query, and inserting the saved key into a sensitive `x-goog-api-key` header. OpenAI and Anthropic routes likewise check the Rig placeholder authentication and endpoint before inserting the saved key into their documented headers. For all three routes, the Reqwest wrapper requires POST and JSON, compares the serialized bytes with the reviewed body, requires HTTPS, and bounds time and response size; streaming and multipart requests are rejected. Loopback tests now verify the resulting request body and provider auth header through the actual Reqwest/Rig transport. They do not exercise live vendor endpoints or production TLS/proxy configuration. Never weaken credential handling for library symmetry.

The 0.44.0 `Streamed` API documents drop-based cancellation; it does not expose the older `cancel()` method shown by older Rig source pages. Code Atlas owns cancellation by dropping the in-flight call future. A later Reqwest/Rig loopback regression verifies that this closes a pending local socket; it does not prove whether a live provider received or completed work before cancellation. See the current [capability matrix](ai-backend-capability-matrix.md) for the remaining live-provider and production-transport gaps.

## Alternatives and unresolved evidence

`genai` remains the fallback candidate if the Rig core prototype cannot satisfy the adapter contract with a small dependency set. Its stable release exposes direct chat/streaming for the three vendors and explicit provider identities; the main concern for this product is the stable API's lack of a documented explicit stream-cancel method, plus the need to opt out of model-prefix inference. Re-check the 0.7 line only after a non-prerelease release and rerun the same contract.

At the original 2026-10-13 comparison checkpoint, the provider wires were connected to the reviewed request boundary for file chapters, optional orientation, selective section updates, and PR Guides; the compile spike had not yet run transport fixtures. Follow-up work added per-operation mock fixtures, distinct Code Lens and PR change-explanation routes, and Reqwest/Rig loopback tests. The current matrix records that evidence. No API key or live request was used for this comparison, so live vendor billing, data handling, eligibility, response parsing, and production TLS/proxy behavior remain unverified.

**Implementation follow-up (2026-10-14 snapshot):** the app added a distinct transient Code Lens request scoped to an exact hash-verified snapshot range. At that checkpoint, provider-by-operation Code Lens fixtures and the separate PR change-explanation operation remained open; this note preserves the comparison's 2026-10-13 evidence and is not the current implementation status.

**Later implementation update (2026-10-14):** the app now also has a transient single-hunk PR change-explanation route for Gemini, OpenAI, and Anthropic. It reviews and sends one local diff hunk, validates side-aware citations, and does not create PR Guide history. Follow-up loopback Reqwest/Rig fixtures verify credential placement and socket closure on cancellation; those local checks do not establish vendor eligibility, production TLS/proxy behavior, or whether a provider accepted work before the local cancellation. The current [capability matrix](ai-backend-capability-matrix.md) records the complete implementation and remaining acceptance gaps.

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
