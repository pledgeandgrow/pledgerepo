# Integrations

## Integrating npm with External Services

npm can be integrated with various external services:

- **CI/CD platforms** — GitHub Actions, GitLab CI, CircleCI, Jenkins
- **Container platforms** — Docker, Kubernetes
- **Private registries** — GitHub Packages, Verdaccio, Artifactory, Nexus
- **Cloud platforms** — AWS, Azure, Google Cloud

## About Access Tokens

Access tokens authenticate the npm CLI with the registry without requiring interactive login.

### Token Types

| Type | Description | Use Case |
|------|-------------|----------|
| **Classic Automation** | Full access, no 2FA required | CI/CD pipelines |
| **Classic Publish** | Can publish and read | Limited publishing |
| **Classic Read-only** | Can only read packages | Downloading in CI |
| **Granular Access** | Scoped permissions, per-package | Fine-grained control |

### Creating and Viewing Access Tokens

#### Via Website

1. Go to [npmjs.com](https://www.npmjs.com) > Access Tokens
2. Click "Generate New Token"
3. Select token type
4. Copy the token (shown only once)

#### Via CLI

```bash
# Create a token
npm token create --read-only

# List tokens
npm token list

# Revoke a token
npm token revoke <token-id>
```

### Using Tokens in CI/CD

```bash
# Set in environment
export NPM_TOKEN=your-token-here

# Or in .npmrc
echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" >> ~/.npmrc
```

### Best Practices

- **Use granular tokens** — Limit scope to specific packages
- **Set expiration dates** — Rotate tokens regularly
- **Never commit tokens** — Use environment variables or secrets
- **Use read-only tokens** — For install-only workflows
- **Revoke unused tokens** — Clean up old tokens

## Revoking Access Tokens

### Via Website

1. Go to Access Tokens page
2. Find the token
3. Click "Delete"

### Via CLI

```bash
npm token revoke <token-id>
```

## Using Private Packages in a CI/CD Workflow

### GitHub Actions

```yaml
name: CI
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'
      - run: npm ci
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
      - run: npm test
      - run: npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### GitLab CI

```yaml
build:
  image: node:20
  script:
    - npm config set //registry.npmjs.org/:_authToken $NPM_TOKEN
    - npm ci
    - npm test
    - npm publish
  variables:
    NPM_TOKEN: $CI_NPM_TOKEN
```

### CircleCI

```yaml
jobs:
  build:
    docker:
      - image: cimg/node:20.0
    steps:
      - checkout
      - run:
          name: Install dependencies
          command: |
            echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" >> ~/.npmrc
            npm ci
          environment:
            NPM_TOKEN: $NPM_TOKEN
```

### Using npm ci vs npm install

| `npm ci` | `npm install` |
|----------|---------------|
| Requires `package-lock.json` | Works without lockfile |
| Removes `node_modules` first | Updates existing |
| Never modifies lockfile | May update lockfile |
| Faster, reproducible | Slower, may vary |
| **Use in CI/CD** | **Use in development** |

## Docker and Private Modules

### Dockerfile with Private Packages

```dockerfile
FROM node:20-slim

WORKDIR /app

# Copy package files
COPY package.json package-lock.json ./

# Configure npm auth
ARG NPM_TOKEN
RUN echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" > ~/.npmrc

# Install dependencies
RUN npm ci

# Remove .npmrc for security
RUN rm ~/.npmrc

# Copy source
COPY . .

# Build
RUN npm run build

CMD ["node", "dist/index.js"]
```

### Build with Token

```bash
docker build --build-arg NPM_TOKEN=your-token -t my-app .
```

### Multi-stage Build (Recommended)

```dockerfile
# Build stage
FROM node:20-slim AS builder
WORKDIR /app
COPY package*.json ./
ARG NPM_TOKEN
RUN echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" > ~/.npmrc
RUN npm ci
COPY . .
RUN npm run build
RUN rm ~/.npmrc

# Production stage
FROM node:20-slim
WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./
CMD ["node", "dist/index.js"]
```

### Using .npmrc in Docker

Create a project-level `.npmrc`:

```ini
//registry.npmjs.org/:_authToken=${NPM_TOKEN}
```

**Warning:** Never bake the token into the image. Use build args and remove the `.npmrc` after install.

## Private Registry Configuration

### Using GitHub Packages

```ini
# .npmrc
@my-org:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

### Using Verdaccio

```ini
# .npmrc
registry=http://localhost:4873
//localhost:4873/:_authToken=your-token
```

### Using Multiple Registries

```ini
# Default registry
registry=https://registry.npmjs.org

# Scoped registry
@my-org:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```
