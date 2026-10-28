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

The Tauri config requests local `.app` and `.dmg` bundles using the checked-in macOS artwork, enables Tauri's hardened runtime setting, and declares macOS 14.0 as the minimum system version. `npm run bundle:macos` writes the current Rust schema and minimum-system metadata, runs `CI=true tauri build --no-sign --bundles app,dmg`, then checks the bundle identifier, product/version fields, embedded metadata, that the bundle's `LSMinimumSystemVersion` matches configuration, exactly one matching DMG, executable integrity, and the DMG checksum with `hdiutil verify`. `.github/workflows/macos-package-verification.yml` runs this unsigned package check for pull requests, pushes to `main`, and manual dispatch. This is a CI preflight, not a release pipeline or release artifact; it does not verify a Developer ID signature, notarization ticket, or clean-account install. There is no database migration series in the current file-based storage implementation, so the metadata manifest records the current chapter, visual, index, and recovery formats instead of inventing a migration version.

## CI state — 2026-10-27

The latest main-branch package check for `5da90610b667041b9233d2bb466ef0a2116b7fa4` (run [#1133](https://github.com/appunni-m/software-stories/actions/runs/40445231185)) passed. The red runs in the October 27 push burst came from several intermediate states pushed before their fixes landed, rather than one persistent CI defect:

- Runs [#1120](https://github.com/appunni-m/software-stories/actions/runs/40428352754) and [#1121](https://github.com/appunni-m/software-stories/actions/runs/40428619612) failed in the new license-inventory step: the first runner had no `cargo-about` command; the next could not run its offline Cargo inventory because locked crate `aes v0.9.3` was not cached. Run [#1122](https://github.com/appunni-m/software-stories/actions/runs/40429119100) then correctly rejected stale generated third-party notices after dependency changes.
- Runs #1123–#1126 exposed source errors while UI and native code were being edited: TypeScript/JSX syntax and type errors, followed by Rust type and borrow errors. The final failure in that burst, [#1129](https://github.com/appunni-m/software-stories/actions/runs/40440522260), was a TypeScript parse error at `src/workspace/Workspace.tsx:995`; run [#1131](https://github.com/appunni-m/software-stories/actions/runs/40441024537) passed after the repair. Run #1130 was cancelled by the workflow's configured superseding-run concurrency rule, not a build failure.

Runs #1131–#1133 are successful on the current `main`. The workflow remains an unsigned packaging preflight; the signed/notarized tag workflow is separate and requires Apple credentials. The burst diagnosis does not prove every historical failure had the same cause.

Local artifact evidence (2026-10-14): `npm run bundle:macos` completed on this arm64 Mac and produced an unsigned `.app` and `.dmg`. At that time the verifier checked the bundle identity/version and embedded data-format manifest against current Rust constants, confirmed `LSMinimumSystemVersion` was absent because no floor had yet been declared, found exactly one matching DMG, and passed `hdiutil verify`. Setting `CI=true` allowed Tauri's noninteractive DMG packaging path to complete. The machine has Command Line Tools only (`xcode-select -p` returns `/Library/Developer/CommandLineTools`; `xcodebuild -version` reports that full Xcode is required), so full Xcode is not required for this unsigned local preflight. This local evidence complements the successful workflow run above but does not make the app releasable.

The app declares macOS 14.0 as its deployment floor, and the package verifier checks that the generated metadata and `LSMinimumSystemVersion` match that declaration. This proves the package metadata is consistent; compatibility across supported architectures and clean-account installation remain unqualified. Release decisions remain open: provide an Apple Developer ID signing identity/team and notarization credentials. `security find-identity -v -p codesigning` previously found zero valid identities; recheck before release. The configured floor follows the selected Apple Silicon Torch wheel. Direct distribution requires Developer ID signing and notarization; Apple requires hardened runtime and a secure timestamp for notarization. Clean-account installation, offline use, Keychain behavior, and notarization remain unverified.

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

## Live CI and release checkpoint — 2026-10-27 21:40 UTC

The repository's latest GitHub Actions state is not a new failure: the unsigned package run for `6e250a9` is pending ([run 40587917928](https://github.com/appunni-m/software-stories/actions/runs/40587917928)); the preceding `c4772f2` run is in progress ([run 40586768118]); and `2716cbc` succeeded ([run 40585236354]). The workflow uses one concurrency group per branch with `cancel-in-progress: false`, so the native macOS packaging runs queue in order instead of canceling the active build. The prior successful build took about 19 minutes, which explains the short queue observed here. This checkpoint does not yet establish the final result for `6e250a9`.

The GitHub releases endpoint returned no releases and `git ls-remote --tags origin` returned no tag refs. There is still no first release. The existing unsigned verification workflow is only a package preflight; signing/notarization credentials and clean-account installation remain required release gates.

## Live CI and release checkpoint — 2026-10-27 21:52 UTC

After pushing `8b1111d`, the run queue is: [#1177](https://github.com/appunni-m/software-stories/actions/runs/40586768118) succeeded, [#1178](https://github.com/appunni-m/software-stories/actions/runs/40587917928) is in progress, and [#1180](https://github.com/appunni-m/software-stories/actions/runs/40591027428) is pending for the latest commit. The intervening [#1179](https://github.com/appunni-m/software-stories/actions/runs/40590203665) was cancelled when the newer push replaced the single pending run; it did not fail its build. This is serialized native macOS packaging, not evidence of repeated build failures. Runs #1174–#1177 are successful. GitHub still reports no published release, and the repository has no tags. The signed workflow and clean-account install have not run; Developer ID/notarization credentials and clean-install acceptance remain release blockers.

## Live CI and release checkpoint — 2026-10-28 03:35 IST

After pushing `17b3e23`, package-verification run [#1181](https://github.com/appunni-m/software-stories/actions/runs/40591915713) is in progress. Run [#1178](https://github.com/appunni-m/software-stories/actions/runs/40587917928) succeeded; [#1179](https://github.com/appunni-m/software-stories/actions/runs/40590203665) and [#1180](https://github.com/appunni-m/software-stories/actions/runs/40591027428) were cancelled as newer pushes superseded queued work under the workflow's single-pending-run policy. They did not fail compilation or packaging. The workflow preserves an active build and retains only the newest pending build, so batching pushes avoids redundant macOS runs. There is still no release tag or published GitHub Release. The unsigned package check is not a public release; Developer ID/notarization credentials and clean-account acceptance remain open.

## Live CI and release checkpoint — 2026-10-28 02:26 UTC

The latest full unsigned package build is [run #1206](https://github.com/appunni-m/software-stories/actions/runs/40618887830) for `41d87fb`; setup, Rust cache restore, dependency installation, bundle build, and inspection succeeded. [Run #1207](https://github.com/appunni-m/software-stories/actions/runs/40619430719) for `eafdfd5` is still building the unsigned bundle. [Run #1208](https://github.com/appunni-m/software-stories/actions/runs/40620336216) for current `main` at `4fac06b` is pending; that commit changes only docs, so the package-input check is expected to skip the native build when it starts.

The recent actual CI failure was [run #1192](https://github.com/appunni-m/software-stories/actions/runs/40606128208), which stopped at GitHub runner setup because the new `Swatinem/rust-cache` action reference pointed to a nonexistent commit. It did not reach source compilation or packaging. The pin was corrected in `ee45cbd`; [run #1193](https://github.com/appunni-m/software-stories/actions/runs/40606390228) then completed the full unsigned package build successfully. Runs #1194–#1197 were queued cancellations after newer pushes replaced the single pending run; they were not build failures.

GitHub currently has no published release. The signed-candidate workflow still requires a `v*` tag and Apple signing/notarization credentials, and a clean-account install has not been accepted. The successful unsigned package check is not a release.

## CI queue correction and release checkpoint — 2026-10-28 04:14 UTC

The latest 100 completed package runs contain 63 successes, 25 concurrency cancellations, and 12 failures. The newest real failure, [run #1192](https://github.com/appunni-m/software-stories/actions/runs/40606128208), stopped during runner setup: its `Swatinem/rust-cache` SHA had a transposed character and could not be resolved. Commit `ee45cbd` corrected the pin to the official `v2.9.2` commit (`6323deb102c322ba6fcbdcafc7e3dddab59af2b6`). Run [#1193](https://github.com/appunni-m/software-stories/actions/runs/40606390228) passed after the correction; later full package run [#1219](https://github.com/appunni-m/software-stories/actions/runs/40628858319) also passed, including Rust cache restore and bundle inspection.

At this checkpoint, package run [#1221](https://github.com/appunni-m/software-stories/actions/runs/40629232479) remains in progress. Documentation-only run [#1222](https://github.com/appunni-m/software-stories/actions/runs/40630132903) was cancelled when the workflow correction triggered run [#1223](https://github.com/appunni-m/software-stories/actions/runs/40631074833), which is pending. The workflow now excludes `docs/**` and `.gitignore` pushes before they enter the branch concurrency queue, so later documentation-only pushes cannot displace package-input verification. Pull requests still use the existing exact base/head scope check.

The repository has no published GitHub release and no release tag. The signed-candidate workflow runs only for `v*` tags and requires five Apple signing/notarization secrets; clean-account installation and signed/notarized artifact acceptance remain open. A local `security find-identity -v -p codesigning` check at 04:14 UTC found zero valid signing identities, confirming that this Mac cannot currently produce a Developer ID signed candidate.
