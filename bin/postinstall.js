// Pledgepack postinstall script — downloads the native binary for the current platform.
// In development, the binary is already built via cargo and this is a no-op.

import { createWriteStream, existsSync, mkdirSync, renameSync, chmodSync, rmSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform, arch } from 'node:os';
import { spawnSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Platform mapping — maps Node.js platform/arch to our release targets
const PLATFORM_MAP = {
  'linux-x64':    { target: 'x86_64-unknown-linux-gnu',   ext: '.tar.gz' },
  'linux-arm64':  { target: 'aarch64-unknown-linux-gnu',  ext: '.tar.gz' },
  'darwin-x64':   { target: 'x86_64-apple-darwin',        ext: '.tar.gz' },
  'darwin-arm64': { target: 'aarch64-apple-darwin',       ext: '.tar.gz' },
  'win32-x64':    { target: 'x86_64-pc-windows-msvc',     ext: '.zip' },
  'win32-arm64':  { target: 'aarch64-pc-windows-msvc',    ext: '.zip' },
};

const platformKey = `${platform()}-${arch()}`;
const mapped = PLATFORM_MAP[platformKey];
const binaryName = platform() === 'win32' ? 'pledge.exe' : 'pledge';

// Check if binary already exists in target/ (dev mode — already built)
const localRelease = join(__dirname, '..', 'target', 'release', binaryName);
const localDebug = join(__dirname, '..', 'target', 'debug', binaryName);

if (existsSync(localRelease) || existsSync(localDebug)) {
  // Dev mode — binary already built, nothing to do
  process.exit(0);
}

// Check if binary already downloaded (e.g., npm package with staged binaries)
const stagedBinary = join(__dirname, platformKey, binaryName);
if (existsSync(stagedBinary)) {
  process.exit(0);
}

// Check if binary is directly in bin/ (already installed)
const directBinary = join(__dirname, binaryName);
if (existsSync(directBinary)) {
  process.exit(0);
}

if (!mapped) {
  console.warn('');
  console.warn('  \x1b[33mpledge\x1b[0m: No prebuilt binary for ' + platformKey);
  console.warn('  Build from source: cargo build --release');
  console.warn('');
  process.exit(0);
}

// Download the prebuilt binary from GitHub Releases
const GITHUB_REPO = 'pledgeandgrow/pledgerepo';
const packageName = `pledge-${mapped.target}${mapped.ext}`;
const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/latest/download/${packageName}`;

const destDir = join(__dirname, platformKey);
const archivePath = join(destDir, packageName);

async function downloadAndExtract() {
  console.log('');
  console.log('  \x1b[36mpledge\x1b[0m: Downloading native binary...');
  console.log('  \x1b[2m  → ' + downloadUrl + '\x1b[0m');
  console.log('');

  mkdirSync(destDir, { recursive: true });

  // Download the archive
  try {
    const response = await fetch(downloadUrl, { redirect: 'follow' });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} — ${response.statusText}`);
    }
    const buffer = Buffer.from(await response.arrayBuffer());
    writeFileSync(archivePath, buffer);
  } catch (err) {
    console.warn('');
    console.warn('  \x1b[33mpledge\x1b[0m: Failed to download binary: ' + err.message);
    console.warn('  Falling back to source build...');
    console.warn('');
    return tryBuildFromSource();
  }

  // Extract the archive
  if (mapped.ext === '.zip') {
    // Windows: use PowerShell to extract
    const result = spawnSync('powershell', [
      '-NoProfile', '-Command',
      `Expand-Archive -Path "${archivePath}" -DestinationPath "${destDir}" -Force`
    ], { stdio: 'inherit' });
    if (result.status !== 0) {
      console.warn('  \x1b[31mpledge\x1b[0m: Failed to extract zip');
      return tryBuildFromSource();
    }
  } else {
    // Unix: use tar
    const result = spawnSync('tar', ['xzf', archivePath, '-C', destDir], { stdio: 'inherit' });
    if (result.status !== 0) {
      console.warn('  \x1b[31mpledge\x1b[0m: Failed to extract tar.gz');
      return tryBuildFromSource();
    }
  }

  // Clean up the archive
  try { rmSync(archivePath, { force: true }); } catch {}

  // Make binary executable (Unix only)
  if (platform() !== 'win32') {
    const binaryPath = join(destDir, binaryName);
    if (existsSync(binaryPath)) {
      chmodSync(binaryPath, 0o755);
    }
  }

  // Verify the binary exists
  const finalBinary = join(destDir, binaryName);
  if (!existsSync(finalBinary)) {
    console.warn('  \x1b[31mpledge\x1b[0m: Binary not found after extraction');
    return tryBuildFromSource();
  }

  console.log('  \x1b[32m✓\x1b[0m Binary installed: ' + finalBinary);
  console.log('');
}

function tryBuildFromSource() {
  console.warn('  \x1b[33mpledge\x1b[0m: Attempting to build from source...');
  console.warn('  Make sure Rust is installed: https://rustup.rs');
  console.warn('');

  // Check if cargo is available
  const cargoCheck = spawnSync('cargo', ['--version'], { stdio: 'pipe' });
  if (cargoCheck.status !== 0) {
    console.warn('  \x1b[31mpledge\x1b[0m: Rust/Cargo is not installed.');
    console.warn('  Install Rust: https://rustup.rs');
    console.warn('');
    return;
  }

  // Check if Cargo.toml exists in the parent directory (source checkout)
  const sourceDir = join(__dirname, '..');
  if (!existsSync(join(sourceDir, 'Cargo.toml'))) {
    console.warn('  \x1b[31mpledge\x1b[0m: Source not available in this installation.');
    console.warn('  To build from source:');
    console.warn('    git clone https://github.com/pledgeandgrow/pledgerepo');
    console.warn('    cd pledgerepo && cargo build --release');
    console.warn('');
    return;
  }

  const result = spawnSync('cargo', ['build', '--release'], {
    cwd: sourceDir,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    console.warn('  \x1b[31mpledge\x1b[0m: Failed to build from source.');
    console.warn('  Make sure Rust is installed: https://rustup.rs');
    console.warn('');
  }
}

downloadAndExtract().catch((err) => {
  console.warn('  \x1b[31mpledge\x1b[0m: ' + err.message);
  tryBuildFromSource();
});
