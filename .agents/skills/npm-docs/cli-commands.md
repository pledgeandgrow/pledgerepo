# npm CLI Commands Reference (v12)

> **Note:** In npm v12, the following commands were **removed**: `npm adduser` (use `npm login`), `npm star` / `npm stars` / `npm unstar`, and `npm shrinkwrap` (use `package-lock.json` with `bundleDependencies`).

## Command List

### npm

The main npm command.

```bash
npm <command> [args]
```

Common commands:

| Command | Description |
|---------|-------------|
| `npm install` | Install dependencies |
| `npm uninstall` | Remove a package |
| `npm publish` | Publish a package |
| `npm update` | Update packages |
| `npm audit` | Check for vulnerabilities |
| `npm run` | Run a script |
| `npm init` | Create package.json |

### npm access

Manage package access levels.

```bash
# Grant access
npm access grant <permissions> <package> <user>
# permissions: read-only | read-write

# Revoke access
npm access revoke <package> <user>

# Set access level
npm access set <property> <value> <package>
# e.g., npm access set requires-2fa true @scope/pkg

# List collaborators
npm access list collaborators <package>

# Get status
npm access get status <package>
```

### npm approve-scripts

Approve scripts from dependencies.

```bash
npm approve-scripts [package...]
```

### npm audit

Check for security vulnerabilities.

```bash
npm audit [--json] [--audit-level=<level>] [--fix] [--force]
```

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--audit-level` | Minimum severity: `low`, `moderate`, `high`, `critical` |
| `--fix` | Auto-fix vulnerabilities |
| `--force` | Allow breaking changes when fixing |
| `--dry-run` | Show what would be fixed |

### npm bugs

Open bugs page in browser.

```bash
npm bugs [<package>]
```

### npm cache

Manage the npm cache.

```bash
# Add to cache
npm cache add <tarball>

# Clean cache
npm cache clean [name]

# Verify cache
npm cache verify

# List cache
npm cache ls [name]
```

### npm ci

Clean install from lockfile (for CI/CD).

```bash
npm ci
```

- Removes `node_modules` before install
- Requires `package-lock.json`
- Never modifies `package-lock.json`
- Faster than `npm install`

### npm completion

Enable tab completion for npm commands.

```bash
npm completion
```

### npm config

Manage npm configuration.

```bash
# Get a config value
npm config get <key>

# Set a config value
npm config set <key> <value>

# Delete a config value
npm config delete <key>

# List all config
npm config list

# List with sources
npm config list -l

# Edit .npmrc
npm config edit
```

### npm dedupe

Reduce duplication in `node_modules`.

```bash
npm dedupe
```

### npm deny-scripts

Deny scripts from dependencies.

```bash
npm deny-scripts [package...]
```

### npm deprecate

Deprecate a package version.

```bash
npm deprecate <package>@<version> "<message>"
```

### npm diff

Show diff between package versions.

```bash
npm diff [--spec=<spec>] [--diff=<spec>]
```

### npm dist-tag

Manage distribution tags.

```bash
# Add a tag
npm dist-tag add <package>@<version> <tag>

# List tags
npm dist-tag ls <package>

# Remove a tag
npm dist-tag rm <package> <tag>
```

### npm docs

Open package docs in browser.

```bash
npm docs [<package>]
```

### npm doctor

Check npm environment health.

```bash
npm doctor
```

Checks: npm version, node version, registry connectivity, permissions, etc.

### npm edit

Edit an installed package.

```bash
npm edit <package>
```

### npm exec

Run a command from a local or remote npm package.

```bash
npm exec -- <command> [args]
# or
npx <command> [args]
```

### npm explain

Explain why a package is installed.

```bash
npm explain <package>
```

### npm explore

Browse an installed package's directory.

```bash
npm explore <package> -- <command>
```

### npm find-dupes

Find duplicate packages in `node_modules`.

```bash
npm find-dupes
```

### npm fund

Check for packages seeking funding.

```bash
npm fund [<package>]
```

### npm get

Get a config value (alias for `npm config get`).

```bash
npm get <key>
```

### npm help

Get help on a command.

```bash
npm help <command>
```

### npm help-search

Search npm help documentation.

```bash
npm help-search <term>
```

### npm init

Create a new package.json.

```bash
npm init [--yes|-y] [--scope=<scope>]
```

### npm install

Install dependencies.

```bash
npm install [<package>] [flags]
```

| Flag | Description |
|------|-------------|
| `-g, --global` | Install globally |
| `-D, --save-dev` | Save as devDependency |
| `-O, --save-optional` | Save as optionalDependency |
| `-E, --save-exact` | Save exact version |
| `--no-save` | Don't save to package.json |
| `--omit=dev` | Skip devDependencies |
| `--legacy-peer-deps` | Ignore peer dependencies |
| `--force` | Force reinstall |
| `--dry-run` | Show what would be installed |

### npm install-ci-test

Install with `npm ci` then run tests.

```bash
npm install-ci-test
```

### npm install-scripts

Manage install scripts from dependencies.

```bash
npm install-scripts [flags]
```

### npm install-test

Install and run tests.

```bash
npm install-test
```

### npm link

Symlink a local package globally or link a global package locally.

```bash
# Link current package globally
npm link

