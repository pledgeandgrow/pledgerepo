#!/usr/bin/env node
// Pledgepack CLI launcher — resolves the native binary for the current platform
// and forwards all arguments to it.
//
// Resolution order:
//   1. Local cargo build (target/release or target/debug — dev mode)
//   2. Postinstall download location (bin/{platform}/{binary})
//   3. Direct binary in bin/ (legacy)

import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));

const plat = platform();
const ar = arch();
const binaryName = plat === 'win32' ? 'pledge.exe' : 'pledge';
const platformKey = `${plat}-${ar}`;

// Resolve binary: local build → postinstall download → direct
let binaryPath = null;

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

if (!binaryPath) {
  console.error('');
  console.error('  \x1b[31mpledge\x1b[0m binary not found.');
  console.error('');
  console.error('  Platform: ' + platformKey);
  console.error('');
  console.error('  This can happen if:');
  console.error('    1. The postinstall script failed to download the binary');
  console.error('    2. Your platform is not yet supported');
  console.error('    3. You installed with --ignore-scripts');
  console.error('');
  console.error('  To fix:');
  console.error('    npm rebuild pledgepack');
  console.error('');
  console.error('  Or build from source:');
  console.error('    git clone https://github.com/pledgeandgrow/pledgerepo');
  console.error('    cd pledgerepo && cargo build --release');
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
