# Security

## About Audit Reports

npm audit scans your project for known security vulnerabilities in dependencies.

### Running an Audit

```bash
# Check for vulnerabilities
npm audit

# Get JSON output
npm audit --json

# Fix vulnerabilities automatically
npm audit fix

# Fix with breaking changes (major updates)
npm audit fix --force

# Dry run
npm audit fix --dry-run
```

### Audit Report Fields

| Field | Description |
|-------|-------------|
| `severity` | `low`, `moderate`, `high`, `critical` |
| `via` | Dependency path to the vulnerability |
| `effects` | Packages affected by the vulnerability |
| `range` | Affected version range |
| `fixAvailable` | Whether a fix is available |

### Audit in CI/CD

```bash
# Fail build on high or critical vulnerabilities
npm audit --audit-level=high

# In CI scripts
npm audit --audit-level=critical || true
```

## Auditing Package Dependencies for Security Vulnerabilities

### Regular Audits

Run `npm audit` regularly:
- After installing new dependencies
- Before deploying
- As part of CI/CD pipeline

### Understanding the Output

```
# npm audit report

vulnerabilities:
  1 low
  2 moderate
  1 high
  0 critical

run `npm audit fix` to fix them
```

### Manual Review

For vulnerabilities that can't be auto-fixed:
1. Review the advisory
2. Check if your code uses the vulnerable functionality
3. Upgrade manually or find an alternative package

## Generating Provenance Statements

Provenance provides verifiable information about how a package was built and published.

### Prerequisites

- Package must be public
- Published from GitHub Actions
- GitHub repository must be public

### Setup

1. **Configure GitHub Actions workflow**:

```yaml
name: Publish
on:
  release:
    types: [published]
jobs:
  publish:
    runs-on: ubuntu-latest
    permissions:
      id-token: write  # Required for provenance
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'
      - run: npm ci
      - run: npm publish --provenance
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

2. **Publish with provenance**:

```bash
npm publish --provenance
```

### Verifying Provenance

Provenance information is displayed on the npm package page and can be verified using Sigstore.

## Trusted Publishing with OIDC

Trusted publishing allows publishing packages without long-lived tokens by using OpenID Connect (OIDC) for authentication.

### How It Works

1. Configure your npm package to trust a specific GitHub repository/workflow
2. GitHub Actions generates an OIDC token
3. npm verifies the token and allows publishing

### Setup

1. Go to your package settings on npmjs.com
2. Add a trusted publisher (GitHub repository, workflow, environment)
3. Remove `NPM_TOKEN` from your GitHub secrets
4. Publish using OIDC:

```yaml
- run: npm publish --provenance --access public
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}  # Still needed for now
```

## Staged Publishing

Staged publishing allows you to publish a package in stages, giving users time to review before it becomes fully available.

### How It Works

1. Publish a staged version
2. The package is visible but marked as staged
3. After review, promote it to full availability

```bash
# Publish staged
npm publish --staged

# Promote to full
npm dist-tag add <package>@<version> latest
```

## About ECDSA Registry Signatures

npm signs all published packages with ECDSA signatures to verify authenticity.

### How Signatures Work

- Each package published to the npm registry is automatically signed
- The signature is stored in the package's `dist` metadata
- Signatures use the ECDSA P-256 algorithm

## Verifying ECDSA Registry Signatures

### Automatic Verification

npm CLI automatically verifies signatures during installation (npm v9+).

### Manual Verification

```bash
npm verify-signatures <package-name>@<version>
```

### Signature Verification in CI

```bash
npm config set verify-signatures true
npm install
```

## Requiring 2FA for Package Publishing and Settings Modification

### Enabling 2FA for Publishing

1. Go to Account Settings > Two-Factor Authentication
2. Enable 2FA for "Authorization and writes"
3. Use `--otp` flag when publishing:

```bash
npm publish --otp=123456
```

### Requiring 2FA for a Package

```bash
npm access set requires-2fa true @scope/package-name
```

This requires all publishers of the package to have 2FA enabled.

## Reporting Malware in an npm Package

If you find malware in an npm package:

1. **Do not execute the package**
2. Report to npm security: `security@npmjs.com`
3. Include the package name, version, and evidence
4. File a report at [github.com/npm/security](https://github.com/npm/security)

## Security Best Practices

- **Run `npm audit` regularly** — In development and CI
- **Use 2FA** — Enable for your account and packages
- **Verify signatures** — Ensure package authenticity
- **Use provenance** — Publish with build provenance
- **Lock dependencies** — Use `package-lock.json` or `npm ci`
- **Review dependencies** — Check for unnecessary or suspicious packages
- **Keep updated** — Regularly update to fix known vulnerabilities
- **Use `npm ci`** — For reproducible installs in CI/CD
- **Avoid `npm install` in production** — Use `npm ci --omit=dev`
