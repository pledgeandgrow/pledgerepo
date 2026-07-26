# Installing Packages

## Searching for and Choosing Packages to Download

### Using the CLI

```bash
# Search the registry
npm search <keyword>

# View package details
npm view <package-name>

# View specific fields
npm view <package-name> versions
npm view <package-name> dependencies
npm view <package-name> dist.tarball
```

### Using the Website

Browse packages at [npmjs.com](https://www.npmjs.com). Each package page shows:

- Weekly downloads
- GitHub stars
- Last publish date
- Dependencies
- License
- README

### Choosing Packages

Consider:
- **Maintenance** — Recently updated, active maintainer
- **Popularity** — High download count
- **Dependencies** — Fewer is better
- **License** — Compatible with your project
- **Security** — No known vulnerabilities (`npm audit`)
- **Tests** — Has test coverage

## Downloading and Installing Packages Locally

### Install a Package

```bash
# Install latest version
npm install <package-name>

# Install specific version
npm install <package-name>@<version>

# Install a version range
npm install <package-name>@^1.0.0

# Save as dependency (default in npm 5+)
npm install <package-name>

# Save as dev dependency
npm install --save-dev <package-name>

# Save as optional dependency
npm install --save-optional <package-name>

# Install exact version (no ^ or ~)
npm install --save-exact <package-name>
# or
npm install -E <package-name>
```

### Install from Different Sources

```bash
# From git
npm install git+https://github.com/user/repo.git

# From GitHub
npm install github:user/repo

# From a tarball
npm install ./package.tgz

# From a URL
npm install https://example.com/package.tgz

# From a local folder
npm install ./local-package
```

### Install All Dependencies

```bash
npm install
```

This reads `package.json` and installs all dependencies.

## Downloading and Installing Packages Globally

```bash
npm install -g <package-name>
```

Global packages are installed in a system-wide location and their binaries are added to your PATH.

### Common Global Packages

- `typescript`
- `eslint`
- `prettier`
- `nodemon`
- `npm` itself

### List Global Packages

```bash
npm list -g
npm list -g --depth=0
```

### Update Global Packages

```bash
npm update -g <package-name>
npm update -g
```

### Uninstall Global Packages

```bash
npm uninstall -g <package-name>
```

## Resolving EACCES Permissions Errors When Installing Packages Globally

### Problem

```
npm ERR! Error: EACCES: permission denied, access '/usr/local/lib/node_modules'
```

### Solution 1: Use a Version Manager (Recommended)

Use `nvm`, `fnm`, or `nvm-windows` to avoid permission issues entirely:

```bash
nvm install --lts
nvm use --lts
```

### Solution 2: Change npm's Default Directory

```bash
mkdir ~/.npm-global
npm config set prefix '~/.npm-global'
# Add to ~/.bashrc or ~/.zshrc:
export PATH=~/.npm-global/bin:$PATH
source ~/.bashrc
```

### Solution 3: Use npx

Instead of installing globally, use `npx` to run packages:

```bash
npx <package-name>
```

### Solution 4: Fix Permissions (macOS/Linux)

```bash
sudo chown -R $(whoami) $(npm config get prefix)/{lib/node_modules,bin,share}
```

## Updating Packages Downloaded from the Registry

### Update All Packages

```bash
npm update
```

### Update a Specific Package

```bash
npm update <package-name>
```

### Check for Outdated Packages

```bash
npm outdated
```

Output columns:
- **Current** — Installed version
- **Wanted** — Maximum version satisfying `package.json` semver
- **Latest** — Latest version in the registry

### Update to Latest (Ignoring semver)

```bash
npm install <package-name>@latest
```

## Using npm Packages in Your Projects

### CommonJS

```js
const express = require('express')
const app = express()
```

### ESM

```js
import express from 'express'
const app = express()
```

### Conditional Imports

```js
// Dynamic import
const module = await import('some-package')

// Conditional
if (process.env.NODE_ENV === 'production') {
  const prodModule = await import('prod-only-package')
}
```

## Using Deprecated Packages

When a package is deprecated:
- npm shows a warning during install
- The package still works but may have security issues
- Consider migrating to an alternative

```bash
npm install <deprecated-package>
# npm WARN deprecated <package>@<version>: <deprecation message>
```

## Uninstalling Packages and Dependencies

```bash
# Remove from node_modules and package.json
npm uninstall <package-name>

# Remove a dev dependency
npm uninstall --save-dev <package-name>

# Remove globally
npm uninstall -g <package-name>

# Remove without updating package.json
npm uninstall --no-save <package-name>
```

### Remove Unused Dependencies

```bash
# Install depcheck
npx depcheck

# Then manually remove unused packages
npm uninstall <unused-package>
```
