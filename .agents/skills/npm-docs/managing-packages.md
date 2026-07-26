# Managing Published Packages

## Updating Your Published Package Version Number

### Using npm version

```bash
# Patch release (1.0.0 → 1.0.1)
npm version patch

# Minor release (1.0.1 → 1.1.0)
npm version minor

# Major release (1.1.0 → 2.0.0)
npm version major

# Pre-release
npm version prepatch
npm version preminor
npm version premajor
npm version prerelease

# Specific version
npm version 2.0.0
```

`npm version` creates a git commit and tag automatically.

### After Bumping Version

```bash
npm publish
```

## Updating and Managing Your Published Packages

### Updating Package Code

1. Make your changes
2. Bump the version: `npm version <type>`
3. Publish: `npm publish`
4. Push the git tag: `git push --follow-tags`

### Adding dist-tags

```bash
npm dist-tag add <package>@<version> <tag>
```

## Changing Package Visibility

### Public to Private

```bash
npm access restricted @scope/package-name
```

### Private to Public

```bash
npm access public @scope/package-name
```

**Note:** Unscoped packages cannot be made private. Only scoped packages can change visibility.

## Adding Collaborators to Private Packages

### Grant Access

```bash
npm access grant read-write @scope/package-name username
```

### Revoke Access

```bash
npm access revoke @scope/package-name username
```

### List Collaborators

```bash
npm access list collaborators @scope/package-name
```

## Deprecating and Undeprecating Packages

### Deprecate a Package Version

```bash
npm deprecate <package>@<version> "Message: use version X instead"
```

### Deprecate All Versions

```bash
npm deprecate <package> "This package is no longer maintained"
```

### Undeprecate

```bash
npm undeprecate <package>@<version>
```

Or use `npm undeprecate` (npm v11+):

```bash
npm undeprecate <package>
```

## Transferring a Package to Another User Account

### Prerequisites

- Both accounts must have npm accounts
- The recipient must accept the transfer
- The package must not have been transferred in the last 24 hours

### Steps

1. Go to the package page on npmjs.com
2. Click "Settings" > "Transfer Package"
3. Enter the recipient's username
4. The recipient receives an email to accept

### Via CLI

```bash
npm owner add <recipient-username> <package-name>
npm owner rm <your-username> <package-name>
```

## Unpublishing Packages from the Registry

### Within 72 Hours of Publishing

You can unpublish a package within 72 hours:

```bash
npm unpublish <package-name> -f
```

### After 72 Hours

After 72 hours, the npm [Unpublish Policy](https://docs.npmjs.com/policies/unpublish) applies:

- If the package has no dependents and was published within the last 72 hours, it can be unpublished
- If the package has no dependents and is older than 72 hours, contact npm support
- If the package has dependents, it generally cannot be unpublished

### Unpublishing a Specific Version

```bash
npm unpublish <package-name>@<version>
```

**Warning:** Unpublishing can break projects that depend on your package. Consider deprecating instead.

## Package Maintenance Best Practices

- **Keep dependencies updated**: Regularly run `npm audit` and `npm update`
- **Respond to issues**: Monitor your GitHub issues and npm support emails
- **Deprecate, don't unpublish**: Use deprecation messages to guide users to newer versions
- **Use dist-tags**: Label pre-release versions with `beta` or `next` tags
- **Maintain a changelog**: Document changes between versions
- **Test before publishing**: Use `npm publish --dry-run` to verify contents
