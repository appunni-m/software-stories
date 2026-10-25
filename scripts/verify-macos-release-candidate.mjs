import { createHash } from 'node:crypto';
import { access, readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const config = JSON.parse(await readFile(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const bundleRoot = join(root, 'src-tauri', 'target', 'release', 'bundle');
const appPath = join(bundleRoot, 'macos', `${config.productName}.app`);
const dmgDirectory = join(bundleRoot, 'dmg');

function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 120_000, maxBuffer: 2_000_000 });
  if (result.error || result.status !== 0) {
    const detail = [result.error?.message, result.stderr?.trim(), result.stdout?.trim()].filter(Boolean).join('\n');
    throw new Error(`${command} ${args.join(' ')} failed${detail ? `:\n${detail}` : '.'}`);
  }
  return `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
}

if (process.platform !== 'darwin') throw new Error('Signed macOS release verification requires macOS.');
await access(appPath);

const signature = run('/usr/bin/codesign', ['-dv', '--verbose=4', appPath]);
if (!/^Authority=Developer ID Application:/m.test(signature)) {
  throw new Error('The app is not signed with a Developer ID Application identity.');
}
if (!/^TeamIdentifier=[A-Z0-9]+$/m.test(signature)) {
  throw new Error('The app signature does not include an Apple Developer team identifier.');
}
run('/usr/bin/codesign', ['--verify', '--deep', '--strict', '--verbose=2', appPath]);
run('/usr/sbin/spctl', ['--assess', '--type', 'execute', '--verbose=4', appPath]);
run('/usr/bin/xcrun', ['stapler', 'validate', '-v', appPath]);

const prefix = `${config.productName}_${config.version}_`;
const diskImages = (await readdir(dmgDirectory)).filter((name) => name.startsWith(prefix) && name.endsWith('.dmg'));
if (diskImages.length !== 1) throw new Error(`Expected exactly one release-candidate DMG, found ${diskImages.length}.`);
const dmgPath = join(dmgDirectory, diskImages[0]);
run('/usr/bin/xcrun', ['stapler', 'validate', '-v', dmgPath]);
run('/usr/bin/hdiutil', ['verify', dmgPath]);

const digest = createHash('sha256').update(await readFile(dmgPath)).digest('hex');
const checksumPath = join(dmgDirectory, `${diskImages[0]}.sha256`);
await writeFile(checksumPath, `${digest}  ${diskImages[0]}\n`);
console.log(`Verified signed and notarized release candidate: ${appPath}`);
console.log(`Verified disk image: ${dmgPath}`);
console.log(`SHA-256 ${digest}  ${diskImages[0]}`);
