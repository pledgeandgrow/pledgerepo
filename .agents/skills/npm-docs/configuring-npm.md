# Configuring npm

## .npmrc

The `.npmrc` file configures npm behavior. It can exist at four levels:

| Location | Scope |
|----------|-------|
| `/path/to/project/.npmrc` | Project-level (highest priority) |
| `~/.npmrc` | User-level |
| `$PREFIX/etc/npmrc` | Global-level |
| `/path/to/npm/install/npmrc` | Built-in (lowest priority) |

### Common .npmrc Settings

```ini
# Registry
registry=https://registry.npmjs.org/

# Scoped registry
@my-org:registry=https://npm.pkg.github.com

# Authentication
//registry.npmjs.org/:_authToken=${NPM_TOKEN}
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}

# Default scope
scope=@my-org

# Install location
prefix=${HOME}/.npm-global

# Strict SSL
strict-ssl=true

# Proxy
proxy=http://proxy.example.com:8080
https-proxy=http://proxy.example.com:8080

# Ignore scripts
ignore-scripts=true

# Foreground scripts
foreground-scripts=true

# Save exact versions
save-exact=true

# Omit optional dependencies
omit=optional

# Progress bar
progress=false

# Log level
loglevel=warn

# Cache
cache=${HOME}/.npm/_cacache

# Node version
node-version=20

# Engine strict
engine-strict=true

# Fund message
fund=false

# Audit
audit=true

# Before/after scripts
enable-pre-post-scripts=true
```

### Environment Variables

npm respects these environment variables:

| Variable | Description |
|----------|-------------|
| `NPM_TOKEN` | Auth token |
| `NPM_CONFIG_REGISTRY` | Registry URL |
| `NPM_CONFIG_PREFIX` | Global install prefix |
| `NPM_CONFIG_CACHE` | Cache directory |
| `NPM_CONFIG_LOGLEVEL` | Log level |
| `NODE_ENV` | Environment (affects devDependencies) |
| `npm_config_<key>` | Any config key as env var |

## package.json

See `package-json.md` for full `package.json` documentation.

Key configuration fields:

```json
{
  "name": "my-package",
  "version": "1.0.0",
  "type": "module",
  "engines": {
    "node": ">=18.0.0",
    "npm": ">=9.0.0"
  },
  "publishConfig": {
    "registry": "https://registry.npmjs.org",
    "access": "public"
  },
  "scripts": {
    "start": "node index.js",
    "test": "jest"
  }
}
```

## package-lock.json

The `package-lock.json` file locks dependency versions for reproducible installs.

### Purpose

- Records exact versions installed
- Ensures consistent installs across environments
- Enables `npm ci` for fast, reproducible CI installs
- Contains integrity hashes for security

### Structure

```json
{
  "name": "my-package",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.18.0"
      }
    },
    "node_modules/express": {
      "version": "4.18.2",
      "resolved": "https://registry.npmjs.org/express/-/express-4.18.2.tgz",
      "integrity": "sha512-..."
    }
  },
  "dependencies": {
    "express": {
      "version": "4.18.2",
      "resolved": "...",
      "integrity": "..."
    }
  }
}
```

### Lockfile Versions

| Version | npm Version | Description |
|---------|-------------|-------------|
| 1 | npm v5/v6 | Legacy format |
| 2 | npm v7+ | Includes `packages` section |
| 3 | npm v7+ | Simplified, `packages` only (recommended) |

### Best Practices

- **Commit `package-lock.json`** to version control
- **Use `npm ci`** in CI/CD for reproducible installs
- **Don't manually edit** — let npm manage it
- **Regenerate** when changing dependencies: `rm package-lock.json && npm install`

## Folders

npm uses several standard directories:

### Global Install Location

```bash
# Check global prefix
npm config get prefix

# Check global root
npm root -g
```

Default locations:
- **Unix**: `/usr/local/lib/node_modules` or `~/.npm-global/lib/node_modules`
- **Windows**: `C:\Users\<user>\AppData\Roaming\npm`
- **nvm**: `~/.nvm/versions/node/<version>/lib/node_modules`

### Local Install Location

```
project/
├── node_modules/      # Installed packages
├── package.json
├── package-lock.json
└── .npmrc
```

### Cache Location

```bash
# Check cache location
npm config get cache

# Default: ~/.npm/_cacache
```

### npm Directories

| Directory | Purpose |
|-----------|---------|
| `node_modules/` | Installed packages |
| `~/.npm/_cacache/` | Package cache |
| `~/.npm/_logs/` | Debug logs |
| `$PREFIX/lib/node_modules/` | Global packages |
| `$PREFIX/bin/` | Global executables |

## Install

### How npm Installs Packages

1. **Resolve** — Read `package.json` and determine dependencies
2. **Fetch** — Download tarballs from registry
3. **Extract** — Extract tarballs to `node_modules`
4. **Link** — Create symlinks for bin files
5. **Build** — Run install scripts (if not ignored)
6. **Validate** — Check `engines` and `os` requirements

### Install Flags

| Flag | Description |
|------|-------------|
| `--save` | Save to dependencies (default) |
| `--save-dev` | Save to devDependencies |
| `--save-optional` | Save to optionalDependencies |
| `--save-exact` | Save exact version |
| `--save-peer` | Save to peerDependencies |
| `--no-save` | Don't save to package.json |
| `--global` / `-g` | Install globally |
| `--omit=dev` | Skip devDependencies |
| `--omit=optional` | Skip optionalDependencies |
| `--omit=peer` | Skip peerDependencies |
| `--legacy-peer-deps` | Ignore peer dependency conflicts |
| `--force` | Force reinstall |
| `--ignore-scripts` | Skip lifecycle scripts |
| `--no-audit` | Skip audit |
| `--no-fund` | Skip fund message |
| `--dry-run` | Show what would be installed |
| `--install-strategy=nested` | Use nested install (legacy) |
| `--install-strategy=hoisted` | Use hoisted install (default) |

## .npm-extension

The `.npm-extension` file allows extending npm CLI with custom commands.

### Creating an Extension

```json
// my-extension/package.json
{
  "name": "my-npm-extension",
  "version": "1.0.0",
  "npm": {
    "commands": {
      "my-command": "./commands/my-command.js"
    }
  }
}
```

### Using Extensions

Install globally:

```bash
npm install -g my-npm-extension
```

Then use:

```bash
npm my-command
```
