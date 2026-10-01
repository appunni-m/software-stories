import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { readFile, stat } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { randomUUID } from 'node:crypto';
import { ROOT, CACHE, hash, settings, publicStory, saveJSON, loadJSON } from './util.mjs';
import { captureRepository, snapshotInfo, loadSnapshot, searchSources } from './repository.mjs';
import { generateStory } from './agent.mjs';
import { narrate, audioIsCurrent, cachedNarration } from './speech.mjs';
import { exportVideo } from './export.mjs';
import { selfStory, introductionOrigin } from './self-story.mjs';
import { isCurrentNarrative, narrativeVersion } from '../shared/narrative.mjs';
import { closeVoiceWorker } from './voice-worker.mjs';

const stories = new Map(), jobs = new Map(), activeJobs = new Map();
const snapshot = await captureRepository(ROOT);
const introduction = selfStory(snapshot);
const savedIntroduction = await loadJSON(resolve(CACHE, 'self-story.json'));
if (savedIntroduction?.snapshotId === snapshot.id && savedIntroduction.chapters.filter(chapter => !chapter.parentId).every(isCurrentNarrative)) Object.assign(introduction, savedIntroduction);
else if (savedIntroduction) for (const chapter of introduction.chapters) {
  const previous = savedIntroduction.chapters.find(item => item.narration === chapter.narration && audioIsCurrent(item.audio));
  if (previous) chapter.audio = previous.audio;
}
for (const chapter of introduction.chapters) if (!audioIsCurrent(chapter.audio)) chapter.audio = await cachedNarration(chapter);
stories.set(introduction.id, introduction);
await saveStory(introduction);
async function saveStory(story) {
  await saveJSON(resolve(CACHE, 'journeys', `${story.id}.json`), story);
  if (story.id === introduction.id) await saveJSON(resolve(CACHE, 'self-story.json'), story);
}
async function getStory(id) {
  if (!/^[a-f0-9-]{36}$/.test(id ?? '')) throw new Error('Invalid story ID.');
  const story = stories.get(id) ?? await loadJSON(resolve(CACHE, 'journeys', `${id}.json`));
  if (!story) throw new Error('Story not found.');
  if (story.origin === introductionOrigin && story.chapters.filter(chapter => !chapter.parentId).some(chapter => !isCurrentNarrative(chapter))) {
    const updated = selfStory(await getSnapshot(story.snapshotId)).chapters;
    story.chapters.filter(chapter => !chapter.parentId).forEach((chapter, i) => {
      const {id:unused, ...content} = updated[i];
      Object.assign(chapter, content);
    });
    await saveStory(story);
  }
  for (const chapter of story.chapters) if (!audioIsCurrent(chapter.audio)) chapter.audio = await cachedNarration(chapter);
  stories.set(id, story); return story;
}
async function getSnapshot(id) {
  if (!/^[a-f0-9]{24}$/.test(id ?? '')) throw new Error('Invalid source snapshot.');
  const snapshot = await loadSnapshot(id);
  if (!snapshot) throw new Error('Source snapshot not found.');
  return snapshot;
}
async function prepareRecap(story, chapter, backend, progress) {
  if (isCurrentNarrative(chapter)) return;
  const snapshot = await getSnapshot(story.snapshotId);
  progress(`Preparing the software story for ${chapter.title}`);
  const generated = await generateStory(snapshot, { backend, mode: 'follow-up', filePath: chapter.sourcePath ?? null, query: `Rewrite “${chapter.title}” as a software explanation. Explain the problem this code addresses, introduce its responsibilities and relevant code structure, explain a supported design consequence, and use a concrete example to make the behavior imaginable. Explain the software itself, not the video or its animations. Preserve the specific subject, correct unsupported statements, and use the saved sources.`, previous: { title: chapter.title, narration: chapter.narration, claims: chapter.claims } }, progress);
  Object.assign(chapter, generated.chapters[0], { audio: null });
  await saveStory(story);
}
function job(work, key) {
  if (key && activeJobs.has(key)) return { jobId: activeJobs.get(key) };
  if ([...jobs.values()].filter(job => ['queued', 'running'].includes(job.status)).length >= 5) throw new Error('The local queue is full; wait for the current work.');
  const record = { id: randomUUID(), status: 'queued', message: 'Starting', createdAt: new Date().toISOString(), result: null };
  jobs.set(record.id, record);
  if (key) activeJobs.set(key, record.id);
  Promise.resolve().then(async () => {
    record.status = 'running';
    try { record.result = await work(message => { record.message = message; }); record.status = 'done'; record.message = 'Ready'; }
    catch (error) { record.status = 'error'; record.message = error.message; }
    if (key) activeJobs.delete(key);
    await saveJSON(resolve(CACHE, 'jobs', `${record.id}.json`), record);
  });
  return { jobId: record.id };
}
const json = (response, data, code = 200) => { response.writeHead(code, { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store' }); response.end(JSON.stringify(data)); };
async function body(request) {
  if (!request.headers['content-type']?.startsWith('application/json')) throw new Error('Use a JSON request.');
  let value = '';
  for await (const chunk of request) { value += chunk; if (value.length > 20_000) throw new Error('Request is too large.'); }
  return JSON.parse(value);
}
async function sendFile(request, response, path, type, download = false) {
  const info = await stat(path), range = request.headers.range;
  let start = 0, end = info.size - 1, code = 200;
  if (range) {
    const match = /^bytes=(\d+)-(\d*)$/.exec(range);
    if (!match) { response.writeHead(416, { 'Content-Range': `bytes */${info.size}` }); return response.end(); }
    start = Number(match[1]); end = match[2] ? Math.min(Number(match[2]), end) : end;
    if (start > end || start >= info.size) { response.writeHead(416, { 'Content-Range': `bytes */${info.size}` }); return response.end(); }
    code = 206;
  }
  const headers = { 'Content-Type': type, 'Content-Length': end - start + 1, 'Accept-Ranges': 'bytes', 'Cache-Control': download ? 'no-cache' : 'private, max-age=31536000, immutable' };
  if (code === 206) headers['Content-Range'] = `bytes ${start}-${end}/${info.size}`;
  if (download) headers['Content-Disposition'] = 'attachment; filename="software-story.mp4"';
  response.writeHead(code, headers);
  if (request.method === 'HEAD') return response.end();
  createReadStream(path, { start, end }).on('error', () => response.destroy()).pipe(response);
}
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://localhost');
    if (!/^(127\.0\.0\.1|localhost)(:\d+)?$/.test(request.headers.host ?? '')) return json(response, { error: 'Use the local preview address.' }, 403);
    const path = url.pathname;
    if (request.headers.origin && new URL(request.headers.origin).host !== request.headers.host) return json(response, { error: 'Only the local application can make this request.' }, 403);
    if (request.method === 'GET' && path === '/api/bootstrap') return json(response, { story: publicStory(introduction), settings: { voice: settings.voice, agent: settings.agent, agentModel: settings.agentModel, agentReasoningEffort: settings.agentReasoningEffort, ttsBackend: settings.ttsEngine === 'custom-voice' ? 'cpu' : settings.ttsBackend, playbackVolume: settings.playbackVolume }, components: ['flow', 'sequence', 'branch', 'state', 'memory', 'diff', 'code'] });
    if (request.method === 'GET' && path.startsWith('/api/jobs/')) {
      const record = jobs.get(path.split('/').pop());
      return json(response, record ?? { status: 'error', message: 'Job not found after a server restart.' });
    }
    if (request.method === 'GET' && path.startsWith('/api/stories/')) return json(response, publicStory(await getStory(path.split('/').pop())));
    if (request.method === 'GET' && path === '/api/files') {
      const snapshot = await getSnapshot(url.searchParams.get('snapshot'));
      return json(response, snapshot.files.map(({ path, kind, text }) => ({ path, kind, lines: text.split('\n').length })));
    }
    if (request.method === 'GET' && path === '/api/source') {
      const snapshot = await getSnapshot(url.searchParams.get('snapshot'));
      const file = snapshot.files.find(file => file.path === url.searchParams.get('path'));
      if (!file) return json(response, { error: 'Source not found in this snapshot.' }, 404);
      return json(response, { ...file, snapshotId: snapshot.id });
    }
    if (request.method === 'POST') {
      const data = await body(request);
      if (path === '/api/repositories') {
        if (typeof data.path !== 'string' || !data.path.trim()) throw new Error('Enter a local repository path.');
        return json(response, snapshotInfo(await captureRepository(data.path, { baseRef: data.baseRef })));
      }
      if (path === '/api/search') return json(response, searchSources(await getSnapshot(data.snapshotId), String(data.query ?? '').slice(0, 1000), 12));
      if (path === '/api/recap') {
        const story = await getStory(data.storyId), chapter = story.chapters.find(item => item.id === data.chapterId);
        if (!chapter) throw new Error('Chapter not found.');
        if (isCurrentNarrative(chapter)) return json(response, { result: { story: publicStory(story) } });
        return json(response, job(async progress => {
          await prepareRecap(story, chapter, data.backend ?? settings.agent, progress);
          return { story: publicStory(story) };
        }, `recap:${story.id}:${chapter.id}:${data.backend ?? settings.agent}`));
      }
      if (path === '/api/explain') {
        if (typeof data.query !== 'string' || !data.query.trim() || data.query.length > 2000) throw new Error('Enter a question of 1–2000 characters.');
        const snapshot = await getSnapshot(data.snapshotId);
        if (data.filePath !== undefined && (typeof data.filePath !== 'string' || !snapshot.files.some(file => file.path === data.filePath))) throw new Error('Choose a file from this source snapshot.');
        const previousStory = data.storyId ? await getStory(data.storyId) : null;
        if (previousStory && previousStory.snapshotId !== snapshot.id) throw new Error('A follow-up must use the same source snapshot.');
        const previousChapter = previousStory?.chapters.find(chapter => chapter.id === data.chapterId);
        if (data.chapterId && !previousChapter) throw new Error('Chapter not found.');
        const focusNode = data.nodeId ? previousChapter?.scene.nodes.find(node => node.id === data.nodeId) : null;
        if (data.nodeId && !focusNode) throw new Error('Choose a responsibility from this explanation.');
        const mode = data.mode ?? 'follow-up';
        if (!['architecture', 'feature', 'pr', 'follow-up'].includes(mode)) throw new Error('Unknown story mode.');
        if (mode === 'pr' && !snapshot.baseCommit) throw new Error('Capture this repository with a Git base reference first.');
        const requestKey = hash({ plannerVersion: narrativeVersion, model: settings.agentModel, reasoningEffort: settings.agentReasoningEffort, snapshot: snapshot.id, query: data.query, backend: data.backend ?? settings.agent, mode, parentId: previousChapter?.id ?? null, filePath: data.filePath ?? null, nodeId: data.nodeId ?? null });
        const existingChapter = previousStory?.chapters.find(chapter => chapter.requestKey === requestKey && isCurrentNarrative(chapter));
        if (existingChapter) return json(response, { result: { story: publicStory(previousStory), chapterId: existingChapter.id, reused: true } });
        return json(response, job(async progress => {
          const generated = await generateStory(snapshot, { backend: data.backend ?? settings.agent, mode: data.mode ?? 'follow-up', query: data.query, filePath: data.filePath ?? null, focusNode, previous: previousChapter ? { title: previousChapter.title, narration: previousChapter.narration, claims: previousChapter.claims } : null }, progress);
          const story = previousStory ?? { id: randomUUID(), snapshotId: snapshot.id, repository: snapshotInfo(snapshot), title: generated.title, chapters: [], createdAt: new Date().toISOString(), mode: data.mode ?? 'architecture', origin: `${data.backend ?? settings.agent} explanation` };
          const chapters = generated.chapters.map(chapter => ({ ...chapter, id: randomUUID(), parentId: previousChapter?.id ?? null, sourcePath: data.filePath ?? null, originLabel: String(data.originLabel ?? '').slice(0, 150), requestKey, audio: null }));
          story.chapters.push(...chapters);
          stories.set(story.id, story); await saveStory(story);
          return { story: publicStory(story), chapterId: chapters[0].id };
        }, `explain:${previousStory?.id ?? 'new'}:${requestKey}`));
      }
      if (path === '/api/narrate') {
        const story = await getStory(data.storyId);
        const chapters = data.all ? story.chapters : story.chapters.filter(chapter => chapter.id === data.chapterId);
        if (!chapters.length) throw new Error('Chapter not found.');
        if (chapters.every(chapter => isCurrentNarrative(chapter) && audioIsCurrent(chapter.audio))) return json(response, { result: { story: publicStory(story) } });
        return json(response, job(async progress => {
          for (const chapter of chapters) {
            await prepareRecap(story, chapter, data.backend ?? settings.agent, progress);
            chapter.audio = await narrate(chapter, progress); await saveStory(story);
          }
          return { story: publicStory(story) };
        }, `narrate:${story.id}:${chapters.map(chapter => chapter.id).join(',')}`));
      }
      if (path === '/api/export') {
        const original = await getStory(data.storyId);
        return json(response, job(async progress => {
          for (const chapter of original.chapters) await prepareRecap(original, chapter, data.backend ?? settings.agent, progress);
          const story = structuredClone(original);
          const video = await exportVideo(story, progress);
          for (const chapter of story.chapters) { const target = original.chapters.find(item => item.id === chapter.id); if (target?.narration === chapter.narration) target.audio = chapter.audio; }
          await saveStory(original);
          return { url: video.url, story: publicStory(original) };
        }));
      }
      return json(response, { error: 'Unknown action.' }, 404);
    }
    if (['GET', 'HEAD'].includes(request.method) && /^\/media\/[a-f0-9]{24}\.wav$/.test(path)) return await sendFile(request, response, resolve(CACHE, 'audio', path.split('/').pop()), 'audio/wav');
    if (['GET', 'HEAD'].includes(request.method) && /^\/exports\/[a-f0-9-]{36}\.mp4$/.test(path)) return await sendFile(request, response, resolve(CACHE, 'video', path.split('/').pop()), 'video/mp4', true);
    const staticPath = path === '/' ? '/index.html' : path;
    let file;
    if (staticPath.startsWith('/shared/')) file = resolve(ROOT, 'src', '.' + staticPath);
    else file = resolve(ROOT, 'public', '.' + staticPath);
    const prefix = staticPath.startsWith('/shared/') ? resolve(ROOT, 'src/shared') : resolve(ROOT, 'public');
    if (!file.startsWith(prefix + '/')) return json(response, { error: 'File not found.' }, 404);
    const types = { '.html': 'text/html; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.svg': 'image/svg+xml' };
    const content = await readFile(file);
    response.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream', 'Cache-Control': 'no-cache', 'X-Content-Type-Options': 'nosniff' }); response.end(content);
  } catch (error) { json(response, { error: error.code === 'ENOENT' ? 'File not found.' : error.message }, error.code === 'ENOENT' ? 404 : 400); }
});
const port = Number(process.env.PORT ?? 4317);
server.listen(port, '127.0.0.1', () => console.log(`Software Stories preview: http://127.0.0.1:${port}\nStory: ${introduction.id}\nSnapshot: ${snapshot.id}`));
for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => { closeVoiceWorker(); server.close(() => process.exit(0)); setTimeout(() => process.exit(0), 4000).unref(); });
