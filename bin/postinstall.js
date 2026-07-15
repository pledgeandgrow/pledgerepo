// Pledgepack postinstall script — downloads the native binary for the current platform.
// In development, the binary is already built via cargo and this is a no-op.

import { createWriteStream, existsSync, mkdirSync, renameSync, chmodSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform, arch } from 'node:os';
import { spawnSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Platform mapping
const PLATFORM_MAP = {
  'linux-x64': 'linux-x64',
  'linux-arm64': 'linux-arm64',
  'darwin-x64': 'darwin-x64',
  'darwin-arm64': 'darwin-arm64',
  'win32-x64': 'win32-x64',
  'win32-arm64': 'win32-arm64',
};

const platformKey = `${platform()}-${arch()}`;
const mappedPlatform = PLATFORM_MAP[platformKey];

// Check if binary already exists in target/ (dev mode)
const localBinary = join(__dirname, '..', 'target', 'release', platform() === 'win32' ? 'pledge.exe' : 'pledge');
const localDebugBinary = join(__dirname, '..', 'target', 'debug', platform() === 'win32' ? 'pledge.exe' : 'pledge');

if (existsSync(localBinary) || existsSync(localDebugBinary)) {
  // Dev mode — binary already built, nothing to do
  process.exit(0);
}

if (!mappedPlatform) {
  console.warn('');
  console.warn('  \x1b[33mpledge\x1b[0m: No prebuilt binary for ' + platformKey);
  console.warn('  Build from source: cargo build --release');
  console.warn('');
  process.exit(0);
}

// In a real publish scenario, we'd download from GitHub releases
// For now, just log that the user needs to build from source
console.warn('');
console.warn('  \x1b[33mpledge\x1b[0m: Prebuilt binary download not yet available.');
console.warn('  Please build from source: cargo build --release');
console.warn('');

// Attempt to build from source if cargo is available
const cargoResult = spawnSync('cargo', ['build', '--release'], {
  cwd: join(__dirname, '..'),
  stdio: 'inherit',
});

if (cargoResult.status !== 0) {
  console.warn('  \x1b[31mpledge\x1b[0m: Failed to build from source.');
  console.warn('  Make sure Rust and Zig are installed: https://rustup.rs');
  console.warn('');
}

process.exit(0);
