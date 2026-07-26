# Using npm

## Registry

The npm registry is the public database of JavaScript packages at `https://registry.npmjs.org`.

### Configuring the Registry

```bash
# Set default registry
npm config set registry https://registry.npmjs.org

# Use a private registry
npm config set registry https://my-private-registry.com

# Scoped registry
npm config set @my-org:registry https://npm.pkg.github.com
```

### Registry API

```bash
# Get package metadata
npm view <package>

# Get all versions
npm view <package> versions --json

# Get dist-tags
npm view <package> dist-tags --json

# Download a tarball
npm pack <package>

# Ping the registry
npm ping
```

### Using Multiple Registries

```ini
# .npmrc
registry=https://registry.npmjs.org
@my-org:registry=https://npm.pkg.github.com
@internal:registry=https://internal-registry.company.com
```

## Package Spec

A package spec defines what to install. Formats:

### By Name

```bash
npm install <name>              # Latest
npm install <name>@<version>    # Specific version
npm install <name>@<range>      # Version range
npm install <name>@<tag>        # Dist-tag (e.g., beta)
```

### By Scope

```bash
npm install @scope/name
npm install @scope/name@1.0.0
```

### By Tarball

```bash
npm install ./package.tgz
npm install https://example.com/package.tgz
```

### By Git URL

```bash
npm install git+https://github.com/user/repo.git
npm install git+ssh://git@github.com:user/repo.git
npm install github:user/repo
npm install github:user/repo#branch-name
npm install github:user/repo#commit-sha
```

### By Local Path

```bash
npm install ./local-package
npm install ../sibling-package
```

### By Workspace

```bash
npm install --workspace=<workspace-name>
# or
npm install -w <workspace-name>
```

## Config

npm configuration can be set via:

1. **Command-line flags** — Highest priority
2. **Environment variables** — `npm_config_*`
3. **Project `.npmrc`** — In project root
4. **User `.npmrc`** — In `~/.npmrc`
5. **Global `.npmrc`** — In `$PREFIX/etc/npmrc`
6. **Built-in defaults** — Lowest priority

### Key Config Values

| Key | Default | Description |
|-----|---------|-------------|
| `registry` | `https://registry.npmjs.org` | Package registry URL |
| `scope` | — | Default scope |
| `prefix` | platform-specific | Global install location |
| `cache` | `~/.npm/_cacache` | Cache directory |
| `loglevel` | `notice` | Log verbosity |
| `save` | `true` | Save to package.json |
| `save-exact` | `false` | Save exact versions |
| `omit` | — | Dependencies to skip |
| `ignore-scripts` | `false` | Skip lifecycle scripts |
| `foreground-scripts` | `false` | Show script output |
| `engine-strict` | `false` | Enforce engines field |
| `strict-ssl` | `true` | Verify SSL certificates |
| `audit` | `true` | Run audit on install |
| `fund` | `true` | Show fund message |

## Logging

### Log Levels

From most to least verbose:

1. `silly`
2. `verbose`
3. `info`
4. `notice` (default)
5. `warn`
6. `error`
7. `silent`

### Setting Log Level

```bash
npm install --loglevel=verbose
npm install --loglevel=error
npm install --silent
npm install --quiet
```

### Debug Logs

npm generates debug logs in `~/.npm/_logs/`:

```bash
# Generate a debug log
npm install --loglevel=verbose 2>&1 | tee npm-debug.log

# Find debug logs
ls ~/.npm/_logs/
```

## Scope

Scopes group packages under a namespace:

```bash
# Set default scope
npm config set scope @my-org

# Install a scoped package
npm install @my-org/package

# Create a scoped package
npm init --scope=@my-org

# Publish a scoped package
npm publish --access public
```

### Scope and Registry

Scopes can be associated with specific registries:

```ini
# .npmrc
@my-org:registry=https://npm.pkg.github.com
@internal:registry=https://internal-registry.com
```

## Scripts

### Overview

The `scripts` field in `package.json` defines command shortcuts:

```json
{
  "scripts": {
    "start": "node index.js",
    "test": "jest",
    "build": "tsc",
    "dev": "nodemon index.js"
  }
}
```

### Running Scripts

```bash
npm run <script>
npm run-script <script>
npm start        # Shortcut for npm run start
npm test         # Shortcut for npm run test
```

### Pre & Post Scripts

Scripts with `pre` or `post` prefixes run automatically:

```json
{
  "scripts": {
    "precompress": "echo 'Starting compression'",
    "compress": "gzip -r ./dist",
    "postcompress": "echo 'Compression done'"
  }
}
```

`npm run compress` runs: `precompress` → `compress` → `postcompress`

### Life Cycle Scripts

