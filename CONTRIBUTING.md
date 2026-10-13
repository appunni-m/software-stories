# Contributing

Code Atlas is an in-progress desktop app. Product behavior and open delivery gates are recorded in the [blueprint](docs/PRODUCT_BLUEPRINT.md) and [Kata backlog](docs/KATA_BACKLOG.md). The Kata task files are scoped contracts; check the implementation before assuming a behavior exists.

For a source-backed first run, see [Getting started](docs/GETTING_STARTED.md). For local setup and workflow failures, see [Troubleshooting](docs/TROUBLESHOOTING.md).

## Prerequisites

- Node.js 20.19 or newer in the 20.x line, or 22.12 or newer, as required by the locked Vite toolchain.
- npm, using the checked-in `package-lock.json`.
- A current stable Rust toolchain and Cargo for Tauri and Rust commands. The workspace does not currently pin a Rust toolchain or declare an MSRV.
- macOS for exercising the native desktop shell and macOS Keychain integrations.

## Set up and work locally

```sh
npm ci
npm run dev
```

The Vite preview is available at `http://127.0.0.1:4173` with strict port binding; if it is occupied, startup fails instead of opening another port. Reuse the existing preview process rather than starting a second one. It is a browser preview; native repository picking, Keychain access, and provider calls need the Tauri shell:

```sh
npm run tauri dev
```

Before handing off a change, run:

```sh
npm run check
```

That command validates the synthetic chapter corpus, runs all Node test files, builds the TypeScript/Vite app, runs the Rust library tests, and checks the Rust crate. For a chapter-quality report, see [Chapter quality gate](docs/CHAPTER_QUALITY_GATE.md); a generated metadata file is not a human-reviewed passing report.

## Design constraints

- Keep repository source reads inside captured snapshots and validate citations against their captured bytes.
- Preserve exact-context review and explicit consent for every outbound request. Users may choose among backends that have passed their acceptance gates; never silently route, retry, or fall back through another provider or runtime, including a paid service.
- Keep generated chapters unaccepted until the user explicitly reviews and accepts them.
- Treat direct model APIs, speech synthesis, and coding-agent runtimes as distinct integration types. Check the M7 tasks before adding a provider abstraction.
- Update the relevant docs and acceptance criteria when code behavior changes. The [technical plan](docs/TECHNICAL_PLAN.md) distinguishes current source behavior from planned architecture.

## Project contact and review process

This checkout does not publish a repository URL, issue tracker, maintainer contact, contribution review policy, or pull-request instructions. Confirm the project owner's preferred private coordination route before proposing an external contribution. No source license is included, so do not assume that public reuse or redistribution is permitted.
