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
The Tauri config requests local `.app` and `.dmg` bundles using the checked-in macOS artwork, enables Tauri's hardened runtime setting, and declares macOS 14.0 as the minimum system version. `npm run bundle:macos` writes the current Rust schema and minimum-system metadata, runs `CI=true tauri build --no-sign --bundles app,dmg`, then checks the bundle identifier, product/version fields, embedded metadata, that the bundle's `LSMinimumSystemVersion` matches configuration, exactly one matching DMG, executable integrity, and the DMG checksum with `hdiutil verify`. `.github/workflows/macos-package-verification.yml` runs this unsigned package check for pull requests, pushes to `main`, and manual dispatch. The [push run for commit `8e28d0a`](https://github.com/appunni-m/software-stories/actions/runs/38499017429) completed successfully, including its “Build and inspect unsigned macOS bundles” step. This is a CI preflight, not a release pipeline or release artifact; it does not verify a signature, notarization ticket, or clean-account install. There is no database migration series in the current file-based storage implementation, so the metadata manifest records the current chapter, visual, index, and recovery formats instead of inventing a migration version.

Local artifact evidence (2026-10-14): `npm run bundle:macos` completed on this arm64 Mac and produced an unsigned `.app` and `.dmg`. At that time the verifier checked the bundle identity/version and embedded data-format manifest against current Rust constants, confirmed that `LSMinimumSystemVersion` was absent, found exactly one matching DMG, and passed `hdiutil verify`. Setting `CI=true` allowed Tauri's noninteractive DMG packaging path to complete. The machine has Command Line Tools only (`xcode-select -p` returns `/Library/Developer/CommandLineTools`; `xcodebuild -version` reports that full Xcode is required), so full Xcode is not required for this unsigned local preflight. This local evidence complements the successful workflow run above but does not make the app releasable.

Release decisions remain open: validate macOS 14.0 as the supported deployment floor across the release workflow, and provide an Apple Developer ID signing identity/team plus notarization credentials. `security find-identity -v -p codesigning` currently finds zero valid identities. The configured floor follows the selected Apple Silicon Torch wheel; it does not qualify all supported architectures or clean-account compatibility. Direct distribution requires Developer ID signing and notarization; Apple requires hardened runtime and a secure timestamp for notarization. Clean-account installation, offline use, Keychain behavior, and notarization remain unverified.

The current storage path, optional Keychain behavior, export limits, and removal steps are documented in [Local data and permissions](../LOCAL_DATA.md). That guide states that Offline export is not a complete backup and that removing a project from Recents does not delete its stored data.

References: [Tauri macOS bundle configuration](https://v2.tauri.app/reference/config/#macconfig), [Tauri macOS signing and notarization](https://v2.tauri.app/distribute/sign/macos/), and [Apple notarization requirements](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

## Pending work — 2026-10-15

- [x] Declare macOS 14.0 as the deployment floor for the selected runtime.
- [ ] Validate the floor and architectures through supported release workflows and clean-account installation.
- [ ] Provide Developer ID signing/team and notarization credentials through secure local configuration; current unsigned preflight is not a release.
- [ ] Produce signed/notarized artifacts and record identity, schema/runtime versions and checksums.
- [ ] Install on a clean supported account; exercise offline browsing/audio/export, Keychain, permission denial and disk failure recovery.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## macOS deployment-floor package check — 2026-10-21

The configured minimum is now macOS 14.0, matching the selected Apple Silicon Pocket TTS runtime wheel. The package verifier embeds that configured value and compares it with `LSMinimumSystemVersion`; it accepts an absent value only when configuration explicitly has no floor. An initial bundle exposed the old verifier's contradictory “must be absent” rule. After updating the verifier and the user-facing compatibility docs, `npm run bundle:macos` succeeded. It built an unsigned 38.98 MiB `.app` and 14.75 MiB `.dmg`; the app identifier/version, embedded data-format/minimum-system metadata, executable integrity, and plist minimum 14.0 matched, and `hdiutil verify` accepted the DMG checksum. The executable SHA-256 is `e163fbf702f7ae660c0c4d769a3b6abe2e0ae59f84e26fa1457f3d1868f2f43c`; the DMG SHA-256 is `51049e63d8b1ca88320d3e51bd4f9a77436ba69c5e1e770e25851a5c86073713`.

See the [full artifact record](../acceptance/2026-10-21-macos-deployment-floor.md) for the exact command and release limitations.

This verifies the local unsigned package preflight only. The app was not installed or launched; signing, notarization, clean-account compatibility, offline use, and the remaining release acceptance remain open. No user workspace content was included in the bundle.

## Executable integrity checkpoint — 2026-10-17

A local disk-full failure left a truncated Mach-O in a bundle that a later standalone Tauri command reported as successful. It failed to launch; the previous working app was restored. Package-command success and plist identity alone are insufficient. The packaging verifier now uses `codesign --verify --strict --ignore-resources` on the named bundled executable before metadata/DMG acceptance. Missing and truncated executables are rejected by deterministic regressions; this verifies embedded executable integrity without claiming resource sealing, Developer ID identity or notarization.

After clearing generated build caches and restoring complete compiler output, the final unsigned debug bundle independently matched the full executable length and SHA-256 and launched through computer use. The saved Codex connection, Fontdone source and a child Guide at section 2/3 were observed. Native tests: 295 passed, 11 explicit probes ignored. Frontend tests: 145 passed. [Failure, rollback and final artifact details](../acceptance/2026-10-17-fontdone-quotes-and-tool-timing.md#packaging-recovery-and-installed-follow-up). This does not close the signed release or clean-account gates. Disk-full behavior still needs to be qualified in the supported release pipeline and clean install journey.
