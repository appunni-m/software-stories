# Code Atlas

Code Atlas is an in-progress Mac-first desktop app for understanding a codebase from its actual files. The intended guide explains eligible files individually, links claims to captured source, and lets readers pause, inspect code, and choose when to continue. A repository overview is an orientation layer, not a substitute for file explanations.

The project currently runs from a source checkout; this repository does not provide a signed and notarized installer or a user download. Start with the [first-session guide](docs/GETTING_STARTED.md) for the native app workflow and its limits.

The current source implements a [conversational developer workspace](docs/design/WORKSPACE_EXPLORATION.md): a left activity bar for Explorer, Search, Changes and Explorations; a pinned chat composer; captured source beside Guide / Flow / Chat history. Follow-ups stack at the current level, and **Explore further** opens a branch attached to its answer. Codex discovers supporting implementation automatically after a revocable snapshot sharing permission. Connection setup is reused from the header.

The [interactive design preview](workspace-preview.html) uses sample answers and is separate from the native application. Run `npm run dev` from the repository root to open it at `http://127.0.0.1:4173/workspace-preview.html`. Earlier Guide / Files / Changes screenshots and the Atlas / Code Lens / Walkthrough concept board are historical references. The [all-page audit](docs/kata-tasks/64-workspace-ui-audit.md) tracks packaged design acceptance; builds and regression checks do not establish visual quality or release readiness.

## What the current source contains

**Current delivery: Codex first, without a mandatory API key.** The user deferred automatic Claude support to later work (`8gqw`). Codex has source adapters for file chapters, overviews, section repairs, exact code selections, PR Guides, hunks and bounded reading plans. Building/testing and a small synthetic installed-account validation are now authorized; source wiring alone does not establish accepted support. Claude generation stays unavailable behind its empty qualification registry. Full multi-backend support remains open. See [required operations](docs/AGENT_OPERATION_PARITY.md), [agent setup](docs/AGENT_GENERATION.md) and [the current Codex gate](docs/kata-tasks/45-agent-first-generation.md).

