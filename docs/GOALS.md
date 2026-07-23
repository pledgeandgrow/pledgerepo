# PledgePack — Roadmap v3: 85 Goals

> Strategic goals across four pillars: Developer Experience, Differentiation, Plugin Ecosystem, and Developer Tooling.
>
> All goals are strictly within PledgePack's responsibility as defined in [CONNECTION.md](../CONNECTION.md). PledgePack is a **dumb bundler** — it transforms files and serves them. It does NOT handle React rendering, routing semantics, SSR/SSG/ISR, RSC, data fetching, server actions, components, i18n routing, auth, or SEO.
>
> See [pledgestack/docs/GOALS.md](../../pledgejs/docs/GOALS.md) for PledgeStack-specific goals.
>
> 75 of 85 goals completed — see [ARCHIVE.md](./ARCHIVE.md) for completed goals.

## Pillar 4: Developer Tooling

### Build & Analysis (Uncompleted goals from ARCHIVE.md)

70. **Build output verification** — Post-build integrity check: verify all chunks exist, no broken import references, all assets resolved. `pledge build --verify` flag. Fails build on missing output files.

76. **Dependency-graph-aware test re-run** — `pledge test --watch` only re-runs tests affected by changed files, not all tests. Uses module graph to determine test impact set.

77. **Test parallelization across cores** — Run test files in parallel using rayon thread pool. `test: { parallel: true, max_workers: 4 }` config.

78. **Mutation testing** — `pledge test --mutate` injects code mutations to measure test effectiveness. Reports mutation score per file.

### Dev Experience

79. **Build progress streaming** — Real-time build progress over WebSocket in dev mode. Per-module transform status. `pledge dev` shows which modules are transforming.

80. **Config file hot reload** — Reload `pledge.config.ts` changes without restarting dev server. Watch config file and re-initialize engine on change.

81. **Friendly error messages with suggestions** — Enhanced error messages with "Did you mean...?" for import paths, config fields, and CLI commands. Color-coded severity levels.

### Deployment & Output

83. **Docker image generation** — Generate Dockerfile + .dockerignore for production deployment. Multi-stage build with minimal final image. `pledge build --docker` flag.

84. **Base path configuration** — `base: '/my-app/'` config for deploying under a subpath. All asset URLs and import paths adjusted automatically.

### Integrations

85. **`pledge storybook` — Zero-config Storybook** — Built-in Storybook integration using PledgePack as the builder (replaces Vite). `pledge storybook` command auto-detects `*.stories.tsx` files in `app/` directory, launches Storybook with PledgePack's dev server and HMR. No separate Storybook config or Vite dependency needed. (From [pledgejs/LIMITATIONS.md](../../pledgejs/LIMITATIONS.md) — Storybook integration plan.)
