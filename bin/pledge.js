#!/usr/bin/env node
// Pledgepack CLI launcher — resolves the native binary for the current platform
// and forwards all arguments to it.

import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync } from 'node:fs';
import { platform, arch } from 'node:os';
import { createRequire } from 'node:module';

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const plat = platform();
const ar = arch();
const binaryName = plat === 'win32' ? 'pledge.exe' : 'pledge';

// Platform-specific npm packages (like esbuild/swc pattern)
const platformPackages = {
  'darwin': {
    'arm64': '@pledgejs/pledgepack-darwin-arm64',
    'x64': '@pledgejs/pledgepack-darwin-x64',
  },
  'linux': {
    'x64': '@pledgejs/pledgepack-linux-x64-gnu',
  },
  'win32': {
    'x64': '@pledgejs/pledgepack-win32-x64-msvc',
  },
};

// Possible binary locations:
// 1. Platform-specific npm optional dependency (production)
// 2. Already built in target/release/ (dev mode)
// 3. Already built in target/debug/ (dev mode)
// 4. Downloaded to bin/{platform-key}/ (npm install with GitHub Releases)
// 5. Staged in bin/platform/ (npm publish with CI-staged binaries)
// 6. Direct binary in bin/ (local install)
const candidates = [];

// 1. Try platform-specific package
const packageName = platformPackages[plat]?.[ar];
if (packageName) {
  try {
    const pkgPath = require.resolve(packageName);
    const pkgDir = dirname(pkgPath);
    candidates.push(
      join(pkgDir, 'bin', 'pledgepack.exe'),
      join(pkgDir, 'bin', 'pledgepack'),
      join(pkgDir, 'bin', binaryName),
    );
  } catch {
    // Platform package not installed — skip
  }
}

// 2-6. Local/dev paths
candidates.push(
  join(__dirname, '..', 'target', 'release', binaryName),
  join(__dirname, '..', 'target', 'debug', binaryName),
  join(__dirname, `${plat}-${ar}`, binaryName),
  join(__dirname, 'platform', `${plat}-${ar}`, binaryName),
  join(__dirname, binaryName),
);

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
