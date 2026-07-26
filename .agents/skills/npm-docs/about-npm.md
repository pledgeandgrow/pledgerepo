# About npm

## What is npm?

npm is the world's largest software registry. It consists of:

- **The registry** — A public database of JavaScript packages at `https://registry.npmjs.org`
- **The CLI** — The `npm` command-line tool for interacting with the registry
- **The website** — `https://npmjs.com` for browsing packages and managing accounts

### Use npm to:

- Adapt packages of code for your apps, or incorporate packages as they are
- Download standalone tools you can use right away
- Run packages without downloading using `npx`
- Share code with any npm user, anywhere
- Restrict code to specific developers
- Create organizations to coordinate package maintenance, coding, and developers
- Form virtual teams by using organizations
- Manage multiple versions of code and code dependencies
- Update applications easily when underlying code is updated
- Discover multiple ways to solve the same puzzle
- Find other developers who are working on similar problems and projects

## The Public npm Registry

The public npm registry is a database of JavaScript packages, each comprised of software and metadata. Open source developers and developers at companies use the npm registry to:

- Contribute packages to the entire community or members of their organizations
- Download packages to use in their own projects

You can also use a private npm package registry like GitHub Packages or the open source [Verdaccio](https://verdaccio.org) project to develop packages internally.

## About Packages

A **package** is a file or directory that is described by a `package.json` file. A package must contain a `package.json` file in order to be published to the npm registry.

### Package Formats

A package is any of the following:

- **a)** A folder containing a program described by a `package.json` file
- **b)** A gzipped tarball containing (a)
- **c)** A URL that resolves to (b)
- **d)** A `<name>@<version>` that is published on the registry with (c)
- **e)** A `<name>@<tag>` that points to (d)
- **f)** A `<name>` that has a `latest` tag satisfying (e)
- **g)** A git url that, when cloned, results in (a)

### Git URL Formats

Git URLs used for npm packages can be formatted as:

```
git://github.com/user/project.git#commit-ish
git+ssh://user@hostname:project.git#commit-ish
git+http://user@hostname/project/blah.git#commit-ish
git+https://user@hostname/project/blah.git#commit-ish
```

The `commit-ish` can be any tag, sha, or branch. The default is `HEAD`.

**Note:** Installing any package directly from git will not install git submodules or workspaces.

## About Modules

A **module** is any file or directory in the `node_modules` directory that can be loaded by the Node.js `require()` or `import` syntax.

To be loaded by `require()`, a module must be one of:

- A folder with a `package.json` file containing a `"main"` field
- A JavaScript file

To use the `import` syntax, a module should include `"type": "module"` in its `package.json`:

```json
{
  "name": "my-package",
  "type": "module"
}
```

**Key distinction:** Since modules are not required to have a `package.json` file, not all modules are packages. Only modules that have a `package.json` file are also packages.

## About Scopes

A **scope** is a namespace for packages. Scopes are preceded by an `@` symbol:

```
@scope/package-name
```

### Scopes and Package Visibility

- **Unscoped packages** are always public
- **Private packages** are always scoped
- **Scoped packages** are private by default; you must pass `--access public` when publishing to make them public

### Package Access Matrix

| Scope | Access Level | Visibility | Who Can Create |
|-------|-------------|------------|----------------|
| Unscoped | Public | Public | User accounts only |
| Scoped (user) | Public | Public | User accounts |
| Scoped (user) | Restricted | Private | User accounts (paid) |
| Scoped (org) | Public | Public | Organization members |
| Scoped (org) | Restricted | Private | Organization members (paid) |

**Note:** Only user accounts can create and manage unscoped packages. Organizations can only manage scoped packages.

## About Public Packages

Public packages are freely available to download and use. Anyone can:

- Search for and download public packages
- Publish unscoped public packages (with a free account)
- Publish scoped public packages (with a free account)

## About Private Packages

Private packages are scoped packages that are restricted to specific users or organizations. They require:

- A paid npm account (for user-scoped private packages)
- A paid organization (for org-scoped private packages)

Private packages are not visible or downloadable by users who don't have access.

## Sharing and Collaboration

- **Public packages** — Free to share with anyone
- **Private packages** — Requires a paid account; restricted to specific developers
- **Organizations** — Create groups to coordinate package maintenance, coding, and developers
- **Teams** — Form virtual teams within organizations for access control

## Getting Started

To get started with npm:

1. Create an account at [npmjs.com/signup](https://www.npmjs.com/signup)
2. Install Node.js and npm using the [official installer](https://nodejs.org/) or a version manager
3. Use the CLI to interact with the registry

Your account will be available at `http://www.npmjs.com/~yourusername`.
