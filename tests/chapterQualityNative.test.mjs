import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';
import { assertNativeBuild, currentNativeMetadata, defaultProfile, nativeSourceHash, runNativeQuality, validateCurrentProposal } from '../scripts/lib/chapter-quality-native.mjs';

const artifactRoot = new URL('../docs/acceptance/artifacts/', import.meta.url);
const workspace = resolve(dirname(fileURLToPath(import.meta.url)), '..');
async function savedAttempt(name) {
  const artifact = JSON.parse(await readFile(new URL(name, artifactRoot), 'utf8'));
  const sourceJson = artifact.request.userPrompt.split('SELECTED SOURCE JSON:\n')[1].split('\n\nSELECTED LOCAL INDEX EVIDENCE JSON:\n')[0];
  const sources = new Map(JSON.parse(sourceJson).map((file) => [file.path, { text: file.contents, sha256: file.sha256 }]));
  return { artifact, sources };
}

describe('current native chapter quality boundary', () => {
  it('replays the preserved real Fontdone answer without a model call', async () => {
    const { artifact, sources } = await savedAttempt('2026-10-17-fontdone-file-layout-v2.json');
    const result = validateCurrentProposal(artifact.responseText, sources, artifact.source.path);
    assert.equal(result.valid, true, JSON.stringify(result));
    assert.equal(result.counts.sourceAnchors, 23);
    assert.equal(result.counts.sections, 4);
    // This mechanical result does not alter the preserved failed teaching review.
    assert.equal(artifact.humanQualityOrListeningPass, false);
  });

  it('keeps the original five leaf-parent errors rejected', async () => {
    const { artifact, sources } = await savedAttempt('2026-10-17-fontdone-file-teaching-v2-rejected.json');
    const result = validateCurrentProposal(artifact.responseText, sources, artifact.source.path);
    assert.equal(result.valid, false);
    assert.match(result.message, /parent|container|leaf/i);
  });

  it('fails closed for source infrastructure errors instead of counting them as a candidate failure', async () => {
    const { artifact, sources } = await savedAttempt('2026-10-17-fontdone-file-layout-v2.json');
    const changed = new Map([...sources].map(([path, source]) => [path, { ...source, text: `${source.text}\nchanged` }]));
    assert.throws(() => validateCurrentProposal(artifact.responseText, changed, artifact.source.path), /changed hash/);
    assert.throws(() => runNativeQuality({ action: 'metadata', profile: { ...defaultProfile, modelId: 'different' } }), /GPT-6 Luna\/max/);
  });

  it('rejects old and mismatched helper build identities', () => {
    const hash = nativeSourceHash();
    assertNativeBuild({ protocolVersion: 1, nativeBuildSha256: hash }, hash);
    for (const response of [{}, { protocolVersion: 0, nativeBuildSha256: hash }, { protocolVersion: 1, nativeBuildSha256: '0'.repeat(64) }]) {
      assert.throws(() => assertNativeBuild(response, hash), /stale or incompatible/);
    }
  });

  it('fingerprints shared instructions, native validators and dependency locks', async () => {
    const root = await mkdtemp(join(tmpdir(), 'chapter-quality-source-hash-'));
    try {
      for (const path of ['src/provider', 'resources', 'capabilities']) await mkdir(join(root, path), { recursive: true });
      const paths = ['build.rs', 'Cargo.toml', 'Cargo.lock', 'tauri.conf.json', 'src/provider/visual_scenes.rs', 'src/chapters.rs'];
      for (const path of paths) await writeFile(join(root, path), `original:${path}`);
      const original = nativeSourceHash(root);
      for (const path of paths) {
        await writeFile(join(root, path), `changed:${path}`);
        assert.notEqual(nativeSourceHash(root), original, path);
        await writeFile(join(root, path), `original:${path}`);
      }
      await writeFile(join(root, 'resources/claude_generation_qualifications.json'), '{}');
      assert.equal(nativeSourceHash(root), original);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  it('rejects stale report metadata before considering its review outcomes', async () => {
    const reports = join(workspace, 'benchmarks/chapter-quality/review/reports');
    await mkdir(reports, { recursive: true });
    const directory = await mkdtemp(join(reports, 'stale-contract-test-'));
    try {
      const report = JSON.parse(await readFile(join(workspace, 'benchmarks/chapter-quality/review/REPORT_TEMPLATE.json'), 'utf8'));
      const metadata = currentNativeMetadata();
      for (const key of Object.keys(report)) if (Object.hasOwn(metadata, key)) report[key] = metadata[key];
      const manifestBytes = await readFile(join(workspace, 'benchmarks/chapter-quality/corpus/manifest.json'));
      report.corpusId = JSON.parse(manifestBytes).corpusId;
      report.corpusManifestSha256 = createHash('sha256').update(manifestBytes).digest('hex');
      report.status = 'pass';
      report.modelVersion = 'not-returned';
      report.reviewer = 'rejection-test-only';
      report.reviewedAt = '2026-10-17T00:00:00Z';
      report.requestAssemblySha256 = '0'.repeat(64);
      const path = join(directory, 'report.json');
      await writeFile(path, JSON.stringify(report));
      const result = spawnSync(process.execPath, [join(workspace, 'scripts/check-chapter-quality.mjs'), '--report', relative(workspace, path)], { cwd: workspace, encoding: 'utf8' });
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /requestAssemblySha256 is stale/);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
