# Contributing

Code Atlas is an in-progress desktop app. Product behavior and open delivery gates are recorded in the [blueprint](docs/PRODUCT_BLUEPRINT.md) and [Kata backlog](docs/KATA_BACKLOG.md). The Kata task files are scoped contracts; check the implementation before assuming a behavior exists.

For a source-backed first run, see [Getting started](docs/GETTING_STARTED.md). For local setup and workflow failures, see [Troubleshooting](docs/TROUBLESHOOTING.md).

## Prerequisites

- Node.js 20.19 or newer in the 20.x line, or 22.12 or newer, as required by the locked Vite toolchain.
- npm, using the checked-in `package-lock.json`.
- Rust 1.85 or newer and Cargo for the Rust 2024 edition. Use current stable for development. The workspace does not pin a Rust toolchain or declare `rust-version`; compatibility of the locked dependencies with Rust 1.85 has not been verified.
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

That command first builds the native quality helper, validates the synthetic chapter corpus, runs all Node test files, builds the TypeScript/Vite app, runs the Rust library tests, and checks the Rust crate. For a chapter-quality report, see [Chapter quality gate](docs/CHAPTER_QUALITY_GATE.md); a generated metadata file is not a human-reviewed passing report.

## Design constraints

- Keep repository source reads inside captured snapshots and validate citations against their captured bytes.
- Preserve exact-context review and explicit consent for every outbound request. Users may choose among backends that have passed their acceptance gates; never silently route, retry, or fall back through another provider or runtime, including a paid service.
- Keep generated chapters unaccepted until the user explicitly reviews and accepts them.
- Treat direct model APIs, speech synthesis, and coding-agent runtimes as distinct integration types. Check the M7 tasks before adding a provider abstraction.
- Update the relevant docs and acceptance criteria when code behavior changes. The [technical plan](docs/TECHNICAL_PLAN.md) distinguishes current source behavior from planned architecture.

## Public contact and review expectations

The [public GitHub repository](https://github.com/appunni-m/software-stories) exposes an [Issues page](https://github.com/appunni-m/software-stories/issues), but no triage or response policy is published. Do not post credentials, private source, or vulnerability details there; see [Security reporting](SECURITY.md). No maintainer contact, response time, or contribution review policy is published, so opening an issue or submitting a change does not imply that it will be accepted. The repository's source is licensed under [MIT](LICENSE); third-party components and assets may have separate terms recorded in [third-party notices](THIRD_PARTY_NOTICES.md).
