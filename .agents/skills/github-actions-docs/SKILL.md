---
name: github-actions-docs
version: "2026"
tags:
  - github-actions
  - ci
  - cd
  - ci-cd
  - automation
  - workflow
  - yaml
  - runners
  - github-hosted-runners
  - self-hosted-runners
  - larger-runners
  - actions-runner-controller
  - arc
  - secrets
  - oidc
  - security
  - artifact-attestations
  - github-token
  - permissions
  - custom-actions
  - javascript-actions
  - composite-actions
  - docker-actions
  - action-yml
  - metadata-syntax
  - reusable-workflows
  - workflow-call
  - workflow-dispatch
  - events
  - triggers
  - push
  - pull-request
  - pull-request-target
  - schedule
  - cron
  - contexts
  - expressions
  - matrix
  - strategy
  - concurrency
  - caching
  - actions-cache
  - artifacts
  - upload-artifact
  - download-artifact
  - environment-variables
  - github-env
  - github-output
  - github-path
  - github-step-summary
  - workflow-commands
  - annotations
  - filter-patterns
  - deployment
  - azure
  - aws
  - gke
  - aks
  - limits
  - metrics
  - status-badge
  - environments
  - protection-rules
  - required-reviewers
  - wait-timer
  - debug-logging
  - billing
  - rest-api
  - migration
  - jenkins
  - circleci
  - gitlab-ci
  - azure-pipelines
  - service-containers
  - postgresql
  - redis
  - database-testing
description: |
  GitHub Actions — workflows, events, jobs, runners, contexts, expressions, secrets, caching, reusable workflows.
---

# GitHub Actions

