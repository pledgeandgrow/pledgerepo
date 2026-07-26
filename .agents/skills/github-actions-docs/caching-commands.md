# GitHub Actions — Caching, Artifacts, Variables & Workflow Commands

## Dependency Caching

Caching dependencies speeds up workflow runs by reusing downloaded files between runs.

### Using the cache action

```yaml
- name: Cache node modules
  id: cache-npm
  uses: actions/cache@v4
  env:
    cache-name: cache-node-modules
  with:
    path: ~/.npm
    key: ${{ runner.os }}-build-${{ env.cache-name }}-${{ hashFiles('**/package-lock.json') }}
    restore-keys: |
      ${{ runner.os }}-build-${{ env.cache-name }}-
      ${{ runner.os }}-build-
      ${{ runner.os }}-
```

### Input parameters for the cache action

| Parameter | Required | Description |
|-----------|----------|-------------|
| `key` | Yes | Cache key (max 512 characters). Can use contexts, expressions, functions |
| `path` | Yes | Path(s) on the runner to cache/restore. Supports glob patterns |
| `restore-keys` | No | Alternative restore keys, one per line. Used if no exact match for `key` |
| `enableCrossOsArchive` | No | Allow cross-OS cache sharing (default: `false`) |

### Output parameters

| Output | Description |
|--------|-------------|
| `cache-hit` | `true` if exact match found for key, `false` otherwise |

### Cache key matching

Cache keys are matched exactly. If no exact match, `restore-keys` are tried sequentially as partial matches (prefix matching).

```yaml
restore-keys: |
  npm-feature-${{ hashFiles('package-lock.json') }}
  npm-feature-
  npm-
```

### Using contexts to create cache keys

```yaml
key: ${{ runner.os }}-${{ hashFiles('**/package-lock.json') }}
# Evaluates to e.g. "Linux-d5ea0750"
```

### Using the output of the cache action

```yaml
- if: ${{ steps.cache-npm.outputs.cache-hit != 'true' }}
  name: List the state of node modules
  continue-on-error: true
  run: npm list
```

### setup-* actions with built-in caching

Many `setup-*` actions have built-in caching:

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: '20'
    cache: 'npm'

- uses: actions/setup-python@v5
  with:
    python-version: '3.12'
    cache: 'pip'
```

### Cache access restrictions

- Caches are scoped to branches. Default branch caches are available to all branches.
- Pull requests from forks cannot read or write caches.
- Low-trust triggers (e.g., `pull_request` from forks) have restricted cache access.

### Best practices for cache security

- **Don't store sensitive information** in cache paths
- **Save caches from trusted triggers** only
- **Follow workflow security best practices** to prevent malicious cache entries

### Usage limits and eviction policy

- Maximum cache size: 10 GB per repository (GitHub Free/Pro/Team)
- Caches not accessed in 7 days are evicted
- Cache storage limits cannot be increased by GitHub Support

**Source**: [Dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)

---

## Artifacts

Artifacts are files generated during a workflow run. They can be uploaded, downloaded, and shared between jobs.

### Uploading artifacts

```yaml
- name: Upload build artifact
  uses: actions/upload-artifact@v4
  with:
    name: build-output
    path: |
      dist/
      build/
    retention-days: 14
    compression-level: 6
```

### Downloading artifacts

```yaml
- name: Download build artifact
  uses: actions/download-artifact@v4
  with:
    name: build-output
    path: ./downloaded
```

### Artifact retention

- Default retention: 90 days (configurable per repository)
- Maximum retention: 400 days
- Artifacts can be deleted manually or via API

### Passing data between jobs

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: mkdir -p dist && echo "build" > dist/app.txt
      - uses: actions/upload-artifact@v4
        with:
          name: app
          path: dist/

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: app
          path: dist/
      - run: cat dist/app.txt
```

---

## Default Environment Variables

These are set automatically by GitHub and available in every step. They are NOT accessible through the `env` context — use the corresponding context property instead (e.g., `${{ github.ref }}` instead of `$GITHUB_REF`).

### Key default environment variables

