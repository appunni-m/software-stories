# Third-party notices and license scope

This file describes the current repository and unsigned macOS bundle. It is
not a complete software bill of materials or a legal review. Do not redistribute
the app until the open notice-inventory item below is resolved.

## Scope of the project MIT license

The root [MIT License](LICENSE) covers Code Atlas source authored by the
project's contributors, except files that carry their own terms or are listed
here as excluded. It does not license:

- Captured third-party repository source, generated explanations or narration,
  and evaluation records under `docs/acceptance/` and `benchmarks/`.
- Design previews and screenshots under `docs/design/` pending a file-level
  provenance and reuse review.
- Research snapshots under `docs/research/snapshots/` or any third-party
  code, data, model, voice, or media they describe or contain.
- Repository captures, generated answers, audio, and other user data stored
  outside this source checkout by Code Atlas.

Do not copy or include those materials in a source or app distribution unless
their owner and terms are separately established. The app does not package
local captures or saved explanations in its `.app` or `.dmg`.

## Known bundled third-party material

- The app-managed `uv` setup utility is dual-licensed MIT OR Apache-2.0. Its
  upstream MIT and Apache-2.0 license texts are retained in
  `src-tauri/resources/third-party/` and copied into the app resources.
- The source and fixed model assets for Pocket TTS are downloaded separately
  during local voice setup; they are not embedded in the app bundle. The
  selected Alba MacKenna voice recording is attributed under CC BY 4.0. See
  [provider disclosures](docs/PROVIDER_DISCLOSURES.md) and the
  [Pocket runtime record](docs/research/pocket-tts-runtime.md).
- The Code Atlas icon is authored in `src-tauri/icons/code-atlas.svg`; the
  platform-sized icon files in that directory are generated variants.

## Open packaged dependency notice check

The macOS app bundles a JavaScript runtime built from the locked npm dependency
graph and a Rust executable built from Cargo dependencies. The lockfiles record
versions and package metadata, but this checkout has not yet generated and
verified a complete notice file for every dependency in the shipped app. The
existing bundle includes the `uv` notices; that alone does not satisfy the
dependency notice check. Keep the distribution task open until the selected
target's full dependency license texts and attributions are included in the
bundle and verified against `package-lock.json` and `Cargo.lock`.
