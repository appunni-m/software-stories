# Direct API and Gemini narration disclosures

This document summarizes the versioned disclosures represented in Code Atlas source. It is not a live price, quota, eligibility, retention, or billing check. Direct text APIs and speech synthesis are separate request paths with separate provider/model records.

## Direct text API providers

The versioned Gemini, OpenAI, and Anthropic records are defined in `src-tauri/src/provider.rs`. The selected provider and model are explicit; the app does not discover models or switch providers after failure. Prices, model IDs, and provider terms are static release metadata checked on 13 October 2026 and must be verified against the linked first-party sources before release. The app cannot inspect the account, key tier, billing, quota, retention configuration, eligibility, or regional availability behind a user-supplied key.

### Data use and eligibility notices

- **Gemini API:** the app warns that Google may use content sent through unpaid Gemini API services to improve products and may have people review it. Google's terms have a paid-service data-use exception for users in the EEA, Switzerland, and the United Kingdom, including users of unpaid quota. Age, professional/business-use, supported-region, and account terms still apply.
- **OpenAI API:** the app discloses that API data is not used for training by default, while abuse-monitoring logs can contain prompts and responses and are normally retained for up to 30 days, subject to current account, legal, and safety controls. The reviewed Responses request sets `store: false`; this avoids Responses application-state storage and does not establish zero retention for all processing.
- **Anthropic API:** the app's structured-output Messages request uses the provider's schema feature. Its notice describes the documented Zero Data Retention qualification for that feature, including schema caching for up to 24 hours and applicable account, trust-and-safety, and legal exceptions. Verify current retention terms and account eligibility before sending source.

Each provider has a separate price, terms, supported-region, and API-key reference in the app. A listed price or a model's feature flag is not a live availability, account-eligibility, free-use, or per-operation capability check.

### Key handling and request boundary

The key is saved in a provider-specific macOS Keychain item and is never returned to the WebView. Rust loads it only after request review and explicit consent, keeps it in a zeroizing wrapper, and sends it in a sensitive HTTPS authentication header: `x-goog-api-key` for Gemini, `Authorization: Bearer` for OpenAI, and `x-api-key` for Anthropic. Keys are not included in the JSON request, request preview, local repository data, exports, or application-authored logs.

The current shared API route uses pinned `rig-core` 0.44.0 provider wires and a narrow Reqwest transport wrapper. Immediately before sending, the wrapper checks that the method, endpoint, JSON body, content type, and provider credential placeholder match the reviewed request. It requires HTTPS, applies a 90-second timeout, and bounds response bodies. The current app transport is unary; streaming is disabled even where a provider model supports streaming.

The user must explicitly choose a provider and model, inspect the request purpose, exact selected source/diff, prompt/settings, endpoint, body digest, cost uncertainty, and disclosure, and confirm the unchecked privacy, billing, and age/region/use controls. Opening settings, loading the catalog, saving a key, and building or rebuilding a request preview do not contact a model API. A request review is fingerprinted, expires after fifteen minutes, and can be consumed once. No provider/model fallback or automatic retry occurs.

### Current operation coverage

The shared route is connected to file chapters, optional repository orientation, one-section regeneration, PR Guides, and a distinct transient Code Lens range-explanation operation. This implementation status does not satisfy the full t28 acceptance contract: Code Lens provider-by-operation mocked routing and cancellation fixtures have not been completed, and there is no separate PR change-explanation operation. See the [capability matrix](research/ai-backend-capability-matrix.md) for the current status. Returned chapters and PR Guides remain unaccepted until the local validation and user-review flows complete; Code Lens explanations are citation-validated but are not persisted as chapter drafts.

For a file chapter, the reviewed request contains the target file and up to eleven selected supporting files from the same immutable snapshot. Selected source is capped at 250 KB and the encoded request body at 512 KB; secret-like, binary, invalid-encoding, skipped, and uncaptured files are rejected. A PR Guide has a separate review based on two local Git refs resolved to immutable commits and up to twelve selected bounded text-diff files. Its fingerprint covers the selected diff evidence and changing the refs or selection invalidates consent.

