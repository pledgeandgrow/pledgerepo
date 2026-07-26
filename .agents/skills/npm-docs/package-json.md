# package.json and Dependencies

## Creating a package.json File

The `package.json` file is the manifest of your package. It must exist to publish to the npm registry.

### Quick Creation

```bash
npm init
# Or use defaults
npm init -y
```

### Full Example

```json
{
  "name": "my-package",
  "version": "1.0.0",
  "description": "A brief description",
  "main": "index.js",
  "type": "module",
  "scripts": {
    "test": "echo \"Error: no test specified\" && exit 1",
    "start": "node index.js",
    "build": "tsc"
  },
  "keywords": ["example", "demo"],
  "author": "Your Name <you@example.com>",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/user/repo.git"
  },
  "bugs": {
    "url": "https://github.com/user/repo/issues"
  },
  "homepage": "https://github.com/user/repo#readme",
  "dependencies": {
    "express": "^4.18.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  },
  "peerDependencies": {
    "react": ">=18.0.0"
  },
  "engines": {
    "node": ">=18.0.0"
  },
  "files": ["dist/", "index.js"],
  "bin": {
    "my-cli": "./bin/cli.js"
  },
  "exports": {
    ".": "./dist/index.js",
    "./utils": "./dist/utils.js"
  }
}
```

## Key Fields

| Field | Description |
|-------|-------------|
| `name` | Package name (lowercase, no spaces, unique on registry) |
| `version` | Semantic version (e.g., `1.0.0`) |
| `description` | Short description for npm search |
| `main` | Entry point for `require()` |
| `type` | Module type: `"module"` for ESM, `"commonjs"` for CJS |
| `scripts` | Command shortcuts run via `npm run` |
| `keywords` | Search keywords |
| `author` | Author name and email |
| `license` | SPDX license identifier |
| `repository` | Source repository URL |
| `bugs` | Issue tracker URL |
| `homepage` | Project homepage |
| `dependencies` | Production dependencies |
| `devDependencies` | Development dependencies |
| `peerDependencies` | Packages required by consumers |
| `engines` | Required Node.js/npm versions |
| `files` | Files to include in published package |
| `bin` | Executable commands |
| `exports` | Modern entry point resolution |
| `publishConfig` | Registry and access settings for publishing |
| `workspaces` | Monorepo workspace packages |

## Package Name Guidelines

- Must be lowercase
- Maximum 214 characters
- Cannot start with `.` or `_`
- Cannot contain spaces
- Can contain hyphens and underscores
- Scoped packages: `@scope/name`
- Must be unique on the registry

## Specifying Dependencies and devDependencies

### dependencies

Runtime dependencies required for your package to function:

```json
{
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "~4.17.0"
  }
}
```

### devDependencies

Development-only dependencies (not installed in production):

```json
{
  "devDependencies": {
    "jest": "^29.0.0",
    "typescript": "^5.0.0"
  }
}
```

Install a dev dependency:

```bash
npm install --save-dev <package>
# or
npm install -D <package>
```

### peerDependencies

Packages that consumers must install (expected to be provided by the host):

```json
{
  "peerDependencies": {
    "react": ">=18.0.0"
  }
}
```

### optionalDependencies

Dependencies that are installed if available but don't fail if missing:

```json
{
  "optionalDependencies": {
    "fsevents": "^2.3.0"
  }
}
```

### bundleDependencies

Dependencies bundled and shipped with your package:

```json
{
  "bundleDependencies": ["my-private-dep"]
}
```

## Semantic Versioning (semver)

### Version Format

```
MAJOR.MINOR.PATCH
```

- **MAJOR** — Breaking changes
- **MINOR** — New features, backward compatible
- **PATCH** — Bug fixes, backward compatible

### Incrementing Versions

| Change Type | Version Bump |
|-------------|-------------|
| Bug fix only | PATCH: 1.0.0 → 1.0.1 |
| New feature, backward compatible | MINOR: 1.0.1 → 1.1.0 |
| Breaking changes | MAJOR: 1.1.0 → 2.0.0 |

### Version Range Syntax

| Range | Meaning | Example |
|-------|---------|---------|
| `1.0.0` | Exact version | Only 1.0.0 |
| `^1.0.0` | Compatible with 1.0.0 (same major) | >=1.0.0 <2.0.0 |
| `~1.0.0` | Approximately equivalent (same minor) | >=1.0.0 <1.1.0 |
| `1.0.x` | Any patch version | >=1.0.0 <1.1.0 |
| `1.x` | Any minor version | >=1.0.0 <2.0.0 |
| `*` or `x` | Any version | Any |
| `>=1.0.0` | At least 1.0.0 | >=1.0.0 |
| `>1.0.0` | Greater than 1.0.0 | >1.0.0 |
| `<=2.0.0` | At most 2.0.0 | <=2.0.0 |
| `1.0.0 - 2.0.0` | Range | >=1.0.0 <=2.0.0 |
| `1.0.0 || 2.0.0` | OR | 1.0.0 or 2.0.0 |

### Example

```json
{
  "dependencies": {
    "my_dep": "^1.0.0",
    "another_dep": "~2.2.0"
  }
}
```

## Creating Node.js Modules

A Node.js module is any file or directory that can be loaded with `require()` or `import`.

### CommonJS Module

```js
// math.js
function add(a, b) { return a + b }
module.exports = { add }

// index.js
const { add } = require('./math')
console.log(add(1, 2))
```

### ESM Module

```js
// math.js
export function add(a, b) { return a + b }

// index.js
import { add } from './math.js'
console.log(add(1, 2))
```

Set `"type": "module"` in `package.json` for ESM by default.

## About Package README Files

The `README.md` file is displayed on the npm package page. Best practices:

- Include installation instructions
- Show usage examples
- Document the API
- Include a license section
- Keep it up to date

```markdown
# my-package

A brief description.

## Installation

\`\`\`bash
npm install my-package
\`\`\`

## Usage

\`\`\`js
const myPackage = require('my-package')
\`\`\`

## License

MIT
```

## The `exports` Field

The modern way to define entry points:

```json
{
  "exports": {
    ".": {
      "import": "./dist/index.mjs",
      "require": "./dist/index.cjs",
      "types": "./dist/index.d.ts"
    },
    "./utils": {
      "import": "./dist/utils.mjs",
      "require": "./dist/utils.cjs"
    },
    "./package.json": "./package.json"
  }
}
```

This enables:
- Conditional exports (ESM/CJS)
- Subpath exports
- Encapsulation (only exported paths are accessible)
