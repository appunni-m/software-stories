import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const target = 'aarch64-apple-darwin';
const outputPath = join(root, 'THIRD_PARTY_NOTICES.md');
const licenseTextsOutputPath = join(root, 'THIRD_PARTY_LICENSE_TEXTS.txt');
const cargoManifest = 'src-tauri/Cargo.toml';
const cargoAboutConfig = 'src-tauri/about.toml';
const npmFallback = 'src-tauri/resources/third-party/react-remove-scroll-bar-LICENSE-MIT.txt';
const inputPaths = [
  'package.json',
  'package-lock.json',
  cargoManifest,
  'src-tauri/Cargo.lock',
  cargoAboutConfig,
  'src-tauri/tauri.conf.json',
  npmFallback,
  'src-tauri/resources/third-party/uv-LICENSE-MIT.txt',
  'src-tauri/resources/third-party/uv-LICENSE-APACHE.txt',
  'src-tauri/icons/code-atlas.svg',
  'src-tauri/icons/icon.icns',
  'src-tauri/icons/32x32.png',
  'src-tauri/icons/128x128.png',
  'src-tauri/icons/128x128@2x.png',
  'scripts/generate-third-party-notices.mjs',
];
const legalFileName = /^(?:LICENSE|LICENCE|COPYING|NOTICE|PATENTS|COPYRIGHT)(?:$|[._-])/i;

function run(command, args) {
  return execFileSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'inherit'],
  });
}

function readText(path) {
  return readFileSync(path, 'utf8').replace(/\r\n?/g, '\n').replace(/[\t ]+$/gm, '').trimEnd() + '\n';
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function sourceFingerprint(path) {
  return sha256(readFileSync(join(root, path)));
}

function licenseExpression(value) {
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) return value.map(licenseExpression).join(' OR ');
  if (value && typeof value === 'object') return String(value.type ?? value.name ?? '');
  return '';
}

function person(value) {
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) return value.map(person).filter(Boolean).join('; ');
  if (value && typeof value === 'object') {
    return [value.name, value.email && `<${value.email}>`, value.url && `(${value.url})`]
      .filter(Boolean)
      .join(' ');
  }
  return '';
}

function repositoryUrl(value) {
  const raw = typeof value === 'string' ? value : value?.url;
  return typeof raw === 'string' ? raw.replace(/^git\+/, '').replace(/\.git$/, '') : '';
}

function legalFiles(directory) {
  return readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && legalFileName.test(entry.name))
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b));
}

function licenseTextBlock(map, { ecosystem, id, name, packageId, file, text }) {
  const normalized = text.replace(/\r\n?/g, '\n').replace(/[\t ]+$/gm, '').trimEnd() + '\n';
  const digest = sha256(normalized);
  const key = `${ecosystem}:${id}:${digest}`;
  let block = map.get(key);
  if (!block) {
    block = { ecosystem, id, name, digest, text: normalized, sources: new Set() };
    map.set(key, block);
  }
  block.sources.add(`${packageId} — ${file}`);
}

function tableCell(value) {
  return String(value ?? '').replaceAll('|', '\\|').replace(/[\r\n]+/g, ' ').trim() || '—';
}

function renderLicenseTexts(npmLicenseBlocks, cargoLicenseBlocks, bundledAssetLicenseBlocks, noticeBlocks) {
  const blocks = [
    ...npmLicenseBlocks.values(),
    ...cargoLicenseBlocks.values(),
    ...bundledAssetLicenseBlocks.values(),
    ...noticeBlocks.values(),
  ].sort((a, b) => a.ecosystem.localeCompare(b.ecosystem) || a.id.localeCompare(b.id) || a.name.localeCompare(b.name) || a.digest.localeCompare(b.digest));
  const output = [
    'CODE ATLAS THIRD-PARTY LICENSE AND NOTICE TEXTS',
    'Generated with scripts/generate-third-party-notices.mjs.',
    'See THIRD_PARTY_NOTICES.md for package/version inventory and provenance.',
    '',
  ];
  for (const block of blocks) {
    output.push(
      '='.repeat(78),
      `${block.ecosystem}: ${block.name} (${block.id})`,
      'Included for:',
      ...[...block.sources].sort().map((source) => `  ${source}`),
      '',
      block.text.trimEnd(),
      '',
    );
  }
  return `${output.join('\n').trimEnd()}\n`;
}