| Variable | Example Value | Description |
|----------|---------------|-------------|
| `CI` | `true` | Always set to `true` |
| `GITHUB_ACTION` | `__run` | Name of the action |
| `GITHUB_ACTIONS` | `true` | Always `true` when running in GitHub Actions |
| `GITHUB_ACTOR` | `octocat` | User that triggered the workflow |
| `GITHUB_API_URL` | `https://api.github.com` | GitHub API URL |
| `GITHUB_BASE_REF` | `main` | Base branch of PR (only for PR events) |
| `GITHUB_ENV` | `/path/to/set_env_*` | Path to file for setting env vars |
| `GITHUB_EVENT_NAME` | `push` | Event that triggered the workflow |
| `GITHUB_EVENT_PATH` | `/github/workflow/event.json` | Path to event payload JSON |
| `GITHUB_GRAPHQL_URL` | `https://api.github.com/graphql` | GraphQL API URL |
| `GITHUB_HEAD_REF` | `feature-branch` | Head branch of PR (only for PR events) |
| `GITHUB_JOB` | `greeting_job` | Job ID |
| `GITHUB_OUTPUT` | `/path/to/set_output_*` | Path to file for setting outputs |
| `GITHUB_PATH` | `/path/to/add_path_*` | Path to file for adding system paths |
| `GITHUB_REF` | `refs/heads/main` | Branch or tag ref |
| `GITHUB_REF_NAME` | `feature-branch-1` | Short ref name |
| `GITHUB_REF_PROTECTED` | `true` | Whether branch protections apply |
| `GITHUB_REF_TYPE` | `branch` | `branch` or `tag` |
| `GITHUB_REPOSITORY` | `octocat/Hello-World` | `owner/repo` |
| `GITHUB_REPOSITORY_ID` | `123456789` | Repository ID |
| `GITHUB_REPOSITORY_OWNER` | `octocat` | Repository owner |
| `GITHUB_RETENTION_DAYS` | `90` | Artifact retention days |
| `GITHUB_RUN_ATTEMPT` | `3` | Attempt number of the run |
| `GITHUB_RUN_ID` | `1658821493` | Unique run ID |
| `GITHUB_RUN_NUMBER` | `3` | Run number within repository |
| `GITHUB_SERVER_URL` | `https://github.com` | GitHub server URL |
| `GITHUB_SHA` | `ffac537e...` | Commit SHA |
| `GITHUB_STEP_SUMMARY` | `/path/to/step_summary_*` | Path to file for job summary |
| `GITHUB_TRIGGERING_ACTOR` | `octocat` | User who triggered the run |
| `GITHUB_WORKFLOW` | `My test workflow` | Workflow name |
| `GITHUB_WORKFLOW_REF` | `octocat/repo/.github/workflows/my-workflow.yml@refs/heads/main` | Workflow ref |
| `GITHUB_WORKSPACE` | `/home/runner/work/my-repo/my-repo` | Workspace directory |
| `RUNNER_ARCH` | `X64` | Runner architecture (`X86`, `X64`, `ARM`, `ARM64`) |
| `RUNNER_DEBUG` | `1` | Debug logging enabled |
| `RUNNER_ENVIRONMENT` | `github-hosted` | `github-hosted` or `self-hosted` |
| `RUNNER_NAME` | `Hosted Agent` | Runner name |
| `RUNNER_OS` | `Linux` | Runner OS (`Linux`, `Windows`, `macOS`) |
| `RUNNER_TEMP` | `/tmp` | Temporary directory |
| `RUNNER_TOOL_CACHE` | `/opt/hostedtoolcache` | Tool cache directory |

### Naming conventions for configuration variables

- Only alphanumeric characters (`[a-z]`, `[A-Z]`, `[0-9]`) or underscores (`_`)
- Must not start with `GITHUB_` prefix
- Must not start with a number
- Case insensitive when referenced

### Configuration variable precedence

1. Environment-level (highest)
2. Repository-level
3. Organization-level (lowest)

Note: Environment-level variables are only available on the runner after the job starts, so they don't overwrite `env` and `vars` contexts.

**Source**: [Variables reference](https://docs.github.com/en/actions/reference/workflows-and-actions/variables)

---

## Workflow Commands

Workflow commands let steps communicate with the runner: set env vars, outputs, debug messages, annotations, and more.

### About workflow commands

Most commands use `echo` in a specific format:

```bash
echo "::command-name::command-value"
```

### Setting a debug message

```bash
echo "::debug::Set the Octocat variable"
```

### Setting a notice message

```bash
echo "::notice file=app.js,line=1::Missing semicolon"
```

### Setting a warning message

```bash
echo "::warning file=app.js,line=1,col=5::Missing semicolon"
```

### Setting an error message

```bash
echo "::error file=app.js,line=1,col=5::Missing semicolon"
```

### Grouping log lines

```bash
echo "::group::My group"
echo "Inside group"
echo "::endgroup::"
```

### Masking a value in a log

```bash
echo "::add-mask::Mona The Octocat"
# Subsequent logs will mask this value
```

### Stopping and starting workflow commands

```bash
echo "::stop-commands::flow-token"
echo "::This is not a command, it's just output"
echo "flow-token::"
```

### Environment files

The runner generates temporary files for certain operations:

| File | Purpose |
|------|---------|
| `GITHUB_ENV` | Set environment variables for subsequent steps |
| `GITHUB_OUTPUT` | Set step output parameters |
| `GITHUB_PATH` | Add system paths |
| `GITHUB_STEP_SUMMARY` | Add job summary (Markdown) |

### Setting an environment variable

```bash
echo "MY_ENV_VAR=myValue" >> "$GITHUB_ENV"
```

Multiline strings:

```bash
echo 'MULTI_LINE_VAR<<EOF' >> "$GITHUB_ENV"
echo "Line 1" >> "$GITHUB_ENV"
echo "Line 2" >> "$GITHUB_ENV"
echo 'EOF' >> "$GITHUB_ENV"
```

### Setting an output parameter

```bash
echo "name=value" >> "$GITHUB_OUTPUT"
```

The step must have an `id` to retrieve the output later:

```yaml
steps:
  - id: my-step
    run: echo "result=hello" >> "$GITHUB_OUTPUT"
  - run: echo "${{ steps.my-step.outputs.result }}"
```

### Adding a job summary

```bash
echo "## Build Results" >> "$GITHUB_STEP_SUMMARY"
echo "✅ All tests passed" >> "$GITHUB_STEP_SUMMARY"
```

Supports GitHub-flavored Markdown. Summaries from all steps are combined on the run summary page.

### Adding a system path

```bash
echo "/path/to/add" >> "$GITHUB_PATH"
```

### Sending values to pre and post actions

For actions with `pre` and `post` execution, use `save-state` (deprecated) or `GITHUB_ENV` to pass values.

**Source**: [Workflow commands for GitHub Actions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands)
