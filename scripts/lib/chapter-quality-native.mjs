import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { lstatSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const defaultProfile = Object.freeze({ backendId: 'codex', modelId: 'gpt-6-luna', reasoningEffort: 'max', approvalPolicy: 'never' });
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

// Match build.rs exactly. The excluded Claude receipt would hash itself; every
// validator, shared prompt, request builder and locked dependency is included.
export function nativeSourceHash(nativeRoot = join(root, 'src-tauri')) {
  const inputs = ['build.rs', 'Cargo.toml', 'Cargo.lock', 'tauri.conf.json'];
  function collect(path) {
    if (!lstatSync(join(nativeRoot, path)).isDirectory()) throw new Error('Native input directory must be a real directory');
    for (const name of readdirSync(join(nativeRoot, path))) {
      const child = join(path, name);
      const kind = lstatSync(join(nativeRoot, child));
      if (kind.isDirectory()) collect(child);
      else if (kind.isFile()) inputs.push(child);
      else throw new Error('Native build inputs cannot contain symlinks or special files');
    }
  }
  for (const path of ['src', 'resources', 'capabilities']) collect(path);
  // Rust Path::Ord compares components: a directory "chapter" precedes the
  // sibling "chapter.rs" even though raw bytes would put '.' before '/'.
  inputs.sort((a, b) => {
    const left = a.split(sep), right = b.split(sep);
    for (let i = 0; i < Math.min(left.length, right.length); i++) {
      const order = Buffer.compare(Buffer.from(left[i]), Buffer.from(right[i]));
      if (order) return order;
    }
    return left.length - right.length;
  });
  const hash = createHash('sha256');
  const length = (value) => { const bytes = Buffer.alloc(8); bytes.writeBigUInt64LE(BigInt(value)); return bytes; };
  for (const path of inputs) {
    if (path === 'resources/claude_generation_qualifications.json') continue;
    if (!lstatSync(join(nativeRoot, path)).isFile()) throw new Error('Native build input must be a regular file');
    const name = Buffer.from(path);
    const bytes = readFileSync(join(nativeRoot, path));
    hash.update(length(name.length)).update(name).update(length(bytes.length)).update(bytes);
  }
  return hash.digest('hex');
}

export function assertNativeBuild(response, expected) {
  if (response?.protocolVersion !== 1 || response.nativeBuildSha256 !== expected) {
    throw new Error('Quality helper is stale or incompatible. Run npm run quality:build from this checkout.');
  }
}

export function runNativeQuality(request) {
  const expected = nativeSourceHash();
  const executable = process.env.CODE_ATLAS_QUALITY_HELPER || join(root, 'src-tauri/target/debug/code-atlas-quality');
  const input = JSON.stringify({ protocolVersion: 1, ...request });
  if (Buffer.byteLength(input) > 2 * 1024 * 1024) throw new Error('Quality request exceeds 2 MiB');
  const result = spawnSync(executable, ['--chapter-quality-helper'], {
    cwd: root, input, encoding: 'utf8', timeout: 30_000, maxBuffer: 4 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) {
    throw new Error(`Native quality helper failed (${result.error?.message || result.signal || result.status}). Run npm run quality:build; no legacy validation fallback is used.`);
  }
  let response;
  try { response = JSON.parse(result.stdout); } catch { throw new Error('Quality helper did not return protocol JSON'); }
  assertNativeBuild(response, expected);
  if (response.status !== 'ok') throw new Error(response.message || 'Native quality helper rejected the input');
  return response.data;
}

export function currentNativeMetadata(profile = defaultProfile) {
  const data = runNativeQuality({ action: 'metadata', profile });
  const nativeBuildSha256 = nativeSourceHash();
  return {
    scope: data.scope,
    backendId: data.backendId, modelId: data.modelId,
    reasoningEffort: data.reasoningEffort, approvalPolicy: data.approvalPolicy,
    nativeBuildSha256,
    promptVersion: data.promptVersion,
    promptSha256: sha256(JSON.stringify([data.systemInstruction, data.userPrompt])),
    requestAssemblySha256: sha256(Buffer.concat([
      Buffer.from(nativeBuildSha256), readFileSync(join(root, 'scripts/check-chapter-quality.mjs')),
      readFileSync(fileURLToPath(import.meta.url)),
    ])),
    generationConfigSha256: sha256(JSON.stringify(data.generationConfig)),
    chapterSchemaVersion: data.chapterSchemaVersion,
    responseSchemaVersion: data.responseSchemaVersion,
    responseSchemaSha256: sha256(JSON.stringify(data.responseSchema)),
    modelIds: [...data.modelIds].sort(),
    modelAllowlistSha256: sha256(JSON.stringify([...data.modelIds].sort())),
  };
}

export function validateCurrentProposal(proposal, sources, targetPath, profile = defaultProfile) {
  return runNativeQuality({ action: 'validate', profile, targetPath,
    sources: [...sources].map(([path, source]) => ({ path, text: source.text, sha256: source.sha256 })),
    proposal: typeof proposal === 'string' ? proposal : JSON.stringify(proposal),
  });
}
