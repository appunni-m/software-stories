"""Deterministic worker-contract tests using installed NumPy/PyTorch, no models.

Run with the app-managed Python. Model loading is replaced with finite sample
chunks; these tests do not assess voice quality or perform any download.
"""

import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import wave

import torch

SPEC = importlib.util.spec_from_file_location(
    "atlas_pocket_worker", Path(__file__).parents[1] / "resources/pocket_tts_worker.py"
)
worker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(worker)


class Model:
    def __init__(self, fail_at=None):
        self.calls = []
        self.fail_at = fail_at

    def generate_audio_stream(self, state, text, **options):
        self.calls.append((text, options))
        if len(self.calls) == self.fail_at:
            raise ValueError("synthetic_later_segment_failure")
        yield torch.tensor([0.25, -0.25])


class WorkerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="atlas-pocket-contract-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "assets").mkdir()
        (self.root / "model.yaml").write_text("synthetic config")
        assets = {}
        for name in worker.ASSETS:
            path = self.root / "assets" / name
            path.write_bytes(b"synthetic asset")
            assets[name] = {"size": path.stat().st_size, "sha256": worker.digest(path)}
        (self.root / "ready.json").write_text(json.dumps({
            "modelId": worker.MODEL_ID, "configSha256": worker.digest(self.root / "model.yaml"),
            "assets": assets,
        }))

    def run_worker(self, payload, model):
        stdin = io.TextIOWrapper(io.BytesIO(payload))
        with patch.object(worker.sys, "stdin", stdin), patch.object(
            worker, "load_local_model", return_value=(model, {"voice": "alba"})
        ) as load:
            try:
                worker.synthesize(self.root)
                return load.call_count
            finally:
                self.last_load_count = load.call_count

    def test_whole_script_loads_once_preserves_segment_order_and_pause(self):
        segments = ["First reviewed sentence. ", "Second reviewed sentence."]
        model = Model()
        loads = self.run_worker(json.dumps({"schemaVersion": 1, "segments": segments}).encode(), model)
        self.assertEqual(loads, 1)
        self.assertEqual([text for text, _ in model.calls], segments)
        self.assertTrue(all(options == {"copy_state": True} for _, options in model.calls))
        with wave.open(str(self.root / "active.wav"), "rb") as audio:
            self.assertEqual((audio.getnchannels(), audio.getsampwidth(), audio.getframerate()), (1, 2, 24000))
            self.assertEqual(audio.getnframes(), 2 + 4800 + 2)
            pcm = audio.readframes(audio.getnframes())
        self.assertEqual(pcm[:4], pcm[-4:])
        self.assertEqual(pcm[4:-4], bytes(9600))

    def test_later_segment_failure_removes_incomplete_output(self):
        model = Model(fail_at=2)
        with self.assertRaises(ValueError):
            self.run_worker(json.dumps({"schemaVersion": 1, "segments": ["First.", "Second."]}).encode(), model)
        self.assertFalse((self.root / "active.wav").exists())

    def test_invalid_packet_never_loads_a_model_or_writes_audio(self):
        for payload in [b"plain text", b'{"schemaVersion":1,"schemaVersion":1,"segments":["x"]}',
                        b'{"schemaVersion":true,"segments":["x"]}',
                        b'{"schemaVersion":2,"segments":["x"]}',
                        b'{"schemaVersion":1,"segments":[],"extra":true}',
                        b'{"schemaVersion":1,"segments":[" "]}',
                        json.dumps({"schemaVersion":1,"segments":["x" * 4001]}).encode(),
                        json.dumps({"schemaVersion":1,"segments":["x"] * 65}).encode(),
                        json.dumps({"schemaVersion":1,"segments":["x" * 4000] * 5}).encode()]:
            with self.subTest(payload_bytes=len(payload)):
                model = Model()
                with self.assertRaises((ValueError, UnicodeError)):
                    self.run_worker(payload, model)
                self.assertEqual(self.last_load_count, 0)
                self.assertEqual(model.calls, [])
                self.assertFalse((self.root / "active.wav").exists())

    def test_empty_later_segment_or_nonfinite_audio_discards_the_whole_script(self):
        class BadModel:
            def __init__(self, chunk):
                self.calls = 0
                self.chunk = chunk

            def generate_audio_stream(self, state, text, **options):
                self.calls += 1
                yield torch.tensor([0.25]) if self.calls == 1 else self.chunk

        for chunk in [torch.tensor([]), torch.tensor([float("nan")]), torch.tensor([float("inf")])]:
            with self.subTest(chunk=chunk):
                with self.assertRaises(ValueError):
                    self.run_worker(json.dumps({"schemaVersion": 1, "segments": ["First.", "Second."]}).encode(), BadModel(chunk))
                self.assertFalse((self.root / "active.wav").exists())

    def test_join_pause_and_pcm_both_count_against_the_output_limit(self):
        packet = json.dumps({"schemaVersion": 1, "segments": ["First.", "Second."]}).encode()
        for maximum in [44 + 4 + 9599, 44 + 4 + 9600 + 3]:
            with self.subTest(maximum=maximum), patch.object(worker, "MAX_WAV_BYTES", maximum):
                with self.assertRaises(ValueError):
                    self.run_worker(packet, Model())
                self.assertFalse((self.root / "active.wav").exists())

    def test_utf8_total_boundary_and_json_escape_expansion_preserve_exact_text(self):
        segments = ["é" * 2000] * 4
        model = Model()
        self.run_worker(json.dumps({"schemaVersion": 1, "segments": segments}).encode(), model)
        self.assertEqual([text for text, _ in model.calls], segments)
        (self.root / "active.wav").unlink()
        with self.assertRaises(ValueError):
            self.run_worker(json.dumps({"schemaVersion": 1, "segments": ["é" * 2001]}).encode(), Model())
        self.assertEqual(self.last_load_count, 0)


if __name__ == "__main__":
    unittest.main()
