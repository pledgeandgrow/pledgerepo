// pledgepack — programmatic API
// Spawns the native binary with the given command and arguments.

import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

/**
 * Resolves the native pledgepack binary for the current platform.
 * Searches platform-specific packages first, then local fallbacks.
 * @returns {string|null}
 */
export function resolveBinary() {
  const plat = platform();
  const ar = arch();

  const platformPackages = {
    'darwin': {
      'arm64': '@pledgepack/darwin-arm64',
      'x64': '@pledgepack/darwin-x64',
    },
    'linux': {
      'x64': '@pledgepack/linux-x64-gnu',
      'arm64': '@pledgepack/linux-arm64-gnu',
    },
    'win32': {
      'x64': '@pledgepack/win32-x64-msvc',
      'arm64': '@pledgepack/win32-arm64-msvc',
    },
  };

  const platformEntry = platformPackages[plat];
  const packageName = platformEntry?.[ar];

  if (packageName) {
    try {
      const pkgPath = require.resolve(packageName);
      const pkgDir = dirname(pkgPath);
      const candidates = plat === 'win32'
        ? [join(pkgDir, 'bin', 'pledge.exe'), join(pkgDir, 'bin', 'pledge')]
        : [join(pkgDir, 'bin', 'pledge'), join(pkgDir, 'bin', 'pledge.exe')];
      for (const candidate of candidates) {
        if (existsSync(candidate)) return candidate;
      }
    } catch {
      // Platform package not installed
    }
  }

  // Fallback to local binary locations
  const binaryName = plat === 'win32' ? 'pledge.exe' : 'pledge';
  const localCandidates = [
    join(__dirname, '..', 'target', 'release', binaryName),
    join(__dirname, '..', 'target', 'debug', binaryName),
    join(__dirname, `${plat}-${ar}`, binaryName),
    join(__dirname, 'platform', `${plat}-${ar}`, binaryName),
    join(__dirname, binaryName),
  ];

  for (const candidate of localCandidates) {
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

