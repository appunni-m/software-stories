import { readFile, writeFile, access } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyMacExecutableIntegrity } from './macos-executable-integrity.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const tauriDir = join(root, 'src-tauri');
const config = JSON.parse(await readFile(join(tauriDir, 'tauri.conf.json'), 'utf8'));
const outputPath = join(tauriDir, 'macos-release-metadata.json');

function rustConstant(source, name) {
  const match = source.match(new RegExp(`(?:pub\\s+)?const\\s+${name}\\s*:\\s*(?:u32|&str)\\s*=\\s*(?:"([^"]*)"|(\\d+))\\s*;`));
  if (!match) throw new Error(`Could not find Rust constant ${name}`);
  return match[1] ?? Number(match[2]);
}

async function source(path) {
  return readFile(join(tauriDir, 'src', path), 'utf8');
}

async function metadata() {
  const [chapters, index, recovery, notices, conversations] = await Promise.all([
    source('chapters.rs'), source('index.rs'), source('recovery.rs'), source('app_notices.rs'), source('workspace_conversation.rs'),
  ]);
  return {
    productName: config.productName,
    version: config.version,
    identifier: config.identifier,
    minimumSystemVersion: config.bundle?.macOS?.minimumSystemVersion ?? null,
    dataFormats: {
      chapterSchema: rustConstant(chapters, 'CHAPTER_SCHEMA_VERSION'),
      visualSchema: rustConstant(chapters, 'VISUAL_SCHEMA_VERSION'),
      visualRendererContract: rustConstant(chapters, 'VISUAL_RENDERER_CONTRACT_VERSION'),
      indexSchema: rustConstant(index, 'INDEX_SCHEMA_VERSION'),
      indexer: rustConstant(index, 'INDEXER_VERSION'),
      interruptedOperationJournal: rustConstant(recovery, 'JOURNAL_SCHEMA_VERSION'),
      appNoticeHistory: rustConstant(notices, 'NOTICE_HISTORY_SCHEMA_VERSION'),
      workspaceConversation: rustConstant(conversations, 'CONVERSATION_SCHEMA_VERSION'),
    },
  };
}

const manifest = `${JSON.stringify(await metadata(), null, 2)}\n`;

if (process.argv[2] === 'write') {
  let existing;
  try { existing = await readFile(outputPath, 'utf8'); }
  catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  if (existing === manifest) console.log(`Metadata is current: ${outputPath}`);
  else {
    await writeFile(outputPath, manifest);
    console.log(`Wrote ${outputPath}`);
  }
} else if (process.argv[2] === 'verify') {
  const appPath = join(tauriDir, 'target', 'release', 'bundle', 'macos', `${config.productName}.app`);
  const infoPlist = join(appPath, 'Contents', 'Info.plist');
  const bundledManifest = join(appPath, 'Contents', 'Resources', 'macos-release-metadata.json');
  const dmgDir = join(tauriDir, 'target', 'release', 'bundle', 'dmg');
  const plistValue = (key) => {
    const result = spawnSync('/usr/libexec/PlistBuddy', ['-c', `Print :${key}`, infoPlist], { encoding: 'utf8' });
    if (result.status !== 0) throw new Error(`Could not read ${key} from ${infoPlist}: ${result.stderr.trim()}`);
    return result.stdout.trim();
  };
  const executableName = plistValue('CFBundleExecutable');
  if (!/^[A-Za-z0-9._-]+$/.test(executableName)) throw new Error('Invalid bundled executable name.');
  verifyMacExecutableIntegrity(join(appPath, 'Contents', 'MacOS', executableName));
  const bundledContents = await readFile(bundledManifest, 'utf8');
  if (bundledContents !== manifest) throw new Error('Bundled schema metadata does not match the current Rust schema constants.');
  for (const [bundleName, sourceName] of [
    ['LICENSE', 'LICENSE'],
    ['THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_NOTICES.md'],
  ]) {
    const bundled = await readFile(join(appPath, 'Contents', 'Resources', bundleName));
    const source = await readFile(join(root, sourceName));
    if (!bundled.equals(source)) throw new Error(`Bundled ${bundleName} does not match ${sourceName}.`);
  }
  if (config.bundle?.license !== 'MIT') throw new Error('Tauri bundle license must match the selected MIT license.');
  for (const [key, expected] of [
    ['CFBundleIdentifier', config.identifier],
    ['CFBundleName', config.productName],
    ['CFBundleShortVersionString', config.version],
    ['CFBundleVersion', config.version],
  ]) {
    const actual = plistValue(key);
    if (actual !== expected) throw new Error(`${key} mismatch: expected ${expected}, found ${actual}`);
  }
  const expectedMinimumVersion = config.bundle?.macOS?.minimumSystemVersion ?? null;
  const minimumVersion = spawnSync('/usr/libexec/PlistBuddy', ['-c', 'Print :LSMinimumSystemVersion', infoPlist], { encoding: 'utf8' });
  if (expectedMinimumVersion === null) {
    if (minimumVersion.status === 0) throw new Error(`Unexpected LSMinimumSystemVersion in local bundle: ${minimumVersion.stdout.trim()}`);
  } else {
    if (minimumVersion.status !== 0) throw new Error(`Expected LSMinimumSystemVersion ${expectedMinimumVersion}, but it is absent.`);
    const actualMinimumVersion = minimumVersion.stdout.trim();
    if (actualMinimumVersion !== expectedMinimumVersion) throw new Error(`LSMinimumSystemVersion mismatch: expected ${expectedMinimumVersion}, found ${actualMinimumVersion}`);
  }
  await access(appPath);
  const { readdir } = await import('node:fs/promises');
  const diskImagePrefix = `${config.productName}_${config.version}_`;
  const diskImages = (await readdir(dmgDir)).filter((name) => name.startsWith(diskImagePrefix) && name.endsWith('.dmg'));
  if (diskImages.length !== 1) throw new Error(`Expected exactly one DMG in ${dmgDir}, found ${diskImages.length}`);
  const dmgPath = join(dmgDir, diskImages[0]);
  const checksum = spawnSync('hdiutil', ['verify', dmgPath], { encoding: 'utf8' });
  if (checksum.error || checksum.status !== 0) {
    const details = [checksum.error?.message, checksum.stderr?.trim(), checksum.stdout?.trim()].filter(Boolean).join('\n');
    throw new Error(`DMG checksum verification failed${details ? `: ${details}` : '.'}`);
  }
  console.log(`Verified DMG checksum: ${diskImages[0]}`);
  console.log(JSON.stringify({
    app: appPath,
    dmg: dmgPath,
    identifier: plistValue('CFBundleIdentifier'),
    version: plistValue('CFBundleShortVersionString'),
    minimumSystemVersion: expectedMinimumVersion,
    dataFormats: JSON.parse(bundledContents).dataFormats,
  }, null, 2));
} else {
  throw new Error('Usage: node scripts/macos-bundle-metadata.mjs <write|verify>');
}
