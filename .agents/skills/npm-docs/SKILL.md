---
name: npm-docs
version: "12.x"
tags:
  - npm
  - package-manager
  - nodejs
  - registry
  - cli
  - packages
  - semver
  - publishing
  - workspaces
  - organizations
description: >
  Use this skill whenever the user asks about npm, package management, publishing packages,
  semantic versioning, npm CLI commands, or Node.js dependencies. Covers npm 12.x including
  the registry, scopes, organizations, security (audit, provenance, 2FA), CI/CD integration,
  workspaces, scripts, .npmrc configuration, and the full CLI reference. Use it for any
  npm-related task including publishing, troubleshooting, or configuration.
---

# npm Documentation

## Quick Reference

| Topic | File | Description |
|-------|------|-------------|
| About npm | `about-npm.md` | npm overview, registry, packages, modules, scopes, public/private packages |
| Getting Started | `getting-started.md` | User accounts, 2FA, profile, billing, local env setup, installing Node.js & npm |
| package.json | `package-json.md` | package.json fields, dependencies, semver, README, Node.js modules |
| Publishing | `publishing.md` | Creating & publishing unscoped/scoped/private/org packages, dist-tags |
| Managing Packages | `managing-packages.md` | Updating, deprecating, transferring, unpublishing, visibility, collaborators |
| Installing Packages | `installing-packages.md` | Searching, downloading locally/globally, updating, uninstalling, EACCES |
| Security | `security.md` | Audit, provenance, OIDC, staged publishing, signatures, 2FA, malware |
| Integrations | `integrations.md` | External services, access tokens, CI/CD, Docker |
| Organizations | `organizations.md` | Creating/managing orgs, billing, members, teams, org packages |
| CLI Commands | `cli-commands.md` | All npm CLI commands reference (v12) |
| Configuring npm | `configuring-npm.md` | .npmrc, package.json, package-lock.json, folders, install |
| Using npm | `using-npm.md` | Registry, package spec, config, logging, scope, scripts, workspaces |
| Troubleshooting | `troubleshooting.md` | Common errors, debug logs, version issues |
| Policies | `policies.md` | Terms, conduct, disputes, license, privacy, unpublish, DMCA, security |

## Core Concepts

- **npm** — Node Package Manager; package registry, CLI, and website
- **Package** — A file/directory described by `package.json`
- **Module** — Any file/directory in `node_modules` loadable via `require()` or `import`
- **Scope** — A namespace for packages (e.g., `@myorg/mypackage`)
- **Semantic Versioning** — `MAJOR.MINOR.PATCH` versioning scheme
- **Registry** — The public npm registry at `https://registry.npmjs.org`
- **Organization** — A group for coordinating package maintenance and collaboration
- **Access Token** — Authentication token for CLI/API access
- **Workspaces** — Monorepo support for managing multiple packages

## Quick Start

```bash
# Install a package
npm install <package-name>

# Install globally
npm install -g <package-name>

# Initialize a new project
npm init

# Run a script
npm run <script-name>

# Publish a package
npm publish

# Update packages
npm update

# Audit for vulnerabilities
npm audit
```

## Architecture Overview

```
npm Ecosystem
├── Registry (registry.npmjs.org)
│   ├── Public packages (free)
│   └── Private packages (paid)
├── CLI (npm command)
│   ├── Commands (install, publish, audit, etc.)
│   ├── Config (.npmrc)
│   └── Scripts (package.json scripts)
├── Website (npmjs.com)
│   ├── User accounts
│   ├── Organizations
│   └── Package pages
└── Integrations
    ├── CI/CD workflows
    ├── Docker
    └── Access tokens
```
