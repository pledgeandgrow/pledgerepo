# GitHub Actions — Environments, Debug Logging, Billing, API, GITHUB_TOKEN & Migration

## Deployment Environments

Environments allow you to control the deployment of your workflows with protection rules and secrets scoped to each environment.

### Prerequisites

- Environments, environment secrets, and deployment protection rules are available in **public repositories** for all current GitHub plans
- For **private or internal repositories**, you need GitHub Pro, GitHub Team, or GitHub Enterprise
- Users with GitHub Free can only configure environments for public repositories

### Creating an environment

1. Navigate to your repository on GitHub.
2. Go to **Settings > Environments**.
3. Click **New environment**.
4. Enter a name (max 255 characters, case insensitive, unique within repository).
5. Click **Configure environment**.

### Protection rules

#### Required reviewers

Require approval before a job using the environment can proceed:

1. Select **Required reviewers**.
2. Enter up to **6 people or teams**. Only one approval is needed.
3. Optionally select **Prevent self-review** to prevent users from approving their own runs.
4. Click **Save protection rules**.

#### Wait timer

Add a delay before jobs proceed:

1. Select **Wait timer**.
2. Enter the number of minutes to wait.
3. Click **Save protection rules**.

#### Branch and tag restrictions

Restrict which branches/tags can deploy to this environment:

1. Select the desired option in the **Deployment branches** dropdown (All branches, Protected branches, or Selected branches and tags).
2. If "Selected", click **Add deployment branch or tag rule**.
3. Choose **Branch** or **Tag** as the ref type.
4. Enter the name pattern.
5. Click **Add rule**.

#### Custom protection rules

Enable custom deployment protection rules created with GitHub Apps:

1. Select the custom protection rule you want to enable.
2. Click **Save protection rules**.

#### Prevent admin bypass

1. Deselect **Allow administrators to bypass configured protection rules**.
2. Click **Save protection rules**.

### Environment secrets

Secrets scoped to a specific environment. Only available to jobs that reference the environment, and only after protection rules pass.

1. Under **Environment secrets**, click **Add Secret**.
2. Enter the secret name and value.
3. Click **Add secret**.

### Environment variables

Variables scoped to a specific environment, accessible via the `vars` context.

1. Under **Environment variables**, click **Add Variable**.
2. Enter the variable name and value.
3. Click **Add variable**.

### Using environments in workflows

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: production
      url: ${{ steps.deploy.outputs.url }}
    steps:
      - name: Deploy
        run: ./deploy.sh
```

### Deleting an environment

1. Go to **Settings > Environments**.
2. Click the environment name.
3. Click **Delete environment**.

### How environments relate to deployments

When a job references an environment, a deployment object is created on GitHub. This provides a visual history of deployments to that environment, visible under the repository's deployments page.

**Source**: [Managing environments for deployment](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)

---

## Debug Logging

### Enabling runner diagnostic logging

Provides additional log files about how a runner executes a job. Two extra log files are added:

- **Runner process log** — Coordinating and setting up runners to execute jobs
- **Worker process log** — Logs the execution of a job

To enable:

1. Set the following secret or variable in the repository: `ACTIONS_RUNNER_DEBUG` to `true`.
2. Download the log archive of the workflow run. Diagnostic logs are in the `runner-diagnostic-logs` folder.

### Enabling step debug logging

Increases the verbosity of a job's logs during and after execution.

1. Set the following secret or variable in the repository: `ACTIONS_STEP_DEBUG` to `true`.
2. More debug events will appear in the step logs.

### Using runner.debug context

Conditionally run steps only when debug logging is enabled:

```yaml
steps:
  - if: ${{ runner.debug }}
    run: echo "Debug logging is enabled"
```

Note: Anyone who has access to run a workflow can enable debug logging for a re-run without needing to set secrets.

**Source**: [Enabling debug logging](https://docs.github.com/en/actions/how-tos/monitor-workflows/enable-debug-logging)

---

## Billing & Usage

### Minute limits per OS

GitHub Actions usage is billed per minute, with different rates per operating system:

| OS | Multiplier |
|----|-----------|
| Linux | 1x |
| Windows | 2x |
| macOS | 10x |

### Included minutes

| Plan | Included minutes/month |
|------|----------------------|
| GitHub Free | 2,000 (private repos only) |
| GitHub Pro | 3,000 |
| GitHub Team | 3,000 |
| GitHub Enterprise | 50,000 |

### Usage limits

- **Job execution time**: 6 hours (GitHub-hosted), 72 hours (self-hosted)
- **Workflow run time**: 35 days
- **Job queue time**: 24 hours
- **API requests**: 15,000/hour (Enterprise), 1,000/hour (Free/Pro/Team)
- **Matrix jobs**: max 256 per workflow run
- **Concurrency**: varies by plan (see Actions limits)

### Viewing usage

- **Organization**: Settings > Billing & plans > Actions usage
- **Repository**: Insights > Actions usage
- **Workflow**: API endpoint `GET /repos/{owner}/{repo}/actions/workflows/{workflow_id}/timing`

### Storage billing

- Artifact and cache storage is billed separately from compute minutes
- Storage limits cannot be increased by GitHub Support
- Cache eviction: caches not accessed in 7 days are evicted

**Source**: [About billing for GitHub Actions](https://docs.github.com/en/billing/managing-billing-for-github-actions/about-billing-for-github-actions)

---

## GITHUB_TOKEN Authentication

The `GITHUB_TOKEN` is automatically available in every workflow run. Use it to authenticate API calls and interact with GitHub.

### Using the GITHUB_TOKEN

Reference it with standard secret syntax: `${{ secrets.GITHUB_TOKEN }}`

### Example 1: Passing the GITHUB_TOKEN as an input (GitHub CLI)

```yaml
name: Open new issue
on:
  workflow_dispatch:
