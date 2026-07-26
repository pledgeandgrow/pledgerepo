# GitHub Actions — Security, Secrets & Reusable Workflows

## Using Secrets in GitHub Actions

Secrets are encrypted environment variables that you create in a repository, environment, or organization. They are available to your workflows but are masked in logs.

### Creating secrets for a repository

1. Navigate to your repository on GitHub.
2. Go to **Settings > Secrets and variables > Actions**.
3. Click **New repository secret**.
4. Add a name and value.
5. Click **Add secret**.

### Creating secrets for an environment

1. Go to repository **Settings > Environments**.
2. Select or create an environment.
3. Add environment secrets under **Environment secrets**.

Environment secrets take precedence over repository and organization secrets when the job references the environment.

### Creating secrets for an organization

1. Go to organization **Settings > Secrets and variables > Actions**.
2. Click **New organization secret**.
3. Configure which repositories can access the secret.

### Using secrets in a workflow

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Use secret
        env:
          DEPLOY_TOKEN: ${{ secrets.DEPLOY_TOKEN }}
        run: |
          curl -H "Authorization: token $DEPLOY_TOKEN" \
               https://api.github.com/user
```

### Using secrets with Bash

```yaml
steps:
  - name: Use secret in Bash
    env:
      TOKEN: ${{ secrets.TOKEN }}
    run: |
      echo "Token length: ${#TOKEN}"
```

### Using secrets with PowerShell

```yaml
steps:
  - name: Use secret in PowerShell
    env:
      TOKEN: ${{ secrets.TOKEN }}
    shell: pwsh
    run: |
      Write-Host "Token length: $($env:TOKEN.Length)"
```

### Storing large secrets

For secrets larger than 48 KB, encrypt and store as a base64-encoded file:

```bash
# Encrypt locally
gpg --symmetric --cipher-algo AES256 my_secret.json

# Base64 encode
base64 -i my_secret.json.gpg -o my_secret.json.gpg.base64

# Store the base64 string as a secret
```

In the workflow:

```yaml
steps:
  - name: Decrypt secret
    env:
      LARGE_SECRET_PASSPHRASE: ${{ secrets.LARGE_SECRET_PASSPHRASE }}
    run: |
      echo "$LARGE_SECRET" | base64 -d > secret.json.gpg
      gpg --quiet --batch --yes --decrypt --passphrase="$LARGE_SECRET_PASSPHRASE" \
        --output secret.json secret.json.gpg
```

### Storing Base64 binary blobs

```bash
base64 -i cert.pfx -o cert.pfx.base64
# Store the output as a secret
```

**Source**: [Using secrets in GitHub Actions](https://docs.github.com/en/actions/how-tos/security-for-github-actions/security-guides/using-secrets-in-github-actions)

---

## Security for GitHub Actions

### Security hardening for GitHub Actions

Key security practices:

- **Use OpenID Connect (OIDC)** instead of storing cloud credentials as secrets
- **Use `permissions` key** to restrict `GITHUB_TOKEN` access
- **Use third-party actions with care** — pin to full commit SHA, not tags
- **Use environment secrets** for deployment workflows
- **Use `pull_request_target` carefully** — it has access to secrets but runs on base branch
- **Review pull requests** that modify workflow files
- **Use required reviews** for environments

### Using artifact attestations

Artifact attestations establish build provenance for the software you produce. Use them to verify the software you consume.

```yaml
jobs:
  build:
    permissions:
      id-token: write
      attestations: write
      contents: read
    steps:
      - uses: actions/checkout@v6
      - uses: actions/attest-build-provenance@v2
        with:
          subject-path: ./bin/app
```

### Security hardening your deployments

Use OpenID Connect within your workflows to authenticate with your cloud provider without storing long-lived credentials.

```yaml
jobs:
  deploy:
    permissions:
      id-token: write
      contents: read
    steps:
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789012:role/my-github-actions-role
          aws-region: us-east-1
```

### GITHUB_TOKEN

The `GITHUB_TOKEN` is automatically available in every workflow run. Use the `permissions` key to restrict its access:

```yaml
permissions:
  contents: read
  pull-requests: write
  issues: write
```

**Source**: [Security for GitHub Actions](https://docs.github.com/en/actions/how-tos/security-for-github-actions/security-guides)

---

## Reusing Automations

### Custom Actions

Actions are individual tasks that you can combine to create jobs and customize your workflow. You can create your own actions, or use and customize actions shared by the GitHub community.

#### Types of actions

1. **JavaScript actions** — Run directly on the runner
2. **Composite actions** — Combine multiple steps into a single action
3. **Docker container actions** — Run in a Docker container

#### Creating a JavaScript action

Create an `action.yml` metadata file:

```yaml
name: 'My Action'
description: 'Does something useful'
inputs:
  who-to-greet:
    description: 'Who to greet'
    required: true
    default: 'World'
