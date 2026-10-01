import { resolve } from 'node:path';
import { readFile, mkdir, writeFile, open, stat, unlink } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { CACHE, ROOT, hash, run, settings, loadJSON, saveJSON } from './util.mjs';
import { masterNarration } from './audio.mjs';
import { generateWarmVoice } from './voice-worker.mjs';

const referenceIdentity = settings.ttsEngine === 'custom-voice' ? null : await readFile(resolve(ROOT, settings.reference)).then(hash).catch(() => null);
export const speechProfile = hash({ version: 3, engine: settings.ttsEngine, backend: settings.ttsBackend, voice: settings.voice, referenceIdentity, instructions: settings.voiceInstructions, rate: settings.speechRate, wordsPerMinute: settings.speechWpm, loudness: settings.speechLoudness });
export const audioIsCurrent = audio => audio?.profile === speechProfile;

let queue = Promise.resolve();
async function lockedGeneration(metadataPath, progress, work) {
  await mkdir(resolve(CACHE, 'audio'), { recursive: true });
  const lockPath = metadataPath + '.lock', token = randomUUID();
  let handle, announced = false;
  while (!handle) {
    const ready = await loadJSON(metadataPath);
    if (audioIsCurrent(ready)) return ready;
    try { handle = await open(lockPath, 'wx'); await handle.writeFile(token); }
    catch (error) {
      if (error.code !== 'EEXIST') throw error;
      const info = await stat(lockPath).catch(() => null);
      // A heartbeat, rather than a PID, works across local process namespaces.
      if (info && Date.now() - info.mtimeMs > 90_000) { await unlink(lockPath).catch(() => {}); continue; }
      if (!announced) { progress('This narration is already being prepared; sharing that work'); announced = true; }
      await new Promise(resolve => setTimeout(resolve, 1500));
    }
  }
  const heartbeat = setInterval(() => { const now = new Date(); handle.utimes(now, now).catch(() => {}); }, 15_000);
  try { return await work(); }
  finally {
    clearInterval(heartbeat); await handle.close();
    if (await readFile(lockPath, 'utf8').catch(() => null) === token) await unlink(lockPath).catch(() => {});
  }
}
export async function cachedNarration(chapter) {
  const pins = await loadJSON(resolve(ROOT, '.cache/voice/model-revisions.json'));
  const preset = settings.ttsEngine === 'custom-voice';
  const model = preset ? { revision: pins?.['Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice'] } : await loadJSON(resolve(ROOT, '.cache/voice/gpu-model.json'));
  if (!model) return null;
  const id = hash({ profile: speechProfile, reference: preset ? 'Serena named speaker' : referenceIdentity, revision: model.revision, narration: chapter.narration, frames: preset ? 1600 : 1024 }).slice(0, 24);
  const cached = await loadJSON(resolve(CACHE, 'audio', `${id}.json`));
  return audioIsCurrent(cached) ? cached : null;
}
export async function narrate(chapter, progress = () => {}) {
  const preset = settings.ttsEngine === 'custom-voice';
  const pins = await loadJSON(resolve(ROOT, '.cache/voice/model-revisions.json'));
  const model = preset ? { model: 'Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice', revision: pins?.['Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice'] } : await loadJSON(resolve(ROOT, '.cache/voice/gpu-model.json'));
  if (!model) throw new Error('Qwen3-TTS 1.7B is not installed. Complete the local voice setup first.');
  const reference = resolve(ROOT, settings.reference);
  const id = hash({ profile: speechProfile, reference: preset ? 'Serena named speaker' : hash(await readFile(reference)), revision: model.revision, narration: chapter.narration, frames: preset ? 1600 : 1024 }).slice(0, 24);
  const output = resolve(CACHE, 'audio');
  const metadataPath = resolve(output, `${id}.json`);
  const existing = await loadJSON(metadataPath);
  if (existing) { progress('Reusing Serena narration'); return existing; }
  const work = async () => {
    const cached = await loadJSON(metadataPath);
    if (cached) return cached;
    await mkdir(resolve(output, 'raw'), { recursive: true });
    const textFile = resolve(output, `${id}.txt`), resultFile = resolve(output, `${id}.result.json`);
    await writeFile(textFile, chapter.narration);
    progress(preset ? 'Preparing the fixed Serena voice · local CPU' : `Serena is narrating on ${settings.ttsBackend === 'vulkan' ? 'your GPU' : 'CPU'}`);
    let result;
    if (preset) {
      await run(resolve(ROOT, '.venv-voice/bin/python'), [resolve(ROOT, 'scripts/narrate_serena.py'), '--text-file', textFile, '--output', resolve(output, 'raw', `${id}.wav`), '--result', resultFile, '--instructions', settings.voiceInstructions], { timeout: 1_200_000 });
      result = await loadJSON(resultFile);
    } else {
      result = await generateWarmVoice(model, reference, chapter.narration, resolve(output, 'raw', `${id}.wav`), progress);
      await saveJSON(resultFile, result);
    }
    if (!result?.audio_path || (!preset && settings.ttsBackend === 'vulkan' && !result.gpu_layer_offload_verified)) throw new Error('Narration did not produce a verified audio result.');
    const path = resolve(output, `${id}.wav`);
    progress('Keeping the chosen pace and a gentle, consistent volume');
    const targetDuration = chapter.narration.split(/\s+/).length * 60 / (settings.speechWpm ?? 120);
    const rate = preset ? Math.max(.85, Math.min(1.15, result.duration_seconds / targetDuration)) : settings.speechRate;
    const mastering = await masterNarration(result.audio_path, path, { rate, loudness: settings.speechLoudness });
    const audio = { id, path, profile: speechProfile, sha256: hash(await readFile(path)), duration: mastering.duration, generationSeconds: result.total_seconds, workerPid: result.pid, modelLoads: result.modelLoads, voice: settings.voice, model: model.model, backend: preset ? 'cpu' : settings.ttsBackend, gpuVerified: result.gpu_layer_offload_verified, mastering };
    try {
      progress('Matching visual cues to the spoken narration');
      const alignmentPath = resolve(output, `${id}.alignment.json`);
      await run(resolve(ROOT, '.venv-voice/bin/python'), [resolve(ROOT, 'scripts/align_narration.py'), '--text-file', textFile, '--audio', path, '--result', alignmentPath], { timeout: 120_000 });
      audio.alignment = await loadJSON(alignmentPath);
    } catch { audio.timingNote = 'Spoken alignment unavailable; visual timing is estimated.'; }
    await saveJSON(metadataPath, audio);
    return audio;
  };
  const pending = queue.then(() => lockedGeneration(metadataPath, progress, work));
  queue = pending.catch(() => {});
  return pending;
}