jobs:
  open-issue:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      issues: write
    steps:
      - run: |
          gh issue --repo ${{ github.repository }} \
            create --title "Issue title" --body "Issue body"
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Example 2: Calling the REST API

```yaml
name: Create issue on commit
on: [push]
jobs:
  create_issue:
    runs-on: ubuntu-latest
    permissions:
      issues: write
    steps:
      - name: Create issue using REST API
        run: |
          curl --request POST \
            --url https://api.github.com/repos/${{ github.repository }}/issues \
            --header 'authorization: Bearer ${{ secrets.GITHUB_TOKEN }}' \
            --header 'content-type: application/json' \
            --data '{
              "title": "Automated issue for commit: ${{ github.sha }}",
              "body": "This issue was automatically created by the GitHub Action workflow **${{ github.workflow }}**.\n\n The commit hash was: _${{ github.sha }}_."
            }' \
            --fail
```

### Modifying permissions

Use the `permissions` key to restrict the `GITHUB_TOKEN` to least required access:

```yaml
permissions:
  contents: read
  issues: write
  pull-requests: write
```

Available permissions include: `actions`, `checks`, `contents`, `deployments`, `id-token`, `issues`, `packages`, `pages`, `pull-requests`, `repository-projects`, `security-events`, `statuses`, and `discussions`.

### Granting additional permissions

If you need more permissions than `GITHUB_TOKEN` provides:

1. **Create a GitHub App** and generate an installation access token within your workflow
2. **Create a personal access token**, store it as a secret, and use `${{ secrets.SECRET_NAME }}`

### Security best practices

- Always limit `GITHUB_TOKEN` permissions to the minimum required
- Actions can access `GITHUB_TOKEN` via `github.token` context even without explicit passing
- Use `permissions` key at workflow or job level