| Script | When It Runs |
|--------|-------------|
| `prepare` | Before pack/publish, on local install |
| `prepublishOnly` | Before publish only |
| `prepack` | Before tarball creation |
| `postpack` | After tarball creation |
| `preinstall` | Before dependencies are installed |
| `install` | After dependencies are installed |
| `postinstall` | After install completes |
| `dependencies` | After node_modules changes |
| `publish` | After publish |
| `postpublish` | After publish completes |
| `version` | After version bump, before commit |
| `postversion` | After version bump commit |

### Life Cycle Operation Order

**npm install:**
1. `preinstall`
2. `install`
3. `postinstall`
4. `prepublish` (deprecated)
5. `preprepare`
6. `prepare`
7. `postprepare`

**npm publish:**
1. `prepublishOnly`
2. `prepack`
3. `prepare`
4. `postpack`
5. `publish`
6. `postpublish`

**npm pack:**
1. `prepack`
2. `prepare`
3. `postpack`

### Script Environment

Scripts have access to environment variables:

| Variable | Description |
|----------|-------------|
| `npm_lifecycle_event` | Current lifecycle event name |
| `npm_package_name` | Package name |
| `npm_package_version` | Package version |
| `npm_config_*` | npm config values |
| `PATH` | Includes `node_modules/.bin` |

### Passing Arguments

```bash
npm run build -- --watch
# Passes --watch to the build script
```

### Best Practices

- Use `pre` and `post` hooks for setup/teardown
- Keep scripts simple — use separate script files for complex logic
- Use `cross-env` for cross-platform environment variables
- Document custom scripts in your README

## Workspaces

Workspaces enable monorepo support — managing multiple packages in a single repository.

### Configuration

```json
// Root package.json
{
  "name": "my-monorepo",
  "private": true,
  "workspaces": [
    "packages/*",
    "apps/*"
  ]
}
```

### Workspace Commands

```bash
# Install all workspace dependencies
npm install

# Run a command in a specific workspace
npm run build --workspace=packages/my-package
# or
npm run build -w packages/my-package

# Run in all workspaces
npm run build --workspaces
# or
npm run build -ws

# Add a dependency to a workspace
npm install lodash --workspace=packages/my-package

# Run a command in all workspaces that have it
npm run test --workspaces --if-present

# Execute in a workspace
npm exec --workspace=packages/my-package -- jest
```

### Workspace Features

- **Shared `node_modules`** — Dependencies are hoisted to root
- **Cross-workspace linking** — Workspaces can depend on each other
- **Parallel scripts** — Run scripts across all workspaces
- **Selective operations** — Target specific workspaces

### Workspace Dependencies

```json
// packages/app/package.json
{
  "name": "@my-org/app",
  "dependencies": {
    "@my-org/utils": "workspace:*"
  }
}
```

`workspace:*` resolves to the local workspace package.

### Common Workspace Patterns

```bash
# Build all packages in order
npm run build --workspaces --if-present

# Test a specific package
npm test -w packages/utils

# Add a shared dependency to all workspaces
npm install lodash -ws

# Publish a workspace package
npm publish -w packages/my-package
```

## Organizations

See `organizations.md` for full documentation on npm organizations.

## Dependency Selectors

npm query supports CSS-like selectors for dependency trees.

### Combinators

| Combinator | Meaning |
|------------|---------|
| `>` | Direct descendant/child |
| (space) | Any descendant/child |
| `~` | Sibling |

### Selectors

| Selector | Meaning |
|----------|---------|
| `*` | Universal selector |
| `#<name>` | Dependency by name (equivalent to `[name="..."]`) |
| `#<name>@<version>` | Dependency by name and version |
| `,` | Selector list delimiter |
| `.<type>` | Dependency type selector |
| `:<pseudo>` | Pseudo selector |

### Dependency Type Selectors

| Selector | Description |
|----------|-------------|
| `.prod` | In `dependencies` or child of said dependency |
| `.dev` | In `devDependencies` or child of said dependency |
| `.optional` | In `optionalDependencies` or `peerDependenciesMeta` with `optional: true` |
| `.peer` | In `peerDependencies` |
| `.workspace` | In `workspaces` section |
| `.bundled` | In `bundleDependencies` or child of said dependency |

### Pseudo Selectors

| Selector | Description |
|----------|-------------|
| `:not(<selector>)` | Negation |
| `:has(<selector>)` | Has matching descendant |
| `:is(<selector list>)` | Matches any in list |
| `:root` | Root node/dependency |
| `:scope` | Node queried against |
| `:empty` | No dependencies |
| `:private` | Package is private |
| `:link` | Linked (workspaces or `npm link`) |
| `:deduped` | Has been deduped |
| `:overridden` | Has been overridden |
| `:extraneous` | Exists but not defined as a dependency |
| `:invalid` | Version out of specified range |
| `:missing` | Not found on disk |
| `:semver(<spec>, [selector], [function])` | Semver comparison |
| `:path(<path>)` | Glob matching on path |
| `:type(<type>)` | By package type (tag, version, range, alias, registry) |
| `:outdated(<type>)` | Dependency is outdated |
| `:vuln(<selector>)` | Has a known vulnerability |

