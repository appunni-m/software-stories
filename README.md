# Code Atlas

Code Atlas is an in-progress Mac-first desktop app for understanding a codebase from its actual files. The intended guide explains eligible files individually, links claims to captured source, and lets readers pause, inspect code, and choose when to continue. A repository overview is an orientation layer, not a substitute for file explanations.

The project currently runs from a source checkout; this repository does not provide a signed and notarized installer or a user download. Start with the [first-session guide](docs/GETTING_STARTED.md) for the native app workflow and its limits.

![Concept board showing Atlas, Code Lens, and Walkthrough](docs/design/platform-overview.png)

This image is a design concept, not a screenshot of the running app. Product behavior and release goals are described in the [blueprint](docs/PRODUCT_BLUEPRINT.md); they are not proof that every acceptance gate has passed.

## What the current source contains

- A Tauri desktop shell with local repository selection, bounded snapshot capture, Atlas, Code Lens, and a Rust index for a defined Rust, TypeScript, TSX, and JavaScript subset.
- Local file-by-file coverage, Chapter Studio editing and review, immutable JSON chapter revisions, and a manually paced Walkthrough with source-linked citations and typed visual hotspots.
- A **Compare local revisions** project tool for bounded, read-only comparisons of commits already present in a local Git repository. It shows file classifications, exact changed-line ranges, diff hunks, parser-backed declarations where all available source revisions parse cleanly, and distinct base/head commit and blob identities. A separate PR Guide panel can send selected diff evidence through an explicitly selected Gemini, OpenAI, or Anthropic API after exact request review and consent; generated claims must cite the correct base/head hunk, and accepting or rejecting appends separate local PR Guide history. Each eligible text hunk also has a transient **Explain hunk** flow: it reviews and sends only that hunk, returns locally validated base/head citations, and writes neither PR Guide history nor Git state. These local comparison flows never fetch, check out, or write Git state.
- Walkthrough can export one self-contained HTML file from selected accepted revisions. Its preview reports audio availability per section and pairs included cached tracks with their reviewed transcript; it also previews paths and excerpts and supports section navigation and supported typed diagrams. Live Atlas/Code Lens destinations are unavailable offline.
- Direct Gemini, OpenAI, and Anthropic API paths for file chapters, optional repository orientation, selective section updates, PR Guides, and transient single-hunk PR change explanations. A reviewed request sends only the selected captured context after explicit confirmation; persisted chapter/PR Guide content remains unaccepted until the user reviews it.
- Optional Gemini text-to-speech for an edited section script. The speech request is reviewed separately and sends the selected script and voice settings, not source excerpts by default.

These are source-level capabilities, not a claim of a finished or released product. Gemini, OpenAI, and Anthropic are selectable direct APIs for the request paths listed above, including transient Code Lens range and single-hunk PR change explanations. Those explanation results are locally citation-checked and temporary; they do not create chapters or PR Guide history. Offline fixtures now cover reviewed dispatch, complete-consent gating, changed-review rejection, and single-call synthetic quota failures for each currently wired operation/provider pair. Provider-specific HTTP error mapping, credential non-disclosure through the real transport, complete history-preservation coverage across saved operations, cancellation/transport-close behavior, and live provider eligibility remain incomplete. Coding-agent backends such as Codex, Claude Agent, Gemini CLI, and Google Antigravity remain separate planned integrations in [M7](docs/kata-tasks/phase-7.md).

## Start here

- [Getting started](docs/GETTING_STARTED.md) — run the current Mac app from source and complete a local first session.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — diagnose repository, snapshot, chapter, provider, and narration issues.
- [Local data and permissions](docs/LOCAL_DATA.md) — find local snapshots, understand Keychain access, exports, and deletion.
- [Product blueprint](docs/PRODUCT_BLUEPRINT.md) — intended user journey, file coverage, trust rules, and release gates.
- [Interaction contract](docs/INTERACTION_CONTRACT.md) — controls, navigation destinations, side effects, and acceptance walkthrough.
- [Technical plan](docs/TECHNICAL_PLAN.md) — implemented architecture notes and clearly marked future boundaries.
- [Chapter schema](docs/CHAPTER_SCHEMA.md) — persisted chapter format, evidence validation, visuals, and revisions.
- [Provider disclosures](docs/PROVIDER_DISCLOSURES.md) — direct API provider and Gemini narration data-use, billing, eligibility, and request scope.
- [Chapter quality gate](docs/CHAPTER_QUALITY_GATE.md) — synthetic corpus and human-reviewed release report workflow.
- [Kata backlog](docs/KATA_BACKLOG.md) — milestone map and issue definitions.
- [Design concepts](docs/design/) — visual references for the three connected workspaces.

## Local development

The lockfile's Vite toolchain requires Node.js 20.19 or newer in the 20.x line, or 22.12 or newer. The project does not declare a formal Node or Rust minimum, and it does not pin a Rust toolchain. Use a current stable Rust toolchain with Cargo for native commands.

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

Local browsing, capture, indexing, and opening saved chapters do not need an API key or provider call. Opening settings or preparing a request does not send one. A confirmed direct text request sends the exact reviewed snapshot context or PR diff to the explicitly selected Gemini, OpenAI, or Anthropic API. A confirmed narration request sends the reviewed speech script and selected voice/style settings to Google's Gemini speech API. Read the [provider disclosures](docs/PROVIDER_DISCLOSURES.md) before using either path.

## Project state and trust routes

The full desired experience is still being built. Single-section stale-guide updates and the user-facing revision compare/restore workflow are implemented; one request cannot update several stale sections as a batch. Polished onboarding, release hardening, relationship impact in Git comparisons, and the remaining PR Guide and offline-export acceptance evidence are still outstanding. The source contains the reviewed local PR Guide and self-contained offline export workflows, but their Kata acceptance walkthroughs have not been completed. Check the blueprint and Kata backlog for the target and outstanding gates.

The earlier prototype archive is kept locally under `deprecated/` in the original working copy and is intentionally excluded from this repository. It is not required to build or use the current app; use this README and `docs/` for the current project.

There is no `LICENSE` file in this checkout, so its license and reuse permissions are unspecified. No private security-reporting address, support contact, public issue tracker, or contribution review policy is published here. See [SECURITY.md](SECURITY.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for the current limits and local development instructions.

Kata project name: `code-atlas-rebuild`. Issue definitions live in [`docs/kata-tasks/`](docs/kata-tasks/); their task contracts describe intended work and should not be treated as proof that a release gate passed.
