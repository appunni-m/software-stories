# Codex validation — October 16, 2026

## Current result

**The packaged native app completed the installed-account Codex file-chapter flow.** With no saved executable selection, it discovered the desktop Codex, reused the existing ChatGPT account, loaded **`gpt-6-luna` / `max`**, and generated a source-cited chapter after local review and explicit consent. No direct API key was used. Claude remains deferred.

This is one synthetic native baseline, not full operation, quality or production acceptance. The user authorized builds, tests, computer use and a small synthetic account validation. Model requests retained the user's GPT-6 Luna/max constraint.

## Installed runtime and simpler setup

The runtime is the existing executable:

```text
/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex
```

It reports `0.162.0-alpha.17.2`. The older first PATH match (`0.149.1`) was the wrong selection. No Codex installation/update, credential copy or auth-file inspection was needed for the native flow. An earlier temporary official npm candidate was removed and was never selected in Code Atlas.

Selecting Codex now discovers the installed desktop runtime and connects to its existing account automatically. Installation paths and model controls are available under collapsed advanced settings. Setup/account/model observation sends no repository source; generation still requires a fresh reviewed request and consent. Disconnect stops this app's connection without logging the user out of their existing CLI.

## Supported protocol and reviewed-source boundary

The earlier adapter required nonexistent restricted-read fields in this runtime's schema. That integration blocker was resolved using the runtime's supported experimental **named permission profiles**, checked against its generated schema before inference.

The app passes the same fixed profile to thread and turn: minimal platform reads, reads of app-owned staged workspace roots, no filesystem writes and network disabled for model tools. It verifies effective configuration and the returned thread profile/read-only policy. Command, browser, computer, web, plugin, skill-discovery and multi-agent tools are disabled; inherited MCP servers are disabled in the thread configuration. Source staging remains bounded to reviewed captured context. The official Codex runtime retains its normal account-service connection.

The model-free [permission probe](../../scripts/verify-codex-permissions.py) passed three native checks with synthetic files: staged reads allowed, an outside file denied and staged writes denied. **No network probe was performed.** This does not establish adversarial configuration-race, execution identity or universal isolation qualification. The remaining limits in [review identity](../CODEX_REVIEW_IDENTITY.md) still apply.

Ignored unknown noncritical user preferences and the deliberate host-skill-discovery notice no longer abort a valid connection. Critical or unexplained warnings still reject the connection. Effective restrictions are checked independently; the app does not rewrite the user's Codex configuration.

## Native app evidence

The synthetic Git repository contained only [cart.ts](../../benchmarks/codex-account-smoke/cart.ts), SHA-256 `bd1859f143bc0f6eb5858ea58da490c7844b91602b5ca19e614e1d46c8f2680a`. Its source and Git state remained unchanged.

1. Opened the release `.app`, restored the synthetic snapshot/file and chose Codex with its saved executable preference temporarily absent.
2. Observed automatic desktop discovery, existing ChatGPT authentication and GPT-6 Luna/max selection.
3. Continued to source review, reviewed the fixed source/settings, consented and generated.
4. Received **“Price Subtotals and Taxed Totals”**, a validated unsaved preview containing two behavior sections and an evidence-limits section.
5. Saved **r1 Draft**, then explicitly accepted it as **r2 Accepted**, retaining separate revision history.
6. Opened a citation in Files: **Citation verified · lines 1–6**, exact UTF-8 range `[0,256)`.
7. Opened the section reader and used **Continue** to advance from section one to section two. The reader remained manually paced with source actions.

An earlier native response was rejected by chapter validation. Its raw response was not retained, so the specific cause is unknown. A later valid request completed; no validator was relaxed. Automatic bounded repair and useful validation diagnostics still need implementation/acceptance. This baseline does not prove every generated explanation will pass.

A separate native transport/common-validator diagnostic produced [the recorded synthetic proposal](../../benchmarks/codex-account-smoke/2026-10-16-native-proposal.json). Its title differs from the GUI chapter above. An offline regression now replays it without an index, validates three canonical sections, and rejects an invented source quote. It contains no credentials, account identifiers or private source. The temporary diagnostic module was removed. The [earlier minimal account result](../../benchmarks/codex-account-smoke/2026-10-16-result.json) remains separate historical evidence.

## Automated and package evidence

`CARGO_INCREMENTAL=0 npm run check` passed: offline corpus validation, **98 frontend tests, 185 Rust tests passed and two ignored**, production frontend build and Cargo check. The ignored installed-runtime metadata test was separately run with the selected Codex executable and passed; it performs account/model observation without inference. The other ignored test was not blanket-enabled.

`npm run bundle:macos` builds the local unsigned app and aarch64 DMG with metadata/checksum verification. Native checks used that release app. Eight native dead-code warnings and a frontend chunk-size warning remain. Unsigned local packaging is not signing, notarization, accessibility or release acceptance.

## Narration and remaining acceptance

The native Pocket setup initially failed before downloading Python. Reproduction exposed conflicting `--managed-python` and `UV_PYTHON_PREFERENCE` options. Setup now uses only the enforced `only-managed` preference, with a regression test; the corrected Python preparation command passed. The rebuilt app then completed Python, dependencies and pinned model/voice setup and reported **Pocket TTS is ready**. The first native synthesis exposed a second integration error: an outer inference-mode context created tensors that Pocket’s background no-grad cache worker could not mutate. The adapter now uses `torch.no_grad()`; a source regression guards that choice. A corrected offline worker generated a valid 4-second mono PCM16/24 kHz WAV from synthetic text. [PyTorch documents that inference mode is thread-local](https://docs.pytorch.org/docs/2.14/generated/torch.autograd.grad_mode.inference_mode.html); the exact installed Pocket 3.3.0 worker/thread implementation was inspected. Packaged playback/cache evidence is tracked below and in `q6g3`/`dbmd`.

Local narration now keeps script editing and technical audio metadata collapsed, removes the duplicate local request-review action/fingerprint, and presents **What you will hear → Generate locally and play**. Full reviewed text and all validation remain available.

The successful synthetic chapters returned `visual.kind: "none"`; no generated-diagram quality or narration quality was accepted. File generation has a native baseline, while overview, section repair, range, planned chapter, bounded plan, PR Guide and hunk operations still require their own native evidence. Cancellation, expiry, quota, account/runtime changes, crash/recovery, concurrency and adversarial boundaries remain open. Keep `z3an`, `2r6n`, `xymb` and release/quality gates open; Claude remains deferred in `8gqw`.

## Primary protocol references

- [Official named permissions](https://learn.chatgpt.com/docs/permissions).
- [Official app-server protocol](https://learn.chatgpt.com/docs/app-server); installed experimental schemas were checked independently.
- [Official Codex authentication](https://learn.chatgpt.com/docs/auth).
