#!/usr/bin/env python3
"""Align the known narration to local word timestamps; retain uncertainty."""
import argparse
from difflib import SequenceMatcher
import json
import math
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
def tokens(text):
    return re.findall(r"[^\W_]+", text.lower(), re.UNICODE)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--text-file', type=Path, required=True)
    parser.add_argument('--audio', type=Path, required=True)
    parser.add_argument('--result', type=Path, required=True)
    args = parser.parse_args()
    import soundfile as sf
    from scipy.signal import resample_poly
    from faster_whisper import WhisperModel
    signal, rate = sf.read(args.audio, dtype='float32')
    if signal.ndim == 2:
        signal = signal.mean(axis=1)
    common = math.gcd(rate, 16000)
    signal = resample_poly(signal, 16000 // common, rate // common)
    model = WhisperModel('tiny.en', device='cpu', compute_type='int8', cpu_threads=2, download_root=str(ROOT / '.cache/voice/asr'), local_files_only=True)
    segments, _ = model.transcribe(signal, language='en', beam_size=5, word_timestamps=True, condition_on_previous_text=False)
    observed = []
    for segment in segments:
        for word in segment.words:
            parts = tokens(word.word)
            for i, part in enumerate(parts):
                interval = (word.end-word.start)/len(parts)
                observed.append({'word': part, 'start': word.start+i*interval, 'end': word.start+(i+1)*interval})
    known = tokens(args.text_file.read_text())
    aligned = [None] * len(known)
    for block in SequenceMatcher(None, known, [word['word'] for word in observed], autojunk=False).get_matching_blocks():
        for offset in range(block.size):
            word = observed[block.b+offset]
            aligned[block.a+offset] = {'start': round(word['start'], 3), 'end': round(word['end'], 3)}
    coverage = sum(word is not None for word in aligned)/max(1,len(aligned))
    result = {'method': 'local-whisper-tiny.en', 'coverage': round(coverage,3), 'words': aligned}
    if coverage < .8:
        raise ValueError(f'Only {coverage:.0%} of narration aligned; use estimated timing.')
    args.result.write_text(json.dumps(result))
    print(f'Aligned {coverage:.0%} of spoken words locally.')

if __name__ == '__main__':
    main()
