import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { describe, it } from 'node:test';
import {
  buildOfflineExportPreview,
  buildOfflineGuideHtml,
  buildOfflineNarrationCoverage,
  isOfflineExportableRevision,
  isSafeOfflineSourcePath,
  OFFLINE_PACKAGE_SCHEMA_VERSION,
} from '../src/lib/offlineExport.ts';

const sourceAnchor = (anchorId = 'anchor:entry', path = 'src/main.ts', excerpt = 'export function main() { return run(); }') => ({
  anchorId,
  path,
  symbolId: 'symbol:main',
  snapshotId: 'snapshot:fixture',
  sourceSha256: 'a'.repeat(64),
  startByte: 0,
  endByteInclusive: excerpt.length - 1,
  startLine: 1,
  endLine: 2,
  startColumn: 1,
  endColumn: excerpt.length,
  excerpt,
});

const typedVisual = () => ({
  schemaVersion: 2,
  kind: 'dataFlow',
  title: 'Request path',
  layoutHint: { direction: 'leftToRight' },
  nodes: [
    { nodeId: 'node:entry', label: 'handleRequest', detail: 'HTTP entry point', hotspotId: 'hotspot:code', evidenceAnchorIds: ['anchor:entry'] },
    { nodeId: 'node:worker', label: 'runWorker', detail: null, hotspotId: null, evidenceAnchorIds: ['anchor:entry'] },
  ],
  edges: [{ edgeId: 'edge:call', fromNodeId: 'node:entry', toNodeId: 'node:worker', label: 'calls', evidenceAnchorIds: ['anchor:entry'] }],
});

const section = ({ visual = typedVisual(), anchors = [sourceAnchor()], sectionId = 'section:request', order = 0 } = {}) => ({
  sectionId,
  sectionRevisionId: `section-revision:${sectionId}`,
  order,
  title: 'Handle a request',
  prose: 'The handler delegates to a worker.',
  spokenText: 'The handler delegates to a worker.',
  claims: [{
    claimId: `claim:${sectionId}`,
    category: { kind: 'codeBehavior' },
    prose: 'The handler calls the worker.',
    anchors,
    dependencies: [],
  }],
  dependencies: [],
  visual,
  hotspots: [{
    hotspotId: 'hotspot:code',
    label: 'Open implementation',
    destination: { kind: 'codeLens', anchorId: 'anchor:entry' },
  }],
});

const chapter = ({ revisionId = 'revision:one', lifecycle = 'accepted', target = { kind: 'overview' }, sections = [section()], title = 'Request flow' } = {}) => ({
  schemaVersion: 5,
  chapterId: `chapter:${revisionId}`,
  revisionId,
  revisionNumber: 3,
  projectId: 'project:local',
  snapshotId: 'snapshot:fixture',
  title,
  lifecycle,
  validationStatus: 'valid',
  createdAtUnixMs: 1,
  target: { targetId: 'target:one', scope: target },
  sections,
});

const narrationWav = () => {
  const sampleRate = 24_000;
  const sampleCount = sampleRate;
  const dataBytes = sampleCount * 2;
  const wav = Buffer.alloc(44 + dataBytes);
  wav.write('RIFF', 0);
  wav.writeUInt32LE(wav.length - 8, 4);
  wav.write('WAVE', 8);
  wav.write('fmt ', 12);
  wav.writeUInt32LE(16, 16);
  wav.writeUInt16LE(1, 20);
  wav.writeUInt16LE(1, 22);
  wav.writeUInt32LE(24_000, 24);
  wav.writeUInt32LE(48_000, 28);
  wav.writeUInt16LE(2, 32);
  wav.writeUInt16LE(16, 34);
  wav.write('data', 36);
  wav.writeUInt32LE(dataBytes, 40);
  for (let sample = 0; sample < sampleCount; sample += 1) {
    const phase = (2 * Math.PI * 440 * sample) / sampleRate;
    wav.writeInt16LE(Math.round(Math.sin(phase) * 4_000), 44 + sample * 2);
  }
  return wav;
};

