---
kind: task
key: distribution-license
title: Choose and publish the project's distribution license
priority: 1
labels: docs, release
---

## User outcome

Contributors can tell whether and under what terms they may use, modify, and redistribute Code Atlas.

## Current state

The owner selected MIT on 21 October 2026. The root `LICENSE`, npm/Cargo metadata, README, and Tauri metadata agree. `THIRD_PARTY_NOTICES.md` inventories all 122 locked production npm packages and 391 external crates in the Apple Silicon macOS release graph (`tauri/custom-protocol`); `THIRD_PARTY_LICENSE_TEXTS.txt` carries the corresponding full license and notice texts, including the bundled `uv` terms and Code Atlas's reviewed upstream-license fallback for `react-remove-scroll-bar@2.3.8`.

The app's fixed icon is the Code Atlas-authored `src-tauri/icons/code-atlas.svg` and generated platform variants. The `uv` installer is checksum-verified and downloaded during explicit Pocket TTS setup, not embedded. Python, the pinned Pocket TTS packages, model weights, and Alba voice are also downloaded to app data during that setup. Their separate terms and Alba attribution are documented in the Pocket TTS runtime and provider disclosures. Captured repositories, generated explanations/audio, benchmarks, acceptance fixtures, design previews, research snapshots, and local user data remain outside the MIT grant and app package.

On 21 October 2026, `npm run bundle:macos` passed the full inventory regeneration check, built the unsigned Apple Silicon `.app` and `.dmg`, verified exact copies of `LICENSE`, `THIRD_PARTY_NOTICES.md`, and `THIRD_PARTY_LICENSE_TEXTS.txt` in the app, checked executable integrity and bundle metadata, and verified the DMG checksum. This qualifies the license and notice package; it does not claim the unsigned artifact is ready for public redistribution or provide a license compatibility certification.

## Required work

- Add the owner-selected canonical license text and required notices.
- Make README, Cargo/npm/Tauri metadata and release materials agree.
- Check source, bundled asset and generated-file provenance; document exclusions or attribution requirements.
- Keep user-generated explanations, narration and local repository data outside the licensed source distribution unless ownership and terms are explicitly addressed.

## Acceptance evidence

- Owner-selected canonical MIT terms are in the root `LICENSE`; README, `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` agree.
- `npm run licenses:check` regenerates and compares the complete npm/Cargo license and notice inventory against the locked production and macOS release graphs and all recorded source fingerprints.
- `npm run bundle:macos` passed and verified the unsigned app, exact notice resources, metadata, executable integrity, minimum macOS version, and DMG checksum.
- The Open Source documentation audit reported zero errors and no missing-license finding.
- Captured content and unshipped downloaded model/voice/runtime assets have separate documented terms and are excluded from the app's packaged resources and the source MIT grant.
