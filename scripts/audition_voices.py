#!/usr/bin/env python3
"""Generate comparable local Qwen3-TTS auditions and measured audio metadata."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import gc
import hashlib
from importlib.metadata import version
import json
import os
from pathlib import Path
import re
import threading
import time


ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / ".cache" / "voice"


def write_json(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def read_config(path: Path) -> dict:
    config = json.loads(path.read_text(encoding="utf-8"))
    if config.get("version") != 1:
        raise ValueError("Expected audition config version 1.")
    for field in ("language", "text", "smoke_text"):
        if not isinstance(config.get(field), str) or not config[field].strip():
            raise ValueError(f"{field} must be a nonempty string.")
    if not isinstance(config.get("seed"), int):
        raise ValueError("seed must be an integer.")
    voices = config.get("voices")
    if not isinstance(voices, list) or not voices:
        raise ValueError("At least one voice is required.")
    identifiers = set()
    for voice in voices:
        identifier = voice.get("id", "")
        if not isinstance(identifier, str) or not re.fullmatch(r"[a-z0-9-]+", identifier):
            raise ValueError("Voice IDs must contain lowercase letters, numbers, or hyphens.")
        if identifier in identifiers:
            raise ValueError(f"Duplicate voice ID: {identifier}")
        identifiers.add(identifier)
        expected = {
            "preset": "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
            "designed": "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign",
        }.get(voice.get("kind"))
        if expected is None or voice.get("model") != expected:
            raise ValueError(f"{identifier}: unsupported voice kind or model.")
        if not isinstance(voice.get("instructions"), str) or not voice["instructions"].strip():
            raise ValueError(f"{identifier}: instructions are required.")
        if voice["kind"] == "preset" and not voice.get("speaker"):
            raise ValueError(f"{identifier}: a preset speaker is required.")
    return config


@contextmanager
def progress(label: str):
    stop = threading.Event()
    started = time.monotonic()

    def heartbeat():
        while not stop.wait(20):
            print(f"{label}: {time.monotonic() - started:.0f}s elapsed", flush=True)

    worker = threading.Thread(target=heartbeat, daemon=True)
    worker.start()
    try:
        yield
    finally:
        stop.set()
        worker.join()


def pin_revision(model_id: str, offline: bool) -> str:
    from huggingface_hub import HfApi

    path = CACHE / "model-revisions.json"
    pins = json.loads(path.read_text()) if path.exists() else {}
    if model_id not in pins:
        if offline:
            raise ValueError(f"No pinned revision for {model_id}; run online once first.")
        pins[model_id] = HfApi().model_info(model_id, token=False).sha
        write_json(path, pins)
    return pins[model_id]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=ROOT / "experiments/voice/audition.json")
    parser.add_argument("--output", type=Path, default=ROOT / "experiments/voice/output")
    parser.add_argument("--voice", action="append", help="Voice ID to generate; repeat to select several.")
    parser.add_argument("--smoke", action="store_true", help="Generate the shorter test sentence.")
    parser.add_argument("--check", action="store_true", help="Validate configuration without dependencies.")
    parser.add_argument("--offline", action="store_true", help="Use previously downloaded models only.")
    parser.add_argument("--device", choices=("cpu", "cuda", "mps"), default="cpu")
    parser.add_argument("--precision", choices=("float32", "bfloat16"), default="float32")
    parser.add_argument("--int8", action="store_true", help="Quantize CPU linear layers to reduce memory.")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--max-new-tokens", type=int, default=512)
    args = parser.parse_args()
    config = read_config(args.config)
    known = {voice["id"] for voice in config["voices"]}
    requested = set(args.voice or known)
    if requested - known:
        parser.error(f"Unknown voices: {', '.join(sorted(requested - known))}")
    if args.threads < 1 or args.max_new_tokens < 1:
        parser.error("threads and max-new-tokens must be positive.")
    if args.int8 and (args.device != "cpu" or args.precision != "float32"):
        parser.error("int8 linear quantization requires CPU and float32 loading.")
    if args.check:
        print(json.dumps({"status": "valid", "voices": sorted(requested)}, indent=2))
        return

    # Keep all caches inside this workspace; never change the user's global runtime.
    os.environ["HF_HOME"] = str(CACHE / "huggingface")
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    os.environ["HF_HUB_DISABLE_IMPLICIT_TOKEN"] = "1"
    os.environ["NUMBA_CACHE_DIR"] = str(CACHE / "numba")
    os.environ["TOKENIZERS_PARALLELISM"] = "false"
    if args.offline:
        os.environ["HF_HUB_OFFLINE"] = "1"
        os.environ["TRANSFORMERS_OFFLINE"] = "1"

    import numpy as np
    import soundfile as sf
    import torch
    from qwen_tts import Qwen3TTSModel

    torch.set_num_threads(args.threads)
    torch.set_num_interop_threads(1)
    dtype = torch.float32 if args.precision == "float32" else torch.bfloat16
    text = config["smoke_text"] if args.smoke else config["text"]
    args.output.mkdir(parents=True, exist_ok=True)
    report = {"text": text, "language": config["language"], "voices": []}
    report_path = args.output / ("smoke-report.json" if args.smoke else "report.json")

    for voice in config["voices"]:
        if voice["id"] not in requested:
            continue
        revision = pin_revision(voice["model"], args.offline)
        identity = {
            "voice": voice,
            "revision": revision,
            "text": text,
            "language": config["language"],
            "seed": config["seed"],
            "device": args.device,
            "precision": args.precision,
            "int8_linear_layers": args.int8,
            "max_new_tokens": args.max_new_tokens,
            "qwen_tts_version": version("qwen-tts"),
            "torch_version": torch.__version__,
        }
        digest = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
        basename = voice["id"] + ("-smoke" if args.smoke else "") + "-" + digest[:12]
        audio_path = args.output / (basename + ".wav")
        metadata_path = args.output / (basename + ".json")
        if audio_path.exists() and metadata_path.exists():
            metadata = json.loads(metadata_path.read_text())
            if hashlib.sha256(audio_path.read_bytes()).hexdigest() == metadata["audio_sha256"]:
                print(f"Reusing {audio_path}", flush=True)
                report["voices"].append(metadata)
                write_json(report_path, report)
                continue

        print(f"Loading {voice['model']} at {revision}", flush=True)
        with progress(f"Loading {voice['id']}"):
            model = Qwen3TTSModel.from_pretrained(
                str(CACHE / "huggingface/hub" / ("models--" + voice["model"].replace("/", "--")) / "snapshots" / revision) if args.offline else voice["model"],
                revision=revision,
                device_map=args.device,
                dtype=dtype,
                attn_implementation="sdpa",
                low_cpu_mem_usage=True,
                local_files_only=args.offline,
            )
            if args.int8:
                torch.ao.quantization.quantize_dynamic(
                    model.model, {torch.nn.Linear}, dtype=torch.qint8, inplace=True
                )
                gc.collect()
            model.model.eval()

        torch.manual_seed(config["seed"])
        started = time.monotonic()
        print(f"Generating {voice['id']}: {text}", flush=True)
        with progress(f"Generating {voice['id']}"):
            generation = {
                "text": text,
                "language": config["language"],
                "instruct": voice["instructions"],
                "non_streaming_mode": True,
                "max_new_tokens": args.max_new_tokens,
            }
            if voice["kind"] == "preset":
                wavs, sample_rate = model.generate_custom_voice(
                    speaker=voice["speaker"], **generation
                )
            else:
                wavs, sample_rate = model.generate_voice_design(**generation)
        elapsed = time.monotonic() - started
        waveform = np.asarray(wavs[0], dtype=np.float32)
        if waveform.ndim != 1 or waveform.size == 0 or not np.isfinite(waveform).all():
            raise ValueError(f"{voice['id']}: invalid generated waveform.")
        peak = float(np.max(np.abs(waveform)))
        if peak == 0 or peak > 1:
            raise ValueError(f"{voice['id']}: silent or out-of-range waveform (peak={peak}).")
        sf.write(audio_path, waveform, sample_rate, subtype="PCM_16")
        duration = waveform.size / sample_rate
        metadata = {
            **identity,
            "id": voice["id"],
            "audio_path": str(audio_path.resolve()),
            "audio_sha256": hashlib.sha256(audio_path.read_bytes()).hexdigest(),
            "duration_seconds": round(duration, 3),
            "generation_seconds": round(elapsed, 3),
            "real_time_factor": round(elapsed / duration, 3),
            "sample_rate": sample_rate,
            "peak_amplitude": round(peak, 6),
            "rms_amplitude": round(float(np.sqrt(np.mean(waveform ** 2))), 6),
            "quality_status": "needs_listening",
        }
        write_json(metadata_path, metadata)
        report["voices"].append(metadata)
        write_json(report_path, report)
        print(f"Saved {audio_path} ({duration:.1f}s audio in {elapsed:.1f}s)", flush=True)
        del model, wavs, waveform
        gc.collect()
    print(f"Audition report: {report_path}", flush=True)


if __name__ == "__main__":
    main()