it('exports schema-5 authored speech without a visual, with its exact transcript and cached audio', async () => {
  const current = chapter({ sections: [section({ visual: null })] });
  current.schemaVersion = 5;
  current.sections[0].spokenText = 'A request reaches the handler. The worker takes over the next part of the work, and control returns to the caller.';
  const original = structuredClone(current);
  const exactText = current.sections[0].spokenText;
  const audio = { chapterRevisionId: current.revisionId, sectionId: current.sections[0].sectionId,
    exactText, audio: { fingerprint: 'b'.repeat(64), mimeType: 'audio/wav', audioBase64: narrationWav().toString('base64'), cacheHit: true } };
  const preview = buildOfflineExportPreview([current], [current.revisionId], true, [audio]);
  assert.equal(preview.chapters[0].sections[0].spokenText, exactText);
  assert.equal(preview.chapters[0].sections[0].narrationTranscript, exactText);
  const result = await buildOfflineGuideHtml([current], { selectedRevisionIds: [current.revisionId],
    includeExcerpts: true, includeNarrationAudio: true, narrationAudio: [audio], exportedAtUnixMs: 10 });
  assert.ok(result.html.includes(exactText));
  assert.ok(result.html.includes('The handler calls the worker.'));
  assert.equal(result.manifest.assets.filter(a => a.kind === 'typedVisual').length, 0);
  assert.equal(result.manifest.assets.filter(a => a.kind === 'narrationAudio').length, 1);
  const transcript = result.manifest.assets.find(a => a.kind === 'narrationTranscript');
  assert.equal(transcript.sha256, createHash('sha256').update(exactText).digest('hex'));
  assert.equal(transcript.byteLength, Buffer.byteLength(exactText));
  assert.deepEqual(current, original);
});

