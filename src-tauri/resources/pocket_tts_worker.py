"""Code Atlas's finite Pocket TTS worker. No server, repository access, or CLI text args."""

import hashlib
import importlib.metadata
import json
import logging
import os
from pathlib import Path
import sys
import wave

PACKAGE_VERSION = "3.3.0"
PYTHON_VERSION = "3.13.14"
PACKAGES = {
    "pocket-tts": PACKAGE_VERSION,
    "torch": "2.14.1",
    "numpy": "2.5.4",
    "huggingface-hub": "2.2.0",
    "pyyaml": "6.0.3",
}
MODEL_ID = "pocket-tts-3.3.0:english_2026-09:e7205b6e:alba:4e1e0a3e:pcm-v1"
MAX_TEXT_BYTES = 16_000
MAX_SEGMENT_BYTES = 4_000
MAX_SEGMENTS = 64
MAX_PACKET_BYTES = MAX_TEXT_BYTES * 6 + 1024
MAX_WAV_BYTES = 12 * 1024 * 1024
SEGMENT_PAUSE_PCM = bytes(9_600)  # 200 ms, mono PCM16 at 24 kHz.
REPOSITORY = "kyutai/pocket-tts-without-voice-cloning"
ASSETS = {
    "model.safetensors": (
        "languages/english_2026-09/model.safetensors",
        "e7205b6ee50e654a5ea19f0e9df2b0813b05e921",
    ),
    "tokenizer.json": (
        "languages/english_2026-09/tokenizer.json",
        "00eac05ed3d16bdc3f6b5d598874019c34a89214",
    ),
    "alba.safetensors": (
        "languages/english_2026-09/embeddings/alba.safetensors",
        "4e1e0a3e611c51c0b4ed8174fc10f32a54644303",
    ),
}


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def load_local_model(root):
    from pocket_tts import TTSModel

    model = TTSModel.load_model(config=root / "model.yaml").to("cpu")
    if model.sample_rate != 24_000:
        raise ValueError("unexpected_sample_rate")
    state = model.get_state_for_audio_prompt(root / "assets" / "alba.safetensors")
    return model, state


def setup(root):
    import shutil
    import pocket_tts
    import yaml
    from huggingface_hub import hf_hub_download

    assets = root / "assets"
    assets.mkdir(exist_ok=True)
    manifest_assets = {}
    for name, (filename, revision) in ASSETS.items():
        source = hf_hub_download(
            repo_id=REPOSITORY,
            filename=filename,
            revision=revision,
            token=False,
        )
        target = assets / name
        temporary = assets / (name + ".part")
        shutil.copyfile(source, temporary)
        temporary.replace(target)
        manifest_assets[name] = {"size": target.stat().st_size, "sha256": digest(target)}

    source_config = Path(pocket_tts.__file__).parent / "config" / "english_2026-09.yaml"
    config = yaml.safe_load(source_config.read_text(encoding="utf-8"))
    # Local, fixed model/embedding files prevent gated voice-cloning downloads
    # and remove every remote path from the generation configuration.
    config["weights_path"] = str(assets / "model.safetensors")
    config["weights_path_without_voice_cloning"] = None
    config["flow_lm"]["lookup_table"]["tokenizer_path"] = str(assets / "tokenizer.json")
    config_path = root / "model.yaml"
    config_path.write_text(yaml.safe_dump(config), encoding="utf-8")
    os.environ["HF_HUB_OFFLINE"] = "1"
    load_local_model(root)
    manifest = {
        "modelId": MODEL_ID,
        "packageVersion": PACKAGE_VERSION,
        "pythonVersion": PYTHON_VERSION,
        "packages": PACKAGES,
        "assets": manifest_assets,
        "configSha256": digest(config_path),
    }
    temporary = root / "ready.json.part"
    temporary.write_text(json.dumps(manifest), encoding="utf-8")
    temporary.replace(root / "ready.json")


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate_packet_key")
        result[key] = value
    return result