- A Tauri desktop shell with a persistent explorer, conversation workspace and Changes activity, local repository selection, bounded snapshot capture, immutable source browsing beside explanations, an optional relationship map, and a Rust index for bounded Rust, TypeScript/TSX, JavaScript, Python, and Go subsets. These are partial structural capabilities, not language qualification; see the [language capability matrix](docs/LANGUAGE_CAPABILITIES.md).
- Local file-by-file coverage, Chapter Studio editing and review, immutable JSON chapter revisions, and a manually paced Walkthrough with source-linked citations and typed visual hotspots.
- A **Compare local revisions** project tool for bounded, read-only comparisons of commits already present in a local Git repository. It shows file classifications, exact changed-line ranges, diff hunks, parser-backed declarations where all available source revisions parse cleanly, and distinct base/head commit and blob identities. A separate PR Guide panel defaults to Codex with a ChatGPT account and also offers explicitly selected Gemini, OpenAI, or Anthropic APIs after exact request review and consent; generated claims must cite the correct base/head hunk, and accepting or rejecting appends separate local PR Guide history. Each eligible text hunk also has a transient **Explain hunk** flow: it reviews and sends only that hunk, returns locally validated base/head citations, and writes neither PR Guide history nor Git state. These local comparison flows never fetch, check out, or write Git state.
- The Guide reader can export one self-contained HTML file from selected accepted revisions. Its preview reports section/scene audio coverage and matching reviewed transcripts, paths and excerpts. HTML visual compositions and cached scene clips are included with manual navigation; graphs use source outlines. Live local Files destinations and automatic scene playback are unavailable offline. See [offline use and limits](docs/OFFLINE_EXPORT.md).
- Direct Gemini, OpenAI, and Anthropic API paths for file chapters, optional repository orientation, selective section updates, PR Guides, and transient single-hunk PR change explanations. A reviewed request sends only the selected captured context after explicit confirmation; persisted chapter/PR Guide content remains unaccepted until the user reviews it.
- A Codex account route for file chapters and an overview of 1–12 selected captured files: managed app-server over stdio, browser sign-in, runtime model/reasoning choices, reviewed source, progress/cancellation, and locally validated unaccepted drafts. One-section stale repairs now share the scoped evidence builder and preservation validator. Exact Files source ranges, PR Guides and single-hunk explanations now share immutable context, managed dispatch and the existing citation validators; range/hunk results stay transient and guides use separate unaccepted PR history. A synthetic packaged-app file chapter has completed generation, draft save, acceptance, citation navigation and manual section advance through the existing ChatGPT account. Other operations and full runtime/account/distribution qualification remain open.
- Optional Pocket TTS narration, configured once in **Connections → Voice** and used across answers. On Apple Silicon Macs running macOS 14 or later, **Prepare voice** downloads the pinned runtime, English model, and Alba voice into app storage; setup needs internet, disk space, and available memory, but no preinstalled `uv` or Python. After setup, narration is synthesized locally without an API key, provider usage fee, or listening server. Pocket/Alba is the default; the single offered voice is Alba MacKenna, credited under CC BY 4.0 in Voice details. Repair is available under **Voice details and troubleshooting** after setup succeeds. Gemini speech is a separate, explicitly selected cloud option with exact-text review and consent; there is no automatic fallback. See [local narration setup and limits](docs/GETTING_STARTED.md#edit-narrate-and-take-a-guide-offline), [provider disclosures](docs/PROVIDER_DISCLOSURES.md), and [Pocket runtime](docs/research/pocket-tts-runtime.md).

- **Question-led explanations**: ask in the pinned composer; Codex selects implementation using captured paths, symbols and static relationships, then saves a cited chapter draft. Code is the source of truth; docs are hints or explicitly deprecated. The earlier route planner and sequential chapter-batch implementation remain in source, but their staged Guide wizard is superseded by the new workspace and is not a normal navigation destination. Multi-chapter orchestration remains tracked in `a1sm`. See the [current workspace flow](docs/design/WORKSPACE_EXPLORATION.md); [AI guide flow](docs/AI_GUIDE_FLOW.md) retains the historical route contracts.

These are capabilities under development, not a finished or released product. **October 16 workspace regression validation:** 133 frontend tests, 203 Rust tests and the production build pass (three explicit live checks are ignored by default). Earlier installed-account metadata and a GPT-6 Luna/max synthetic route validation passed separately; see the [AI guide flow evidence](docs/acceptance/2026-10-16-ai-guide-flow.md). The unsigned native app automatically discovered the existing desktop Codex and completed a GPT-6 Luna/max synthetic chapter: generation → draft → accepted revision → clickable source → manual section navigation. See [dated evidence and remaining work](docs/acceptance/2026-10-16-codex-validation.md). Native Pocket setup, section audio playback and cache reuse also passed a synthetic baseline. Other operations, reliability, narration quality, visual quality, design parity, accessibility and release readiness remain open. Claude, Gemini CLI and Antigravity automatic support remains later work in [M7](docs/kata-tasks/phase-7.md).

## Start here

- [Getting started](docs/GETTING_STARTED.md) — run the current Mac app from source and complete a local first session.
- [0.1.0 release readiness draft](docs/releases/0.1.0-draft.md) — unreleased status, parser coverage, capture limits, and open gates.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — diagnose repository, snapshot, chapter, provider, and narration issues.
- [Language capability matrix](docs/LANGUAGE_CAPABILITIES.md) — see which source and structure operations are available and which still need qualification.
- [Local data and permissions](docs/LOCAL_DATA.md) — find local snapshots, understand Keychain access, exports, and deletion.
- [Product blueprint](docs/PRODUCT_BLUEPRINT.md) — intended user journey, file coverage, trust rules, and release gates.
- [Interaction contract](docs/INTERACTION_CONTRACT.md) — controls, navigation destinations, side effects, and acceptance walkthrough.
- [Technical plan](docs/TECHNICAL_PLAN.md) — implemented architecture notes and clearly marked future boundaries.
- [Repository indexing and context research](docs/research/2026-10-17-repository-context-and-indexing.md) — twelve product/toolkit approaches, reusable indexing options and the proposed retrieval architecture; native range retrieval is measured, with a synthetic account-backed tool-loop baseline; large-repository and complete-guide acceptance remain pending.
- [Chapter schema](docs/CHAPTER_SCHEMA.md) — persisted chapter format, evidence validation, visuals, and revisions.
- [Provider disclosures](docs/PROVIDER_DISCLOSURES.md) — direct APIs, the initial Codex account route, local Pocket narration, and optional Gemini speech.
- [Chapter quality gate](docs/CHAPTER_QUALITY_GATE.md) — synthetic corpus and human-reviewed release report workflow.
- [Kata backlog](docs/KATA_BACKLOG.md) — milestone map and issue definitions.
- [Current Kata status](docs/KATA_STATUS.md) — a dated snapshot of live issue counts, priorities, definitions, and recent acceptance evidence.
- [Historical pending-work notes](docs/PENDING_WORK.md) — an October 15 engineering checkpoint with later dated updates; use the current Kata snapshot for issue status.
- [Current workspace design](docs/design/WORKSPACE_EXPLORATION.md) — interaction contract, standard components, preview and implementation tasks.
- [Earlier screen proposals](docs/design/PROPOSED_SIMPLIFICATION.md) — historical Guide / Files / Changes references.

The [local retrieval checkpoint](docs/acceptance/2026-10-17-range-retrieval.md) records native range search and its limits. The [owned Codex investigation checkpoint](docs/acceptance/2026-10-17-codex-owned-retrieval.md) adds one live GPT-6 Luna/max synthetic answer using iterative tools: five calls and 197 distinct source bytes. Workspace chat now uses that route in source. Fontdone broad/focused explanations, speed and packaged-app acceptance remain open.

The [versioned visual-scene checkpoint](docs/acceptance/2026-10-17-scene-v2.md) adds concrete transformation checks while preserving saved guides. A fresh broad run remains partial after three drafts; installed quality qualification remains pending.

The [saved-route replay correction](docs/acceptance/2026-10-17-saved-route-replay.md)
preserves older route digests and chapter links across known record/prompt
formats. Local regressions reopen a real saved synthetic route; packaged
upgrade and full repository-guide acceptance remain pending.

The [latest Fontdone qualification](docs/acceptance/2026-10-17-fontdone-source-value.md)
preserves three drafts through normal-gray rasterization/bitmap output, but still
ends partial near ten minutes. Its source-valid code-card scenes still have
misleading arrows and dense narration. Full completion, visual-story, listening
and installed usability gates remain open. [Whole-script Pocket ownership](docs/acceptance/2026-10-17-pocket-whole-script.md)
now loads the model once per script; a real isolated worker sample and mechanical
checks pass, without claiming native playback or speech-quality acceptance.

## Local development

The lockfile's Vite toolchain requires Node.js 20.19 or newer in the 20.x line, or 22.12 or newer. The Rust crate uses edition 2024 ([edition release](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)); locked Tantivy 0.26.2 declares Rust 1.86 as its minimum. The project does not set Cargo's `rust-version` or pin a toolchain, and the complete dependency graph has not been qualified at that floor. Use current stable Rust with Cargo for native commands (this checkpoint used rustc 1.98.1).

```sh
npm ci
npm run dev
```

`npm run dev` starts the Vite browser preview at `http://127.0.0.1:4173`. The port is fixed; startup fails if it is already in use, so the preview does not silently move to another port. It is not equivalent to the native app: native repository picking, Keychain access, and provider operations require the Tauri shell.

```sh
npm run tauri dev
npm run check
```

`npm run check` runs, in order: the native quality-helper build, synthetic chapter-corpus validation, every Node test in `tests/*.test.mjs`, the TypeScript/Vite build, Rust library tests, and `cargo check` for the Tauri crate.

`npm run quality:metadata` (after `npm run quality:build`) prints the fingerprints used by a chapter-quality review. `npm run quality:gate -- --report benchmarks/chapter-quality/review/reports/run.json` checks a completed human-reviewed report; generating metadata alone does not pass that gate.

`npm run bundle:macos` is configured to request an unsigned `.app` and `.dmg` and verify their identity and embedded schema metadata. A macOS GitHub Actions preflight is configured to run that command on pull requests and pushes to `main`; it does not sign, notarize, or publish the bundle. The packaged app declares macOS 14 as its minimum; a signed user-download artifact has not been qualified.

After changing the npm or Cargo dependency graph, regenerate the packaged license inventory with `cargo install cargo-about --version 0.9.2 --locked` followed by `npm run licenses:generate`. The macOS bundle command checks the recorded manifest, lockfile, icon, and bundled-license fingerprints before packaging.

## Data and provider behavior

Local browsing, capture, indexing, and opening saved chapters do not need an API key or provider call. Building a review sends no source. A confirmed direct text request sends the reviewed context to the selected API. When Codex is selected, the header restores the saved connection at startup and generation reuses it. Restoration, Connect and model loading can start the managed runtime and contact its account services; workspace question/metadata and agent-selected captured ranges are shared under the versioned snapshot permission; legacy reviewed runs/batches retain their exact source confirmations. Sign-in is an explicit one-time setup when needed; model/reasoning preferences are shared across projects. Disconnect pauses automatic reuse until you connect again. This initial route uses the runtime's native managed ChatGPT login, not a Code Atlas implementation of direct Sign in with ChatGPT. Runtime model listings do not prove account entitlement or remaining quota.

Pocket setup downloads dependencies and fixed assets; subsequent synthesis is configured to use local files and keep the script on this Mac. A separately selected and confirmed Gemini speech request sends the reviewed script and voice/style settings to Google. Read the [provider disclosures](docs/PROVIDER_DISCLOSURES.md) and [Pocket runtime limits](docs/research/pocket-tts-runtime.md) before using these paths. The October 16 native Pocket baseline completed setup, synthetic section synthesis, playback/replay/pause and cache reuse. Listening quality, clean-account installation, cancellation and export qualification remain open.

## Project state and trust routes

The full desired experience is still being built. Single-section stale-guide updates and the user-facing revision compare/restore workflow are implemented; one request cannot update several stale sections as a batch. Static cross-file relationship evidence in local Git comparisons is implemented with revision-specific source anchors and explicit partial-result reporting. Polished onboarding, release hardening, and the remaining PR Guide and offline-export acceptance evidence are still outstanding. The source contains the reviewed local PR Guide and self-contained offline export workflows, but their Kata acceptance walkthroughs have not been completed. Check the blueprint and Kata backlog for the target and outstanding gates.

## License

Code Atlas source code authored by its contributors is licensed under the [MIT License](LICENSE). The MIT license does not grant rights to third-party source captured for evaluation, generated explanations or narration, local repository data, or design/evaluation artifacts whose terms are not stated. Read the [third-party inventory](THIRD_PARTY_NOTICES.md) and the [full dependency and bundled-asset license texts](THIRD_PARTY_LICENSE_TEXTS.txt); do not treat this repository license as permission to reuse excluded material.

The [public GitHub repository](https://github.com/appunni-m/software-stories) exposes an [Issues page](https://github.com/appunni-m/software-stories/issues), but no triage or response policy is published. Do not post credentials, private source, or vulnerability details there. No monitored private security-reporting route, general support contact, maintainer contact, or contribution review policy is published. See [SECURITY.md](SECURITY.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for the current limits and local development instructions.

Kata project name: `code-atlas-rebuild`. Issue definitions live in [`docs/kata-tasks/`](docs/kata-tasks/); their task contracts describe intended work and should not be treated as proof that a release gate passed.

## Conversational developer workspace

The native workspace source uses a left activity bar for Explorer, Search, Changes and exploration history. Ask at the top; inspect captured source beside the explanation; switch Guide / Flow / Chat history within an exploration. Follow-ups stack at the current level. Explore further creates a branch attached to its answer, with parent/root navigation. Editing, versioned chapter review, source-linked visuals and Pocket narration remain available from each explanation. Configure AI and voice in **Connections** once; **Listen** prepares or reuses local audio without showing engine/setup forms on the answer. See the [native Connections observations](docs/acceptance/2026-10-16-connections.md).

New workspace generation uses the connected Codex account and discovers supporting code automatically. The first question asks for permission to share context from the selected snapshot; that permission is revocable and checked natively against the snapshot, account and runtime. Explanations save as drafts, not automatically accepted truth. Source discovery and generation run without per-stage dialogs, and reading another file or opening settings does not cancel them. An app restart does not automatically resend an interrupted question.

Search covers captured paths, indexed symbol names and a bounded local scan of immutable captured text. Refine the query when a search limit is reached. Changes includes up to 80 local commits and one-click parent comparison. This is a read-only repository explorer. The [workspace contract](docs/design/WORKSPACE_EXPLORATION.md) and [all-page UI audit](docs/kata-tasks/64-workspace-ui-audit.md) describe remaining qualification work. The separate browser design preview uses illustrative checkout data and no AI.
