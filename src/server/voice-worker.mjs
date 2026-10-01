import { spawn } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { mkdir } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import { resolve } from 'node:path';
import { randomUUID } from 'node:crypto';
import { ROOT, settings } from './util.mjs';

// One resident process per server. Model weights survive requests; the C++
// worker resets the sampler, context, and waveform before each utterance.
export class ResidentWorker {
  constructor(command, args, { env = process.env, onLog = () => {}, readyTimeout = 90_000, requestTimeout = 600_000 } = {}) {
    this.requests = new Map(); this.alive = true; this.log = ''; this.requestTimeout = requestTimeout;
    this.child = spawn(command, args, { env, stdio: ['pipe', 'pipe', 'pipe'] });
    this.ready = new Promise((accept, reject) => { this.acceptReady = accept; this.rejectReady = reject; });
    this.ready.catch(() => {});
    this.timer = setTimeout(() => this.stop(new Error('The local voice worker did not become ready.')), readyTimeout);
    this.child.stderr.on('data', chunk => { this.log = (this.log + chunk).slice(-500_000); onLog(chunk); });
    this.child.stdin.on('error', error => this.stop(error));
    this.child.on('error', error => this.stop(error));
    this.child.on('exit', code => this.stop(new Error(`The local voice worker stopped (${code}). ${this.log.slice(-600)}`)));
    createInterface({ input: this.child.stdout }).on('line', line => {
      let message; try { message = JSON.parse(line); } catch { return; }
      if (message.type === 'ready') { clearTimeout(this.timer); this.info = message; this.acceptReady(message); return; }
      const pending = this.requests.get(message.id);
      if (!pending) return;
      this.requests.delete(message.id); clearTimeout(pending.timer);
      if (message.type === 'error') pending.reject(new Error(message.error));
      else pending.accept(message);
    });
  }
  async request(data) {
    await this.ready;
    if (!this.alive) throw new Error('The local voice worker is unavailable.');
    const id = randomUUID();
    return new Promise((accept, reject) => {
      const timer = setTimeout(() => this.stop(new Error('Local speech generation timed out.')), this.requestTimeout);
      this.requests.set(id, { accept, reject, timer });
      this.child.stdin.write(JSON.stringify({ ...data, id }) + '\n');
    });
  }
  stop(error = new Error('The voice worker was closed.')) {
    if (!this.alive) return;
    this.alive = false; clearTimeout(this.timer); this.rejectReady(error);
    for (const pending of this.requests.values()) { clearTimeout(pending.timer); pending.reject(error); }
    this.requests.clear(); this.child.kill('SIGTERM');
    const force = setTimeout(() => { if (this.child.exitCode === null) this.child.kill('SIGKILL'); }, 3000); force.unref();
  }
}

let resident;
export function closeVoiceWorker() { resident?.stop(); }
export async function generateWarmVoice(model, reference, text, output, progress) {
  if (!resident?.alive) {
    const runtime = resolve(ROOT, '.cache/voice/llama/b11146/llama-b11146');
    await mkdir(resolve(ROOT, '.cache/voice'), { recursive: true });
    const logPath = resolve(ROOT, '.cache/voice/voice-worker.log');
    const log = createWriteStream(logPath);
    const gpu = settings.ttsBackend === 'vulkan';
    progress(`Loading Serena once on ${gpu ? 'your GPU' : 'CPU'}; keeping her ready for subsequent clips`);
    resident = new ResidentWorker(resolve(ROOT, '.cache/voice/voice-worker'), [
      '-m', resolve(model.directory, 'Qwen3-TTS-12Hz-1.7B-Base-Q8_0.gguf'),
      '--mmproj', resolve(model.directory, 'mmproj-Qwen3-TTS-12Hz-1.7B-Base-Q8_0.gguf'),
      '--device', gpu ? 'Vulkan0' : 'none', '-ngl', gpu ? '99' : '0',
      '--tts-lang', 'en', '--tts-speaker-file', reference, '-n', '1024', '-t', '4', '-c', '2048',
      '--seed', '42', '--temp', '0.9', '--top-k', '50', '--top-p', '1', '--min-p', '0', '--repeat-penalty', '1', '--offline', '--verbosity', '4',
      ...(!gpu ? ['--no-mmproj-offload'] : [])
    ], { env: { ...process.env, LD_LIBRARY_PATH: runtime, SOFTWARE_STORIES_BACKEND_DIR: runtime, VK_ICD_FILENAMES: '/usr/share/vulkan/icd.d/radeon_icd.json', XDG_CACHE_HOME: resolve(ROOT, '.cache') }, onLog: chunk => log.write(chunk) });
    resident.child.on('close', () => log.end());
    resident.logPath = logPath;
  }
  const worker = resident;
  await worker.ready;
  const gpuVerified = /offloaded 29\/29 layers to GPU/.test(worker.log);
  if (settings.ttsBackend === 'vulkan' && !gpuVerified) { worker.stop(); throw new Error('The voice worker did not verify GPU layer offloading.'); }
  progress('Serena is ready; generating this clip without reloading the model');
  const result = await worker.request({ text, output });
  return { ...result, gpu_layer_offload_verified: gpuVerified, log_path: worker.logPath };
}
