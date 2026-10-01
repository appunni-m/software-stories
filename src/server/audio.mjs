import { run } from './util.mjs';

export async function masterNarration(input, output, { rate = 0.9, loudness = -25 } = {}) {
  if (!Number.isFinite(rate) || rate < 0.7 || rate > 1.2 || !Number.isFinite(loudness) || loudness < -32 || loudness > -18) throw new Error('Invalid narration mastering settings.');
  // Preserve pitch while slowing the voice. Measure after stretching so both
  // passes use the same signal; no compression, reverb, or stereo widening.
  const pace = `atempo=${rate}`;
  const measure = await run('ffmpeg', ['-hide_banner', '-nostdin', '-i', input, '-af', `${pace},loudnorm=I=${loudness}:TP=-8:LRA=11:print_format=json`, '-f', 'null', '-']);
  const measurements = JSON.parse(measure.stderr.match(/\{[^{}]*"input_i"[^{}]*\}/)?.[0] ?? 'null');
  if (!measurements || !Number.isFinite(Number(measurements.input_i))) throw new Error('Cannot master a silent narration.');
  const filter = `${pace},loudnorm=I=${loudness}:TP=-8:LRA=11:measured_I=${measurements.input_i}:measured_TP=${measurements.input_tp}:measured_LRA=${measurements.input_lra}:measured_thresh=${measurements.input_thresh}:offset=${measurements.target_offset}:linear=true:print_format=json`;
  await run('ffmpeg', ['-y', '-hide_banner', '-nostdin', '-i', input, '-af', filter, '-ar', '48000', '-ac', '2', '-c:a', 'pcm_s24le', output]);
  const probe = await run('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'json', output]);
  return { duration: Number(JSON.parse(probe.stdout).format.duration), rate, targetLufs: loudness, peakCeilingDb: -8, sampleRate: 48000, channels: 2, codec: 'pcm_s24le' };
}
