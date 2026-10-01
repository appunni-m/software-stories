#!/usr/bin/env python3
"""Optional offline intelligibility check; no audio is sent to a service."""
import argparse
import json
from math import gcd
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('files',nargs='+',type=Path)
    args=parser.parse_args()
    import soundfile as sf
    from scipy.signal import resample_poly
    from faster_whisper import WhisperModel
    root=Path(__file__).resolve().parents[1]
    model=WhisperModel('tiny.en',device='cpu',compute_type='int8',cpu_threads=4,download_root=str(root/'.cache/voice/asr'),local_files_only=True)
    for file in args.files:
        audio,rate=sf.read(file,dtype='float32')
        if audio.ndim==2: audio=audio.mean(axis=1)
        common=gcd(rate,16000)
        audio=resample_poly(audio,16000//common,rate//common)
        segments,_=model.transcribe(audio,language='en',beam_size=5,condition_on_previous_text=False)
        print(json.dumps({'file':str(file),'transcript':' '.join(segment.text for segment in segments)}),flush=True)

if __name__=='__main__':main()
