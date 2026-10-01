#!/usr/bin/env python3
"""Audition a generated voice with Qwen3-TTS 1.7B on Vulkan or CPU."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

from audition_voices import ROOT, read_config, write_json


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--backend", choices=("vulkan", "cpu"), default="vulkan")
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--max-frames", type=int, default=512)
    parser.add_argument("--cpu-codec", action="store_true", help="Keep the audio codec on CPU.")
    parser.add_argument("--text-file", type=Path, help="Narration text instead of the audition passage.")
    parser.add_argument("--output", type=Path, help="Directory for generated clips.")
    parser.add_argument("--result", type=Path, help="Write the resulting metadata to this JSON file.")
    args = parser.parse_args()
    if args.max_frames < 1:
        parser.error("max-frames must be positive")
    reference = args.reference.resolve(strict=True)
    config = read_config(ROOT / "experiments/voice/audition.json")
    text = config["smoke_text"] if args.smoke else config["text"]
    if args.text_file:
        text = args.text_file.read_text(encoding="utf-8").strip()
    if not text or len(text) > 4000:
        parser.error("Narration must contain 1–4000 characters.")
    manifest = json.loads((ROOT / ".cache/voice/gpu-model.json").read_text())
    runtime = ROOT / ".cache/voice/llama/b11146"
    provenance = json.loads((runtime / "provenance.json").read_text())
    binary = runtime / "llama-b11146/llama-tts"
    directory = Path(manifest["directory"])
    model = directory / "Qwen3-TTS-12Hz-1.7B-Base-Q8_0.gguf"
    codec = directory / "mmproj-Qwen3-TTS-12Hz-1.7B-Base-Q8_0.gguf"
    for path in (binary, model, codec):
        if not path.is_file():
            raise FileNotFoundError(path)
    identity = {
        "backend": args.backend,
        "cpu_codec": args.cpu_codec or args.backend == "cpu",
        "model": manifest["model"],
        "revision": manifest["revision"],
        "model_sha256": sha256(model),
        "codec_sha256": sha256(codec),
        "runtime": provenance,
        "reference": str(reference),
        "reference_sha256": sha256(reference),
        "text": text,
        "language": "en",
        "seed": config["seed"],
        "max_frames": args.max_frames,
        "log_verbosity": 4,
    }
    digest = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()[:12]
    output = args.output.resolve() if args.output else ROOT / "experiments/voice/output"
    output.mkdir(parents=True, exist_ok=True)
    basename = f"{reference.stem}-{args.backend}-{digest}"
    audio = output / f"{basename}.wav"
    metadata_path = audio.with_suffix(".json")
    log_path = audio.with_suffix(".log")
    if audio.exists() and metadata_path.exists():
        metadata = json.loads(metadata_path.read_text())
        verified = args.backend == "cpu" or metadata.get("gpu_layer_offload_verified") is True
        if verified and sha256(audio) == metadata["audio_sha256"]:
            if args.result:
                write_json(args.result, metadata)
            print(f"Reusing {audio}", flush=True)
            return
    partial = audio.with_suffix(".partial.wav")
    command = [
        str(binary), "-m", str(model), "--mmproj", str(codec),
        "--device", "Vulkan0" if args.backend == "vulkan" else "none",
        "-ngl", "99" if args.backend == "vulkan" else "0",
        "--tts-lang", "en", "--tts-speaker-file", str(reference),
        "-p", text, "-o", str(partial), "-n", str(args.max_frames),
        "-t", "4", "-c", "2048", "--seed", str(config["seed"]),
        "--temp", "0.9", "--top-k", "50", "--top-p", "1",
        "--min-p", "0", "--repeat-penalty", "1", "--offline", "--perf",
        "--verbosity", "4",
    ]
    if identity["cpu_codec"]:
        command.append("--no-mmproj-offload")
    environment = os.environ.copy()
    environment["VK_ICD_FILENAMES"] = "/usr/share/vulkan/icd.d/radeon_icd.json"
    environment["XDG_CACHE_HOME"] = str(ROOT / ".cache")
    started = time.monotonic()
    print(f"Generating on {args.backend}; log: {log_path}", flush=True)
    with log_path.open("w") as log:
        process = subprocess.Popen(command, env=environment, stdout=log, stderr=subprocess.STDOUT)
        try:
            while True:
                try:
                    code = process.wait(timeout=20)
                    break
                except subprocess.TimeoutExpired:
                    elapsed = time.monotonic() - started
                    print(f"{args.backend}: {elapsed:.0f}s elapsed", flush=True)
                    if elapsed > 600:
                        raise TimeoutError("Speech generation exceeded 10 minutes.")
        except BaseException:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise
    elapsed = time.monotonic() - started
    if code:
        raise RuntimeError(f"llama-tts exited with {code}; inspect {log_path}")
    log_text = log_path.read_text(errors="replace")
    offload_lines = [
        line for line in log_text.splitlines()
        if "offloaded" in line or "Vulkan0 model buffer" in line
    ]
    gpu_verified = any(re.search(r"offloaded [1-9]\d*/\d+ layers to GPU", line) for line in offload_lines)
    if args.backend == "vulkan" and not gpu_verified:
        raise RuntimeError(f"GPU layer offloading was not confirmed; inspect {log_path}")
    import numpy as np
    import soundfile as sf
    waveform, sample_rate = sf.read(partial, dtype="float32")
    if waveform.ndim != 1 or waveform.size == 0 or not np.isfinite(waveform).all():
        raise ValueError("Generated audio is empty, non-mono, or nonfinite.")
    if np.max(np.abs(waveform)) == 0:
        raise ValueError("Generated audio is silent.")
    partial.replace(audio)
    duration = waveform.size / sample_rate
    metadata = {
        **identity,
        "audio_path": str(audio),
        "audio_sha256": sha256(audio),
        "duration_seconds": round(duration, 3),
        "total_seconds": round(elapsed, 3),
        "real_time_factor_including_load": round(elapsed / duration, 3),
        "sample_rate": sample_rate,
        "peak_amplitude": float(np.max(np.abs(waveform))),
        "quality_status": "needs_listening",
        "log_path": str(log_path),
        "gpu_offload_log": offload_lines,
        "gpu_layer_offload_verified": gpu_verified,
        "command": command,
    }
    write_json(metadata_path, metadata)
    if args.result:
        write_json(args.result, metadata)
    print(f"Saved {audio} ({duration:.1f}s audio in {elapsed:.1f}s including load)", flush=True)


if __name__ == "__main__":
    main()
