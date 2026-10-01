# Software Stories

A local, interactive guide to unfamiliar code, told through its purpose, design, and behavior. Start with responsibilities and data flow, then select a part or ask a question to generate a deeper explanation. The first journey explains this project itself.

## Run this checkout

```sh
npm ci --ignore-scripts
npm start
```

Open **http://127.0.0.1:4317**. Node.js 22 or newer, ripgrep, and FFmpeg are required. This checkout already has the local voice environment and downloaded model files. They are cached and excluded from version control.

For a different repository, use **Open repository**, enter its local checkout path, and choose architecture, feature, or change explanation. Change explanations also require a Git base reference. Use an existing configured Codex or Claude CLI login; the application does not buy or provision model access. A locally installed CLI does not imply its model runs on-device.

## What works

- Saved source snapshots, content hashes, numbered excerpts, and source inspection.
- Codex and Claude CLI adapters for structured explanations and follow-up questions.
- Citation checks for source paths, line ranges, exact quotes, and evidence kinds.
- Composable flow, sequence, branch, state, memory, comparison, and code scenes.
- Narration starts from a developer’s problem, introduces responsibilities as they become relevant, and explains behavior through a concrete situation. Design intent needs documented evidence; inferred design consequences are labeled. Visuals support that explanation rather than supplying its script. Chapters have room for detail, capped at 160 spoken words.
- A versioned narrative contract rewrites older animation commentary before narration or export, preserving chapter IDs, source snapshots, and position in the journey.
- Audio playback, seeking, slower speed options, volume control, compact layouts, and exploratory chapters.
- A stack of visited explanations with restored playback positions. Loading, playing, and paused details block further exploration; Back remains available and detaches late responses.
- File-specific narrated walkthroughs and a one-click simpler explanation.
- A readable light theme and responsive layouts selected from the player width.
- Visual-form selection with a reason, step-specific evidence, and local spoken-word timing when alignment succeeds.
- Local Qwen3-TTS **1.7B** on this machine’s AMD RX 580 through Vulkan, with a fixed Serena reference. The native named Serena speaker on CPU remains configurable.
- Cached explanations and narration; repeated exploration reopens an existing chapter.
- Local narrated H.264/AAC MP4 export with the same SVG renderer used by the player.

## Code map

| Responsibility | Source |
| --- | --- |
| Local HTTP API and background jobs | `src/server/index.mjs` |
| Repository capture, source search, Git comparisons | `src/server/repository.mjs` |
| Agent prompts and CLI adapters | `src/server/agent.mjs` |
| Story contract and evidence validation | `src/shared/story.mjs` |
| Reusable visual components and timeline rendering | `src/shared/render.mjs` |
| Navigation stack and narration timeline | `src/shared/journey.mjs`, `src/shared/timeline.mjs` |
| Narration cache and local speech backends | `src/server/speech.mjs`, `scripts/narrate_serena.py`, `scripts/audition_gpu.py` |
| SVG rasterization and video encoding | `src/server/export.mjs` |
| Curated source-backed introduction | `src/server/self-story.mjs` |
| Player, source viewer, and chat | `public/app.mjs` |

`config/settings.json` selects the narrator, voice reference, GPU/CPU backend, CLI model, reasoning effort, and export resolution. Codex explanations explicitly use `gpt-6-luna` with `xhigh` reasoning, and both settings participate in the explanation cache identity. `project-plan.json` records the scope and remaining work.

## Verify

```sh
npm test
npm run check
```

Tests exercise double-click locking, paused-detail locking, Back during loading, late-response rejection, focused retrieval, spoken-cue timing, fabricated citations, incorrect evidence kinds, malformed scenes, deterministic rendering, all scene templates, immutable snapshots, symlink boundaries, and actual Git before/after comparisons.

## Voice assets