const sceneChapter = () => {
  const first = section();
  first.claims = [
    { ...first.claims[0], claimId: 'claim:input', prose: 'main receives input.' },
    { ...first.claims[0], claimId: 'claim:return', prose: 'run returns the result.', anchors: [sourceAnchor('anchor:return')] },
  ];
  first.visual.nodes[1].evidenceAnchorIds = ['anchor:return'];
  first.prose = first.spokenText = first.claims.map(claim => claim.prose).join(' ');
  first.visual.presentation = { schemaVersion: 3, scenes: first.claims.map((_claim, claimIndex) => ({
    claimIndex, title: claimIndex === 0 ? 'Incoming call' : 'Returned result', narration: first.claims[claimIndex].prose, mode: 'composition', elements: [
      { kind: 'row', parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
      { kind: 'code', parentIndex: 0, nodeIndex: claimIndex, tone: 'accent', value: 'run()', valueOrigin: 'source' },
    ],
  })) };
  return chapter({ sections: [first] });
};

const sceneTrack = (sceneId, exactText = `Reviewed ${sceneId}.`) => ({
  chapterRevisionId: 'revision:one', sectionId: 'section:request', sceneId, exactText,
  audio: { fingerprint: 'b'.repeat(64), mimeType: 'audio/wav', audioBase64: narrationWav().toString('base64'), cacheHit: true },
});

describe('offline chapter export', () => {
  it('exports current authored scenes with source links and matching cached tracks', async () => {
    const accepted = sceneChapter();
    const before = structuredClone(accepted);
    const result = await buildOfflineGuideHtml([accepted], {
      selectedRevisionIds: [accepted.revisionId], includeExcerpts: true,
      includeNarrationAudio: true, narrationAudio: [sceneTrack('claim:input'), sceneTrack('claim:return')],
    });
    assert.equal((result.html.match(/class="offline-story-scene"/g) ?? []).length, 2);
    assert.equal((result.html.match(/<audio controls/g) ?? []).length, 2);
    assert.match(result.html, /href="#evidence-1-1"/);
    assert.match(result.html, /href="#section-1-1-scene-2"/);
    assert.deepEqual(accepted, before);
  });
  it('includes only selected accepted revisions and records revision and snapshot identities', async () => {
    const accepted = chapter();
    const draft = chapter({ revisionId: 'revision:draft', lifecycle: 'draft' });
    const rejected = chapter({ revisionId: 'revision:rejected', lifecycle: 'rejected' });
    const result = await buildOfflineGuideHtml([accepted, draft, rejected], {
      selectedRevisionIds: ['revision:one'],
      includeExcerpts: true,
      exportedAtUnixMs: 10,
    });

    assert.deepEqual(result.manifest.chapters, [{
      revisionId: 'revision:one', revisionNumber: 3, snapshotId: 'snapshot:fixture', title: 'Request flow',
    }]);
    assert.equal(result.manifest.schemaVersion, OFFLINE_PACKAGE_SCHEMA_VERSION);
    assert.equal(result.manifest.exportedAtUnixMs, 10);
    assert.match(result.filename, /request-flow-offline-guide\.html$/);
    assert.doesNotMatch(result.html, /revision:draft|revision:rejected/);
    await assert.rejects(buildOfflineGuideHtml([accepted, draft], {
      selectedRevisionIds: ['revision:draft'], includeExcerpts: true,
    }), /Only accepted/);
  });

  it('bundles cited excerpts by default at the API level and hashes excerpts and safe typed visuals', async () => {
    const preview = buildOfflineExportPreview([chapter()], ['revision:one'], true);
    assert.equal(preview.chapters[0].sections[0].typedVisual, 'included');
    const result = await buildOfflineGuideHtml([chapter()], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: true, exportedAtUnixMs: 10,
    });
    const excerpt = result.manifest.assets.find((asset) => asset.kind === 'citedExcerpt');
    const visual = result.manifest.assets.find((asset) => asset.kind === 'typedVisual');
    assert.ok(excerpt);
    assert.equal(excerpt.sha256, createHash('sha256').update('export function main() { return run(); }').digest('hex'));
    assert.equal(excerpt.path, 'src/main.ts');
    assert.equal(excerpt.snapshotId, 'snapshot:fixture');
    assert.equal(excerpt.byteLength, Buffer.byteLength('export function main() { return run(); }'));
    assert.ok(visual);
    assert.equal(visual.mediaType, 'application/json');
    assert.equal(visual.sha256, createHash('sha256').update(JSON.stringify(typedVisual())).digest('hex'));
    assert.equal(visual.byteLength, Buffer.byteLength(JSON.stringify(typedVisual())));
    assert.equal(visual.chapterRevisionId, 'revision:one');
    assert.equal(visual.sectionId, 'section:request');
    assert.match(visual.sha256, /^[a-f0-9]{64}$/);
    assert.match(result.html, /Cited excerpts are bundled below their sections/);
    assert.match(result.html, /Request path/);
    assert.match(result.html, /Typed diagram data · JSON/);
  });

  it('renders HTML compositions with local source links and manual scene navigation', async () => {
    const visual = typedVisual();
    visual.presentation = { schemaVersion: 3, scenes: [{
      claimIndex: 0, title: 'Request input', narration: 'The request input reaches the route before the worker handles it.', mode: 'composition', elements: [
        { kind: 'row', parentIndex: null, nodeIndex: null, tone: 'neutral', value: '', valueOrigin: 'none' },
        { kind: 'code', parentIndex: 0, nodeIndex: 0, tone: 'accent', value: 'run()', valueOrigin: 'source' },
      ],
    }] };
    const revisions = [chapter({ sections: [section({ visual })] })];
    const preview = buildOfflineExportPreview(revisions, ['revision:one'], true);
    const notice = preview.chapters[0].sections[0].visualNote;
    assert.match(notice, /HTML scenes are included/);
    assert.equal(preview.chapters[0].sections[0].typedVisual, 'included');
    const result = await buildOfflineGuideHtml(revisions, {
      selectedRevisionIds: ['revision:one'], includeExcerpts: true, exportedAtUnixMs: 10,
    });
    assert.match(result.html, /class="offline-scene"/);
    assert.match(result.html, /class="scene-element scene-code scene-tone-accent scene-content" href="#evidence-1-1"/);
    assert.match(result.html, /<pre><code>run\(\)<\/code><\/pre>/);
    assert.match(result.html, /id="section-1-1-scene-1"/);
    assert.doesNotMatch(result.html, /interactive visual layout is available in Code Atlas/);
    const asset = result.manifest.assets.find((candidate) => candidate.kind === 'typedVisual');
    assert.equal(asset.sha256, createHash('sha256').update(JSON.stringify(visual)).digest('hex'));
  });

  it('embeds selected cached WAV tracks with their exact sanitized reviewed transcript and hashes in schema 4', async () => {
    const wav = narrationWav();
    const persistedScript = 'Outdated accepted script must not appear beside different audio.';
    const exactReviewedText = 'Read the cached voice script. Token ghp_0123456789abcdefghijklmnopqrstuvwx at /Users/alice/private-project.';
    const safeReviewedText = 'Read the cached voice script. Token [redacted credential] at [local path omitted]';
    const accepted = chapter();
    accepted.sections[0].spokenText = persistedScript;
    const audio = {
      chapterRevisionId: 'revision:one',
      sectionId: 'section:request',
      exactText: exactReviewedText,
      audio: {
        fingerprint: 'b'.repeat(64),
        mimeType: 'audio/wav',
        audioBase64: wav.toString('base64'),
        cacheHit: true,
      },
    };
    const preview = buildOfflineExportPreview([accepted], ['revision:one'], false, [audio]);
    assert.equal(preview.chapters[0].sections[0].narrationTranscript, safeReviewedText);
    const result = await buildOfflineGuideHtml([accepted], {
      selectedRevisionIds: ['revision:one'],
      includeExcerpts: false,
      includeNarrationAudio: true,
      narrationAudio: [audio],
      exportedAtUnixMs: 10,
    });
    const bundled = result.manifest.assets.find((asset) => asset.kind === 'narrationAudio');
    const transcript = result.manifest.assets.find((asset) => asset.kind === 'narrationTranscript');
    assert.equal(result.manifest.schemaVersion, 4);
    assert.equal(result.manifest.features.narrationAudio, 'included');
    assert.deepEqual(bundled, {
      assetId: 'narration:revision:one:section:request',
      kind: 'narrationAudio',
      mediaType: 'audio/wav',
      chapterRevisionId: 'revision:one',
      snapshotId: 'snapshot:fixture',
      sectionId: 'section:request',
      cacheFingerprint: 'b'.repeat(64),
      sha256: createHash('sha256').update(wav).digest('hex'),
      byteLength: wav.length,
    });
    assert.deepEqual(transcript, {
      assetId: 'narration-transcript:revision:one:section:request',
      kind: 'narrationTranscript',
      mediaType: 'text/plain',
      chapterRevisionId: 'revision:one',
      snapshotId: 'snapshot:fixture',
      sectionId: 'section:request',
      cacheFingerprint: 'b'.repeat(64),
      sha256: createHash('sha256').update(safeReviewedText).digest('hex'),
      byteLength: Buffer.byteLength(safeReviewedText),
    });
    assert.match(result.html, /media-src data:/);
    assert.match(result.html, /<audio controls preload="none" aria-label="Narration for Handle a request" src="data:audio\/wav;base64,/);
    assert.match(result.html, /Selected cached narration is bundled in its matching sections/);
    assert.match(result.html, /Narration transcript/);
    assert.match(result.html, /Read the cached voice script\. Token \[redacted credential\] at \[local path omitted\]/);
    assert.doesNotMatch(result.html, /Outdated accepted script must not appear beside different audio/);
    assert.doesNotMatch(result.html, /ghp_0123456789abcdefghijklmnopqrstuvwx|alice|private-project/);
    assert.doesNotMatch(result.html, /https?:\/\//);
  });

  it('reports when audio was requested but no matching cached track exists', async () => {
    const result = await buildOfflineGuideHtml([chapter()], {
      selectedRevisionIds: ['revision:one'],
      includeExcerpts: false,
      includeNarrationAudio: true,
    });
    assert.equal(result.manifest.features.narrationAudio, 'none-cached');
    assert.equal(result.manifest.assets.some((asset) => asset.kind === 'narrationAudio'), false);
    assert.match(result.html, /no cached section audio was available/);
    assert.doesNotMatch(result.html, /<audio /);
  });

  it('marks each selected section as included, transcript-only, or unavailable in the audio preview', () => {
    const included = section({ sectionId: 'section:included', order: 0 });
    const notCached = section({ sectionId: 'section:not-cached', order: 1 });
    const unreadable = section({ sectionId: 'section:unreadable', order: 2 });
    const unverifiable = section({ sectionId: 'section:unverifiable', order: 3 });
    const noTranscript = section({ sectionId: 'section:no-transcript', order: 4 });
    noTranscript.spokenText = '';
    const accepted = chapter({ sections: [included, notCached, unreadable, unverifiable, noTranscript] });
    const cached = [included, noTranscript].map((item) => ({
      chapterRevisionId: accepted.revisionId,
      sectionId: item.sectionId,
      exactText: 'The cached reviewed script.',
      audio: { fingerprint: 'b'.repeat(64), mimeType: 'audio/wav', audioBase64: '', cacheHit: true },
    }));
    const omissions = [
      { chapterRevisionId: accepted.revisionId, sectionId: notCached.sectionId, status: 'omitted', message: 'No matching cached audio; transcript-only.' },
      { chapterRevisionId: accepted.revisionId, sectionId: unreadable.sectionId, status: 'unavailable', message: 'Audio cache could not be read; transcript-only.' },
    ];
    const coverage = buildOfflineNarrationCoverage(
      [accepted], [accepted.revisionId], true, 'unavailable', cached, omissions,
    );

    assert.equal(coverage.size, accepted.sections.length);
    assert.equal(coverage.get(`revision:one\u0000${included.sectionId}`).status, 'included');
    assert.deepEqual(coverage.get(`revision:one\u0000${notCached.sectionId}`), {
      status: 'transcript-only', message: 'No matching cached audio; transcript-only.',
    });
    assert.deepEqual(coverage.get(`revision:one\u0000${unreadable.sectionId}`), {
      status: 'unavailable', message: 'Audio cache could not be read; transcript-only.',
    });
    assert.equal(coverage.get(`revision:one\u0000${unverifiable.sectionId}`).status, 'unavailable');
    assert.equal(coverage.get(`revision:one\u0000${noTranscript.sectionId}`).status, 'included');

    const browserOnly = buildOfflineNarrationCoverage([accepted], [accepted.revisionId], true, 'desktop-only', [], []);
    assert.equal(browserOnly.get(`revision:one\u0000${notCached.sectionId}`).status, 'unavailable');
    assert.equal(browserOnly.get(`revision:one\u0000${noTranscript.sectionId}`).status, 'unavailable');
    const loading = buildOfflineNarrationCoverage([accepted], [accepted.revisionId], true, 'loading', [], []);
    assert.equal(loading.get(`revision:one\u0000${notCached.sectionId}`).status, 'checking');
    assert.equal(loading.get(`revision:one\u0000${noTranscript.sectionId}`).status, 'checking');
    const unavailable = buildOfflineNarrationCoverage([accepted], [accepted.revisionId], true, 'unavailable', [], []);
    assert.equal(unavailable.get(`revision:one\u0000${noTranscript.sectionId}`).status, 'unavailable');
    const readyWithoutTrack = buildOfflineNarrationCoverage([accepted], [accepted.revisionId], true, 'ready', [], []);
    assert.equal(readyWithoutTrack.get(`revision:one\u0000${notCached.sectionId}`).status, 'transcript-only');
    assert.equal(readyWithoutTrack.get(`revision:one\u0000${noTranscript.sectionId}`).status, 'no-transcript');
    const notSelected = buildOfflineNarrationCoverage([accepted], [accepted.revisionId], false, 'idle', [], []);
    assert.equal(notSelected.get(`revision:one\u0000${notCached.sectionId}`).status, 'not-selected');
  });

  it('rejects malformed or unsupported narration assets before producing a package', async () => {
    const invalid = {
      chapterRevisionId: 'revision:one',
      sectionId: 'section:request',
      exactText: 'A cached narration transcript.',
      audio: {
        fingerprint: 'c'.repeat(64),
        mimeType: 'audio/wav',
        audioBase64: Buffer.from('not a WAV').toString('base64'),
        cacheHit: true,
      },
    };
    await assert.rejects(buildOfflineGuideHtml([chapter()], {
      selectedRevisionIds: ['revision:one'],
      includeExcerpts: false,
      includeNarrationAudio: true,
      narrationAudio: [invalid],
    }), /not the supported 24 kHz mono PCM WAV format/);
  });

  it('has an explicit omit-excerpts mode while keeping path and line metadata', async () => {
    const result = await buildOfflineGuideHtml([chapter()], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: false, exportedAtUnixMs: 10,
    });
    assert.equal(result.manifest.features.citedExcerpts, 'omitted');
    assert.equal(result.manifest.assets.some((asset) => asset.kind === 'citedExcerpt'), false);
    assert.equal(result.manifest.assets.some((asset) => asset.kind === 'typedVisual'), true);
    assert.match(result.html, /Cited source excerpts were omitted/);
    assert.match(result.html, /src\/main\.ts:1-2/);
    assert.match(result.html, /Source excerpt omitted from this package/);
    assert.match(result.html, /Privacy review:/);
  });

  it('renders chapter text and typed visuals as inert escaped text, with manual section navigation', async () => {
    const first = section({
      sectionId: 'section:first', order: 0,
      anchors: [sourceAnchor()],
    });
    first.title = '<script>steal()</script>';
    first.prose = 'Text <img src=x onerror=steal()> stays text.';
    first.visual.nodes[0].label = '<script>diagram()</script>';
    first.hotspots[0].label = '<svg onload=steal()>';
    first.hotspots[0].destination = { kind: 'section', sectionId: 'section:second' };
    const second = section({ sectionId: 'section:second', order: 1, visual: null });
    const result = await buildOfflineGuideHtml([chapter({ sections: [first, second] })], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: true,
    });

    assert.doesNotMatch(result.html, /<script>|<img src=x|<svg onload=/i);
    assert.match(result.html, /&lt;script&gt;steal\(\)&lt;\/script&gt;/);
    assert.match(result.html, /&lt;script&gt;diagram\(\)&lt;\/script&gt;/);
    assert.match(result.html, /href="#section-1-2"/);
    assert.match(result.html, /Previous: &lt;script&gt;steal/);
    assert.match(result.html, /Continue: Handle a request/);
    assert.match(result.html, /&lt;svg onload=steal\(\)&gt;/);
    assert.match(result.html, /Narration audio was not selected/);
    assert.match(result.html, /Narration transcript/);
  });

  it('is a self-contained offline document with resolvable section, evidence, and hotspot navigation', async () => {
    const first = section({ sectionId: 'section:first', order: 0 });
    first.hotspots = [
      { hotspotId: 'hotspot:next', label: 'Next section', destination: { kind: 'section', sectionId: 'section:second' } },
      { hotspotId: 'hotspot:evidence', label: 'Bundled code', destination: { kind: 'codeLens', anchorId: 'anchor:entry' } },
      { hotspotId: 'hotspot:file', label: 'Open file in Atlas', destination: { kind: 'atlasFile', path: 'src/main.ts' } },
      { hotspotId: 'hotspot:relationship', label: 'Open relationship in Atlas', destination: { kind: 'relationship', relationshipId: 'relationship:worker' } },
    ];
    first.visual.nodes = [
      { ...first.visual.nodes[0], hotspotId: 'hotspot:next' },
      { ...first.visual.nodes[1], hotspotId: 'hotspot:evidence' },
      { nodeId: 'node:atlas-file', label: 'External file target', detail: null, hotspotId: 'hotspot:file', evidenceAnchorIds: ['anchor:entry'] },
      { nodeId: 'node:atlas-relationship', label: 'External relationship target', detail: null, hotspotId: 'hotspot:relationship', evidenceAnchorIds: ['anchor:entry'] },
    ];
    const second = section({ sectionId: 'section:second', order: 1 });
    const firstChapter = chapter({ revisionId: 'revision:first', sections: [first, second] });
    const secondChapter = chapter({ revisionId: 'revision:second', title: 'Worker details' });
    const result = await buildOfflineGuideHtml([firstChapter, secondChapter], {
      selectedRevisionIds: ['revision:first', 'revision:second'],
      includeExcerpts: true,
      exportedAtUnixMs: 10,
    });

    const ids = new Set([...result.html.matchAll(/\bid="([^"]+)"/g)].map((match) => match[1]));
    const fragmentLinks = [...result.html.matchAll(/\bhref="#([^"]+)"/g)].map((match) => match[1]);
    assert.ok(fragmentLinks.length > 0);
    for (const target of fragmentLinks) assert.ok(ids.has(target), `missing offline navigation target #${target}`);

    const resourceLinks = [...result.html.matchAll(/\b(?:src|href)="([^"]+)"/g)].map((match) => match[1]);
    assert.ok(resourceLinks.every((url) => url.startsWith('#') || url.startsWith('data:')));
    assert.doesNotMatch(result.html, /<script\b|<link\b[^>]*href=/i);
    assert.match(result.html, /This file works without Code Atlas or a network connection/);
    assert.match(result.html, /Atlas destination unavailable offline/);
    assert.match(result.html, /Atlas relationship unavailable offline/);
    assert.match(result.html, /Narration transcript/);
    assert.equal(result.manifest.chapters.length, 2);
    assert.deepEqual(result.manifest.chapters.map((entry) => entry.revisionId), ['revision:first', 'revision:second']);
  });

  it('omits hidden references and redacts common secrets and local paths from copied prose and excerpts', async () => {
    const exposedKey = 'ghp_0123456789abcdefghijklmnopqrstuvwx';
    const sensitive = chapter({ sections: [section({ anchors: [
      sourceAnchor('anchor:public', 'src/main.ts', `const api_key = "${exposedKey}";`),
      sourceAnchor('anchor:hidden', 'src/.env', 'PRIVATE_KEY=not-for-export'),
    ] })] });
    sensitive.sections[0].prose = `The checkout is /Users/alice/private-project and reads .ssh/id_ed25519. ${exposedKey}`;
    sensitive.sections[0].spokenText = `Tell the user about ${exposedKey} from /Users/alice/private-project.`;
    const preview = buildOfflineExportPreview([sensitive], ['revision:one'], true);
    assert.equal(preview.omittedHiddenReferences, 1);
    assert.deepEqual(preview.sourcePaths, ['src/main.ts']);
    assert.ok(preview.redactedCredentials > 0);
    assert.doesNotMatch(JSON.stringify(preview), /alice|private-project|\.ssh|id_ed25519|src\/\.env|ghp_0123456789/);
    assert.match(preview.chapters[0].sections[0].spokenText, /\[redacted credential\]/);
    assert.match(JSON.stringify(preview), /redacted credential/);

    const result = await buildOfflineGuideHtml([sensitive], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: true,
    });
    assert.doesNotMatch(result.html, /alice|private-project|\.ssh|id_ed25519|src\/\.env|ghp_0123456789|PRIVATE_KEY=not-for-export/);
    assert.match(result.html, /hidden or sensitive paths were omitted/);
    assert.match(result.html, /likely credential was redacted/);
  });

  it('does not export unsafe target paths or allow an unselected duplicate revision ID', async () => {
    assert.equal(isSafeOfflineSourcePath('src/main.ts'), true);
    for (const path of ['/Users/me/repo/src.ts', '../secret', 'src/.env', 'config/credentials', 'keys/signing.pem', 'src\\main.ts']) {
      assert.equal(isSafeOfflineSourcePath(path), false, path);
    }
    const unsafeTarget = chapter({ target: { kind: 'file', path: '.ssh/id_ed25519' } });
    assert.equal(isOfflineExportableRevision(unsafeTarget), false);
    await assert.rejects(buildOfflineGuideHtml([unsafeTarget], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: true,
    }), /Only accepted/);
    await assert.rejects(buildOfflineGuideHtml([chapter()], {
      selectedRevisionIds: ['revision:one', 'revision:one'], includeExcerpts: true,
    }), /selected more than once/);
    const oldChapter = chapter(); oldChapter.schemaVersion = 4;
    assert.equal(isOfflineExportableRevision(oldChapter), false);
    await assert.rejects(buildOfflineGuideHtml([oldChapter], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: true,
    }), /Only accepted/);
  });

  it('attaches each cached scene to its exact reviewed transcript and records independent hashes', async () => {
    const accepted = sceneChapter();
    const tracks = [sceneTrack('claim:input', 'Reviewed incoming narration.'), sceneTrack('claim:return', 'Reviewed return narration.')];
    const before = structuredClone(accepted);
    const preview = buildOfflineExportPreview([accepted], [accepted.revisionId], true, tracks);
    assert.deepEqual(preview.chapters[0].sections[0].sceneNarration.map(track => [track.sceneId, track.cached, track.transcript]), [
      ['claim:input', true, 'Reviewed incoming narration.'], ['claim:return', true, 'Reviewed return narration.'],
    ]);
    const result = await buildOfflineGuideHtml([accepted], { selectedRevisionIds: [accepted.revisionId], includeExcerpts: true,
      includeNarrationAudio: true, narrationAudio: tracks, exportedAtUnixMs: 10 });
    const audio = result.manifest.assets.filter(asset => asset.kind === 'narrationAudio');
    assert.deepEqual(audio.map(asset => asset.sceneId), ['claim:input', 'claim:return']);
    assert.equal(new Set(audio.map(asset => asset.assetId)).size, 2);
    assert.ok(audio.every(asset => asset.sha256 === createHash('sha256').update(narrationWav()).digest('hex')));
    assert.equal((result.html.match(/<audio controls/g) ?? []).length, 2);
    const frames = result.html.split('class="offline-story-scene"');
    assert.match(frames[1], /Reviewed incoming narration\./);
    assert.doesNotMatch(frames[1], /Reviewed return narration\./);
    assert.match(frames[2], /Reviewed return narration\./);
    assert.match(result.html, /href="#section-1-1-scene-2"/);
    assert.match(result.html, /href="#section-1-1-scene-1"/);
    assert.equal(result.manifest.features.visualScenes, 'included-when-supported');
    assert.deepEqual(accepted, before);
  });

  it('reports partial scene audio and never substitutes a whole-section recording', async () => {
    const accepted = sceneChapter(); const tracks = [sceneTrack('claim:input')];
    const coverage = buildOfflineNarrationCoverage([accepted], [accepted.revisionId], true, 'ready', tracks, []);
    const state = coverage.get(`revision:one\u0000section:request`);
    assert.equal(state.status, 'partial');
    assert.match(state.message, /1 of 2 scenes/);
    assert.equal(buildOfflineNarrationCoverage([accepted], [accepted.revisionId], true, 'ready', [sceneTrack('claim:foreign')], [])
      .get(`revision:one\u0000section:request`).status, 'unavailable');
    const result = await buildOfflineGuideHtml([accepted], { selectedRevisionIds: [accepted.revisionId], includeExcerpts: true,
      includeNarrationAudio: true, narrationAudio: tracks });
    assert.equal(result.manifest.assets.filter(asset => asset.kind === 'narrationAudio').length, 1);
    assert.match(result.html, /No cached audio is included for this scene/);
    assert.match(result.html, /run returns the result/);
    const invalid = [
      [sceneTrack('claim:foreign')],
      [tracks[0], tracks[0]],
      [tracks[0], { ...tracks[0], sceneId: undefined }],
    ];
    for (const narrationAudio of invalid) await assert.rejects(buildOfflineGuideHtml([accepted], {
      selectedRevisionIds: [accepted.revisionId], includeExcerpts: true, includeNarrationAudio: true, narrationAudio,
    }), /authored scene|more than once|without mixing/);
    const omitted = await buildOfflineGuideHtml([accepted], { selectedRevisionIds: [accepted.revisionId], includeExcerpts: true,
      includeNarrationAudio: false, narrationAudio: tracks });
    assert.equal(omitted.manifest.features.narrationAudio, 'omitted');
    assert.doesNotMatch(omitted.html, /Reviewed claim:input/);
  });

  it('filters scene titles and values from visible HTML and typed assets, and omits source-backed values on request', async () => {
    const accepted = sceneChapter();
    const secret = 'ghp_0123456789abcdefghijklmnopqrstuvwx';
    for (const claim of accepted.sections[0].claims) claim.anchors[0].excerpt = `const example = "${secret}";`;
    for (const scene of accepted.sections[0].visual.presentation.scenes) {
      scene.title = `At /Users/alice/private-project ${secret}`;
      scene.elements[1].value = secret;
    }
    const result = await buildOfflineGuideHtml([accepted], { selectedRevisionIds: [accepted.revisionId], includeExcerpts: true });
    assert.doesNotMatch(result.html, new RegExp(secret));
    assert.doesNotMatch(result.html, /\/Users\/alice/);
    assert.match(result.html, /\[redacted credential\]/);
    assert.match(result.html, /\[local path omitted\]/);
    const ordinary = sceneChapter();
    const preview = buildOfflineExportPreview([ordinary], [ordinary.revisionId], false);
    assert.match(preview.chapters[0].sections[0].visualNote, /Source-backed visual values are omitted/);
    const omitted = await buildOfflineGuideHtml([ordinary], { selectedRevisionIds: [ordinary.revisionId], includeExcerpts: false });
    assert.doesNotMatch(omitted.html, /run\(\)/);
    assert.match(omitted.html, /Source-backed visual values were omitted/);
    assert.match(omitted.html, /class="offline-scene"/);
  });

  it('shows a clear message for unsupported visuals and invalid selection', async () => {
    const unsupported = chapter({ sections: [section({ visual: { schemaVersion: 9, html: '<script>run()</script>' } })] });
    const preview = buildOfflineExportPreview([unsupported], ['revision:one'], false);
    assert.equal(preview.chapters[0].sections[0].typedVisual, 'unavailable');
    const result = await buildOfflineGuideHtml([unsupported], {
      selectedRevisionIds: ['revision:one'], includeExcerpts: false,
    });
    assert.match(result.html, /This diagram is unavailable in the offline format/);
    assert.doesNotMatch(result.html, /<script>run\(\)<\/script>/);
    await assert.rejects(buildOfflineGuideHtml([chapter()], { selectedRevisionIds: [], includeExcerpts: true }), /Select at least one/);
  });
});
