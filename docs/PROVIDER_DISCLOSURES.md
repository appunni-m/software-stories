# Google Gemini provider disclosures

This document records the Google Gemini disclosures currently represented in Code Atlas source for direct text generation and optional narration. It is not a live price, quota, eligibility, or account-billing check. The app displays the relevant request disclosure before sending selected repository source or speech text.

## Record

- Disclosure ID: `gemini-disclosure-2026-10-13-v1`
- Sources checked: 13 October 2026
- Provider: Google Gemini API, called directly from the macOS app
- Account: user supplies their own API key; Code Atlas cannot identify its Cloud project or inspect billing/quota
- Text generation: one explicit HTTPS `generateContent` request after exact-context review and three separate confirmations
- Speech generation: separately reviewed HTTPS request(s) containing one section's edited speech text and speech settings; long scripts are split at sentence boundaries where possible and otherwise at whole-word boundaries, and the review lists the exact segments; its disclosure has its own version

## Versioned model allowlist

| Model ID | Shipped description | Google-published Free Tier status checked 13 October 2026 | Published paid Standard input / output rate |
|---|---|---|---|
| `gemini-3.8-flash` | Default for detailed explanations; Google's model page describes long-horizon software engineering and complex tasks. | Standard input and output tokens listed as free of charge on the Free Tier. Availability and quota can change. This does not guarantee the user's request is free. | `$0.75 / $3.75` per million tokens through 31 December 2026; `$1.50 / $7.50` from 1 January 2027. |
| `gemini-3.1-flash-lite` | Lower-cost alternative for lightweight explanations; Google's model page describes high-volume, low-latency use. | Standard input and output tokens listed as free of charge on the Free Tier. Availability and quota can change. This does not guarantee the user's request is free. | `$0.25 / $1.50` per million tokens. |

The model list and rates are release metadata, not fetched from Google by the app. Code Atlas does not auto-discover models, silently change the chosen model, or retry through another model. A Google billing/quota rejection is shown as an error. Verify the linked pricing page before a release and update the disclosure version, check date, tests, and UI together.

## Data use and eligibility shown before send

The UI discloses that Google's Gemini API terms say prompts and generated responses submitted through unpaid services, including unpaid API quota, may be used to provide, improve, and develop Google products and machine-learning technologies; human reviewers may read, annotate, and process the input and output; and Google says not to submit sensitive, confidential, or personal information to unpaid services.

The UI also discloses the terms' EEA, Switzerland, and UK paid-service data-use exception for users in those regions, even when using unpaid Gemini API quota. It links to the terms so the user can check the version applicable to their account and region.

Google's developer terms require users to be at least 18, in a supported region, and use the service for professional or business development rather than consumer use. Code Atlas does not verify age, location, account, intent, billing, or quota.

The user sees three independent, initially unchecked acknowledgements before the Send button enables:

1. Privacy and data use, including possible human review and the regional terms exception.
2. Billing uncertainty, including the fact that Code Atlas cannot tell whether the API key's Cloud project has billing enabled. A Free Tier model does not guarantee a `$0` charge.
3. Age, supported region, and professional/business-use requirements.

## API key handling

The key is saved only in macOS Keychain under the Code Atlas service/account identifiers. The UI never reads the saved value back. Rust loads it into a zeroizing wrapper immediately before a confirmed request and sends it in the HTTPS `x-goog-api-key` header, marked sensitive by the HTTP client. The key is not part of the JSON body, request preview, local snapshot/chapter files, export, or application-authored logs.

Google's key guide says to treat keys like passwords and recommends API restrictions; it states that new AI Studio keys are authorization keys and that unrestricted standard keys are rejected by the Gemini API. Users create/restrict their key in Google AI Studio. Code Atlas cannot apply restrictions to the user's Google account.

## Request scope and consent lifetime

The request is assembled from one captured target file plus zero to eleven selected supporting files from the same immutable snapshot. Exact source contents, hashes, paths, prompt text, generation settings, token/byte estimate, provider, model, endpoint, encoded JSON body, body SHA-256, purpose, snapshot, and disclosure version are shown before send. Selected source is capped at 250 KB and the encoded request body at 512 KB. The target must be included. Secret-like, binary, invalid-encoding, skipped, and uncaptured files are rejected.