### :semver() Function

The `:semver()` pseudo selector compares fields using semver methods:

- **spec** — A semver version or range
- **selector** — An attribute selector (default `[version]`)
- **function** — One of: `satisfies`, `intersects`, `subset`, `gt`, `gte`, `gtr`, `lt`, `lte`, `ltr`, `eq`, `neq`, or `infer` (default)

When `infer` is used: if both values are versions, `eq` is used; if both are ranges, `intersects` is used; if mixed, `satisfies` is used.

### Attribute Selectors

| Selector | Description |
|----------|-------------|
| `[name=value]` | Exact match |
| `[name~=value]` | Whitespace-separated match |
| `[name*=value]` | Substring match |
| `[name^=value]` | Starts with |
| `[name$=value]` | Ends with |

### Examples

```bash
# All dependencies
npm query "*"

# Direct dependencies
npm query ":root > .dependencies > *"

# By name
npm query "[name=express]"
npm query "#express"

# By name and version
npm query "#express@4.18.0"

# By type
npm query ".devDependencies > *"
npm query ".prod"
npm query ".workspace"

# Pseudo-selectors
npm query ":not(:empty)"
npm query ":root"
npm query ":missing"
npm query ":extraneous"
npm query ":vuln(.prod)"

# Semver
npm query ":semver(^1.0.0)"
npm query "[name=lodash]:semver(>=4.0.0)"

# Outdated
npm query ":outdated(major)"

# Has a specific dependency
npm query ":has([name=react])"

# Private packages
npm query ":private"

# Combined
npm query ".prod:not(:empty):has(.dev)"
```

## Developers

### Creating a CLI Tool

```json
{
  "name": "my-cli",
  "bin": {
    "my-cli": "./bin/cli.js"
  }
}
```

```bash
#!/usr/bin/env node
console.log('Hello from my-cli')
```

Publish and install globally:

```bash
npm publish
npm install -g my-cli
my-cli
```

### Creating a Library

```json
{
  "name": "my-lib",
  "main": "dist/index.js",
  "exports": {
    ".": "./dist/index.js",
    "./utils": "./dist/utils.js"
  }
}
```

## Removal

### Uninstalling npm

npm is bundled with Node.js. To remove npm:

1. Uninstall Node.js
2. Remove npm directories:

```bash
# Unix
rm -rf ~/.npm
rm -rf /usr/local/lib/node_modules/npm

# Windows
rmdir /s %APPDATA%\npm
rmdir /s %APPDATA%\npm-cache
```

### Clean Install

To start fresh:

```bash
rm -rf node_modules package-lock.json
npm install
```

## Changelog (npm CLI v12)

### v12.0.0 (2026-07-08) — Breaking Changes

- `npm view --json` now always returns an array
- `npm sbom --sbom-format=cyclonedx` reports the `name` field from `package.json` instead of on-disk directory name
- npm no longer registers man pages with the system when installed globally (`man npm-install` won't work, but `npm help install` is unaffected)
- `npm pkg` output is no longer forced to JSON — single values are returned without wrapping
- `npm shrinkwrap` removed; `npm-shrinkwrap.json` no longer loaded. Rename to `package-lock.json`; use `bundleDependencies` if needed
- Twitter and Freenode profile fields removed from npm registry
- npm no longer resolves path to node via `whichnode` — `process.execPath` is used directly
- `npm pack` and `npm publish` `--json` output format changed to be consistent
- `star`, `stars`, and `unstar` commands removed
- `npm adduser` command removed — create accounts on the website, use `npm login` for CLI auth
- Default license for `npm init` changed from `"ISC"` to empty string (field omitted if not set)
- npm now supports `node ^22.22.2 || ^24.15.0 || >=26.0.0`
- `allow-git` and `allow-remote` now default to `"none"` — set to `"all"` or `"root"` for git/tarball dependencies
- Root `preinstall` now runs before dependencies are installed
- Unknown configs in `.npmrc`, unknown CLI flags, and single-hyphen multi-char shorthands now throw instead of warning

### v12.0.1 (2026-07-10) — Bug Fixes

- `npm view` avoids wrapping array results
- Fixed bundled sigstore from dev dependency conflict

Full changelog: [docs.npmjs.com/cli/v12/using-npm/changelog](https://docs.npmjs.com/cli/v12/using-npm/changelog)
