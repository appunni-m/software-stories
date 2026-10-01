import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, rename } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const CACHE = resolve(ROOT, '.cache/stories');
export const hash = value => createHash('sha256').update(typeof value === 'string' || Buffer.isBuffer(value) ? value : JSON.stringify(value)).digest('hex');
export const settings = JSON.parse(await readFile(resolve(ROOT, 'config/settings.json'), 'utf8'));
export async function saveJSON(path, data) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${process.pid}.${Math.random().toString(16).slice(2)}.tmp`;
  await writeFile(temporary, JSON.stringify(data, null, 2));
  await rename(temporary, path);
}
export async function loadJSON(path) {
  try { return JSON.parse(await readFile(path, 'utf8')); }
  catch (error) { if (error.code === 'ENOENT') return null; throw error; }
}
export function run(command, args = [], { cwd = ROOT, input, timeout = 600_000, env = process.env, onOutput } = {}) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, { cwd, env, stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', finished = false, timedOut = false;
    const timer = setTimeout(() => { timedOut = true; child.kill('SIGTERM'); setTimeout(() => { if (!finished) child.kill('SIGKILL'); }, 3000).unref(); }, timeout);
    child.stdout.on('data', chunk => { stdout = (stdout + chunk).slice(-4_000_000); onOutput?.(chunk.toString()); });
    child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-20_000); });
    child.on('error', error => { finished = true; clearTimeout(timer); reject(error); });
    child.on('close', code => {
      finished = true; clearTimeout(timer);
      if (timedOut) reject(new Error(`${command} did not finish within ${Math.round(timeout / 1000)} seconds. Retry or select the other CLI backend.`));
      else if (code === 0) accept({ stdout, stderr });
      else reject(new Error(`${command} exited with ${code ?? 'a timeout'}: ${stderr.slice(-2000) || stdout.slice(-2000)}`));
    });
    child.stdin.on('error', () => {});
    child.stdin.end(input);
  });
}
export const publicChapter = chapter => ({ ...chapter, audio: chapter.audio ? { ...chapter.audio, path: undefined, url: `/media/${chapter.audio.id}.wav` } : null });
export const publicStory = story => ({ ...story, chapters: story.chapters.map(publicChapter) });
