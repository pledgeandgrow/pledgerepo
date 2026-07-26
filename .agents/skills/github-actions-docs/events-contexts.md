# GitHub Actions — Events, Contexts & Expressions

## Events That Trigger Workflows

Workflow triggers are events that cause a workflow to run. Some events have multiple activity types. You can specify which activity types will trigger a workflow run.

### Complete list of events

| Event | Description |
|-------|-------------|
| `branch_protection_rule` | Branch protection rules changed (created, edited, deleted) |
| `check_run` | Check run created, rerequested, completed, or requested action |
| `check_suite` | Check suite completed or requested |
| `create` | Branch or tag created |
| `delete` | Branch or tag deleted |
| `deployment` | Deployment created |
| `deployment_status` | Deployment status added |
| `discussion` | Discussion created, edited, deleted, answered, etc. |
| `discussion_comment` | Discussion comment created, edited, deleted |
| `fork` | Repository forked |
| `gollum` | Wiki page created or updated |
| `image_version` | Image version created or updated |
| `issue_comment` | Issue or PR comment created, edited, deleted |
| `issues` | Issue opened, edited, deleted, labeled, etc. |
| `label` | Label created, edited, deleted |
| `merge_group` | Merge group added to merge queue |
| `milestone` | Milestone created, closed, opened, edited, deleted |
| `page_build` | GitHub Pages site built |
| `public` | Repository made public |
| `pull_request` | PR opened, edited, closed, labeled, synchronized, reopened |
| `pull_request_review` | PR review submitted, edited, dismissed |
| `pull_request_review_comment` | PR review comment created, edited, deleted |
| `pull_request_target` | Like `pull_request` but runs in base branch context with secrets |
| `push` | Commit pushed to branch or tag |
| `registry_package` | Package published or updated |
| `release` | Release created, published, unpublished, edited, deleted |
| `repository_dispatch` | External event via REST API |
| `schedule` | Scheduled (cron) |
| `status` | Commit status updated |
| `watch` | Repository starred |
| `workflow_call` | Called by another workflow (reusable workflows) |
| `workflow_dispatch` | Manually triggered from Actions tab |
| `workflow_run` | Another workflow completed or was requested |

### Key event details

#### push

```yaml
on:
  push:
    branches: [main, 'releases/**']
    tags: ['v*']
    paths: ['src/**', '**.js']
    paths-ignore: ['docs/**']
```

#### pull_request

```yaml
on:
  pull_request:
    types: [opened, synchronize, reopened]
    branches: [main]
    paths: ['src/**']
```

#### schedule

```yaml
on:
  schedule:
    - cron: '30 5 * * 1,3'  # 5:30 AM every Monday and Wednesday
```

Note: Scheduled workflows run on the default branch only. The `github.actor` for scheduled workflows is the user who last edited the workflow file's cron schedule.

#### workflow_dispatch

```yaml
on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Environment to deploy to'
        type: choice
        options: [staging, production]
        required: true
      debug:
        type: boolean
        default: false
```

#### workflow_call

```yaml
on:
  workflow_call:
    inputs:
      config-path:
        type: string
        required: true
    secrets:
      token:
        required: true
```

#### workflow_run

```yaml
on:
  workflow_run:
    workflows: [Build]
    types: [completed]
    branches: [main]
```

**Source**: [Events that trigger workflows](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)

---

## Contexts Reference

Contexts are objects that contain information about the workflow run, runner, jobs, steps, and more. Access context properties using expression syntax: `${{ <context>.<property> }}`.

### Available contexts

| Context | Description |
|---------|-------------|
| `github` | Information about the workflow run, event, repository, actor |
| `env` | Environment variables set in workflow, job, or step |
| `vars` | Repository, organization, or environment variables |
| `job` | Information about the current job |
| `jobs` | Information about jobs in a reusable workflow |
| `steps` | Information about steps that have already run |
| `runner` | Information about the runner executing the job |
| `secrets` | Encrypted secrets available to the workflow |
| `strategy` | Information about the matrix execution strategy |
| `matrix` | Matrix properties for the current job |
| `needs` | Outputs of jobs that are dependencies |
| `inputs` | Inputs of a reusable or manually triggered workflow |

### Context availability

| Context | Workflow file | Reusable workflow | Called workflow | Manual dispatch |
|---------|:---:|:---:|:---:|:---:|
| `github` | ✅ | ✅ | ✅ | ✅ |
| `env` | ✅ | ✅ | ✅ | ✅ |
| `vars` | ✅ | ✅ | ✅ | ✅ |
| `job` | ✅ | ✅ | ✅ | ✅ |
| `jobs` | ❌ | ✅ | ❌ | ❌ |
| `steps` | ✅ | ✅ | ✅ | ✅ |
| `runner` | ✅ | ✅ | ✅ | ✅ |
| `secrets` | ✅ | ✅ | ✅ | ✅ |
| `strategy` | ✅ | ✅ | ✅ | ✅ |
| `matrix` | ✅ | ✅ | ✅ | ✅ |
| `needs` | ✅ | ✅ | ✅ | ✅ |
| `inputs` | ❌ | ✅ | ✅ | ✅ |

