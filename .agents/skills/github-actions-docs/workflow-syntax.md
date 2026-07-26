# GitHub Actions — Workflow Syntax Reference

## About YAML Syntax for Workflows

Workflow files use YAML syntax, and must have either a `.yml` or `.yaml` file extension. You must store workflow files in the `.github/workflows` directory of your repository.

## Top-level keys

### name

The name of the workflow. GitHub displays the names of your workflows under your repository's "Actions" tab. If omitted, GitHub displays the workflow file path relative to the root of the repository.

```yaml
name: CI
```

### run-name

The name for workflow runs generated from the workflow. Can include expressions and can reference the `github` and `inputs` contexts.

```yaml
run-name: Deploy to ${{ inputs.environment }} by ${{ github.actor }}
```

### on

To automatically trigger a workflow, use `on` to define which events can cause the workflow to run.

```yaml
# Single event
on: push

# Multiple events
on: [push, pull_request]

# With activity types and filters
on:
  pull_request:
    branches: [main]
    types: [opened, synchronize]
```

### on.<event_name>.types

Specifies activity types that trigger a workflow run. By default, all activity types trigger workflows.

```yaml
on:
  issues:
    types: [opened, labeled]
```

### on.<pull_request|pull_request_target>.<branches|branches-ignore>

Filter by branch name. Use `branches` to include or `branches-ignore` to exclude.

```yaml
on:
  pull_request:
    branches: [main, 'releases/**']
```

### on.push.<branches|tags|branches-ignore|tags-ignore>

Filter push events by branch or tag.

```yaml
on:
  push:
    branches: [main]
    tags: ['v*']
```

### on.<push|pull_request>.<paths|paths-ignore>

Filter by file paths.

```yaml
on:
  push:
    paths: ['**.js', 'src/**']
    paths-ignore: ['docs/**']
```

### on.schedule

Trigger on a schedule using POSIX cron syntax.

```yaml
on:
  schedule:
    - cron: '30 5 * * 1,3'
```

### on.workflow_call

Makes a workflow reusable. Define inputs, outputs, and secrets.

```yaml
on:
  workflow_call:
    inputs:
      environment:
        type: string
        required: true
    secrets:
      deploy-token:
        required: true
```

### on.workflow_dispatch

Allows manual triggering from the Actions tab.

```yaml
on:
  workflow_dispatch:
    inputs:
      debug:
        type: boolean
        default: false
```

### permissions

Defines access for the `GITHUB_TOKEN` scopes.

```yaml
permissions:
  contents: read
  pull-requests: write
```

### env

Map of environment variables available to all jobs and steps in the workflow.

```yaml
env:
  NODE_VERSION: 20
```

### defaults

Default settings for all jobs in the workflow.

```yaml
defaults:
  run:
    shell: bash
    working-directory: ./scripts
```

### concurrency

Only one concurrent run per group at a time. Cancel in-progress runs by default.

```yaml
concurrency:
  group: ${{ github.ref }}
  cancel-in-progress: true
```

---

## Jobs

### jobs

A workflow run is made up of one or more jobs, which run in parallel by default. To run jobs sequentially, define dependencies with `needs`.

### jobs.<job_id>

A unique identifier for the job. Must start with a letter or `_` and contain only alphanumeric characters, `-`, or `_`.

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Building"
```

### jobs.<job_id>.name

The name displayed on GitHub for the job.

### jobs.<job_id>.needs

Jobs that must complete successfully before this job runs.

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps: [...]
  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps: [...]
```

### jobs.<job_id>.if

Conditional execution. Uses expression syntax.

```yaml
jobs:
  deploy:
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
```

### jobs.<job_id>.runs-on

The type of machine to run the job on. Can be a string, array, or key:value pair.

```yaml
runs-on: ubuntu-latest
# or
runs-on: [self-hosted, linux, x64]
# or
runs-on:
  group: my-runner-group
  labels: [linux, x64]
```

### jobs.<job_id>.environment

The environment the job references.

```yaml
environment:
  name: production
  url: ${{ steps.deploy.outputs.url }}
```

### jobs.<job_id>.outputs

