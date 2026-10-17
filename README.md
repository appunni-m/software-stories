# Code Atlas

Code Atlas is an in-progress Mac-first desktop app for understanding a codebase from its actual files. The intended guide explains eligible files individually, links claims to captured source, and lets readers pause, inspect code, and choose when to continue. A repository overview is an orientation layer, not a substitute for file explanations.

The project currently runs from a source checkout; this repository does not provide a signed and notarized installer or a user download. Start with the [first-session guide](docs/GETTING_STARTED.md) for the native app workflow and its limits.

The current source implements a [conversational developer workspace](docs/design/WORKSPACE_EXPLORATION.md): a left activity bar for Explorer, Search, Changes and Explorations; a pinned chat composer; captured source beside Guide / Flow / Chat history. Follow-ups stack at the current level, and **Explore further** opens a branch attached to its answer. Codex discovers supporting implementation automatically after a revocable snapshot sharing permission. Connection setup is reused from the header.

The [interactive design preview](http://127.0.0.1:4173/workspace-preview.html) uses sample answers and is separate from the native application. Earlier Guide / Files / Changes screenshots and the Atlas / Code Lens / Walkthrough concept board are historical references. The [all-page audit](docs/kata-tasks/64-workspace-ui-audit.md) tracks packaged design acceptance; builds and regression checks do not establish visual quality or release readiness.

## What the current source contains

**Current delivery: Codex first, without a mandatory API key.** The user deferred automatic Claude support to later work (`8gqw`). Codex has source adapters for file chapters, overviews, section repairs, exact code selections, PR Guides, hunks and bounded reading plans. Building/testing and a small synthetic installed-account validation are now authorized; source wiring alone does not establish accepted support. Claude generation stays unavailable behind its empty qualification registry. Full multi-backend support remains open. See [required operations](docs/AGENT_OPERATION_PARITY.md), [agent setup](docs/AGENT_GENERATION.md) and [the current Codex gate](docs/kata-tasks/45-agent-first-generation.md).

- A Tauri desktop shell with a persistent explorer, conversation workspace and Changes activity, local repository selection, bounded snapshot capture, immutable source browsing beside explanations, an optional relationship map, and a Rust index for a bounded Rust, TypeScript/TSX, and JavaScript subset. These are partial structural capabilities, not language qualification; see the [language capability matrix](docs/LANGUAGE_CAPABILITIES.md).
- Local file-by-file coverage, Chapter Studio editing and review, immutable JSON chapter revisions, and a manually paced Walkthrough with source-linked citations and typed visual hotspots.
- A **Compare local revisions** project tool for bounded, read-only comparisons of commits already present in a local Git repository. It shows file classifications, exact changed-line ranges, diff hunks, parser-backed declarations where all available source revisions parse cleanly, and distinct base/head commit and blob identities. A separate PR Guide panel defaults to Codex with a ChatGPT account and also offers explicitly selected Gemini, OpenAI, or Anthropic APIs after exact request review and consent; generated claims must cite the correct base/head hunk, and accepting or rejecting appends separate local PR Guide history. Each eligible text hunk also has a transient **Explain hunk** flow: it reviews and sends only that hunk, returns locally validated base/head citations, and writes neither PR Guide history nor Git state. These local comparison flows never fetch, check out, or write Git state.
- The Guide reader can export one self-contained HTML file from selected accepted revisions. Its preview reports audio availability per section and pairs included cached tracks with their reviewed transcript; it also previews paths and excerpts and supports section navigation and supported typed diagrams. Live local Files destinations are unavailable offline.
- Direct Gemini, OpenAI, and Anthropic API paths for file chapters, optional repository orientation, selective section updates, PR Guides, and transient single-hunk PR change explanations. A reviewed request sends only the selected captured context after explicit confirmation; persisted chapter/PR Guide content remains unaccepted until the user reviews it.
- A Codex account route for file chapters and an overview of 1–12 selected captured files: managed app-server over stdio, browser sign-in, runtime model/reasoning choices, reviewed source, progress/cancellation, and locally validated unaccepted drafts. One-section stale repairs now share the scoped evidence builder and preservation validator. Exact Files source ranges, PR Guides and single-hunk explanations now share immutable context, managed dispatch and the existing citation validators; range/hunk results stay transient and guides use separate unaccepted PR history. A synthetic packaged-app file chapter has completed generation, draft save, acceptance, citation navigation and manual section advance through the existing ChatGPT account. Other operations and full runtime/account/distribution qualification remain open.
- Optional Pocket TTS narration, configured once in **Connections → Voice** and used across answers. Setup downloads an app-managed runtime and pinned English model/Alba voice on Apple Silicon Macs; it currently requires an existing `uv` installation. Generation uses a finite local worker per segment, without an API key or listening server. Legacy Google preferences and cached audio remain distinct. The global voice connection defaults to Pocket; Gemini speech is available by explicit selection in Connections and requires consent before sending.

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
- [Repository indexing and context research](docs/research/2026-10-17-repository-context-and-indexing.md) — twelve product/toolkit approaches, reusable indexing options and the proposed retrieval architecture; native range retrieval is measured, while agent retrieval and complete guides remain pending.
- [Chapter schema](docs/CHAPTER_SCHEMA.md) — persisted chapter format, evidence validation, visuals, and revisions.
- [Provider disclosures](docs/PROVIDER_DISCLOSURES.md) — direct APIs, the initial Codex account route, local Pocket narration, and optional Gemini speech.
- [Chapter quality gate](docs/CHAPTER_QUALITY_GATE.md) — synthetic corpus and human-reviewed release report workflow.
- [Kata backlog](docs/KATA_BACKLOG.md) — milestone map and issue definitions.
- [Current pending work](docs/PENDING_WORK.md) — October 15 static fixes, remaining bugs, Pocket TTS integration and acceptance tasks.
- [Current workspace design](docs/design/WORKSPACE_EXPLORATION.md) — interaction contract, standard components, preview and implementation tasks.
- [Earlier screen proposals](docs/design/PROPOSED_SIMPLIFICATION.md) — historical Guide / Files / Changes references.

The [repository retrieval checkpoint](docs/acceptance/2026-10-17-range-retrieval.md) records native range-search implementation and its measured limits. Important large files are searchable, but broad single-query recall is 2/12 expected spans; Codex generation still needs the planned retrieval tool loop.

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

`npm run check` runs, in order: synthetic chapter-corpus validation, every Node test in `tests/*.test.mjs`, the TypeScript/Vite build, Rust library tests, and `cargo check` for the Tauri crate.

`npm run quality:metadata` prints the fingerprints used by a chapter-quality review. `npm run quality:gate -- --report benchmarks/chapter-quality/review/reports/run.json` checks a completed human-reviewed report; generating metadata alone does not pass that gate.

`npm run bundle:macos` is configured to request an unsigned `.app` and `.dmg` and verify their identity and embedded schema metadata. A macOS GitHub Actions preflight is configured to run that command on pull requests and pushes to `main`; it does not sign, notarize, or publish the bundle. No user-download artifact has been verified, and the project does not yet declare a supported minimum macOS version.

## Data and provider behavior

Local browsing, capture, indexing, and opening saved chapters do not need an API key or provider call. Building a review sends no source. A confirmed direct text request sends the reviewed context to the selected API. When Codex is selected, the header restores the saved connection at startup and generation reuses it. Restoration, Connect and model loading can start the managed runtime and contact its account services; discovery metadata is sent only after confirmation, and source contents are supplied only for a confirmed reviewed generation run or chapter batch. Sign-in is an explicit one-time setup when needed; model/reasoning preferences are shared across projects. Disconnect pauses automatic reuse until you connect again. This initial route uses the runtime's native managed ChatGPT login, not a Code Atlas implementation of direct Sign in with ChatGPT. Runtime model listings do not prove account entitlement or remaining quota.

Pocket setup downloads dependencies and fixed assets; subsequent synthesis is configured to use local files and keep the script on this Mac. A separately selected and confirmed Gemini speech request sends the reviewed script and voice/style settings to Google. Read the [provider disclosures](docs/PROVIDER_DISCLOSURES.md) and [Pocket runtime limits](docs/research/pocket-tts-runtime.md) before using these paths. The October 16 native Pocket baseline completed setup, synthetic section synthesis, playback/replay/pause and cache reuse. Listening quality, clean-account installation, cancellation and export qualification remain open.

## Project state and trust routes

The full desired experience is still being built. Single-section stale-guide updates and the user-facing revision compare/restore workflow are implemented; one request cannot update several stale sections as a batch. Static cross-file relationship evidence in local Git comparisons is implemented with revision-specific source anchors and explicit partial-result reporting. Polished onboarding, release hardening, and the remaining PR Guide and offline-export acceptance evidence are still outstanding. The source contains the reviewed local PR Guide and self-contained offline export workflows, but their Kata acceptance walkthroughs have not been completed. Check the blueprint and Kata backlog for the target and outstanding gates.

There is no `LICENSE` file in this checkout, so its license and reuse permissions are unspecified. The [public GitHub repository](https://github.com/appunni-m/software-stories) exposes an [Issues page](https://github.com/appunni-m/software-stories/issues), but no triage or response policy is published. Do not post credentials, private source, or vulnerability details there. No monitored private security-reporting route, general support contact, maintainer contact, or contribution review policy is published. See [SECURITY.md](SECURITY.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for the current limits and local development instructions.

Kata project name: `code-atlas-rebuild`. Issue definitions live in [`docs/kata-tasks/`](docs/kata-tasks/); their task contracts describe intended work and should not be treated as proof that a release gate passed.

## Conversational developer workspace

The native workspace source uses a left activity bar for Explorer, Search, Changes and exploration history. Ask at the top; inspect captured source beside the explanation; switch Guide / Flow / Chat history within an exploration. Follow-ups stack at the current level. Explore further creates a branch attached to its answer, with parent/root navigation. Editing, versioned chapter review, source-linked visuals and Pocket narration remain available from each explanation. Configure AI and voice in **Connections** once; **Listen** prepares or reuses local audio without showing engine/setup forms on the answer. See the [native Connections observations](docs/acceptance/2026-10-16-connections.md).

New workspace generation uses the connected Codex account and discovers supporting code automatically. The first question asks for permission to share context from the selected snapshot; that permission is revocable and checked natively against the snapshot, account and runtime. Explanations save as drafts, not automatically accepted truth. Source discovery and generation run without per-stage dialogs, and reading another file or opening settings does not cancel them. An app restart does not automatically resend an interrupted question.

Search covers captured paths, indexed symbol names and a bounded local scan of immutable captured text. Refine the query when a search limit is reached. Changes includes up to 80 local commits and one-click parent comparison. This is a read-only repository explorer. The [workspace contract](docs/design/WORKSPACE_EXPLORATION.md) and [all-page UI audit](docs/kata-tasks/64-workspace-ui-audit.md) describe remaining qualification work. The separate browser design preview uses illustrative checkout data and no AI.
