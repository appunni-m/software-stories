---
kind: task
key: t25
parent: m5
title: Package, sign, notarize, and document the first Mac release
priority: 1
labels: phase-5, release, reliability
blocked_by: t22, t23, t24
---

## User outcome
The user can install a dependable Mac app, understand its requirements, and use local browsing without a provider key.

## Contract
Build distributable macOS artifacts using the chosen Tauri packaging path. Define supported macOS versions, signing/notarization, app data and cache locations, backup/export behavior, first-run permission needs, and how users remove local project metadata. Release notes state parser support, repository limits, provider model/disclosure verification date, and known limitations.

## Acceptance criteria
- A clean Mac account can install and launch the signed/notarized app without a development toolchain.
- Offline repository browsing, accepted guides, and exported guide reading work before key setup.
- Keychain permissions and local data locations are explained in product language.
- Release pipeline verifies artifact identity and bundled schema/migration versions.
- Failed install, permission denial, and disk-space conditions give recovery guidance.

## Excludes
Automatic in-app updates unless separately specified, paid backend services, and claiming provider pricing will remain unchanged.

## Done evidence
Install and exercise the release artifact on a clean supported macOS environment; verify signature/notarization, offline journey, Keychain behavior, release notes, and artifact checksums.

## Local packaging baseline
The Tauri config enables local `.app` and `.dmg` bundles using the checked-in macOS artwork. Run `npm run bundle:macos` to build both bundles unsigned for local inspection. This is not a release artifact.

Release decisions remain open: choose a supported minimum macOS version based on the product's support policy, and provide the Apple signing identity/team and notarization credentials for release builds. The local config explicitly clears Tauri's default minimum-version plist field until that decision is made; this is not a compatibility claim. Neither version nor signing identity is inferred from the build machine. Clean-account install, offline use, Keychain behavior, and notarization still require release validation.