Consent belongs to a fingerprint of that exact request and disclosure. The request preview expires after fifteen minutes, can be used once, and stays only in application memory. There is no network call when opening settings, storing a key, loading the file list, changing context, or building/rebuilding the preview. After explicit Send, Code Atlas makes one HTTPS request with no retry, no model switch, no tool/grounding access, and no paid fallback. During that request the user can confirm **Cancel request**; Code Atlas drops its local HTTP future when possible, but Google may already have received the request. If local completion wins the race, a returned result may still appear as an unaccepted preview that requires review. Nothing is automatically accepted.

The PR Guide purpose is separate from snapshot chapter requests. Its preview is rebuilt from two local Git refs resolved to immutable commits and contains only the selected bounded diff hunks from up to twelve eligible text files. The exact selected diff, serialized evidence in the prompt, prompt/schema/settings, model, endpoint, byte/token estimates, and the same provider data-use, billing, and eligibility notices are shown before consent. The local secret-like scan checks selected paths and diff excerpts but cannot identify every secret; inspect the full diff before sending. Its fingerprint includes both resolved commits and selected evidence, and moving either ref or changing any selected input invalidates the request. A validated response is written only as a separate unaccepted PR Guide draft; claim citations must match exact selected diff lines and the correct base/head commit/blob anchor. A late cancellation race can still produce that unaccepted draft, which the user must inspect before explicitly accepting or rejecting it. PR Guide history never mutates file-chapter records or Git refs.

## Optional section narration

Narration has a separate record: `gemini-tts-disclosure-2026-10-13-v2`, checked 13 October 2026. The source currently selects model ID `gemini-3.8-flash-lite-tts` (displayed as Gemini 3.8 Flash-Lite TTS) and the listed prebuilt voices. These are static app values, not a live capability check.

The exact outbound narration request(s) contain one selected section's edited speech text after any explicitly enabled pronunciation substitutions, the selected voice, delivery style, and WAV response format. Scripts up to 16,000 UTF-8 bytes are split into sequential segments of at most 4,000 bytes, preferring sentence boundaries and otherwise whole-word boundaries; the request review shows the full text and each exact segment. Source files, citations, file paths, and the whole chapter are not included by default. The review is fingerprinted and expires after fifteen minutes. The user confirms the request and current terms before synthesis. A cache hit can be served locally after review without sending another provider request.

As checked in the source disclosure, Google lists this model's standard text and audio output as free on the Free Tier subject to project quota and availability. Code Atlas cannot inspect the key's project tier or remaining quota. The source disclosure lists paid-project rates of `$0.50` per million input text tokens and `$6.00` per million output audio tokens through 31 December 2026 (`$0.0015` per 10 seconds), then `$1.00` and `$12.00` from 1 January 2027. These prices and future rates must be rechecked against Google's pricing page before any release after the check date. A Free Tier label does not establish that a particular request is free or available.

Narration repeats the same source-level Google data-use, regional, eligibility, and billing notices as text generation. For Paid Services, the shipped notice describes Google's stated product-improvement and limited logging terms. The app cannot inspect the key's billing project, region, age, or eligibility. The request is sent directly to Google's Gemini API over HTTPS, without a retry or paid/model fallback; local cancellation cannot establish whether Google received a request already in flight. The transcript and previous cached audio remain available on error.

The speech disclosure and request behavior are defined in `src-tauri/src/narration.rs`; text-generation disclosures are defined in `src-tauri/src/provider.rs`. The dates and terms in this document are release metadata and must be reviewed before distribution.

## First-party sources

- [Gemini API pricing](https://ai.google.dev/gemini-api/docs/pricing)
- [Gemini API terms](https://ai.google.dev/gemini-api/terms)
- [Gemini API available regions](https://ai.google.dev/gemini-api/docs/available-regions)
- [Gemini API key guidance](https://ai.google.dev/gemini-api/docs/api-key)
- [Gemini models](https://ai.google.dev/gemini-api/docs/models)
- [Gemini `generateContent` API](https://ai.google.dev/gemini-api/docs/generate-content/text-generation)
- [Gemini speech generation](https://ai.google.dev/gemini-api/docs/speech-generation)