> **Version**: Latest (2026) | **Source**: [docs.github.com/en/actions](https://docs.github.com/en/actions)

## Overview

GitHub Actions is a CI/CD platform that automates build, test, and deployment pipelines directly from your repository. Workflows are defined as YAML files in `.github/workflows/` and triggered by repository events, schedules, or manual dispatch.

## Quick Reference

| Topic | File | Key Content |
|-------|------|-------------|
| Getting Started | `getting-started.md` | Understanding Actions, quickstart, workflow templates, triggers, runners selection, workflow tasks |
| Workflow Syntax | `workflow-syntax.md` | Full YAML syntax: name, on, jobs, steps, strategy, matrix, container, services, concurrency, permissions, env, defaults, reusable jobs, filter patterns |
| Events, Contexts & Expressions | `events-contexts.md` | All 30+ trigger events, 12 contexts (github, env, vars, job, jobs, steps, runner, secrets, strategy, matrix, needs, inputs), expression operators, functions, status checks |
| Runners | `runners.md` | GitHub-hosted runners (Ubuntu/Windows/macOS), larger runners, self-hosted runners, Actions Runner Controller (ARC) |
| Security & Reuse | `security-reuse.md` | Secrets management, OIDC, artifact attestations, GITHUB_TOKEN permissions, custom actions (JS/composite/Docker), metadata syntax, reusable workflows |
| Managing Workflows | `managing.md` | Manual runs, re-running, canceling, disabling, status badges, auto-labeling issues, scripts, deployment to Azure/AWS/GKE, limits, metrics |
| Caching, Artifacts, Variables & Commands | `caching-commands.md` | Dependency caching (actions/cache, keys, restore-keys), artifacts (upload/download), default environment variables (GITHUB_*, RUNNER_*), config variable precedence, workflow commands (debug, notice, warning, error, group, mask, GITHUB_ENV, GITHUB_OUTPUT, GITHUB_PATH, GITHUB_STEP_SUMMARY) |
| Advanced Topics | `advanced.md` | Deployment environments (protection rules, required reviewers, wait timer, branch restrictions, env secrets/vars), debug logging (runner diagnostic, step debug), billing & usage (minute multipliers, included minutes, storage), GITHUB_TOKEN authentication (API calls, CLI, permissions), REST API (workflows, runs, artifacts, caches, secrets, runners), migration from Jenkins/CircleCI/GitLab/Azure Pipelines (concept mapping tables), service containers (PostgreSQL, Redis) |

## Core Concepts

- **Workflow** — YAML file in `.github/workflows/`, triggered by events, schedule, or manual dispatch
- **Job** — Set of steps on the same runner; jobs run in parallel by default, can depend on each other via `needs`
- **Step** — Individual task: shell script (`run`) or action (`uses`)
- **Action** — Reusable unit: JavaScript, composite, or Docker container
- **Runner** — Server executing jobs: GitHub-hosted (Ubuntu/Windows/macOS), larger, or self-hosted
- **Event** — Trigger: push, pull_request, schedule, workflow_dispatch, workflow_call, and 30+ more
- **Context** — Object with run info: `github`, `env`, `secrets`, `steps`, `runner`, `matrix`, `needs`, `inputs`, etc.
- **Expression** — `${{ }}` syntax for evaluating values, operators, and functions
- **Matrix** — Strategy to run jobs across variable combinations (max 256 jobs)
- **Concurrency** — Cancel in-progress runs when a new run starts in the same group
- **Reusable Workflow** — Workflow called from another workflow via `workflow_call` trigger
- **Secrets** — Encrypted variables at repo, environment, or org level
- **OIDC** — OpenID Connect for cloud auth without long-lived credentials
- **Caching** — `actions/cache` for reusing dependencies between runs
- **Artifacts** — `actions/upload-artifact` and `actions/download-artifact` for sharing files between jobs
- **Workflow Commands** — `::` syntax for annotations, masking, env vars, outputs, job summaries
- **Default Variables** — `GITHUB_*` and `RUNNER_*` env vars set automatically by the runner
- **Environments** — Deployment targets with protection rules (required reviewers, wait timers, branch restrictions)
- **Debug Logging** — `ACTIONS_RUNNER_DEBUG` and `ACTIONS_STEP_DEBUG` for verbose logs
- **Billing** — Per-minute billing with OS multipliers (Linux 1x, Windows 2x, macOS 10x)
- **REST API** — Endpoints for workflows, runs, artifacts, caches, secrets, runners
- **Migration** — Concept mapping from Jenkins, CircleCI, GitLab CI, Azure Pipelines
- **Service Containers** — Database services (PostgreSQL, Redis) for integration testing

## Official Documentation Links

- [GitHub Actions Home](https://docs.github.com/en/actions)
- [Quickstart](https://docs.github.com/en/actions/get-started/quickstart)
- [Understanding GitHub Actions](https://docs.github.com/en/actions/get-started/understanding-github-actions)
- [Writing workflows](https://docs.github.com/en/actions/how-tos/write-workflows)
- [Workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)
- [Events that trigger workflows](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)
- [Contexts reference](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts)
- [Expressions](https://docs.github.com/en/actions/reference/workflows-and-actions/expressions)
- [Metadata syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/metadata-syntax)
- [Actions limits](https://docs.github.com/en/actions/reference/limits)
- [GitHub-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/github-hosted-runners/use-github-hosted-runners)
- [Larger runners](https://docs.github.com/en/actions/how-tos/manage-runners/larger-runners)
- [Self-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/add-runners)
- [Actions Runner Controller](https://docs.github.com/en/actions/concepts/runners/actions-runner-controller)
- [Using secrets](https://docs.github.com/en/actions/how-tos/security-for-github-actions/security-guides/using-secrets-in-github-actions)
- [Security for GitHub Actions](https://docs.github.com/en/actions/how-tos/security-for-github-actions/security-guides)
- [Reuse workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
- [Managing workflow runs](https://docs.github.com/en/actions/how-tos/manage-workflow-runs)
- [Deploying to third-party platforms](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms)
- [About GitHub Actions metrics](https://docs.github.com/en/actions/concepts/metrics)
- [Dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
- [Variables reference](https://docs.github.com/en/actions/reference/workflows-and-actions/variables)
- [Workflow commands](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands)
- [Managing environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)
- [Enabling debug logging](https://docs.github.com/en/actions/how-tos/monitor-workflows/enable-debug-logging)
- [GITHUB_TOKEN authentication](https://docs.github.com/en/actions/tutorials/authenticate-with-github_token)
- [REST API for workflows](https://docs.github.com/en/rest/actions/workflows)
- [Migrating to GitHub Actions](https://docs.github.com/en/actions/migrating-to-github-actions)
- [About billing for GitHub Actions](https://docs.github.com/en/billing/managing-billing-for-github-actions/about-billing-for-github-actions)