### github context

```yaml
${{ github.actor }}        # User that triggered the run
${{ github.event_name }}   # Event that triggered the run
${{ github.ref }}          # Branch or tag ref
${{ github.repository }}   # owner/repo
${{ github.workspace }}    # Runner workspace directory
${{ github.sha }}          # Commit SHA
${{ github.token }}        # GITHUB_TOKEN
```

### env context

```yaml
${{ env.NODE_VERSION }}
```

### vars context

```yaml
${{ vars.DEPLOY_ENV }}
```

### job context

```yaml
${{ job.status }}          # current job status
${{ job.container.id }}    # container ID
```

### steps context

```yaml
${{ steps.build.outputs.id }}
${{ steps.check.outcome }}  # 'success' | 'failure' | 'skipped'
```

### runner context

```yaml
${{ runner.os }}           # 'Linux' | 'Windows' | 'macOS'
${{ runner.arch }}         # 'X64' | 'ARM64'
${{ runner.temp }}         # Temp directory path
${{ runner.tool_cache }}   # Tool cache directory
```

### secrets context

```yaml
${{ secrets.DEPLOY_TOKEN }}
${{ secrets.GITHUB_TOKEN }}  # Automatically available
```

### strategy context

```yaml
${{ strategy.fail-fast }}
${{ strategy.job-index }}
${{ strategy.total }}
```

### matrix context

```yaml
${{ matrix.os }}
${{ matrix.node }}
```

### needs context

```yaml
${{ needs.build.outputs.result }}
${{ needs.deploy.result }}
```

### inputs context

```yaml
${{ inputs.environment }}
${{ inputs.debug }}
```

**Source**: [Contexts reference](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts)

---

## Expressions

Evaluate expressions in workflows using `${{ <expression> }}` syntax.

### Literals

- `null` — Null
- `true`, `false` — Boolean
- `42`, `3.14` — Number
- `'string'` — String

### Operators

| Operator | Description |
|----------|-------------|
| `( )` | Grouping |
| `[ ]` | Array index |
| `.` | Property dereference |
| `!` | Not |
| `<`, `<=`, `>`, `>=` | Comparison |
| `==`, `!=` | Equality |
| `&&` | And |
| `\|\|` | Or |

GitHub ignores case when comparing strings. Performs loose equality comparisons.

### Functions

#### contains(search, item)

Returns `true` if `search` contains `item`.

```yaml
contains('Hello world', 'llo')  # true
contains(github.event.issue.labels.*.name, 'bug')  # true if labeled "bug"
contains(fromJSON('["push", "pull_request"]'), github.event_name)
```

#### startsWith(searchString, searchValue)

```yaml
startsWith('Hello world', 'He')  # true
```

#### endsWith(searchString, searchValue)

```yaml
endsWith('Hello world', 'ld')  # true
```

#### format(string, replaceValue0, replaceValue1, ...)

```yaml
format('Hello {0} {1} {2}', 'Mona', 'the', 'Octocat')  # 'Hello Mona the Octocat'
format('{{Hello {0}!}}', 'World')  # '{Hello World!}'
```

#### join(array, optionalSeparator)

```yaml
join(github.event.issue.labels.*.name, ', ')  # 'bug, help wanted'
```

#### toJSON(value)

Returns a JSON string.

```yaml
toJSON(env)  # JSON string of env context
```

#### fromJSON(string)

Parses JSON string.

```yaml
fromJSON(needs.build.outputs.matrix)  # parsed JSON
fromJSON('["a", "b"]')  # array
```

#### hashFiles(pattern)

Returns a hash for files matching the pattern.

```yaml
hashFiles('**/package-lock.json')  # hash of all lock files
${{ hashFiles('**/yarn.lock') != '' }}  # true if yarn.lock exists
```

#### case(value, ...matches)

Returns the first matching value (new function).

### Status check functions

| Function | Description |
|----------|-------------|
| `success()` | All previous steps succeeded (default) |
| `always()` | Always runs, regardless of status |
| `cancelled()` | Runs only if workflow was cancelled |
| `failure()` | Runs only if any previous step failed |

```yaml
- if: failure()
  run: echo "A previous step failed"
- if: always()
  run: echo "This always runs"
```

### Object filters

Use `.*` to access array properties:

```yaml
github.event.issue.labels.*.name  # array of label names
```

**Source**: [Evaluate expressions in workflows and actions](https://docs.github.com/en/actions/reference/workflows-and-actions/expressions)
