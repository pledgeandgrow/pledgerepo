// pledgepack — programmatic API
// Spawns the native binary with the given command and arguments.

import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Resolves the native pledgepack binary for the current platform.
 * Searches local build output, postinstall download location, then direct binary.
 * @returns {string|null}
 */
export function resolveBinary() {
  const plat = platform();
  const ar = arch();
  const binaryName = plat === 'win32' ? 'pledge.exe' : 'pledge';

  const candidates = [
    join(__dirname, '..', 'target', 'release', binaryName),
    join(__dirname, '..', 'target', 'debug', binaryName),
    join(__dirname, `${plat}-${ar}`, binaryName),
    join(__dirname, 'platform', `${plat}-${ar}`, binaryName),
    join(__dirname, binaryName),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }

  return null;
}

/**
 * Run a pledgepack command programmatically.
 * @param {string[]} args - Arguments to pass to the binary (e.g., ['build'])
 * @param {object} [options] - Spawn options
 * @returns {Promise<number>} Exit code
 */
export function runPledgepack(args = [], options = {}) {
  const binaryPath = resolveBinary();
  if (!binaryPath) {
    return Promise.reject(new Error(
      'pledgepack binary not found. Run "npm rebuild pledgepack" or build from source.'
    ));
  }

  return new Promise((resolve, reject) => {
    const child = spawn(binaryPath, args, {
      stdio: 'inherit',
      cwd: process.cwd(),
      ...options,
    });

    child.on('exit', (code) => resolve(code ?? 1));
    child.on('error', (err) => reject(err));
  });
}

/**
 * Get the path to the native binary, or null if not found.
 * @returns {string|null}
 */
export function getBinaryPath() {
  return resolveBinary();
}

export default { runPledgepack, getBinaryPath, resolveBinary, defineConfig };

/**
 * Identity helper for TypeScript config autocompletion.
 * @template T
 * @param {T} config
 * @returns {T}
 */
export function defineConfig(config) {
  return config;
}
