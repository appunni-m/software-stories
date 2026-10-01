#!/usr/bin/env python3
"""Generate a chapter with the fixed Qwen3-TTS 1.7B Serena speaker offline."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import time

from audition_voices import CACHE, progress, write_json

MODEL = "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--text-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--instructions", required=True)
    args = parser.parse_args()
    text = args.text_file.read_text().strip()
    if not text or len(text) > 4000:
        parser.error("Narration must contain 1–4000 characters.")
    pins = json.loads((CACHE / "model-revisions.json").read_text())
    revision = pins[MODEL]
    directory = CACHE / "huggingface/hub/models--Qwen--Qwen3-TTS-12Hz-1.7B-CustomVoice/snapshots" / revision
    if not directory.is_dir():
        raise FileNotFoundError("Install the pinned Qwen3-TTS 1.7B CustomVoice model first.")
    os.environ.update(HF_HOME=str(CACHE / "huggingface"), HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", HF_HUB_DISABLE_TELEMETRY="1", TOKENIZERS_PARALLELISM="false", NUMBA_CACHE_DIR=str(CACHE / "numba"))
    import numpy as np
    import soundfile as sf
    import torch
    from qwen_tts import Qwen3TTSModel
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    started = time.monotonic()
    with progress("Loading the fixed Serena speaker"):
        model = Qwen3TTSModel.from_pretrained(str(directory), device_map="cpu", dtype=torch.float32, attn_implementation="sdpa", local_files_only=True)
        model.model.eval()
    torch.manual_seed(42)
    with progress("Serena is speaking"):
        waveforms, rate = model.generate_custom_voice(text=text, language="English", speaker="Serena", instruct=args.instructions, non_streaming_mode=True, max_new_tokens=1600, temperature=0.7, subtalker_temperature=0.7)
    waveform = np.asarray(waveforms[0], dtype=np.float32)
    if waveform.ndim != 1 or not waveform.size or not np.isfinite(waveform).all() or not np.any(waveform):
        raise ValueError("Invalid generated waveform.")
    # Save the original floating-point waveform; mastering happens separately.
    args.output.parent.mkdir(parents=True, exist_ok=True)
    sf.write(args.output, waveform, rate, subtype="FLOAT")
    write_json(args.result, {"audio_path": str(args.output.resolve()), "audio_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(), "duration_seconds": waveform.size / rate, "total_seconds": time.monotonic() - started, "sample_rate": rate, "model": MODEL, "revision": revision, "speaker": "Serena", "backend": "cpu", "gpu_layer_offload_verified": False})


if __name__ == "__main__":
    main()
