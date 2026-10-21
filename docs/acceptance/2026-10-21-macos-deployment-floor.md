# Unsigned macOS deployment-floor package verification — 2026-10-21

## Scope

This is a local unsigned package preflight on Apple Silicon. The Tauri config
declares `minimumSystemVersion: 14.0`, matching the checked-in Pocket TTS
Python 3.13 / arm64 wheel lock. It does not prove clean-account compatibility
or qualify another architecture.

The first `npm run bundle:macos` invocation built both artifacts but failed in
the post-build verifier: it rejected `LSMinimumSystemVersion: 14.0` because the
verifier still required that key to be absent. The verifier now embeds the
configured floor in the release metadata and requires the app's plist value to
match it (or requires absence only when the configuration has no floor).

## Verified artifacts

The second `npm run bundle:macos` completed successfully. The command ran the
TypeScript check and Vite production build, built the release Rust application,
bundled the app and DMG, checked executable integrity, verified app identity
and data-format metadata, compared the plist minimum with configuration, and
ran `hdiutil verify` on the sole matching DMG.

- App: `src-tauri/target/release/bundle/macos/Code Atlas.app` (38.98 MiB)
- DMG: `src-tauri/target/release/bundle/dmg/Code Atlas_0.1.0_aarch64.dmg` (14.75 MiB)
- Bundle identifier: `com.codeatlas.desktop`
- Version: `0.1.0`
- `LSMinimumSystemVersion`: `14.0`
- App executable SHA-256: `e163fbf702f7ae660c0c4d769a3b6abe2e0ae59f84e26fa1457f3d1868f2f43c`
- DMG SHA-256: `51049e63d8b1ca88320d3e51bd4f9a77436ba69c5e1e770e25851a5c86073713`

The generated release manifest is embedded in the app bundle. It is a build
input produced by `scripts/macos-bundle-metadata.mjs`, not a checked-in source
file.

## Limits

The bundle used `--no-sign`; it is not distributable. It was not installed or
launched, and the signed/notarized artifact, clean-account install, Keychain,
offline workflow, and architecture support remain unqualified. The user's
existing Code Atlas session and saved workspace were left untouched.