function loadNpmPackages(npmLicenseBlocks, npmNoticeBlocks) {
  const paths = run('npm', ['ls', '--omit=dev', '--all', '--parseable'])
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .filter(Boolean);
  const packages = [];
  for (const directory of paths) {
    const path = resolve(directory);
    if (path === root) continue;
    const packageJsonPath = join(path, 'package.json');
    let manifest;
    try {
      manifest = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
    } catch (error) {
      throw new Error(`Could not read npm package manifest ${packageJsonPath}: ${error.message}`);
    }
    if (!manifest.name || !manifest.version) continue;
    const id = `${manifest.name}@${manifest.version}`;
    const expression = licenseExpression(manifest.license);
    if (!expression) throw new Error(`${id} does not declare a package license expression.`);

    let files = legalFiles(path);
    const isFallbackPackage = manifest.name === 'react-remove-scroll-bar' && manifest.version === '2.3.8';
    if (!files.length && !isFallbackPackage) throw new Error(`${id} has no package license or notice text file.`);
    const packageFiles = files.map((file) => ({ file, text: readText(join(path, file)) }));
    if (isFallbackPackage) {
      packageFiles.push({
        file: 'upstream LICENSE (source package omits this file)',
        text: readText(join(root, npmFallback)),
      });
    }
    files = packageFiles.map((entry) => entry.file);

    packages.push({
      id,
      name: manifest.name,
      version: manifest.version,
      expression,
      author: person(manifest.author),
      repository: repositoryUrl(manifest.repository),
      files,
    });
    for (const entry of packageFiles) {
      const targetMap = /^NOTICE(?:$|[._-])/i.test(entry.file) ? npmNoticeBlocks : npmLicenseBlocks;
      licenseTextBlock(targetMap, {
        ecosystem: 'npm',
        id: expression,
        name: entry.file.startsWith('upstream LICENSE') ? 'MIT License (upstream source copy)' : entry.file,
        packageId: id,
        file: entry.file,
        text: entry.text,
      });
    }
  }
  return packages.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
}

function loadCargoInventory(cargoLicenseBlocks, cargoNoticeBlocks, tempDirectory) {
  const jsonPath = join(tempDirectory, 'cargo-about.json');
  run('cargo', [
    'about',
    'generate',
    '--format',
    'json',
    '--config',
    cargoAboutConfig,
    '--manifest-path',
    cargoManifest,
    '--features',
    'tauri/custom-protocol',
    '--target',
    target,
    '--locked',
    '--output-file',
    jsonPath,
  ]);
  const report = JSON.parse(readFileSync(jsonPath, 'utf8'));
  const crates = report.crates
    .filter((entry) => entry.package.source)
    .map((entry) => {
      const pkg = entry.package;
      return {
        id: `${pkg.name}@${pkg.version}`,
        name: pkg.name,
        version: pkg.version,
        expression: licenseExpression(entry.license || pkg.license),
        authors: person(pkg.authors),
        repository: repositoryUrl(pkg.repository),
        manifest: pkg.manifest_path,
      };
    });
  for (const crate of crates) {
    if (!crate.expression) throw new Error(`${crate.id} does not declare a Cargo license expression.`);
    const directory = dirname(crate.manifest);
    for (const file of legalFiles(directory).filter((name) => /^NOTICE(?:$|[._-])/i.test(name))) {
      licenseTextBlock(cargoNoticeBlocks, {
        ecosystem: 'Cargo',
        id: crate.expression,
        name: file,
        packageId: crate.id,
        file,
        text: readText(join(directory, file)),
      });
    }
  }
  for (const license of report.licenses) {
    const users = license.used_by
      .map(({ crate }) => crate)
      .filter((crate) => crate.source)
      .map((crate) => `${crate.name}@${crate.version}`);
    if (users.length === 0) continue;
    licenseTextBlock(cargoLicenseBlocks, {
      ecosystem: 'Cargo',
      id: license.id,
      name: license.name,
      packageId: users.sort().join(', '),
      file: license.source_path ? basename(license.source_path) : 'SPDX text for a declared license expression',
      text: license.text,
    });
  }
  return crates.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
}

