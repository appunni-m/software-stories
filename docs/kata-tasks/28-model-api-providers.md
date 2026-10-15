---
kind: task
key: t28
parent: m7
title: Add selectable direct model API providers
priority: 1
labels: phase-7, provider, privacy
blocked_by: t10, t12, t15, t20, t27
---

## User outcome
The user can choose a direct model API for every supported text-generation operation and knows which vendor receives the reviewed source context. Google Gemini is one option in an extensible catalog, not the product's only AI path.

## Contract
Keep direct model APIs separate from coding-agent runtimes. Provide selectable Gemini, OpenAI, and Anthropic API adapters for every direct text-generation operation in the product: file chapters, optional orientation, selective section regeneration, on-demand Code Lens explanation, PR guides, and PR change explanations. Maintain an operation-to-capability matrix so each operation shows which API adapters support its schema, streaming, cancellation, and other required features. Speech synthesis remains a separate modality owned by t18; this task must not imply a text-generation client can synthesize speech.

The Code Lens operation is a focused source-range explanation, separate from whole-file chapter generation. Its input is one path in the immutable snapshot, the source SHA-256, an inclusive 1-based line range, and optionally an exact half-open UTF-8 byte range. The request review shows only that selected excerpt. The transient response must contain bounded, concrete claims grounded by unique exact quotes within that excerpt, with citations that reopen the matching snapshot range. It does not create a chapter revision, add coverage, or persist an explanation.

Before choosing an integration, compare currently maintained Rust multi-provider clients such as Rig and `genai` against this app's direct-request needs, required APIs, structured output, streaming, cancellation, provider-specific controls, license, release activity, adoption, and existing Rust dependencies. Date the comparison and record reproducible adoption signals (for example, downloads, dependents, or repository activity) along with their sources. Prefer the most established solution that meets the product's privacy, exact-routing, and capability requirements; do not choose on popularity alone. Choose the smallest well-supported existing abstraction that preserves required provider capabilities; do not pull in an unused agent loop just to make API calls. LiteLLM is a gateway/service alternative to evaluate only if a separately operated gateway is an explicit product choice, not a Rust library equivalent. Record a short decision and capability gaps; use official provider clients only where the chosen shared library cannot preserve a required vendor capability. Do not hand-roll separate HTTP protocol clients or a new AI orchestration framework.

The provider catalog identifies supported model capabilities and per-provider requirements. The same explicitly selected provider/adapter is used end-to-end for a request; disable or bypass any library-level model-name routing or default-provider fallback. Unsupported structured output, streaming, cancellation, or model features are surfaced before send. Google Gemini remains one choice among the catalog. An optional user-configured OpenAI-compatible endpoint may be included only with an explicit endpoint, HTTPS/loopback policy, and endpoint-specific disclosure. Adding a direct API provider requires a catalog entry, capability evidence, and tests for every product operation claimed to support it.

Reuse the local request-review flow. Before a call, show provider, model, endpoint, purpose, exact selected snapshot paths/excerpts, request settings, disclosure/terms, and available usage or cost information. Bind one explicit confirmation to the complete request fingerprint, provider/model/endpoint, and disclosure version. Keep each API key in macOS Keychain. No network request occurs during setup, selection, or review; failed, cancelled, quota-limited, or billing-blocked requests never switch provider or model automatically. Every returned draft still passes local schema/evidence validation and Chapter Studio review.

## Acceptance criteria
- The API catalog initially offers Gemini, OpenAI, and Anthropic direct APIs; provider/model choice is explicit per task and no provider is silently selected as a fallback.
- A dated decision record compares at least Rig and `genai` (or explains why either is no longer a current candidate) using the same documented checklist: maintenance/release activity, license, reproducible adoption signals, provider coverage, structured output, streaming, cancellation, provider-specific controls, privacy fit, and dependency cost. It records the selected library, unsupported capabilities, and any narrowly scoped official-client exceptions.
- One maintained Rust multi-provider library handles common transport and request/response mapping, with documented exceptions for provider-specific features; no new AI agent loop or homegrown per-provider transport is introduced.
- Every direct text-generation operation in the operation matrix, including chapter generation, optional orientation, selective section regeneration, on-demand Code Lens explanation, and PR Guide/PR change explanation text, uses the same explicit API-selection and consent boundary instead of bypassing it with Gemini-specific code. TTS stays explicitly linked to its separate speech capability/task.
- Code Lens review rejects stale source hashes, invalid line/byte ranges, binary or secret-like source, and oversized ranges; it sends only the exact selected excerpt. Local response validation rejects malformed claims, identifiers not present in both prose and quote, and quotes absent or ambiguous in the excerpt. Each returned citation opens those exact captured bytes. The explanation is transient and does not enter chapter history or coverage.
- The local review displays the exact text/context that will be submitted, provider/model/endpoint, applicable terms, and a clear explanation when cost is unknown or the user may incur a charge.
- Provider/model/endpoint, prompt/schema, source hashes, generation settings, and disclosure changes invalidate prior consent.
- Keys, source excerpts, and full prompts do not appear in logs, local application metadata, exports, or diagnostics.
- Mocked transports prove zero requests before consent, correct provider routing for every claimed operation, invalidation when provider/model/endpoint changes, and no cross-provider fallback on errors.

## Excludes
Coding-agent sessions, speech/TTS provider selection, product-hosted gateways, automatic model discovery, vendor subscription-token scraping, paid fallback, and claiming all providers have feature or pricing parity.

## Done evidence
Generate and locally validate fixtures for every operation claimed in the capability matrix through each API adapter using mock transports; verify the selected vendor receives only the reviewed fingerprint, each provider's capability/disclosure UI is accurate, and a failed provider leaves the user on the same explicitly chosen backend without altering accepted guide history. Attach the dated library decision record and adoption evidence.

## References
- [Dated Rig `rig-core` vs `genai` comparison and prototype recommendation (2026-10-13)](../research/2026-10-13-rust-model-client-comparison.md)
- [Rig core concepts](https://rig.rs/docs/concepts/) and [Rig provider integrations](https://docs.rs/rig-core/latest/rig_core/providers/index.html)
- [`genai` Rust multi-provider client](https://github.com/jeremychone/rust-genai)
- [LiteLLM overview](https://docs.litellm.ai/docs/) for the optional gateway/service alternative
- [Gemini API reference](https://ai.google.dev/api/), [OpenAI SDKs and APIs](https://developers.openai.com/api/docs/libraries), and [Anthropic API documentation](https://docs.anthropic.com/en/api/overview)

## Pending work — 2026-10-15

- [ ] Run each advertised operation through each selected API adapter when validation is authorized; use the maintained shared client and exact effective config.
- [ ] Verify redirects are rejected by production transport and consent invalidates on any provider/model/context/endpoint/settings change.
- [ ] Complete Code Lens cited-range, chapter, overview, selective section, PR guide and hunk explanation cases with precise locally validated claims.
- [ ] Keep missing capabilities visible and never substitute another provider/model; TTS remains a separate modality.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.