Map of outputs for the job. Available to dependent jobs.

```yaml
outputs:
  build-id: ${{ steps.build.outputs.id }}
```

### jobs.<job_id>.env

Environment variables for the job.

### jobs.<job_id>.timeout-minutes

Maximum minutes before automatic cancellation. Default: 360.

### jobs.<job_id>.strategy

Strategy matrix for running multiple job variations.

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    node: [18, 20, 22]
    exclude:
      - os: windows-latest
        node: 18
    include:
      - os: ubuntu-latest
        node: 22
        experimental: true
    fail-fast: false
    max-parallel: 2
```

### jobs.<job_id>.continue-on-error

Prevents a job failure from failing the workflow run.

### jobs.<job_id>.container

Run the job inside a Docker container.

```yaml
container:
  image: node:20
  env:
    NODE_ENV: test
  ports: [8080]
  volumes:
    - my_docker_volume:/volume_mount
  options: --cpus 1
```

### jobs.<job_id>.services

Run service containers alongside the job.

```yaml
services:
  redis:
    image: redis
    ports: [6379]
    options: >-
      --health-cmd "redis-cli ping"
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5
```

---

## Steps

### jobs.<job_id>.steps

A sequence of tasks called steps. Each step runs in its own process.

```yaml
steps:
  - name: Checkout
    uses: actions/checkout@v6
  - name: Install
    run: npm install
  - name: Test
    run: npm test
```

### jobs.<job_id>.steps[*].id

Unique identifier for the step. Used to reference step outputs.

### jobs.<job_id>.steps[*].if

Conditional execution.

```yaml
- if: github.event_name == 'pull_request'
  run: echo "PR event"
```

### jobs.<job_id>.steps[*].uses

Selects an action to run.

```yaml
# Versioned action
uses: actions/checkout@v6

# Public action in subdirectory
uses: actions/aws/ec2@main

# Action in same repository
uses: ./.github/actions/my-action

# Docker Hub action
uses: docker://alpine:3.8
```

### jobs.<job_id>.steps[*].run

Runs a command-line program using the runner's shell.

```yaml
- run: |
    echo "Step 1"
    echo "Step 2"
```

### jobs.<job_id>.steps[*].shell

Specify the shell to use.

```yaml
- shell: bash
  run: echo "bash"
- shell: pwsh
  run: echo "pwsh"
- shell: python
  run: |
    import sys
    print(sys.version)
```

### jobs.<job_id>.steps[*].with

Map of input parameters for the action.

```yaml
- uses: actions/checkout@v6
  with:
    fetch-depth: 0
```

### jobs.<job_id>.steps[*].env

Environment variables for the step.

### jobs.<job_id>.steps[*].continue-on-error

Prevents step failure from failing the job. Default: `false`.

### jobs.<job_id>.steps[*].timeout-minutes

Maximum minutes before the step is killed. Default: 360.

### jobs.<job_id>.steps[*].parallel

Run steps in parallel (new feature).

```yaml
- parallel:
  - run: echo "parallel step 1"
  - run: echo "parallel step 2"
```

---

## Reusable Workflows

### jobs.<job_id>.uses

Calls a reusable workflow.

```yaml
jobs:
  reusable:
    uses: ./.github/workflows/reusable.yml
    with:
      environment: production
    secrets:
      deploy-token: ${{ secrets.DEPLOY_TOKEN }}
```

### jobs.<job_id>.secrets.inherit

Inherits all secrets from the calling workflow.

```yaml
secrets: inherit
```

---

## Filter Pattern Cheat Sheet

### Patterns to match branches and tags

- `*` — Matches zero or more characters but does not match `/`
- `**` — Matches zero or more characters including `/`
- `?` — Matches zero or one of the preceding character
- `+` — Matches one or more of the preceding character
- `[abc]` — Matches any one character in brackets
- `!` — Negates a pattern

### Patterns to match file paths

- `*` — Matches any character except `/`
- `**` — Matches any character including `/`
- `?` — Matches zero or one of preceding character
- `+` — Matches one or more of preceding character
- `[abc]` — Matches any one character in brackets

**Source**: [Workflow syntax for GitHub Actions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)
