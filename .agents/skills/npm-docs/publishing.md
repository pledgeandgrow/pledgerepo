# Publishing Packages

## Creating and Publishing Unscoped Public Packages

### 1. Create a package.json

```bash
npm init
```

Ensure your package name is unique and follows [package name guidelines](https://docs.npmjs.com/package-name-guidelines).

### 2. Write your code

Create your main entry file as specified in `package.json` (`main` field).

### 3. Login to npm

```bash
npm login
```

### 4. Publish

```bash
npm publish
```

Your package is now available at `https://www.npmjs.com/package/your-package-name`.

## Creating and Publishing Scoped Public Packages

Scoped packages use the `@scope/name` format.

### 1. Create a scoped package.json

```bash
npm init --scope=@your-username
```

### 2. Publish with public access

Scoped packages are private by default. To publish as public:

```bash
npm publish --access public
```

To set public access permanently:

```json
{
  "publishConfig": {
    "access": "public"
  }
}
```

## Creating and Publishing Private Packages

Private packages are scoped and restricted to authorized users.

### Prerequisites

- A paid npm account (Pro or Teams plan)

### 1. Create a scoped package

```bash
npm init --scope=@your-username
```

### 2. Publish (private by default)

```bash
npm publish
```

### Managing Access

Grant access to other users:

```bash
npm access grant read-write @your-username/private-package username
```

Revoke access:

```bash
npm access revoke @your-username/private-package username
```

## Creating and Publishing Organization Scoped Packages

### 1. Create an organization

See `organizations.md` for details.

### 2. Create a scoped package

```bash
npm init --scope=@your-org
```

### 3. Configure publishConfig

```json
{
  "publishConfig": {
    "access": "public",
    "registry": "https://registry.npmjs.org"
  }
}
```

### 4. Publish

```bash
npm publish
```

For private org packages, omit `--access public` (private by default).

## Package Name Guidelines

- Use lowercase letters only
- Maximum 214 characters
- Cannot start with `.` or `_`
- No spaces
- Hyphens and underscores allowed
- Scoped: `@scope/name`
- Must be unique on the registry
- Avoid using someone else's trademark

## Adding dist-tags to Packages

Distribution tags (dist-tags) allow you to label published versions:

```bash
# Add a tag
npm dist-tag add <package>@<version> <tag>

# List tags
npm dist-tag ls <package>

# Remove a tag
npm dist-tag rm <package> <tag>
```

### Common dist-tags

| Tag | Purpose |
|-----|---------|
| `latest` | Default; most recent stable release |
| `next` | Pre-release or beta channel |
| `beta` | Beta testing channel |
| `legacy` | Maintenance for older versions |

### Publishing with a tag

```bash
npm publish --tag beta
```

## Publishing Best Practices

### Before Publishing

1. **Test your package**: `npm test`
2. **Check the package contents**: `npm pack` and inspect the tarball
3. **Verify `.npmignore` or `files` field**: Don't publish unnecessary files
4. **Update the README**: Ensure documentation is current
5. **Bump the version**: Follow semver

### Controlling Published Files

Use the `files` field in `package.json`:

```json
{
  "files": ["dist/", "index.js", "README.md"]
}
```

Or use `.npmignore`:

```
test/
src/
*.test.js
```

### Dry Run

```bash
npm publish --dry-run
```

This shows what would be published without actually publishing.

## Publishing with Provenance

For enhanced supply chain security, publish with provenance statements:

```bash
npm publish --provenance
```

This requires GitHub Actions and a public package. See `security.md` for details.
