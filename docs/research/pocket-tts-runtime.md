# Pocket TTS runtime

October 21 source update: explicit setup now downloads the pinned Apple Silicon `uv` 0.13.4 release into app storage and verifies its SHA-256 before extraction. It no longer depends on a user-installed `uv`. The release archive and binary are checked against an app-managed manifest before setup reuses the binary. The official [uv 0.13.4 release](https://github.com/astral-sh/uv/releases/tag/0.13.4) publishes the selected artifact and checksum; its MIT and Apache notices are included in the Tauri bundle resources. This change has source and unit-test evidence, but not a fresh packaged-app setup run or a clean-account qualification.

October 21 package-lock update: setup now installs the 49 resolved Python dependencies from the checked-in [`uv` hash lock](../../src-tauri/resources/pocket-tts-requirements-macos-aarch64-py313.lock), with `--require-hashes` and binary wheels only. Its direct pins are maintained in the adjacent [input file](../../src-tauri/resources/pocket-tts-requirements-macos-aarch64-py313.in); regenerate with the recorded `uv pip compile --generate-hashes` command using Python 3.13.14 and the `aarch64-apple-darwin` target. The selected PyTorch wheel requires macOS 14, so the app bundle now declares macOS 14 as its minimum. Setup writes the exact lock into app storage, stores its SHA-256 in the runtime manifest, validates it before synthesis, and includes it in narration-cache identity. This closes the source-level transitive lock gap; fresh packaged setup, clean-account and offline installer qualification remain open.

October 16 native update: the release app completed managed Python/package/model setup and reported **Pocket TTS is ready**. The initial setup failure was reproduced as conflicting uv managed-Python options, then fixed and covered by a regression. A subsequent synthesis failure was traced to inference-mode tensors crossing Pocket’s background cache-worker boundary; synthesis now uses `torch.no_grad()` and the corrected offline worker produced a valid synthetic WAV. That run used an existing uv installation and was not a clean-account installer qualification. The final app installed at `~/Applications/Code Atlas.app` generated and played an 8.85-second section, supported replay/pause and reused its cached audio after section navigation. Cancellation, clean-account packaging, export and listening qualification remain open below. See [dated native evidence](../acceptance/2026-10-16-codex-validation.md).

## Runtime contract

The Rust module [`pocket_tts.rs`](../../src-tauri/src/pocket_tts.rs) exposes status,
explicit setup, section synthesis, a runtime fingerprint and app-exit cleanup.
It embeds [`pocket_tts_worker.py`](../../src-tauri/resources/pocket_tts_worker.py)
into the application binary; no separately discovered script is required.

- The current target is Apple Silicon macOS. Other targets return unavailable.
- Explicit setup downloads `uv` 0.13.4 for Apple Silicon from the fixed
  [Astral release artifact](https://github.com/astral-sh/uv/releases/tag/0.13.4),
  verifies SHA-256 `e0b1d4743ce64c55d9f0f4a444afa6b81125680070ee04e853fcd0f6c1ebf8f1`,
  extracts only the expected executable and records its binary hash in app
  storage. No globally installed `uv`, shell activation or user `uv` config is
  required. Downloads use the system `curl` only after the user starts setup.
- Setup creates `pocket-tts/runtime-3.3.0-v1` under the app data directory. Its
  pinned `uv`, virtual environment, downloaded Python, uv/Hugging Face caches,
  temporary directory, model assets and readiness manifest stay there. It does
  not use the development machine's Pocket TTS sample caches.
- Setup downloads Python 3.13.14 and installs the exact direct and transitive
  dependency set in the checked-in macOS/Python hash lock. `uv pip install`
  requires hashes and binary wheels only; if a matching wheel or hash is
  unavailable, setup fails instead of selecting an unpinned source build.
- The installer ignores user/project uv and curl configuration and inherited
  credentials. The fixed model and voice assets are downloaded without a
  Hugging Face token.
- Explicit setup also repairs an existing installation: it clears only the
  app-managed `venv` and rebuilds the readiness manifest. Download caches remain
  available for reuse. Readiness is written only after downloads and model
  initialization succeed; cancellation racing completion can leave that completed
  installation available.
- Status reads files only. During work it exposes the current phase: Python,
  packages, model download/load, or synthesis. It does not claim a percentage.

## Model and voice identity

The worker uses the English September 2026 configuration shipped in Pocket TTS
3.3.0. It substitutes local paths for these immutable assets from
[`kyutai/pocket-tts-without-voice-cloning`](https://huggingface.co/kyutai/pocket-tts-without-voice-cloning):

| Asset | Repository revision |
| --- | --- |
| `languages/english_2026-09/model.safetensors` | `e7205b6ee50e654a5ea19f0e9df2b0813b05e921` |
| `languages/english_2026-09/tokenizer.json` | `00eac05ed3d16bdc3f6b5d598874019c34a89214` |
| `languages/english_2026-09/embeddings/alba.safetensors` | `4e1e0a3e611c51c0b4ed8174fc10f32a54644303` |

No arbitrary voice upload, custom configuration URL or voice-cloning download is
accepted. Alba's source recording is by **Alba MacKenna**, licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); the precomputed voice
embedding is derived from that recording. See the
[official voice catalog and attribution](https://huggingface.co/kyutai/tts-voices).
Pocket TTS code and model licensing are separate from voice attribution; see the
[Pocket TTS repository](https://github.com/kyutai-labs/pocket-tts).

`runtime_fingerprint()` hashes the embedded worker (including complete asset
revision pins and PCM conversion), interpreter version, pinned core packages
and complete dependency lock.
The local narration cache and its derived review fingerprint include this value;
legacy Google cache keys remain unchanged. A setup manifest stores
asset sizes/hashes and the local configuration hash. The worker checks those
hashes and the installed core package versions before each synthesis.

## Generation and process ownership

Generation runs the installed Python in isolated mode with an explicitly
constructed environment, offline Hugging Face mode and only local model paths.
The edited transcript reaches the worker over stdin, not process arguments or a
saved request file. No API key is needed. Neither stdout nor dependency exception
text is forwarded to logs. There is no server or listening port.

One setup/generation operation owns the runtime until its child is reaped; at
most four others can wait. Queued cancellation or a dropped waiter starts no
process. A strict version-1 stdin packet carries 1–64 reviewed segments, each at
most 4,000 UTF-8 bytes and at most 16,000 total text bytes (97,024 transport
bytes). One finite worker loads the model/voice once for that script, generates
segments in order with `copy_state=True`, joins them with 200 ms PCM pauses,
and exits. Warm reuse is within the script; persistent inference is not implemented.
Every segment must be nonempty and finite; pauses count toward the output cap.
A later failure discards the entire temporary output.
Rust independently checks the WAV header, length and 12 MiB limit, reads at most
that limit plus one byte, and removes the temporary output after success/failure.
The accepted section cache is managed by the narration layer. A changed or
missing lock invalidates runtime readiness and changes the cache fingerprint.

Cancellation, timeouts and dropped request futures stop the owned process group;
the worker thread reaps the direct child. The desktop app must call `shutdown()`
on its actual exit event. Setup phases have finite time limits, and synthesis is
limited to five minutes for the entire script invocation, including nonblocking
stdin delivery. A stalled pipe observes cancellation/deadline and cannot retain
the owner indefinitely. A crash or forced OS termination can
leave partial setup downloads; explicit setup replaces incomplete runtime state.

## Release and acceptance work still required

This is a managed-install approach, not a self-contained offline Mac bundle.
`uv` is pinned and downloaded on explicit setup, but it is not bundled in the
DMG. The Python dependency artifact/hash lock is now checked in. The selected
Apple Silicon wheels set macOS 14 as the declared minimum; the packaged setup
still needs clean-account and fresh-install qualification. The downloaded
Python/native wheels need the release's signing, notarization and quarantine
strategy. Exact storage requirements and installer recovery remain unverified.
Intel Mac support is not exposed.

Before accepting the runtime task, exercise fresh setup, missing system `curl`
or `tar`, low disk, network failure, cancelled downloads, repair, package/model corruption, offline
synthesis, long-script limits, repeated cancellation, app exit and child cleanup.
Record an actual human listening review of source names, acronyms, identifiers
and section pacing. Validate the complete player/cache/export path separately.
The [October 17 whole-script checkpoint](../acceptance/2026-10-17-pocket-whole-script.md)
adds isolated real-worker synthesis and deterministic packet/queue/cancellation
checks. It does not qualify clean setup, installed multi-window/exit behavior,
native cache/export invalidation or human listening.
