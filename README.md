# Code Atlas

Code Atlas is an in-progress Mac-first desktop app for understanding a codebase from its actual files. The intended guide explains eligible files individually, links claims to captured source, and lets readers pause, inspect code, and choose when to continue. A repository overview is an orientation layer, not a substitute for file explanations.

The project currently runs from a source checkout; this repository does not provide a signed and notarized installer or a user download. Start with the [first-session guide](docs/GETTING_STARTED.md) for the native app workflow and its limits.

![Concept board showing Atlas, Code Lens, and Walkthrough](docs/design/platform-overview.png)

This image is a design concept, not a screenshot of the running app. Product behavior and release goals are described in the [blueprint](docs/PRODUCT_BLUEPRINT.md); they are not proof that every acceptance gate has passed.

## What the current source contains

- A Tauri desktop shell with local repository selection, bounded snapshot capture, Atlas, Code Lens, and a Rust index for a defined Rust, TypeScript, TSX, and JavaScript subset.
- Local file-by-file coverage, Chapter Studio editing and review, immutable JSON chapter revisions, and a manually paced Walkthrough with source-linked citations and typed visual hotspots.
- A **Compare local revisions** project tool for bounded, read-only comparisons of commits already present in a local Git repository. It shows file classifications, exact changed-line ranges, diff hunks, parser-backed declarations where all available source revisions parse cleanly, and distinct base/head commit and blob identities. A separate PR Guide panel can send selected diff evidence to Gemini only after exact request review and consent; generated claims must cite the correct base/head hunk, and accepting or rejecting appends separate local PR Guide history. It never fetches, checks out, or writes Git state.
- Walkthrough can export one self-contained HTML file from selected accepted revisions. It previews paths and excerpts, supports section navigation and supported typed diagrams, and can include narration audio already cached on this Mac; it does not include live Atlas/Code Lens destinations.
- Direct Gemini API paths for file chapters and an optional repository overview. A reviewed request sends only the selected captured context after explicit confirmation; generated content is validated and remains unaccepted until the user saves and reviews it.
- Optional Gemini text-to-speech for an edited section script. The speech request is reviewed separately and sends the selected script and voice settings, not source excerpts by default.

These are source-level capabilities, not a claim of a finished or released product. The only direct text-generation provider currently implemented is Gemini. OpenAI and Anthropic direct APIs, and agent backends such as Codex, Claude Agent, and Google Antigravity, are tracked as separate planned integrations in [M7](docs/kata-tasks/phase-7.md). None is presented as currently selectable in this codebase.

## Start here

- [Getting started](docs/GETTING_STARTED.md) — run the current Mac app from source and complete a local first session.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — diagnose repository, snapshot, chapter, provider, and narration issues.
- [Product blueprint](docs/PRODUCT_BLUEPRINT.md) — intended user journey, file coverage, trust rules, and release gates.
- [Interaction contract](docs/INTERACTION_CONTRACT.md) — controls, navigation destinations, side effects, and acceptance walkthrough.
- [Technical plan](docs/TECHNICAL_PLAN.md) — implemented architecture notes and clearly marked future boundaries.
- [Chapter schema](docs/CHAPTER_SCHEMA.md) — persisted chapter format, evidence validation, visuals, and revisions.
- [Provider disclosures](docs/PROVIDER_DISCLOSURES.md) — Gemini text and narration data-use, billing, eligibility, and request scope.
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

`npm run bundle:macos` is configured to build an unsigned `.app` and `.dmg` and verify their embedded metadata. It is a local packaging command, not a published installer or a signed/notarized release. The project does not yet declare a supported minimum macOS version or publish a user download.

## Data and provider behavior

Local browsing, capture, indexing, and opening saved chapters do not need an API key or provider call. Opening settings or preparing a request does not send one. A confirmed Gemini text request sends the selected captured source or, for a PR Guide, the exact selected local diff directly from the app to Google. A confirmed narration request sends the reviewed speech script and selected voice/style settings. Read the [provider disclosures](docs/PROVIDER_DISCLOSURES.md) before using either path.

## Project state and trust routes

The full desired experience is still being built. Fine-grained selective regeneration, a user-facing guide revision compare/restore workflow, polished onboarding, release hardening, relationship impact in Git comparisons, and the remaining PR Guide acceptance evidence are outstanding. The source now contains a reviewed local PR Guide workflow, but its Kata task has not been marked complete. The offline export path is present but has not passed the full offline-package acceptance workflow. Check the blueprint and Kata backlog for the target and outstanding gates.

The earlier prototype archive is kept locally under `deprecated/` in the original working copy and is intentionally excluded from this repository. It is not required to build or use the current app; use this README and `docs/` for the current project.

There is no `LICENSE` file in this checkout, so its license and reuse permissions are unspecified. No private security-reporting address, support contact, public issue tracker, or contribution review policy is published here. See [SECURITY.md](SECURITY.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for the current limits and local development instructions.

Kata project name: `code-atlas-rebuild`. Issue definitions live in [`docs/kata-tasks/`](docs/kata-tasks/); their task contracts describe intended work and should not be treated as proof that a release gate passed.