def reviewed_segments():
    # Text arrives only over an anonymous pipe and is never stored or logged.
    raw = sys.stdin.buffer.read(MAX_PACKET_BYTES + 1)
    if not raw or len(raw) > MAX_PACKET_BYTES:
        raise ValueError("invalid_packet_size")
    packet = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)
    if (not isinstance(packet, dict) or set(packet) != {"schemaVersion", "segments"}
            or type(packet["schemaVersion"]) is not int or packet["schemaVersion"] != 1):
        raise ValueError("invalid_packet")
    segments = packet["segments"]
    if not isinstance(segments, list) or not 1 <= len(segments) <= MAX_SEGMENTS:
        raise ValueError("invalid_segments")
    if any(not isinstance(text, str) or not text.strip()
           or len(text.encode("utf-8")) > MAX_SEGMENT_BYTES for text in segments):
        raise ValueError("invalid_segment")
    if sum(len(text.encode("utf-8")) for text in segments) > MAX_TEXT_BYTES:
        raise ValueError("invalid_text_size")
    return segments


def synthesize(root):
    segments = reviewed_segments()
    import numpy as np
    import torch

    manifest = json.loads((root / "ready.json").read_text(encoding="utf-8"))
    if manifest["modelId"] != MODEL_ID or manifest["configSha256"] != digest(root / "model.yaml"):
        raise ValueError("runtime_changed")
    for name in ASSETS:
        asset = root / "assets" / name
        expected = manifest["assets"][name]
        if asset.stat().st_size != expected["size"] or digest(asset) != expected["sha256"]:
            raise ValueError("asset_changed")
    model, state = load_local_model(root)
    destination = root / "active.wav"
    pcm_bytes = 0
    try:
        # Pocket streams through background threads. Inference-mode tensors
        # cannot be mutated by its no-grad cache workers across that boundary.
        with torch.no_grad(), wave.open(str(destination), "wb") as audio:
            audio.setnchannels(1)
            audio.setsampwidth(2)
            audio.setframerate(24_000)
            for index, text in enumerate(segments):
                if index:
                    pcm_bytes += len(SEGMENT_PAUSE_PCM)
                    if pcm_bytes + 44 > MAX_WAV_BYTES:
                        raise ValueError("audio_too_large")
                    audio.writeframesraw(SEGMENT_PAUSE_PCM)
                segment_bytes = 0
                # Preserve the built-in voice prompt across sequential segments.
                # Pocket may mutate a generation's state in background threads.
                for chunk in model.generate_audio_stream(state, text, copy_state=True):
                    values = chunk.detach().cpu().numpy().reshape(-1)
                    if not np.isfinite(values).all():
                        raise ValueError("nonfinite_audio")
                    size = values.size * 2
                    if pcm_bytes + size + 44 > MAX_WAV_BYTES:
                        raise ValueError("audio_too_large")
                    pcm = (np.clip(values, -1.0, 1.0) * 32767).astype("<i2").tobytes()
                    pcm_bytes += size
                    segment_bytes += size
                    audio.writeframesraw(pcm)
                if segment_bytes == 0:
                    raise ValueError("empty_audio")
    except BaseException:
        destination.unlink(missing_ok=True)
        raise


def main():
    logging.disable(logging.CRITICAL)
    for package, version in PACKAGES.items():
        if importlib.metadata.version(package) != version:
            raise ValueError("package_changed")
    if ".".join(map(str, sys.version_info[:3])) != PYTHON_VERSION:
        raise ValueError("python_changed")
    if len(sys.argv) != 3 or sys.argv[1] not in ("setup", "synthesize"):
        raise ValueError("invalid_operation")
    root = Path(sys.argv[2]).resolve(strict=True)
    if sys.argv[1] == "setup":
        setup(root)
    else:
        synthesize(root)


if __name__ == "__main__":
    try:
        main()
    except BaseException:
        # Dependencies can include input text in exceptions. Never forward it.
        sys.exit(1)
