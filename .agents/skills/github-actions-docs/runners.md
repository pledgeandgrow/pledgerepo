# GitHub Actions — Runners

## GitHub-Hosted Runners

GitHub provides Ubuntu Linux, Windows, and macOS virtual machines to run your workflows. Each workflow run executes in a fresh, newly-provisioned virtual machine.

### Standard GitHub-hosted runners

| Runner | Label | Specs |
|--------|-------|-------|
| Ubuntu | `ubuntu-latest` (or `ubuntu-24.04`, `ubuntu-22.04`) | 4-core CPU, 16 GB RAM, 14 GB SSD |
| Windows | `windows-latest` (or `windows-2022`) | 4-core CPU, 16 GB RAM, 14 GB SSD |
| macOS | `macos-latest` (or `macos-15`, `macos-14`) | 3-core CPU, 14 GB RAM, 14 GB SSD |

### Usage

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - run: npm test
```

### Viewing available runners

You can view available runners for a repository under Settings > Actions > Runners.

### Private repositories

For private repositories, GitHub-hosted runners have different concurrency limits and billing rates depending on your plan.

**Source**: [Using GitHub-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/github-hosted-runners/use-github-hosted-runners)

---

## Larger Runners

GitHub offers larger runners with more CPU, memory, and storage. Available in larger configurations for demanding workloads.

### Managing larger runners

Larger runners can be managed at organization or enterprise level. You can:

- **Manage larger runners** — Create and configure larger runner machines
- **Control access** — Restrict which repositories can use larger runners
- **Run jobs on larger runners** — Use `runs-on` with custom labels
- **Use custom images** — Configure custom OS images

```yaml
jobs:
  heavy-build:
    runs-on: larger-runner
    steps:
      - uses: actions/checkout@v6
```

**Source**: [Using larger runners](https://docs.github.com/en/actions/how-tos/manage-runners/larger-runners)

---

## Self-Hosted Runners

You can host your own runners to run workflows. Self-hosted runners can be added to a repository, an organization, or an enterprise.

### Adding a self-hosted runner to a repository

1. Navigate to your repository on GitHub.
2. Go to **Settings > Actions > Runners > New self-hosted runner**.
3. Select the operating system and architecture.
4. Download and configure the runner application:

```bash
# Create a folder
mkdir actions-runner && cd actions-runner

# Download the latest runner package
curl -o actions-runner-linux-x64-2.321.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.321.0/actions-runner-linux-x64-2.321.0.tar.gz

# Extract the installer
tar xzf actions-runner-linux-x64-2.321.0.tar.gz

# Configure the runner
./config.sh --url https://github.com/owner/repo --token TOKEN

# Run the runner
./run.sh
```

### Adding to an organization

1. Go to organization **Settings > Actions > Runners > New runner**.
2. Follow the same download and configuration steps.

### Adding to an enterprise

1. Go to enterprise **Settings > Actions > Runners > New runner**.

### Using self-hosted runners in workflows

```yaml
jobs:
  build:
    runs-on: [self-hosted, linux, x64]
    steps:
      - uses: actions/checkout@v6
```

### Runner labels

Self-hosted runners can have custom labels. Use labels to target specific runners:

```yaml
runs-on: [self-hosted, gpu, linux]
```

### Running as a service

Install the runner as a service for persistent execution:

```bash
sudo ./svc.sh install
sudo ./svc.sh start
```

### Checking that your runner was added

After configuration, the runner appears under **Settings > Actions > Runners** in your repository/organization. The runner must be idle and online to pick up jobs.

**Source**: [Adding self-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/add-runners)

---

## Actions Runner Controller (ARC)

Actions Runner Controller is a Kubernetes operator that lets you run self-hosted runners on your Kubernetes cluster. ARC manages runner pods that scale up and down based on workflow demand.

Key features:
- Autoscaling based on pending workflow jobs
- Runner pods are ephemeral — created per job and destroyed after
- Supports GitHub-hosted and self-hosted runner modes
- Integrates with your existing Kubernetes infrastructure

**Source**: [Actions Runner Controller](https://docs.github.com/en/actions/concepts/runners/actions-runner-controller)