The default GPU path uses Qwen3-TTS 1.7B Base through Vulkan and the fixed full-precision Serena reference. It requires runtime logs to confirm layer offloading before accepting generated audio. This is not a ROCm/PyTorch path: this checkout has CPU-only PyTorch and no ROCm installation. For the native named Serena speaker, set `ttsEngine: "custom-voice"`; its pinned `Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice` revision is `0c0e3051f131929182e2c023b9537f8b1c68adfe`, and it runs offline in float32 on CPU. The two backends use different conditioning and may sound different. Matching a transcript does not establish voice naturalness.

Narration keeps a target of roughly 120 words per minute, close to the accepted introduction pace. Small bounded tempo adjustments use FFmpeg atempo with pitch preserved. Audio is mastered to −25 LUFS with a −8 dB peak ceiling and delivered as lossless 48 kHz / 24-bit stereo WAV (the mono speech is centered equally in both channels). Video uses 192 kb/s stereo AAC. The player keeps the selected volume and defaults to 60%. Voice, model, text, pacing, and mastering settings participate in the cache identity.

Vulkan reference cloning uses `ttsEngine: "voice-clone"` and `ttsBackend: "vulkan"`. A clean full-precision Serena reference is available at `experiments/voice/output/serena-reference-fp32.wav`; aggressive int8 quantization of all Python model layers produced invalid speech and is not used. The GPU runtime is official llama.cpp **b11146** (archive SHA256 `d3ce40fce7403cc93bcf5718fc46c6efb61ed9709f8e5d9f10c86bf0e30e8fb3`), with `ggml-org/Qwen3-TTS-12Hz-1.7B-Base-GGUF` revision `ca27d74bc954b73dadab5b71ca265d87fc861a7c`, Q8_0 model and projector. Local paths are in `.cache/voice/gpu-model.json`.

The Python environment uses `requirements-voice-cpu.txt`, `requirements-voice.txt`, and the audition lock file.

See the [official Qwen3-TTS repository](https://github.com/QwenLM/Qwen3-TTS) and [llama.cpp TTS documentation](https://github.com/ggml-org/llama.cpp/tree/master/tools/tts) for model setup. `scripts/check_narration.py` optionally checks local speech intelligibility using a cached Whisper tiny.en model and `requirements-audio-evaluation.txt`. Transcription checks the words, not whether a voice is pleasant or perceptually identical.

Speech stays on this machine; the configured explanation CLI may use a hosted model.

## Current limits

Exact citation matching verifies origin, not semantic correctness. Inferences remain interpretations. New scenes provide exact narration cues and relevant claim indices. Local Whisper word timestamps align recognized words to the known narration; missing or uncertain cues fall back to estimated timing, labeled in the player. Older scenes without cues also use estimates. Search combines lexical ranking, the selected responsibility, cited-file priority, and heuristic static import hints; it is not a full symbol or call graph. Source-reading highlights are a guide, not recorded execution. Repository capture currently includes up to 2,000 readable files, 250 KB per file, and roughly 12 MB total; omitted files are counted. Binary assets, generated directories, and common credential directories are excluded.

Sourcebot/Zoekt integration and language-aware navigation are follow-on work. Sourcebot’s paid MCP layer is not a dependency of this project. Rendering uses the free sharp library and local FFmpeg; it does not use a hosted video service.

The interactive component gallery is at **http://127.0.0.1:4317/visual-language.html**. It explains when each visual form is useful and makes missing before/after evidence explicit.

Writing references: [Greg Wilson’s A Simple Web Server](https://aosabook.org/en/500L/a-simple-web-server.html) introduces a design as new requirements make it useful; [Why SQLite Uses Bytecode](https://www.sqlite.org/whybytecode.html) explains an architectural decision through benefits and tradeoffs; [Thinking in React](https://react.dev/learn/thinking-in-react) develops component boundaries around a concrete example. The narrator applies those patterns to supplied source evidence. Visual timing still follows multimedia learning principles, but the narration explains software rather than the animation. The application has not established learning gains for code comprehension.