function renderInventory(packages, crates, noticeBlocks, cargoAboutVersion, licenseTexts) {
  const fingerprints = inputPaths
    .map((path) => `source-sha256: ${path}=${sourceFingerprint(path)}`)
    .join('\n');
  const metadata = [
    '<!-- code-atlas-third-party-inventory: 1',
    `target: ${target}`,
    `npm-package-count: ${packages.length}`,
    `cargo-crate-count: ${crates.length}`,
    `cargo-about: ${cargoAboutVersion.trim()}`,
    `license-texts-sha256: ${sha256(licenseTexts)}`,
    fingerprints,
    '-->',
  ].join('\n');
  const packageRows = packages.map((pkg) =>
    `| ${tableCell(pkg.id)} | ${tableCell(pkg.expression)} | ${tableCell(pkg.author)} | ${tableCell(pkg.repository)} | ${tableCell(pkg.files.join(', '))} |`,
  );
  const crateRows = crates.map((crate) =>
    `| ${tableCell(crate.id)} | ${tableCell(crate.expression)} | ${tableCell(crate.authors)} | ${tableCell(crate.repository)} |`,
  );
  const contents = [
    '# Third-party notices',
    '',
    'This generated inventory accompanies the Code Atlas macOS Apple Silicon app. It lists every transitive npm package in the locked production install and every external Cargo crate in the locked `aarch64-apple-darwin` build graph. Each package keeps its declared license expression. The companion [THIRD_PARTY_LICENSE_TEXTS.txt](THIRD_PARTY_LICENSE_TEXTS.txt) contains the full dependency and bundled-asset license texts and notices, grouped by exact normalized text with copyright and attribution lines retained.',
    '',
    'This is a software notice inventory, not legal advice or a compatibility certification. A package license remains separate from Code Atlas source licensing. The root MIT license does not relicense any dependency.',
    '',
    '## Inventory provenance',
    '',
    `- npm packages: ${packages.length}, from \`npm ls --omit=dev --all\` and the checked-in package lock.`,
    `- Cargo crates: ${crates.length}, target \`${target}\`, generated by ${cargoAboutVersion.trim()} from the checked-in Cargo lock. The Cargo set includes build and procedural-macro dependencies used to produce the app.`,
    '- A dependency lock or manifest change invalidates this file. The macOS packaging preflight verifies its recorded source hashes.',
    '- `react-remove-scroll-bar@2.3.8` declares MIT and lists Anton Korzunov as author in its npm manifest, but its published package omits a license file. The upstream repository license text is preserved in the companion text file; it was obtained from the upstream default branch because the package tarball omits it.',
    '',
    '## Application assets and data boundaries',
    '',
    '- The packaged app contains the Code Atlas-authored icon (`src-tauri/icons/code-atlas.svg`) and its generated platform variants, the application executable, and the resources listed in `src-tauri/tauri.conf.json`. No repository capture, saved answer, narration audio, acceptance fixture, benchmark corpus, or design preview is packaged.',
    '- The app bundles the upstream MIT and Apache-2.0 license texts for the pinned `uv` installer. The `uv` binary is downloaded and checksum-verified only when the user starts Pocket TTS setup; it is not embedded in the app.',
    '- Python, the 49 locked Pocket TTS packages, model weights, and Alba voice assets are downloaded to app data during explicit voice setup; they are not embedded in the app. Their separate upstream terms and Alba CC BY 4.0 attribution are described in [Pocket TTS runtime](docs/research/pocket-tts-runtime.md) and [provider disclosures](docs/PROVIDER_DISCLOSURES.md).',
    '- Source-only `docs/acceptance/`, `benchmarks/`, `docs/design/`, and `docs/research/snapshots/` are not included in the app. Their provenance remains separate from the Code Atlas source MIT grant.',
    '',
    '## npm production dependency inventory',
    '',
    '| Package | Declared license | Author in package metadata | Repository | Included license/notice files |',
    '| --- | --- | --- | --- | --- |',
    ...packageRows,
    '',
    '## Cargo build dependency inventory',
    '',
    '| Crate | Declared license | Authors in Cargo metadata | Repository |',
    '| --- | --- | --- | --- |',
    ...crateRows,
    '',
    '## Project license and local data',
    '',
    'Code Atlas-authored source is covered by the root [MIT License](LICENSE), subject to the exclusions recorded here and in the README. Captured third-party source, generated explanations/audio, and local repository data are not part of the application bundle or this dependency inventory.',
  ];
  return `${metadata}\n\n${contents.join('\n')}\n`;
}

