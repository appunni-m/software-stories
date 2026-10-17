import { spawnSync } from 'node:child_process';

/**
 * Validate the embedded Mach-O integrity signature, including linker ad-hoc
 * signatures. Resource sealing is checked separately for signed releases; a
 * local --no-sign bundle has no resource seal. This is not Developer ID,
 * notarization or bundle-resource proof.
 */
export function verifyMacExecutableIntegrity(executablePath) {
  if (process.platform !== 'darwin') {
    throw new Error('macOS executable integrity checks require macOS.');
  }
  const check = spawnSync('/usr/bin/codesign', ['--verify', '--strict', '--ignore-resources', executablePath], {
    encoding: 'utf8', timeout: 20_000, maxBuffer: 64_000,
  });
  if (check.error || check.status !== 0) {
    throw new Error('macOS executable integrity validation failed. Do not install or distribute this artifact.');
  }
}
