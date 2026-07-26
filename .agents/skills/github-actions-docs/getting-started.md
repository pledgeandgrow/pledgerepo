# GitHub Actions — Getting Started

## Understanding GitHub Actions

GitHub Actions is a continuous integration and continuous delivery (CI/CD) platform that allows you to automate your build, test, and deployment pipeline. You can create workflows that build and test every pull request to your repository, or deploy merged pull requests to production.

GitHub Actions goes beyond just DevOps and lets you run workflows when other events happen in your repository. For example, you can run a workflow to automatically add the appropriate labels whenever someone creates a new issue.

GitHub provides Linux, Windows, and macOS virtual machines to run your workflows, or you can host your own self-hosted runners in your own data center or cloud infrastructure.

### The Components of GitHub Actions

#### Workflows

A workflow is a configurable automated process that will run one or more jobs. Workflows are defined by a YAML file checked in to your repository and will run when triggered by an event in your repository, or they can be triggered manually, or at a defined schedule.

Workflows are defined in the `.github/workflows` directory in a repository. A repository can have multiple workflows, each of which can perform a different set of tasks such as:

- Building and testing pull requests
- Deploying your application every time a release is created
- Adding a label whenever a new issue is opened

You can reference a workflow within another workflow (Reuse workflows).

#### Events

An event is a specific activity in a repository that triggers a workflow run. For example, an activity can originate from GitHub when someone creates a pull request, opens an issue, or pushes a commit to a repository. You can also trigger a workflow to run on a schedule, by posting to a REST API, or manually.

#### Jobs

A job is a set of steps in a workflow that is executed on the same runner. Each step is either a shell script that will be executed, or an action that will be run. Steps are executed in order and are dependent on each other. Since each step is executed on the same runner, you can share data from one step to another.

You can configure a job's dependencies with other jobs; by default, jobs have no dependencies and run in parallel. When a job takes a dependency on another job, it waits for the dependent job to complete before running.

You can also use a matrix to run the same job multiple times, each with a different combination of variables—like operating systems or language versions.

#### Actions

An action is a pre-defined, reusable set of jobs or code that performs specific tasks within a workflow, reducing the amount of repetitive code you write in your workflow files. Actions can perform tasks such as:

- Pulling your Git repository from GitHub
- Setting up the correct toolchain for your build environment
- Setting up authentication to your cloud provider

You can write your own actions, or you can find actions to use in your workflows in the GitHub Marketplace.

#### Runners

A runner is a server that runs your workflows when they're triggered. Each runner can run a single job at a time. GitHub provides Ubuntu Linux, Microsoft Windows, and macOS runners to run your workflows. Each workflow run executes in a fresh, newly-provisioned virtual machine.

GitHub also offers larger runners, which are available in larger configurations. If you need a different operating system or require a specific hardware configuration, you can host your own runners.

**Source**: [Understanding GitHub Actions](https://docs.github.com/en/actions/get-started/understanding-github-actions)

---

## Quickstart

### Prerequisites

- Basic knowledge of how to use GitHub
- A repository on GitHub where you can add files
- Access to GitHub Actions (if the Actions tab is not displayed, Actions may be disabled for the repository)

### Creating your first workflow

1. In your repository on GitHub, create a workflow file called `github-actions-demo.yml` in the `.github/workflows` directory.
2. Copy the following YAML contents into the file:

```yaml
name: GitHub Actions Demo
run-name: ${{ github.actor }} is testing out GitHub Actions 🚀
on: [push]
jobs:
  Explore-GitHub-Actions:
    runs-on: ubuntu-latest
    steps:
      - run: echo "🎉 The job was automatically triggered by a ${{ github.event_name }} event."
      - run: echo "🐧 This job is now running on a ${{ runner.os }} server hosted by GitHub!"
      - run: echo "🔎 The name of your branch is ${{ github.ref }} and your repository is ${{ github.repository }}."
      - name: Check out repository code
        uses: actions/checkout@v6
      - run: echo "💡 The ${{ github.repository }} repository has been cloned to the runner."
      - run: echo "🖥️ The workflow is now ready to test your code on the runner."
      - name: List files in the repository
        run: |
          ls ${{ github.workspace }}
      - run: echo "🍏 This job's status is ${{ job.status }}."
```

3. Click **Commit changes**.
4. Committing the workflow file to a branch triggers the `push` event and runs your workflow.

### Viewing your workflow results

1. On GitHub, navigate to the main page of the repository.
2. Under your repository name, click **Actions**.
3. In the left sidebar, click the workflow you want to display.
4. From the list of workflow runs, click the name of the run you want to see.
5. In the left sidebar, under **Jobs**, click the job name.
6. The log shows how each step was processed. Expand any step to view details.

**Source**: [Quickstart for GitHub Actions](https://docs.github.com/en/actions/get-started/quickstart)

---

## Using Workflow Templates

GitHub provides workflow templates for a variety of languages and tooling. You can use these templates as a starting point for your own workflows.

In your repository, go to the **Actions** tab, and GitHub will suggest templates based on your repository's language and existing workflows.

**Source**: [Using workflow templates](https://docs.github.com/en/actions/how-tos/write-workflows/use-workflow-templates)

---

## Choosing When Your Workflow Runs

You can configure workflows to run on a schedule or to run when certain events happen.

### Triggering a workflow

Define which events can cause the workflow to run with the `on` key:

```yaml
# Single event
on: push

# Multiple events
on: [push, pull_request]

# With activity types
on:
  issues:
    types: [opened, labeled]

# With filters
on:
  push:
    branches: [main, 'releases/**']
    paths: ['**.js']
```

### Scheduled workflows

```yaml
on:
  schedule:
    - cron: '30 5 * * 1,3'
```

### Manual triggers

```yaml
on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Environment to deploy to'
        type: environment
        required: true
```

**Source**: [Choosing when your workflow runs](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run)

---

## Choosing Where Your Workflow Runs

You can specify the compute environment your jobs and workflows run in using `runs-on`:

```yaml
# GitHub-hosted runner
runs-on: ubuntu-latest

# Multiple labels (all must match)
runs-on: [self-hosted, linux, x64, gpu]

# Using a group
runs-on:
  group: my-runner-group
```

**Source**: [Choosing where your workflow runs](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run)

---

## Choosing What Your Workflow Does

Workflows automate tasks in your software development lifecycle. Key capabilities include:

- Running scripts with `run`
- Using actions with `uses`
- Adding scripts to your workflow
- Running job variations with matrix strategy
- Using concurrency to cancel in-progress runs
- Caching workflow dependencies
- Storing and sharing workflow data (artifacts)
- Passing data between jobs

**Source**: [Choosing what your workflow does](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do)