outputs:
  time:
    description: 'The time we greeted'
runs:
  using: 'node20'
  main: 'index.js'
```

The action code (`index.js`):

```javascript
const core = require('@actions/core');
const github = require('@actions/github');

try {
  const whoToGreet = core.getInput('who-to-greet');
  console.log(`Hello ${whoToGreet}!`);
  const time = new Date().toTimeString();
  core.setOutput('time', time);
  const payload = JSON.stringify(github.context.payload, undefined, 2);
  console.log(`Event payload: ${payload}`);
} catch (error) {
  core.setFailed(error.message);
}
```

#### Creating a composite action

```yaml
name: 'My Composite Action'
description: 'Combines multiple steps'
inputs:
  node-version:
    description: 'Node.js version'
    required: true
    default: '20'
runs:
  using: 'composite'
  steps:
    - uses: actions/setup-node@v4
      with:
        node-version: ${{ inputs.node-version }}
    - run: npm install
      shell: bash
    - run: npm test
      shell: bash
```

#### Creating a Docker container action

```yaml
name: 'Docker Action'
description: 'Runs in a container'
runs:
  using: 'docker'
  image: 'Dockerfile'
  args:
    - '--flag'
    - 'value'
  env:
    MY_VAR: 'hello'
```

### Action Metadata Syntax

The `action.yml` (or `action.yaml`) file defines the action's metadata, inputs, outputs, and execution.

#### Key fields

- `name` — Action name (required)
- `author` — Author name
- `description` — Description (required)
- `inputs` — Input parameters
- `outputs` — Output parameters
- `runs` — Execution configuration
- `branding` — Icon and color for marketplace

#### inputs.<input_id>

```yaml
inputs:
  environment:
    description: 'Target environment'
    required: true
    default: 'staging'
    deprecationMessage: 'Use deploy-target instead'
```

#### outputs (JavaScript/Docker)

```yaml
outputs:
  result:
    description: 'The result'
```

#### outputs (Composite)

```yaml
outputs:
  result:
    description: 'The result'
    value: ${{ steps.my-step.outputs.result }}
```

#### runs for JavaScript actions

```yaml
runs:
  using: 'node20'  # or 'node16'
  main: 'index.js'
  pre: 'setup.js'          # Runs before main
  pre-if: 'always()'       # Condition for pre
  post: 'cleanup.js'       # Runs after main
  post-if: 'always()'      # Condition for post
```

#### runs for composite actions

```yaml
runs:
  using: 'composite'
  steps:
    - run: echo "Hello"
      shell: bash
```

#### runs for Docker container actions

```yaml
runs:
  using: 'docker'
  image: 'Dockerfile'  # or 'docker://alpine:3.8'
  pre-entrypoint: 'setup.sh'
  entrypoint: 'main.sh'
  post-entrypoint: 'cleanup.sh'
  args: ['--flag', 'value']
  env:
    MY_VAR: 'hello'
```

#### branding

```yaml
branding:
  icon: 'heart'  # See GitHub's icon list
  color: 'red'   # white, yellow, blue, green, orange, red, purple, gray-dark
```

**Source**: [Metadata syntax reference](https://docs.github.com/en/actions/reference/workflows-and-actions/metadata-syntax)

---

## Reusable Workflows

Reusable workflows let you call a workflow from within another workflow, reducing duplication.

### Creating a reusable workflow

Use `workflow_call` as a trigger:

```yaml
on:
  workflow_call:
    inputs:
      environment:
        type: string
        required: true
      debug:
        type: boolean
        default: false
    outputs:
      deployment-url:
        description: 'URL of the deployment'
        value: ${{ jobs.deploy.outputs.url }}
    secrets:
      deploy-token:
        required: true
```

### Calling a reusable workflow

```yaml
jobs:
  deploy:
    uses: ./.github/workflows/deploy.yml
    with:
      environment: production
      debug: false
    secrets:
      deploy-token: ${{ secrets.DEPLOY_TOKEN }}
```

### Passing inputs and secrets

Inputs and secrets are passed explicitly. Use `secrets: inherit` to pass all secrets:

```yaml
jobs:
  call-workflow:
    uses: ./.github/workflows/reusable.yml
    secrets: inherit
```

### Using a matrix strategy with a reusable workflow

```yaml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    uses: ./.github/workflows/test.yml
    with:
      os: ${{ matrix.os }}
```

### Nesting reusable workflows

Reusable workflows can call other reusable workflows, up to 4 levels deep.

### Using outputs from a reusable workflow

```yaml
jobs:
  deploy:
    uses: ./.github/workflows/deploy.yml
  report:
    needs: deploy
    runs-on: ubuntu-latest
    steps:
      - run: echo "Deployed to ${{ needs.deploy.outputs.deployment-url }}"
```

**Source**: [Reuse workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
