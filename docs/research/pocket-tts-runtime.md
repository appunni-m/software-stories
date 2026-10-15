# Pocket TTS runtime

Source review: 2026-10-15. The adapter is implemented in source; setup, synthesis,
cancellation and packaged behavior have **not been executed or accepted** in this
change because the user requested code review without testing. Earlier standalone
CLI samples do not validate this app-managed runtime.

## Runtime contract

The Rust module [`pocket_tts.rs`](../../src-tauri/src/pocket_tts.rs) exposes status,
explicit setup, section synthesis, a runtime fingerprint and app-exit cleanup.
It embeds [`pocket_tts_worker.py`](../../src-tauri/resources/pocket_tts_worker.py)
into the application binary; no separately discovered script is required.

- The current target is Apple Silicon macOS. Other targets return unavailable.
- Setup requires an existing `uv` executable in `/opt/homebrew/bin`,
  `/usr/local/bin`, or the user's default `~/.local/bin` installation. See the
  [official uv installation instructions](https://docs.astral.sh/uv/getting-started/installation/).
- Setup creates `pocket-tts/runtime-3.3.0-v1` under the app data directory. Its
  virtual environment, downloaded Python, uv/Hugging Face caches, temporary
  directory, model assets and readiness manifest stay there. It does not use
  the development machine's Pocket TTS sample caches.
- Setup downloads Python 3.13.14 and installs pinned Pocket TTS 3.3.0, PyTorch
  2.14.1, NumPy 2.5.4, huggingface-hub 2.2.0 and PyYAML 6.0.3. Packages must have
  compatible binary wheels. Remaining transitive dependencies are resolved by
  uv; a complete artifact/hash lock remains release work.
- The installer ignores user/project uv configuration and inherited credentials.
  Fixed model and voice assets are downloaded without a Hugging Face token.
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
revision pins and PCM conversion), interpreter version and pinned core packages.
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

Only one setup/generation operation can run at a time. Each finite worker loads
its model, writes bounded mono PCM16 24 kHz WAV, and exits. This means the model
is reloaded for each invocation; warm persistent inference is not implemented.
Rust independently checks the WAV header, length and 12 MiB limit, reads at most
that limit plus one byte, and removes the temporary output after success/failure.
The accepted section cache is managed by the narration layer.

Cancellation, timeouts and dropped request futures stop the owned process group;
the worker thread reaps the direct child. The desktop app must call `shutdown()`
on its actual exit event. Setup phases have finite time limits, and synthesis is
limited to five minutes per invocation. A crash or forced OS termination can
leave partial setup downloads; explicit setup replaces incomplete runtime state.

## Release and acceptance work still required

This is a managed-install approach, not a self-contained offline Mac bundle.
The `uv` prerequisite is not yet bundled, signed, pinned or verified by the app.
The downloaded Python/native wheels need the release's signing, notarization,
quarantine and clean-account acceptance strategy. Minimum supported macOS,
available wheel compatibility, exact storage requirements and installer recovery
remain unverified. Intel Mac support is not exposed.

Before accepting the runtime task, exercise fresh setup, missing `uv`, low disk,
network failure, cancelled download, repair, package/model corruption, offline
synthesis, long-script limits, repeated cancellation, app exit and child cleanup.
Record an actual human listening review of source names, acronyms, identifiers
and section pacing. Validate the complete player/cache/export path separately.
None of these checks ran as part of this source-only change.