# Link a global package to local project
npm link <package>
```

### npm ll

List installed packages (long format).

```bash
npm ll [args]
```

Alias for `npm list --long`.

### npm login

Log in to the npm registry.

```bash
npm login [--registry=<url>] [--scope=<scope>]
```

### npm logout

Log out of the npm registry.

```bash
npm logout [--registry=<url>] [--scope=<scope>]
```

### npm ls

List installed packages.

```bash
npm ls [<package>] [--depth=<n>] [--json] [--global]
```

| Flag | Description |
|------|-------------|
| `--depth=<n>` | Max depth (0 = direct deps only) |
| `--json` | JSON output |
| `-g, --global` | List global packages |
| `--all` | Show all dependencies |
| `--omit=dev` | Exclude devDependencies |

### npm org

Manage organizations.

```bash
# List orgs
npm org list

# List members
npm org members <org>

# Add member
npm org members add <org> <user> <role>

# Remove member
npm org members rm <org> <user>
```

### npm outdated

Check for outdated packages.

```bash
npm outdated [<package>]
```

Output: `Package | Current | Wanted | Latest | Location`

### npm owner

Manage package owners.

```bash
# List owners
npm owner ls <package>

# Add owner
npm owner add <user> <package>

# Remove owner
npm owner rm <user> <package>
```

### npm pack

Create a tarball from a package.

```bash
npm pack [<package>] [--dry-run] [--pack-destination=<dir>]
```

### npm patch

Patch a package.

```bash
npm patch <package>@<version>
```

### npm ping

Ping the npm registry.

```bash
npm ping
```

### npm pkg

Manage package.json fields.

```bash
# Get a field
npm pkg get <field>

# Set a field
npm pkg set <field>=<value>

# Delete a field
npm pkg delete <field>
```

### npm prefix

Print the local prefix directory.

```bash
npm prefix [-g]
```

### npm profile

Manage npm profile settings.

```bash
# Get profile
npm profile get

# Set profile property
npm profile set <property> <value>

# Enable 2FA
npm profile enable-2fa

# Disable 2FA
npm profile disable-2fa
```

### npm prune

Remove extraneous packages.

```bash
npm prune [[<package>...]] [--omit=dev] [--dry-run]
```

### npm publish

Publish a package to the registry.

```bash
npm publish [<tarball>|<folder>] [flags]
```

| Flag | Description |
|------|-------------|
| `--access <access>` | `public` or `restricted` |
| `--tag <tag>` | Dist-tag (default: `latest`) |
| `--otp=<code>` | 2FA code |
| `--dry-run` | Show what would be published |
| `--provenance` | Publish with provenance |
| `--staged` | Staged publishing |

### npm query

Query dependencies with CSS-like selectors.

```bash
npm query "<selector>"
```

Examples:

```bash
npm query ":root > .dependencies"
npm query "[name=express]"
npm query ":not(:empty)"
npm query ".workspace > *"
```

### npm rebuild

Rebuild native addons.

```bash
npm rebuild [<package>]
```

### npm repo

Open package repository in browser.

```bash
npm repo [<package>]
```

### npm restart

Restart a package's script.

```bash
npm restart
```

### npm root

Print the root directory of node_modules.

```bash
npm root [-g]
```

### npm run

Run a script defined in package.json.

```bash
npm run <script> [args]
# or
npm run-script <script> [args]
```

### npm sbom

Generate a Software Bill of Materials.

```bash
npm sbom [--sbom-format=<format>]
# formats: cyclonedx | spdx
```

### npm search

Search the npm registry.

```bash
npm search <terms> [--json] [--long]
```

### npm set

Set a config value (alias for `npm config set`).

```bash
npm set <key> <value>
```

### npm stage

Stage a package for publishing.

```bash
npm stage <package>
```

### npm start

Run the `start` script.

```bash
npm start
```

### npm stop

Run the `stop` script.

```bash
npm stop
```

### npm team

Manage organization teams.

```bash
# Create team
npm team create <org:team>

# Destroy team
npm team destroy <org:team>

# Add member
npm team add <org:team> <user>

# Remove member
npm team rm <org:team> <user>

# List teams
npm team ls <org>

# List members
npm team ls <org:team>
```

### npm test

Run the `test` script.

```bash
npm test
# or
npm tst
```

### npm token

Manage access tokens.

```bash
# List tokens
npm token list

# Create token
npm token create [--read-only] [--cidr=<cidr>]

# Revoke token
npm token revoke <id>
```

### npm trust

Trust a package or publisher.

```bash
npm trust <package>
```

### npm undeprecate

Undeprecate a package version.

```bash
npm undeprecate <package>@<version>
```

### npm uninstall

Remove a package.

```bash
npm uninstall <package> [flags]
```

| Flag | Description |
|------|-------------|
| `-g, --global` | Remove globally |
| `-D, --save-dev` | Remove from devDependencies |
| `--no-save` | Don't update package.json |

### npm unpublish

Remove a package from the registry.

```bash
npm unpublish <package> [-f]
npm unpublish <package>@<version>
```

### npm update

Update packages.

```bash
npm update [<package>] [-g]
```

### npm version

Bump the package version.

```bash
npm version <newversion|major|minor|patch>
```

Options:
- `--no-git-tag-version` — Don't create git commit/tag
- `-m <message>` — Custom commit message

### npm view

View package metadata.

```bash
npm view <package>[@<version>] [<field>]
```

Examples:

```bash
npm view express version
npm view express dependencies
npm view express versions --json
```

### npm whoami

Display the current npm username.

```bash
npm whoami
```

### npx

Run a package without installing.

```bash
npx <command> [args]

# Install and run a specific package
npx <package>@<version> [args]

# Run from a git repo
npx github:user/repo [args]

# Use -- to separate flags
npx -- <command> --flag
```

| Flag | Description |
|------|-------------|
| `-p, --package` | Specify package to install |
| `-y, --yes` | Auto-confirm prompts |
| `--no-install` | Don't install if not present |
| `--call <script>` | Run a script from package |