function main() {
  const tempDirectory = mkdtempSync(join(tmpdir(), 'code-atlas-licenses-'));
  try {
    const npmLicenseBlocks = new Map();
    const cargoLicenseBlocks = new Map();
    const npmNoticeBlocks = new Map();
    const cargoNoticeBlocks = new Map();
    const bundledAssetLicenseBlocks = new Map();
    const packages = loadNpmPackages(npmLicenseBlocks, npmNoticeBlocks);
    const crates = loadCargoInventory(cargoLicenseBlocks, cargoNoticeBlocks, tempDirectory);
    for (const [name, id, path] of [
      ['uv MIT License', 'MIT', 'src-tauri/resources/third-party/uv-LICENSE-MIT.txt'],
      ['uv Apache License', 'Apache-2.0', 'src-tauri/resources/third-party/uv-LICENSE-APACHE.txt'],
    ]) {
      licenseTextBlock(bundledAssetLicenseBlocks, {
        ecosystem: 'Bundled app asset',
        id,
        name,
        packageId: 'uv 0.13.4 aarch64-apple-darwin',
        file: path,
        text: readText(join(root, path)),
      });
    }
    const cargoAboutVersion = run('cargo', ['about', '--version']);
    const noticeBlocks = new Map([...npmNoticeBlocks, ...cargoNoticeBlocks]);
    const licenseTexts = renderLicenseTexts(npmLicenseBlocks, cargoLicenseBlocks, bundledAssetLicenseBlocks, noticeBlocks);
    const output = renderInventory(packages, crates, noticeBlocks, cargoAboutVersion, licenseTexts);
    if (process.argv.includes('--check')) {
      const existing = readFileSync(outputPath, 'utf8');
      const existingLicenseTexts = readFileSync(licenseTextsOutputPath, 'utf8');
      if (existing !== output || existingLicenseTexts !== licenseTexts) {
        throw new Error('Third-party notices are stale; run npm run licenses:generate.');
      }
      process.stdout.write(`Verified notices for ${packages.length} npm packages and ${crates.length} Cargo crates.\n`);
      return;
    }
    writeFileSync(outputPath, output);
    writeFileSync(licenseTextsOutputPath, licenseTexts);
    process.stdout.write(`Wrote ${relative(root, outputPath)} for ${packages.length} npm packages and ${crates.length} Cargo crates.\n`);
  } finally {
    rmSync(tempDirectory, { recursive: true, force: true });
  }
}

main();
