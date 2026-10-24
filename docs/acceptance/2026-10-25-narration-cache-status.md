# Narration cache-status correction — 25 October 2026

## Scope

Two source-level corrections address the observed gap between Pocket prewarming
and the reader's status display:

- The reader reports **Checking audio** during its asynchronous cache lookup,
  instead of briefly labeling an unresolved lookup **Audio not prepared**.
- New Pocket WAVs are written under `pocket-narration-cache-v1`. Gemini WAVs
  remain under `gemini-narration-cache-v1`. A matching Pocket key is still
  looked up in the old shared location so an existing clip remains reusable.

The Pocket cache key already included provider/model, runtime fingerprint,
voice, exact speech, segmentation policy and audio format. This change makes the
on-disk directory agree with that distinction without changing fingerprints or
re-synthesizing existing Pocket audio.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --lib narration::tests::` —
  31 passed, 0 failed. Includes namespace separation, previous Pocket-cache
  reuse, canonical status, cache reuse/invalidation, bounded worker,
  cancellation and delayed-worker playback cases.
- `node --experimental-strip-types --test tests/narration.test.mjs
  tests/visualStory.test.mjs` — 60 passed, 0 failed.
- `npm run build` — passed. Vite emitted the existing warning that the main
  minified JavaScript chunk exceeds 500 kB.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` — passed.
- `git diff --check` — passed.

## v45 packaged recheck

The v45 release bundle built successfully, including license notice and metadata
checks and the DMG checksum. It was installed as
`/Users/lazytrot/Applications/Code Atlas.app`; the installed executable matches
the bundle at SHA-256
`d1eb8c3a1c66ccc89095ea82d48f0da1394f913afa86936aa0375c8600d1b44d`.

After relaunch, the app restored Fontdone `main@5249b72371`, the previously
saved Code Lens section, and the connected Codex account. Connections → Voice
showed Pocket TTS ready with Alba and “text stays on this Mac.” The saved
18-second clip showed **Ready to play**. The cache tree contains only
`gemini-narration-cache-v1`, the previous shared location, and no synthesis ran;
the new app therefore reused the matching Pocket entry through its fallback
read. The previous app bundle is recoverable at
`/private/tmp/code-atlas-before-v45-20261025/Code Atlas.previous-install.app`.

## Remaining acceptance

This native check did not capture **Checking audio** before the asynchronous
lookup completed, take the app offline, independently match a WAV hash to the
section, exercise cancellation/failure under load, or assess the spoken content.
No provider or TTS request was made for these changes.
