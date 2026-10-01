import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { run } from '../src/server/util.mjs';
import { masterNarration } from '../src/server/audio.mjs';
import { audioIsCurrent, speechProfile } from '../src/server/speech.mjs';

test('narration mastering slows audio, keeps lossless stereo, and limits loudness', async()=>{
 const directory=await mkdtemp(join(tmpdir(),'story-audio-'));
 try {
  const input=join(directory,'source.wav'),output=join(directory,'master.wav');
  await run('ffmpeg',['-y','-v','error','-f','lavfi','-i','sine=frequency=440:duration=2:sample_rate=24000','-af','volume=4',input]);
  const result=await masterNarration(input,output);
  assert.ok(result.duration>2.15 && result.duration<2.3);
  const probe=JSON.parse((await run('ffprobe',['-v','error','-show_streams','-of','json',output])).stdout).streams[0];
  assert.equal(probe.codec_name,'pcm_s24le'); assert.equal(probe.channels,2); assert.equal(probe.sample_rate,'48000');
  const measurements=await run('ffmpeg',['-hide_banner','-i',output,'-af','loudnorm=I=-25:TP=-8:LRA=11:print_format=json','-f','null','-']);
  const stats=JSON.parse(measurements.stderr.match(/\{[^{}]*"input_i"[^{}]*\}/)[0]);
  assert.ok(Math.abs(Number(stats.input_i)+25)<0.6); assert.ok(Number(stats.input_tp)<=-8);
 } finally {await rm(directory,{recursive:true,force:true});}
});
test('a changed voice profile invalidates old narration',()=>{
 assert.equal(audioIsCurrent({voice:'serena'}),false);
 assert.equal(audioIsCurrent({profile:'old-reference'}),false);
 assert.equal(audioIsCurrent({profile:speechProfile}),true);
});
