import { spawn } from 'node:child_process';
import { mkdir, rename } from 'node:fs/promises';
import { resolve } from 'node:path';
import { once } from 'node:events';
import sharp from 'sharp';
import { CACHE, run, settings } from './util.mjs';
import { renderScene } from '../shared/render.mjs';
import { narrate, audioIsCurrent } from './speech.mjs';

export async function exportVideo(story, progress = () => {}) {
  const directory = resolve(CACHE, 'video', story.id);
  await mkdir(directory, { recursive: true });
  const { width, height, fps } = settings.video;
  const paths = [];
  for (const [index, chapter] of story.chapters.entries()) {
    if (!audioIsCurrent(chapter.audio)) chapter.audio = await narrate(chapter, progress);
    const path = resolve(directory, `${index}.mp4`);
    progress(`Rendering chapter ${index + 1} of ${story.chapters.length}`);
    const encoder = spawn('ffmpeg', ['-y', '-hide_banner', '-loglevel', 'error', '-f', 'image2pipe', '-framerate', String(fps), '-i', 'pipe:0', '-i', chapter.audio.path, '-c:v', 'libx264', '-preset', 'ultrafast', '-crf', '22', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '192k', '-ar', '48000', '-ac', '2', '-shortest', '-threads', '2', path], { stdio: ['pipe', 'ignore', 'pipe'] });
    let stderr = '';
    encoder.stderr.on('data', chunk => { stderr += chunk.toString(); });
    // Handle pipe failures even while a frame is being rasterized.
    encoder.stdin.on('error', () => {});
    const completion = new Promise((accept, reject) => { encoder.on('error', reject); encoder.on('close', code => code === 0 ? accept() : reject(new Error(`Video encoder failed: ${stderr}`))); });
    completion.catch(() => {});
    try {
      const frames = Math.ceil(chapter.audio.duration * fps);
      for (let frame = 0; frame < frames; frame++) {
        if (encoder.exitCode !== null || encoder.stdin.destroyed) throw new Error(`Video encoder stopped: ${stderr}`);
        const image = await sharp(Buffer.from(renderScene(chapter, frame / frames))).resize(width, height).png().toBuffer();
        if (!encoder.stdin.write(image)) await once(encoder.stdin, 'drain');
        if (frame % (fps * 3) === 0) progress(`Rendering chapter ${index + 1}: ${Math.round(frame / frames * 100)}%`);
      }
      encoder.stdin.end();
      await completion;
    } catch (error) { encoder.kill('SIGTERM'); throw error; }
    paths.push(path);
  }
  const concatFile = resolve(directory, 'chapters.txt');
  const { writeFile } = await import('node:fs/promises');
  await writeFile(concatFile, paths.map(path => `file '${path.replaceAll("'", "'\\''")}'`).join('\n'));
  const temporary = resolve(CACHE, 'video', `${story.id}.partial.mp4`);
  await run('ffmpeg', ['-y', '-hide_banner', '-loglevel', 'error', '-f', 'concat', '-safe', '0', '-i', concatFile, '-c', 'copy', '-movflags', '+faststart', temporary]);
  const output = resolve(CACHE, 'video', `${story.id}.mp4`);
  await rename(temporary, output);
  return { path: output, url: `/exports/${story.id}.mp4` };
}
