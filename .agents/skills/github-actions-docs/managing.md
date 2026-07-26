# GitHub Actions — Managing Workflows, Deployment & Reference

## Managing Workflow Runs

### Manually running a workflow

Workflows with `workflow_dispatch` trigger can be run manually from the Actions tab:

1. Navigate to the repository's **Actions** tab.
2. Select the workflow in the left sidebar.
3. Click **Run workflow**.
4. Select the branch and fill in any inputs.
5. Click **Run workflow**.

### Re-running workflows and jobs

1. Navigate to the workflow run.
2. Click **Re-run all jobs** or **Re-run failed jobs**.
3. You can also re-run individual jobs by clicking the job's re-run button.

### Canceling a workflow run

1. Navigate to the workflow run.
2. Click **Cancel workflow**.

### Disabling and enabling a workflow

1. Go to the **Actions** tab.
2. Select the workflow.
3. Click the **...** menu and select **Disable workflow**.
4. To re-enable, select **Enable workflow**.

### Skipping workflow runs

Add `[skip ci]` or `[ci skip]` to a commit message to skip workflow runs triggered by that commit.

### Deleting a workflow run

1. Navigate to the workflow run.
2. Click the **...** menu and select **Delete run**.

### Downloading workflow artifacts

1. Navigate to the workflow run.
2. Scroll to the **Artifacts** section.
3. Click the artifact name to download.

### Removing workflow artifacts

Artifacts can be deleted via the UI or API. You can also set retention policies.

### Managing caches

View and manage workflow caches under **Settings > Actions > Caches**. Caches can be deleted manually or via API.

### Approving workflow runs from forks

For public repositories, you can require manual approval for workflow runs triggered by pull requests from forks:

1. Go to **Settings > Actions > General**.
2. Under **Fork pull request workflows in private repositories**, select **Send tokens to workflows from pull requests**.
3. Enable **Require approval for all outside collaborators**.

**Source**: [Managing workflow runs](https://docs.github.com/en/actions/how-tos/manage-workflow-runs)

---

## Adding a Workflow Status Badge

Display a status badge in your repository README to indicate workflow status:

```markdown
![Workflow Status](https://github.com/owner/repo/actions/workflows/workflow.yml/badge.svg)
```

Or using the badge URL with branch:

```markdown
![CI](https://github.com/owner/repo/actions/workflows/ci.yml/badge.svg?branch=main)
```

**Source**: [Adding a workflow status badge](https://docs.github.com/en/actions/how-tos/monitor-workflows/add-a-status-badge)

---

## Adding Labels to Issues

You can use GitHub Actions to automatically label issues:

```yaml
name: Label issues
on:
  issues:
    types: [opened]
jobs:
  label:
    runs-on: ubuntu-latest
    permissions:
      issues: write
    steps:
      - uses: actions/github-script@v7
        with:
          script: |
            github.rest.issues.addLabels({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              labels: ['triage']
            })
```

**Source**: [Adding labels to issues](https://docs.github.com/en/actions/tutorials/manage-your-work/add-labels-to-issues)

---

## Adding Scripts to Your Workflow

```yaml
steps:
  - name: Run a script
    run: |
      echo "Running script"
      ./scripts/build.sh
    shell: bash

  - name: Run Python script
    shell: python
    run: |
      import sys
      print(f"Python {sys.version}")

  - name: Run inline Node.js
    shell: node
    run: |
      console.log("Hello from Node.js");
```

**Source**: [Adding scripts to your workflow](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/add-scripts)

---

## Deploying to Third-Party Platforms

GitHub Actions provides deployment guides for various platforms:

### Azure App Service

- **Node.js** — Deploy Node.js apps to Azure App Service
- **Python** — Deploy Python apps to Azure App Service
- **Java** — Deploy Java apps to Azure App Service
- **.NET** — Deploy .NET apps to Azure App Service
- **PHP** — Deploy PHP apps to Azure App Service
- **Docker** — Deploy Docker containers to Azure App Service

### Other Azure services

- **Azure Static Web App** — Deploy static sites
- **Azure Kubernetes Service (AKS)** — Deploy to AKS clusters

### Other cloud providers

- **Amazon Elastic Container Service (ECS)** — Deploy to AWS ECS
- **Google Kubernetes Engine (GKE)** — Deploy to GKE

### Apple certificate on macOS runners

Install an Apple certificate on macOS runners for Xcode development to sign applications.

**Source**: [Deploying to third-party platforms](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms)

---

## Actions Limits

### Job concurrency limits for GitHub-hosted runners

GitHub Support can increase job concurrency limits. The maximum concurrent macOS jobs is shared across standard GitHub-hosted runners and GitHub-hosted larger runners.

### Storage limits

GitHub Support cannot increase storage limits for GitHub Actions. Cache storage limits have specific eviction policies.

### Execution time limits

- Job execution time: 6 hours (GitHub-hosted), 72 hours (self-hosted, limited by GITHUB_TOKEN expiry at 24h)
- Workflow run time: 35 days
- Job queue time: 24 hours
- API requests per hour per repository: 15,000 (GitHub Enterprise), 1,000 (Free/Pro/Team)

### Matrix limits

A matrix generates a maximum of 256 jobs per workflow run.

### Workflow limits

- Maximum workflows per repository: 500 (GitHub-hosted), unlimited (self-hosted)
- Maximum action.yml size: 1 MB
- Maximum workflow run name: 255 characters

### Docker Hub rate limits

Docker Hub imposes rate limits on pulls from GitHub Actions. Use GitHub Container Registry or authenticate with Docker Hub to mitigate.

**Source**: [Actions limits](https://docs.github.com/en/actions/reference/limits)

---

## About GitHub Actions Metrics

GitHub Actions metrics are available for organizations and repositories, providing insights into:

- Workflow run success/failure rates
- Job execution times
- Usage minutes by runner type
- Queue times
- Concurrency utilization

Access via **Settings > Actions > Metrics** (organization level) or repository Insights.

**Source**: [About GitHub Actions metrics](https://docs.github.com/en/actions/concepts/metrics)
