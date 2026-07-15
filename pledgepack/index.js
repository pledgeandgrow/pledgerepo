// pledgepack — programmatic API
// Spawns the native binary with the given command and arguments.

import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));

const platformKey = `${platform()}-${arch()}`;
const binaryName = platform() === 'win32' ? 'pledge.exe' : 'pledge';

const candidates = [
  join(__dirname, '..', 'target', 'release', binaryName),
  join(__dirname, '..', 'target', 'debug', binaryName),
  join(__dirname, 'bin', platformKey, binaryName),
  join(__dirname, 'bin', 'platform', platformKey, binaryName),
  join(__dirname, 'bin', binaryName),
];

let binaryPath = null;
for (const candidate of candidates) {
  if (existsSync(candidate)) {
    binaryPath = candidate;
    break;
  }
}

/**
 * Run a pledgepack command programmatically.
 * @param {string[]} args - Arguments to pass to the binary (e.g., ['build'])
 * @param {object} [options] - Spawn options
 * @returns {Promise<number>} Exit code
 */
export function runPledgepack(args = [], options = {}) {
  if (!binaryPath) {
    return Promise.reject(new Error(
      'pledge binary not found. Run "npm rebuild pledgepack" or build from source.'
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
  return binaryPath;
}

export default { runPledgepack, getBinaryPath };