**Source**: [Use GITHUB_TOKEN for authentication](https://docs.github.com/en/actions/tutorials/authenticate-with-github_token)

---

## GitHub Actions REST API

### Workflows API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/repos/{owner}/{repo}/actions/workflows` | GET | List repository workflows |
| `/repos/{owner}/{repo}/actions/workflows/{workflow_id}` | GET | Get a workflow |
| `/repos/{owner}/{repo}/actions/workflows/{workflow_id}` | PUT | Enable a workflow |
| `/repos/{owner}/{repo}/actions/workflows/{workflow_id}` | DELETE | Disable a workflow |
| `/repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches` | POST | Create a workflow dispatch event |
| `/repos/{owner}/{repo}/actions/workflows/{workflow_id}/timing` | GET | Get workflow usage |

### Workflow runs API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/repos/{owner}/{repo}/actions/runs` | GET | List workflow runs |
| `/repos/{owner}/{repo}/actions/runs/{run_id}` | GET | Get a workflow run |
| `/repos/{owner}/{repo}/actions/runs/{run_id}` | DELETE | Delete a workflow run |
| `/repos/{owner}/{repo}/actions/runs/{run_id}/rerun` | POST | Re-run a workflow |
| `/repos/{owner}/{repo}/actions/runs/{run_id}/cancel` | POST | Cancel a workflow run |
| `/repos/{owner}/{repo}/actions/runs/{run_id}/approve` | POST | Approve a workflow run |
| `/repos/{owner}/{repo}/actions/runs/{run_id}/jobs` | GET | List jobs for a workflow run |
| `/repos/{owner}/{repo}/actions/runs/{run_id}/logs` | GET | Download workflow run logs |

### Artifacts API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/repos/{owner}/{repo}/actions/artifacts` | GET | List artifacts for a repository |
| `/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts` | GET | List artifacts for a workflow run |
| `/repos/{owner}/{repo}/actions/artifacts/{artifact_id}` | GET | Get an artifact |
| `/repos/{owner}/{repo}/actions/artifacts/{artifact_id}/{archive_format}` | GET | Download an artifact |
| `/repos/{owner}/{repo}/actions/artifacts/{artifact_id}` | DELETE | Delete an artifact |

### Cache API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/repos/{owner}/{repo}/actions/caches` | GET | List GitHub Actions caches |
| `/repos/{owner}/{repo}/actions/caches/{cache_id}` | DELETE | Delete a GitHub Actions cache |

### Secrets API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/repos/{owner}/{repo}/actions/secrets` | GET | List repository secrets |
| `/repos/{owner}/{repo}/actions/secrets/{secret_name}` | GET | Get a repository secret |
| `/repos/{owner}/{repo}/actions/secrets/{secret_name}` | PUT | Create or update a repository secret |
| `/repos/{owner}/{repo}/actions/secrets/{secret_name}` | DELETE | Delete a repository secret |

### Runners API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/repos/{owner}/{repo}/actions/runners` | GET | List repository runners |
| `/repos/{owner}/{repo}/actions/runners/registration-token` | POST | Create a registration token |
| `/repos/{owner}/{repo}/actions/runners/{runner_id}` | DELETE | Remove a self-hosted runner |

### Triggering a workflow via API

```bash
curl --request POST \
  --url https://api.github.com/repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches \
  --header 'authorization: Bearer $TOKEN' \
  --header 'content-type: application/json' \
  --data '{
    "ref": "main",
    "inputs": {
      "environment": "staging"
    }
  }'
```

**Source**: [REST API endpoints for workflows](https://docs.github.com/en/rest/actions/workflows)

---

## Migrating to GitHub Actions

### General migration approach

1. **Map CI/CD concepts** — Identify equivalent constructs (jobs, steps, pipelines, stages)
2. **Convert build triggers** — Map webhook events, schedules, manual triggers
3. **Convert environment variables and secrets** — Move credentials to GitHub secrets
4. **Convert build steps** — Translate scripts and commands to `run` steps or actions
5. **Test the workflow** — Run on a branch before merging to main

### Migrating from Jenkins

| Jenkins | GitHub Actions |
|---------|---------------|
| Pipeline / Job | Workflow |
| Stage | Job |
| Step | Step |
| `agent` | `runs-on` |
| `environment` | `env` |
| `when` | `if` |
| `post` | `if: always()` / `if: failure()` |
| Jenkinsfile | `.github/workflows/*.yml` |
| Jenkins credentials | GitHub secrets |
| Shared libraries | Reusable workflows / composite actions |
| `sh` | `run` |
| `bat` | `run` (Windows runner) |
| `archiveArtifacts` | `actions/upload-artifact` |
| `input` | `workflow_dispatch` inputs or environment required reviewers |

### Migrating from CircleCI

| CircleCI | GitHub Actions |
|----------|---------------|
| `.circleci/config.yml` | `.github/workflows/*.yml` |
| Workflows | Workflows |
| Jobs | Jobs |
| Steps | Steps |
| `docker:` | `container:` or `runs-on` |
| `orbs` | Reusable workflows / marketplace actions |
| `parameters` | `inputs` (workflow_call/workflow_dispatch) |
| `store_artifacts` | `actions/upload-artifact` |
| `persist_to_workspace` | `actions/upload-artifact` + `actions/download-artifact` |
| `cache` | `actions/cache` |
| `filters` | `on` with branch/path filters |
| `requires` | `needs` |
| `matrix` | `strategy.matrix` |

### Migrating from GitLab CI

| GitLab CI | GitHub Actions |
|-----------|---------------|
| `.gitlab-ci.yml` | `.github/workflows/*.yml` |
| Stages | Jobs with `needs` |
| `script` | `run` |
| `image` | `container` or `runs-on` |
| `only`/`except` | `on` with filters |
| `artifacts` | `actions/upload-artifact` |
| `cache` | `actions/cache` |
| `variables` | `env` / `vars` |
| `include` | Reusable workflows |
| `rules` | `if` |
| `parallel` | `strategy.matrix` |
| `environment` | `environment` |

### Migrating from Azure Pipelines

| Azure Pipelines | GitHub Actions |
|----------------|---------------|
| `azure-pipelines.yml` | `.github/workflows/*.yml` |
| Stages | Jobs with `needs` |
| Jobs | Jobs |
| Steps | Steps |
| `pool` | `runs-on` |
| `variables` | `env` / `vars` |
| `template` | Reusable workflows |
| `PublishBuildArtifact` | `actions/upload-artifact` |
| `DownloadBuildArtifact` | `actions/download-artifact` |
| `Cache` | `actions/cache` |
| `condition` | `if` |
| `strategy.matrix` | `strategy.matrix` |

**Source**: [Migrating to GitHub Actions](https://docs.github.com/en/actions/migrating-to-github-actions)

---

## Using Service Containers for Database Testing

Service containers run alongside your job to provide databases and other services.

### PostgreSQL example

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v6
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - name: Install dependencies
        run: pip install -r requirements.txt
      - name: Run tests
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/test
        run: pytest
```

### Redis example

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      redis:
        image: redis:7
        ports:
          - 6379:6379
        options: --health-cmd "redis-cli ping" --health-interval 10s --health-timeout 5s --health-retries 5
    steps:
      - uses: actions/checkout@v6
      - run: npm test
        env:
          REDIS_URL: redis://localhost:6379
```

### Key notes

- Service containers are only supported on **Linux runners** (`ubuntu-latest`)
- Use `localhost` to connect to service containers (port mapping)
- Use `--health-cmd` to ensure the service is ready before steps run
- For Windows/macOS runners, use a `container` job or run services via `docker run`

**Source**: [About service containers](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container)
