#!/usr/bin/env node
// Pledgepack CLI launcher — resolves the native binary for the current platform
// and forwards all arguments to it.

import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const plat = platform();
const ar = arch();
const binaryName = plat === 'win32' ? 'pledge.exe' : 'pledge';
const platformKey = `${plat}-${ar}`;

// Check @pledgepack/* scoped packages first (npm install path)
const platformPackages = {
  'darwin': { 'arm64': '@pledgepack/darwin-arm64', 'x64': '@pledgepack/darwin-x64' },
  'linux': { 'x64': '@pledgepack/linux-x64-gnu', 'arm64': '@pledgepack/linux-arm64-gnu' },
  'win32': { 'x64': '@pledgepack/win32-x64-msvc', 'arm64': '@pledgepack/win32-arm64-msvc' },
};

let binaryPath = null;

const packageName = platformPackages[plat]?.[ar];
if (packageName) {
  try {
    const pkgPath = require.resolve(packageName);
    const pkgDir = dirname(pkgPath);
    const candidates = plat === 'win32'
      ? [join(pkgDir, 'bin', 'pledge.exe'), join(pkgDir, 'bin', 'pledge')]
      : [join(pkgDir, 'bin', 'pledge'), join(pkgDir, 'bin', 'pledge.exe')];
    for (const candidate of candidates) {
      if (existsSync(candidate)) {
        binaryPath = candidate;
        break;
      }
    }
  } catch {
    // Platform package not installed
  }
}

// Fallback to local binary locations
if (!binaryPath) {
  const candidates = [
    join(__dirname, '..', 'target', 'release', binaryName),
    join(__dirname, '..', 'target', 'debug', binaryName),
    join(__dirname, platformKey, binaryName),
    join(__dirname, 'platform', platformKey, binaryName),
    join(__dirname, binaryName),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      binaryPath = candidate;
      break;
    }
  }
}

if (!binaryPath) {
  console.error('');
  console.error('  \x1b[31mpledge\x1b[0m binary not found.');
  console.error('');
  console.error('  This can happen if:');
  console.error('    1. You installed the package but the postinstall script failed');
  console.error('    2. Your platform is not yet supported: ' + platformKey);
  console.error('');
  console.error('  To build from source:');
  console.error('    git clone https://github.com/pledgeandgrow/pledgerepo');
  console.error('    cd pledgerepo && cargo build --release');
  console.error('');
  console.error('  Or re-run: npm rebuild pledgepack');
  console.error('');
  process.exit(1);
}

// Forward all arguments to the native binary
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  cwd: process.cwd(),
});

child.on('exit', (code) => {
  process.exit(code ?? 1);
});

child.on('error', (err) => {
  console.error('');
  console.error('  \x1b[31mpledge\x1b[0m: Failed to launch binary: ' + err.message);
  console.error('');
  process.exit(1);
});
