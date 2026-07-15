#!/usr/bin/env node
// Pledgepack CLI launcher — downloads the native binary for the current platform
// and forwards all arguments to it.

import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Map platform + arch to the binary name
const platformKey = `${platform()}-${arch()}`;
const binaryName = platform() === 'win32' ? 'pledge.exe' : 'pledge';

// Possible binary locations:
// 1. Already built in target/release/ (dev mode)
// 2. Already built in target/debug/ (dev mode)
// 3. Downloaded to bin/platform-key/ (npm install)
const candidates = [
  join(__dirname, '..', 'target', 'release', binaryName),
  join(__dirname, '..', 'target', 'debug', binaryName),
  join(__dirname, platformKey, binaryName),
  join(__dirname, 'pledge'), // direct symlink
];

let binaryPath = null;
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
  console.error('  This can happen if:');
  console.error('    1. You installed the package but the postinstall script failed');
  console.error('    2. Your platform is not yet supported: ' + platformKey);
  console.error('');
  console.error('  To build from source:');
  console.error('    git clone https://github.com/pledgepack/pledgepack');
  console.error('    cd pledgepack && cargo build --release');
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
