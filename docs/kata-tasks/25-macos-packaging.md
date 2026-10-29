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

## CI queue correction and release checkpoint — 2026-10-28 04:18 UTC

The latest 100 completed package runs contain 63 successes, 25 concurrency cancellations, and 12 failures. The newest real failure, [run #1192](https://github.com/appunni-m/software-stories/actions/runs/40606128208), stopped during runner setup: its `Swatinem/rust-cache` SHA had a transposed character and could not be resolved. Commit `ee45cbd` corrected the pin to the official `v2.9.2` commit (`6323deb102c322ba6fcbdcafc7e3dddab59af2b6`). Run [#1193](https://github.com/appunni-m/software-stories/actions/runs/40606390228) passed after the correction; later full package run [#1219](https://github.com/appunni-m/software-stories/actions/runs/40628858319) also passed, including Rust cache restore and bundle inspection.

Package run [#1221](https://github.com/appunni-m/software-stories/actions/runs/40629232479) has succeeded. Workflow-change run [#1223](https://github.com/appunni-m/software-stories/actions/runs/40631074833) is still in progress. Documentation-only run [#1222](https://github.com/appunni-m/software-stories/actions/runs/40630132903) was cancelled when the workflow correction triggered #1223. The subsequent docs-only push `f13e5f1` created no new Actions run, confirming that `push.paths-ignore` keeps `docs/**` and `.gitignore` pushes out of the branch concurrency queue. Pull requests still use the existing exact base/head scope check.

The repository has no published GitHub release and no release tag. The signed-candidate workflow runs only for `v*` tags and requires five Apple signing/notarization secrets; clean-account installation and signed/notarized artifact acceptance remain open. A local `security find-identity -v -p codesigning` check at 04:14 UTC found zero valid signing identities, confirming that this Mac cannot currently produce a Developer ID signed candidate.

## CI and release checkpoint — 2026-10-28 06:37 UTC

Package-verification run [#1232](https://github.com/appunni-m/software-stories/actions/runs/40643781420) completed successfully for code commit `e0a4008`. Current `main` is `b150626`, whose only change after that run is documentation; the workflow excludes `docs/**`, so the successful package check covers the current code. GitHub currently reports no published releases, and `git ls-remote --tags origin` returned no tag refs. The successful unsigned bundle check is not a signed or notarized release. Developer ID/notarization credentials and clean-account installation acceptance remain release blockers.

## CI and release checkpoint — 2026-10-28 06:46 UTC

The history-search UI correction was pushed as `fd4c013`. Package run [#1233](https://github.com/appunni-m/software-stories/actions/runs/40645757025) started at 06:45 UTC and is still in progress, currently restoring the Rust dependency cache; its conclusion is unknown. Run #1232 remains successful for the prior code state `e0a4008`. GitHub still has no published release or tag, so the first-release status is unchanged: signing, notarization, and clean-account acceptance remain open.

## CI and release checkpoint — 2026-10-28 07:02 UTC

Package run [#1233](https://github.com/appunni-m/software-stories/actions/runs/40645757025) completed successfully at 07:01 UTC for `fd4c013`. This confirms the unsigned macOS app/DMG packaging and inspection after the history-search correction. GitHub reports no release and `git ls-remote --tags origin` returned no tag refs. Signing, notarization, a clean-account install, and the signed release-candidate workflow remain unqualified.

## CI and release checkpoint — 2026-10-28 07:07 UTC

The source changes `549cc84`, `bc0539f`, and `f021800` plus the release note `2b1e7ce` were pushed to `main`. Package run [#1234](https://github.com/appunni-m/software-stories/actions/runs/40648040451) failed during the frontend build at `src/VisualStory.tsx:206,208`: the source-citation focus effect dereferenced the optional visual model after deriving an optional node from it. The effect now requires both `visual` and `node` before reading scene values. The local production frontend build and Rust library compile check pass with this source fix; a new package run is needed to verify it on GitHub. Run #1233 passed for `fd4c013`. There is still no public release or tag; signing, notarization, and clean-account installation remain open.

## Local package and UI check — 2026-10-28 07:53 UTC

`npm run bundle:macos` completed from `dbf3bcd`: license inventory, frontend
build, optimized Rust release build, embedded metadata verification, app/DMG
bundling, and DMG checksum verification all passed. The unsigned app was
installed and opened the existing Fontdone `main@5249b72371` workspace. The
history search fix is visible in the packaged app. This verifies one local
packaging/UI path; it does not qualify signing, notarization, clean-account
installation, offline behavior, or the full app acceptance matrix.

GitHub package run [#1237](https://github.com/appunni-m/software-stories/actions/runs/40650011025)
completed successfully for `f45db3b`; run
[#1238](https://github.com/appunni-m/software-stories/actions/runs/40651183982)
is in progress for `a798861`. The docs-only `dbf3bcd` push is ignored by the
package workflow. The repository still has no tag or published release, and
`security find-identity -v -p codesigning` returns zero valid identities.
Keep release acceptance open.

## CI failure and queue diagnosis — 2026-10-28 11:39 UTC

The latest 100 package-workflow runs contain 69 successes, 27 cancellations,
3 failures, and 1 in-progress run. The cancellations are newer pushes replacing
the single pending run under the configured concurrency group; they are not
compile failures. The three failures have distinct causes:

- Run [#1192](https://github.com/appunni-m/software-stories/actions/runs/40606128208)
  failed before the job began because the pinned `Swatinem/rust-cache` commit
  did not exist. The pin was corrected to the `v2.9.2` commit, and run #1193
  succeeded.
- Run [#1226](https://github.com/appunni-m/software-stories/actions/runs/40634517834)
  reached the frontend build and failed TypeScript because `onGoAtlas` was
  undefined in the removed Code Lens workspace. Later full package runs #1265
  and #1266 succeeded.
- Run [#1234](https://github.com/appunni-m/software-stories/actions/runs/40648040451)
  reached the frontend build and failed TypeScript because the visual-citation
  focus effect dereferenced an optional `visual`. The source was subsequently
  corrected; runs #1265 and #1266 also passed after that correction.

Current `main` at `8da4add3bdbc1e9f71eb0103b19d5e5cb063eb2c` has package run
[#1267](https://github.com/appunni-m/software-stories/actions/runs/40680552632)
in progress. Its runner setup, cache restore, dependency install and license
inventory completed; the unsigned bundle build is still running. Thus CI has
not shown one persistent failure: two recent red builds were real transient
TypeScript errors subsequently fixed, one was a bad action pin already fixed,
and most red-looking run entries were queue cancellations. Do not call the
current commit verified until #1267 finishes. The unsigned package workflow
still is not a signed/notarized release, and clean-account installation remains
open.

## CI completion — 2026-10-28 11:45 UTC

GitHub Actions run [#1267](https://github.com/appunni-m/software-stories/actions/runs/40680552632)
completed successfully for code commit `8da4add3bdbc1e9f71eb0103b19d5e5cb063eb2c`.
The `Build and inspect unsigned bundles` job passed through runner setup, Rust
cache restore, dependency installation, license inventory, unsigned macOS bundle
build, and inspection. This resolves the prior in-progress CI note; it does not
qualify code added after that revision, signing/notarization, a clean-account
install, or a published release. The repository still has no release tag or
published GitHub release, and the local Mac reports zero valid Developer ID
signing identities. Keep `e42a` and the release milestone open.

## Current CI and release state — 2026-10-28

GitHub's latest package-verification run is [#1275](https://github.com/appunni-m/software-stories/actions/runs/40698437236),
which passed for code commit `1e2744b`. Run [#1274](https://github.com/appunni-m/software-stories/actions/runs/40695083941)
also passed; run #1272 was cancelled while queued, not failed during a build.
The workflow preserves the active macOS build and allows only the newest pending
run for a branch, so rapid pushes can replace older queued work. Documentation-
only changes do not trigger package verification because the workflow ignores
`docs/**`; the latest successful package run therefore covers the current code
state, while this note is not a package artifact identity.

The GitHub releases endpoint has no published release, and `git ls-remote --tags
origin` returned no tags. The unsigned package workflow is not the signed
release workflow. A public 0.1.0 release still needs the Apple Developer ID and
notarization credentials, a signed/notarized artifact, clean-account install
acceptance, and the remaining product/accessibility gates. This is a live
checkpoint, not release approval.

## CI and release checkpoint — 2026-10-28 16:34 UTC

Run [#1278](https://github.com/appunni-m/software-stories/actions/runs/40723838544)
passed for `df4741b`. Run
[#1279](https://github.com/appunni-m/software-stories/actions/runs/40727123247)
also completed successfully for `2f6d9d9`; its “Build and inspect unsigned
bundles” job passed. The local app/DMG build, metadata inspection, and checksum
verification also passed for that commit. GitHub's release endpoint returned no
published releases, and `git ls-remote --tags origin` returned no tags. The
first release still requires signing, notarization, clean-account installation,
and product/accessibility acceptance.

## CI and first-release status — 2026-10-29 08:50 UTC

The current code commit `5ca2f4922207252cd0c04ff81f845ab3ba82cf6a` has package-verification run [#1340](https://github.com/appunni-m/software-stories/actions/runs/40843203439) in progress. Runner setup, checkout, package-input detection, Node/Rust setup, Rust cache restore, license inventory, and JavaScript dependency installation passed; unsigned bundle building was still running at 08:50 UTC. The preceding code run [#1339](https://github.com/appunni-m/software-stories/actions/runs/40842607268) passed for `520d7059f9d10f55840cb885e6cf390fbb92aa35`. Current `main` is docs-only commit `3c06d27`; the workflow intentionally ignores `docs/**` and `.gitignore`, so that commit did not start another package build. Do not call #1340 green until GitHub reports its final conclusion.

The recent red runs are a mix of actual defects in intermediate revisions and one workflow setup error, not one continuously broken build. Run [#1234](https://github.com/appunni-m/software-stories/actions/runs/40648040451) failed TypeScript compilation when a visual-citation path assumed an optional value; run [#1226](https://github.com/appunni-m/software-stories/actions/runs/40634517834) failed TypeScript compilation on a removed Code Lens callback; and [#1192](https://github.com/appunni-m/software-stories/actions/runs/40606128208) failed before the job because a pinned Rust-cache action revision did not exist. Those defects were corrected and later package runs passed. Other cancelled entries reflect the serialized macOS build lane replacing a pending run; cancellation is distinct from a build failure.

GitHub's releases endpoint currently returns no releases, and the repository has no release tag. The first release is therefore **not published**. CI currently produces unsigned verification bundles only. A public release still needs Apple Developer ID signing and notarization credentials, a signed/notarized artifact, clean-account installation, and the remaining offline, Keychain, product, and accessibility acceptance. This snapshot records status; it does not qualify the release.

The repository hygiene check found that `.gitignore` already excludes dotenv and direnv files, nested coverage output, Rust `target`, frontend `dist`/`.vite`, and installed `node_modules`. No dotenv or generated coverage files are tracked. Avoid deleting local build or user data based only on directory size; the current tracked large files include source, design references, and acceptance evidence.

## CI and release follow-up — 2026-10-29 09:10 UTC

Package run [#1340](https://github.com/appunni-m/software-stories/actions/runs/40843203439) completed successfully for code commit `5ca2f492`. The new UI code commit `f0969c3ed11c0bfb7c5c5be3c0b1320fff21de1b` has run [#1341](https://github.com/appunni-m/software-stories/actions/runs/40847457421) in progress. Checkout, package-input detection, and Node/Rust setup passed; Rust cache restore was running at 09:10 UTC. This documentation-only follow-up does not alter the package inputs. There are still no remote tags or published GitHub releases; first-release signing, notarization, and clean-account/product acceptance remain open.

## Current CI and first-release status — 2026-10-29 11:28 UTC

Package run [#1351](https://github.com/appunni-m/software-stories/actions/runs/40864420437)
for source commit `25dbbf2` is **in progress** at the unsigned macOS bundle
build step. The previous run [#1350](https://github.com/appunni-m/software-stories/actions/runs/40862945907)
completed successfully for `d99e357`. Current `main` is docs-only commit
`b4c56c5`; `.github/workflows/macos-package-verification.yml` ignores
`docs/**`, so it does not start another package run.

At this checkpoint, the GitHub releases endpoint returns no releases and the
tags endpoint returns no tags. The first release is not published; the active
CI lane only creates unsigned verification bundles. Signing/notarization,
clean-account installation, and remaining product/accessibility acceptance
are still required. Do not treat #1351 as passed until GitHub reports its final
conclusion.