Cancellation drops the in-flight local request future when the user cancels. The provider may already have received or started processing the request, so cancellation cannot promise a refund, erase transmitted content, or revoke provider work. Any late response is discarded or kept only as an unaccepted draft according to the operation's recovery path.

## Optional section narration

Narration has a separate record: `gemini-tts-disclosure-2026-10-13-v2`, checked 13 October 2026. The source currently selects model ID `gemini-3.8-flash-lite-tts` (displayed as Gemini 3.8 Flash-Lite TTS) and the listed prebuilt voices. These are static app values, not a live capability check.

The exact outbound narration request(s) contain one selected section's edited speech text after any explicitly enabled pronunciation substitutions, the selected voice, delivery style, and WAV response format. Scripts up to 16,000 UTF-8 bytes are split into sequential segments of at most 4,000 bytes, preferring sentence boundaries and otherwise whole-word boundaries; the request review shows the full text and each exact segment. Source files, citations, file paths, and the whole chapter are not included by default. The review is fingerprinted and expires after fifteen minutes. The user confirms the request and current terms before synthesis. A cache hit can be served locally after review without sending another provider request.

As checked in the source disclosure, Google lists this model's standard text and audio output as free on the Free Tier subject to project quota and availability. Code Atlas cannot inspect the key's project tier or remaining quota. The source disclosure lists paid-project rates of `$0.50` per million input text tokens and `$6.00` per million output audio tokens through 31 December 2026 (`$0.0015` per 10 seconds), then `$1.00` and `$12.00` from 1 January 2027. These prices and future rates must be rechecked against Google's pricing page before any release after the check date. A Free Tier label does not establish that a particular request is free or available.

Gemini narration has its own Google data-use, regional, eligibility, and billing disclosure. The OpenAI and Anthropic text API disclosures do not apply to this speech request. For Paid Services, the shipped notice describes Google's stated product-improvement and limited logging terms. The app cannot inspect the key's billing project, region, age, or eligibility. The request is sent directly to Google's Gemini API over HTTPS, without a retry or paid/model fallback; local cancellation cannot establish whether Google received a request already in flight. The transcript and previous cached audio remain available on error.

The speech disclosure and request behavior are defined in `src-tauri/src/narration.rs`; text-generation disclosures are defined in `src-tauri/src/provider.rs`. The dates and terms in this document are release metadata and must be reviewed before distribution.

## First-party sources

- [Gemini API pricing](https://ai.google.dev/gemini-api/docs/pricing)
- [Gemini API terms](https://ai.google.dev/gemini-api/terms)
- [Gemini API available regions](https://ai.google.dev/gemini-api/docs/available-regions)
- [Gemini API key guidance](https://ai.google.dev/gemini-api/docs/api-key)
- [Gemini models](https://ai.google.dev/gemini-api/docs/models)
- [Gemini `generateContent` API](https://ai.google.dev/gemini-api/docs/generate-content/text-generation)
- [Gemini speech generation](https://ai.google.dev/gemini-api/docs/speech-generation)
- [OpenAI API models](https://developers.openai.com/api/docs/models) and [pricing](https://developers.openai.com/api/docs/pricing)
- [OpenAI API data controls](https://developers.openai.com/api/docs/guides/your-data) and [structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs)
- [OpenAI terms](https://openai.com/policies/services-agreement/) and [supported countries](https://developers.openai.com/api/docs/supported-countries)
- [OpenAI API keys](https://platform.openai.com/api-keys)
- [Anthropic models](https://platform.claude.com/docs/en/models/overview) and [pricing](https://platform.claude.com/docs/en/about-claude/pricing)
- [Anthropic structured outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs) and [API data retention](https://platform.claude.com/docs/en/manage-claude/api-and-data-retention)
- [Anthropic commercial terms](https://www.anthropic.com/legal/commercial-terms) and [supported regions](https://platform.claude.com/docs/en/api/supported-regions)
- [Anthropic API keys](https://console.anthropic.com/settings/keys)
