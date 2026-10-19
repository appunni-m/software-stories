import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const checker = resolve(root, 'scripts/check-chapter-quality.mjs');
const manifestPath = resolve(root, 'benchmarks/chapter-quality/corpus/manifest.json');
const hasQualityGateExample = existsSync(resolve(root, 'benchmarks/chapter-quality/review/QUALITY_GATE_EXAMPLE.json'));

async function withTemporaryCorpus(run) {
  const tempDirectory = await mkdtemp(join(root, 'tests', '.chapter-quality-corpus-'));
  const corpusDirectory = join(tempDirectory, 'corpus');
  try {
    await cp(dirname(manifestPath), corpusDirectory, { recursive: true });
    await run({ corpusDirectory, manifestPath: join(corpusDirectory, 'manifest.json') });
  } finally {
    await rm(tempDirectory, { recursive: true, force: true });
  }
}

describe('offline chapter-quality corpus checker', () => {
  it('accepts the checked-in corpus against the current parser-language contract', { skip: !hasQualityGateExample && 'Optional benchmark example is absent from this worktree.' }, () => {
    const result = spawnSync(process.execPath, [checker, '--corpus'], { cwd: root, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /6 cases across 3 synthetic repositories/);
    assert.match(result.stdout, /unsupported language/);
    assert.match(result.stdout, /Fact-coverage self-check rejects unrelated evidence spans and accepts containing anchors/);
  });

  it('reports an explicit response-schema version alongside its content hash', async () => {
    const result = spawnSync(process.execPath, [checker, '--metadata'], { cwd: root, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    const metadata = JSON.parse(result.stdout);
    assert.equal(metadata.responseSchemaVersion, 3);
    assert.equal(metadata.backendId, 'codex');
    assert.equal(metadata.modelId, 'gpt-6-luna');
    assert.equal(metadata.reasoningEffort, 'max');
    assert.equal(metadata.approvalPolicy, 'never');
    assert.match(metadata.scope, /no model or account qualification/);
    assert.match(metadata.nativeBuildSha256, /^[a-f0-9]{64}$/);
    assert.match(metadata.responseSchemaSha256, /^[a-f0-9]{64}$/);
    const reportTemplate = JSON.parse(await readFile(join(root, 'benchmarks/chapter-quality/review/REPORT_TEMPLATE.json'), 'utf8'));
    assert.equal(reportTemplate.reportSchemaVersion, 5);
    const rubric = await readFile(join(root, 'benchmarks/chapter-quality/review/RUBRIC.md'), 'utf8');
    assert.match(rubric, new RegExp(`report schema v${reportTemplate.reportSchemaVersion}`));
    assert.match(rubric, /response-schema version and SHA-256/);
  });

  it('rejects a parser-supported language used for the unsupported-language role', async () => {
    await withTemporaryCorpus(async ({ manifestPath: tempManifestPath }) => {
      const manifest = JSON.parse(await readFile(tempManifestPath, 'utf8'));
      const unsupportedCase = manifest.cases.find((item) => item.id === 'unsupported-language');
      unsupportedCase.language = 'TypeScript';
      await writeFile(tempManifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

      const relativeManifestPath = relative(root, tempManifestPath);
      const result = spawnSync(process.execPath, [checker, '--corpus', relativeManifestPath], { cwd: root, encoding: 'utf8' });
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /unsupported-language: TypeScript is currently parser-supported and cannot represent the unsupported-language role/);
    });
  });

  it('rejects a supported parser status for a language outside the current parser set', async () => {
    await withTemporaryCorpus(async ({ manifestPath: tempManifestPath }) => {
      const manifest = JSON.parse(await readFile(tempManifestPath, 'utf8'));
      const serviceCase = manifest.cases.find((item) => item.id === 'service-core');
      serviceCase.language = 'Synthetic Session Policy DSL';
      await writeFile(tempManifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

      const relativeManifestPath = relative(root, tempManifestPath);
      const result = spawnSync(process.execPath, [checker, '--corpus', relativeManifestPath], { cwd: root, encoding: 'utf8' });
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /service-core: parserStatus is supported but Synthetic Session Policy DSL is not in the current parser language set/);
    });
  });
});
