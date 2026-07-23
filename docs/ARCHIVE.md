# PledgePack Goals & Roadmap

## Roadmap v1: 50 Features (All Completed ✅)

### Build Performance

1. ~~**Incremental rebuild graph**~~ ✅ — Only rebuild changed modules and their dependents, skip untouched subtrees entirely instead of full rebuilds. Implemented in `module_graph.rs` with content-hash-based change detection and transitive dependent computation.
2. ~~**Persistent module graph**~~ ✅ — Serialize the module graph to disk between builds for faster cold starts and incremental detection. `SerializableModuleGraph` saves/loads via bincode to `module_graph.bin` in the cache directory.
3. ~~**Parallel dependency optimization**~~ ✅ — Multi-threaded tree shaking and chunk splitting using rayon for large dependency graphs. `mark_side_effects` and `split_chunks` parallelized with `par_iter()` and `partition_map()`.
4. ~~**Lazy dependency scanning**~~ ✅ — Scan only entry-point imports on first build, expand graph lazily as imports are discovered. BFS queue processes modules on-demand, resolving dependencies only when encountered.
5. ~~**Build cache sharing**~~ ✅ — Share transform cache across CI runs via content-addressable storage backed by S3/GCS. `RemoteCache` in `remote.rs` supports HTTP, S3, and GCS backends with automatic fallback.
6. ~~**Git-based cache invalidation**~~ ✅ — Use git tree hashes for cache keys instead of file content hashes for faster invalidation on large repos. `GitCacheInvalidator` in `git_cache.rs` uses `git ls-files` and `git rev-parse HEAD^{tree}`.
7. ~~**Remote cache**~~ ✅ — Network-based cache for team/CI builds, sharing transform results across machines. Integrated in `BuildEngine` with 3-tier fallback: memory → disk → remote.
8. ~~**Memory-mapped output writing**~~ ✅ — Use mmap for writing large build output files instead of buffered I/O. `write_output_file()` uses mmap for files >64KB on Unix, buffered write for smaller files and Windows.

### Dev Server

9. ~~**File system watcher optimizations**~~ ✅ — Use `inotify`/`FSEvents`/`ReadDirectoryChangesW` natively instead of `notify` crate abstraction for lower latency. Implemented in `watcher.rs` with platform-specific native watchers and fallback.
10. ~~**HMR partial updates**~~ ✅ — Send only the changed function/module diff via WebSocket instead of full module replacement. Implemented in `hmr_diff.rs` with `similar` crate (Myers algorithm) for line-level diff computation and `is_small()` heuristic.
11. ~~**Dev server cold boot optimization**~~ ✅ — Lazy-load transform pipeline, only initialize Oxc/Lightning CSS on first request. Implemented in `lazy_pipeline.rs` with deferred initialization and dirty dependency tracking.
12. ~~**WebSocket compression**~~ ✅ — Per-message deflate for HMR WebSocket to reduce bandwidth on large module updates. Applied via `tower-http` `CompressionLayer` with gzip and `Fastest` quality level.
13. ~~**Multi-entry dev server**~~ ✅ — Support multiple HTML entry points with independent HMR contexts in a single dev server. `detect_entries()` auto-detects HTML files and registers per-entry routes.
14. ~~**Dev server middleware chain**~~ ✅ — Configurable middleware pipeline for request processing (auth, logging, headers) before module serving. Implemented in `middleware.rs` with `MiddlewareFn` parsing from config and CORS/rewrite helpers.
15. ~~**On-demand dependency optimization**~~ ✅ — Re-optimize dependencies only when import patterns change, not on every server start. Import patterns tracked per-module in `DevServerState` and compared on each transform.

### Transform & Compilation

16. ~~**WASM target compilation**~~ ✅ — Compile select modules to WASM for compute-heavy workloads with `?wasm` import suffix. `transform_optimizations.rs` detects `?wasm` imports and generates JS glue code for loading WASM modules.
17. ~~**Tree shaking with side-effects detection**~~ ✅ — Heuristic-based side-effect detection for tree shaking unused exports. `analyze_side_effects()` checks for global writes, DOM access, and `console` calls; `tree_shake_module()` removes unused exports.
18. ~~**Cross-chunk variable hoisting**~~ ✅ — Hoist shared variables across chunks to avoid duplicate declarations in split bundles. `analyze_cross_chunk_hoisting()` tracks which chunks import variables from other chunks.
19. ~~**CSS tree shaking**~~ ✅ — Remove unused CSS selectors by analyzing class names in JS/JSX/TSX source code. `extract_used_class_names()` finds `className`, `class`, `:class` attributes including template literals; `shake_css()` filters CSS rules.
20. ~~**Dead code elimination at expression level**~~ ✅ — Remove unreachable branches inside functions. `eliminate_dead_code()` handles `if (false)`, `if (true)`, strict comparison replacements, and `typeof` checks.
21. ~~**Constant folding with type info**~~ ✅ — Fold expressions like `1 + 2` → `3` and `"a" + "b"` → `"ab"`. `fold_constants()` handles numeric, string, boolean, and `typeof` expression folding.
22. ~~**Optional chaining nullish short-circuit**~~ ✅ — Optimize `a?.b?.c` chains to avoid redundant null checks. `optimize_optional_chaining()` simplifies redundant null checks in optional chaining.
23. ~~**Module-level memoization**~~ ✅ — Cache transform results keyed by source hash + transform config hash. `ModuleTransformCache` uses blake3 hashes for cache keys with LRU eviction and path-based invalidation.

### CSS & Styling

24. ~~**Tailwind v4 Oxide engine**~~ ✅ — Native Tailwind v4 engine integration with CSS-first config and Lightning CSS. `tailwind_v4.rs` parses `@theme`, `@utility`, `@variant` directives, generates utility classes from theme tokens, and includes v4 preflight (reset).
25. ~~**CSS-in-JS compile-time extraction**~~ ✅ — Built-in support for styled-components, emotion, and vanilla-extract compile-time transforms. `css_in_js.rs` extracts CSS from template literals and style objects at build time, replacing runtime CSS-in-JS with static CSS + class names.
26. ~~**CSS layer support**~~ ✅ — `@layer` cascade layer management and automatic layer ordering in output. `css_features.rs` parses `@layer` declarations and reorders layer blocks according to `@layer name1, name2;` order statements.
27. ~~**Container queries polyfill**~~ ✅ — Built-in container query transform for older browser targets. `polyfill_container_queries()` in `css_features.rs` generates class-based fallbacks alongside native `@container` rules.
28. ~~**Critical CSS extraction**~~ ✅ — Extract above-the-fold CSS and inline it in HTML `<head>` for faster FCP. `extract_critical_css()` analyzes HTML class/id/tag usage and filters CSS rules; `inline_critical_css()` injects the result into `<head>`.
29. ~~**CSS source maps in dev**~~ ✅ — Accurate CSS source maps pointing to original `.scss`, `.less`, or `.css` files. `generate_css_source_map()` in `css_features.rs` produces v3 source maps with VLQ-encoded line mappings and original source content.
30. ~~**PostCSS plugin caching**~~ ✅ — Cache PostCSS plugin results to avoid re-running expensive plugins on unchanged CSS. `PostCssCache` in `css_features.rs` uses blake3 content hashing to key and cache plugin output.

### Asset Pipeline

31. ~~**MDX compilation**~~ ✅ — `.mdx` file compilation (Markdown + JSX) with frontmatter extraction and component imports. `compile_mdx()` in `asset_pipeline.rs` parses frontmatter, converts markdown to JSX (headings, lists, code blocks, links, bold/italic), and exports a `MDXContent` component.
32. ~~**GraphQL file loading**~~ ✅ — `.graphql`/`.gql` file loading with automatic TypeScript type generation. `parse_graphql()` extracts queries, mutations, subscriptions, and fragments; `graphql_to_module()` generates named exports with PascalCase type declarations.
33. ~~**YAML/CSV/TSV imports**~~ ✅ — Data file imports with typed named exports. `transform_yaml()` parses key-value pairs into named exports; `transform_csv()`/`transform_tsv()` export `columns`, `rowCount`, and `rows` as array of objects.
34. ~~**Image format auto-selection**~~ ✅ — Automatically convert images to WebP/AVIF based on browser support and size savings. `select_image_format()` in `asset_pipeline.rs` picks AVIF for large images with support, WebP for medium, original for small; `generate_picture_element()` produces `<picture>` with multiple `<source>` tags.
35. ~~**Audio/video asset handling**~~ ✅ — Import audio (`.mp3`, `.wav`, `.ogg`) and video (`.mp4`, `.webm`) files with URL exports. `transform_audio_asset()`/`transform_video_asset()` in `asset_pipeline.rs` produce URL or base64 data URI exports based on inline threshold.
36. ~~**PDF asset handling**~~ ✅ — Import `.pdf` files with URL exports and optional inline base64 for small documents. `transform_pdf_asset()` in `asset_pipeline.rs` handles both URL and `data:application/pdf;base64,...` inline exports.
37. ~~**Asset manifest generation**~~ ✅ — JSON manifest mapping all asset imports to their hashed output paths for backend integration. `AssetManifest` in `asset_pipeline.rs` tracks source→output mappings with content hashes, file sizes, and MIME types; `hashed_output_path()` generates `assets/{name}-{hash}.{ext}` paths.

### Plugin System

38. ~~**Plugin hot reload**~~ ✅ — Reload JS plugins without restarting dev server when plugin source changes. `PluginHotReloader` in `plugin_system.rs` watches plugin source files via blake3 content hashing and triggers reload callbacks on change.
39. ~~**Plugin sandboxing improvements**~~ ✅ — JS plugin sandboxing with configurable limits. `SandboxLimits` configures max memory, CPU time, FS reads/writes, allowed paths, network access, and stack depth; `SandboxedFs` enforces path access and read/write limits.
40. ~~**Plugin dependency resolution**~~ ✅ — Allow plugins to import npm packages within the WASM sandbox via pre-bundled imports. `PluginDependencyResolver` pre-bundles dependencies and generates import maps for WASM plugins.
41. ~~**Plugin lifecycle hooks**~~ ✅ — Add `watchStart`, `watchChange`, `watchEnd` hooks for file-watcher-aware plugins. `LifecycleHookRegistry` supports 9 hook types with per-plugin registration and invocation.
42. ~~**Plugin parallel execution**~~ ✅ — Run independent plugin transforms in parallel using rayon for multi-plugin pipelines. `execute_parallel_transforms()` uses rayon's `par_iter`; `group_independent_tasks()` groups tasks by file independence.

### Output & Distribution

43. ~~**Service worker generation**~~ ✅ — Automatic service worker with precaching, runtime caching, and offline fallback for production builds. `service_worker.rs` generates SW code with configurable caching strategies (network-first, cache-first, stale-while-revalidate).
44. ~~**Web App Manifest generation**~~ ✅ — Automatic `manifest.json` generation from config with icons, themes, and display modes. `generate_manifest()` in `service_worker.rs` produces a complete Web App Manifest from `WebAppManifest` config.
45. ~~**Performance budget enforcement**~~ ✅ — Fail build if bundle exceeds configured size limits with per-entry and per-chunk budgets. `check_budget()` in `output_distribution.rs` validates total, entry, chunk, initial load, and per-asset-type sizes against `PerformanceBudget`.
46. ~~**Bundle size diff**~~ ✅ — Compare bundle sizes between builds and fail CI on regressions. `diff_snapshots()` compares `BundleSizeSnapshot` objects; `format_diff_report()` generates markdown reports with regression detection.
47. ~~**Source map explorer**~~ ✅ — Interactive treemap showing which modules contribute to source map size. `build_source_map_tree()` constructs a module contribution tree from source maps; `generate_explorer_html()` produces an interactive HTML treemap visualization.
48. ~~**Multi-format output**~~ ✅ — Generate ESM, CJS, and IIFE outputs simultaneously from a single build for library mode. `generate_multi_format()` in `output_distribution.rs` converts ESM source to CJS, IIFE, and UMD formats with proper export handling.

### DX & Tooling

49. ~~**LSP server**~~ ✅ — Language Server Protocol implementation for import resolution, go-to-definition, and diagnostics in any editor. `lsp_server.rs` provides `LspServerState` with go-to-definition, completion, diagnostics, hover, and document symbols; supports path aliases and node_modules resolution.
50. ~~**Migration tooling**~~ ✅ — Automatic migration from Vite/Webpack/Turbopack configs to `pledge.config.ts` with `pledge migrate`. `migrate_config()` in `migrate.rs` detects and converts Vite, Webpack, and Turbopack configurations to Pledge format.

---

## Roadmap v2: 70 Goals (55 Completed ✅)

### Build Output & Optimization

51. ~~**Build-time environment variable injection**~~ ✅ — Replace static env vars at transform time. `process.env.NODE_ENV` → `"production"` inlined. Tree-shake unreachable env branches (`if (DEV)` blocks eliminated). `define` config and `import.meta.env` injection in `env.rs`.

52. ~~**Module preloading strategy**~~ ✅ — Configurable preloading for critical path chunks. `performance.rs` generates `<link rel="modulepreload">` and `<link rel="prefetch">` based on route chunks.

53. **Build output verification** — Post-build integrity check: verify all chunks exist, no broken import references, all assets resolved. `pledge build --verify` flag. Fails build on missing output files.

54. ~~**Incremental output diff**~~ ✅ — Only write changed chunks to disk between watch-mode rebuilds. Compare content hashes, skip unchanged files. Integrated with function-level incremental cache in watch mode.

55. ~~**WASM SIMD auto-detection**~~ ✅ — Detect WASM SIMD support in build target and generate SIMD-optimized WASM modules when available. `performance.rs` includes WASM streaming compilation with SIMD auto-detection.

### TypeScript & Type Safety

56. **Type checking during build** — Integrated `tsc` type checking in `pledge build` without separate `tsc --noEmit` step. `typeCheck: true` config. Fail build on type errors with formatted output.

57. **Type-aware tree shaking** — Use TypeScript type info to safely remove unused exports. Detect when exports are only used in type positions (`import type`) and exclude them from runtime bundle.

58. ~~**Path mapping auto-resolution**~~ ✅ — Read `tsconfig.json` paths/bases and auto-configure `resolve.alias`. `resolver.rs` reads `tsconfig.json` `compilerOptions.paths` via `from_tsconfig()`.

59. **`.d.ts` bundling for library mode** — Bundle TypeScript declarations into a single `.d.ts` file for library output. Tree-shake unused type declarations. `library: { declarations: 'bundled' }` config.

60. **Type-safe plugin API** — TypeScript types for pledgepack plugins with `Plugin` interface, hook signatures, and return types. Published as `pledgepack/plugins` entry point.

### Testing & Quality

61. ~~**Browser-based test runner**~~ ✅ — `pledge test --ui` generates HTML report and serves it at `localhost:5174` with pass/fail/skip summary, per-test status, error details, and auto-opens browser.

62. **Visual regression testing** — Screenshot comparison between builds. `pledge test --visual` flag. Pixel diff with threshold config. Baseline storage in `.pledge/visual-baselines/`.

63. **Dependency-graph-aware test re-run** — `pledge test --watch` only re-runs tests affected by changed files, not all tests. Uses module graph to determine test impact set. Faster watch-mode feedback.

64. **Test parallelization across cores** — Run test files in parallel using rayon thread pool. `test: { parallel: true, max_workers: 4 }` config. Shared state isolation per worker.

65. **Mutation testing** — `pledge test --mutate` injects code mutations to measure test effectiveness. Reports mutation score per file. Stryker-compatible output format.

### CSS & Styling

66. ~~**CSS Modules with composes** — Full CSS Modules `composes` directive support. `composes: button from './buttons.css'`. Scoped class name resolution across files.~~ ✅

67. ~~**Dark mode CSS generation** — Auto-generate dark mode variants from `prefers-color-scheme` queries. `css: { dark_mode: 'auto' }` config. CSS custom property-based theme switching.~~ ✅

68. ~~**CSS custom properties optimization** — Detect and inline static CSS custom properties. Remove unused `:root` variables. Minify variable names in production.~~ ✅

69. ~~**Scoped CSS for React** — CSS scoping without CSS Modules using automatic attribute selectors. `css: { scope: 'attribute' }` config. `data-v-xxxxx` attribute-based scoping like Vue.~~ ✅

70. ~~**CSS nesting polyfill** — Native CSS nesting (`& > .child`) polyfill for older browsers. Lightning CSS-based transformation with browser target config.~~ ✅

### Performance & Optimization

71. ~~**Automatic route-based chunk splitting** — Analyze route imports and split chunks per-route. Common shared chunks extracted automatically. `splitChunks: { strategy: 'route-aware' }` config.~~ ✅

72. ~~**Module prefetch directives** — Auto-generate `<link rel="modulepreload">` for route dependencies. `<link rel="prefetch">` for likely-next routes. `prefetch: { strategy: 'hover' | 'viewport' | 'load' }` config.~~ ✅

73. ~~**Tree shaking of CSS-in-JS runtime** — Remove styled-components/emotion runtime when all styles are extracted at build time. Zero runtime CSS-in-JS in production.~~ ✅

74. ~~**WASM module streaming compilation** — Compile WASM modules with `WebAssembly.streaming()` instead of buffer-based instantiation. Faster WASM load times.~~ ✅

75. ~~**Precompute module hash at transform time** — Compute content hash during transform pass, not as a separate emit pass. Eliminates redundant file reads in the emit phase.~~ ✅

### Assets & Media

76. ~~**Font subsetting** — Subset fonts to only include characters used in the project. `assets: { font_subset: true }` config. Reduces font file size by 60-90%.~~ ✅

77. ~~**SVG sprite generation** — Combine SVG files into a single sprite sheet with `<symbol>` elements. `import logo from './logo.svg?sprite'` syntax. Reduces HTTP requests.~~ ✅

78. ~~**Video poster frame extraction** — Auto-extract poster frame from video files. `import { src, poster } from './video.mp4'` exports both URL and poster image.~~ ✅

79. ~~**Responsive image srcset generation** — Auto-generate `srcset` with multiple resolutions. `assets: { responsive: { widths: [400, 800, 1200] } }` config. `<img srcset="...">` output.~~ ✅

80. ~~**Asset inlining threshold** — Configurable threshold for inlining assets as base64 data URIs. `assets: { inline_threshold: '4kb' }` config. Assets below threshold inlined automatically.~~ ✅

### Security & Integrity

81. ~~**Subresource Integrity (SRI) hashes** — Generate `integrity` attributes for `<script>` and `<link>` tags. `security: { sri: true }` config. SHA-384 hash output.~~ ✅

82. ~~**Content Security Policy generation** — Auto-generate CSP headers from build output. Hash-based CSP for inline scripts. `security: { csp: 'auto' }` config. Output as `_headers` file.~~ ✅

83. ~~**Dependency vulnerability scanning** — Scan `node_modules` for known vulnerabilities during build. `pledge doctor` includes security audit. CVE database lookup.~~ ✅

84. ~~**License compliance checking** — Scan dependencies for license compatibility. `pledge doctor --licenses` flag. Whitelist/blacklist license types in config.~~ ✅

### Dev Experience

85. ~~**Error overlay with source maps**~~ ✅ — Interactive error overlay in dev server showing original source code with error location. Stack trace mapping to original files. Auto-dismiss on HMR success. Runtime error catching via `window.error` and `unhandledrejection`.

86. **Build progress streaming** — Real-time build progress over WebSocket in dev mode. Per-module transform status. `pledge dev` shows which modules are transforming.

87. **Config file hot reload** — Reload `pledge.config.ts` changes without restarting dev server. Watch config file and re-initialize engine on change.

88. **Friendly error messages with suggestions** — Enhanced error messages with "Did you mean...?" for import paths, config fields, and CLI commands. Color-coded severity levels.

89. **`pledge why` command** — Analyze why a module is included in the bundle. Shows import chain from entry to target module. `pledge why lodash` output shows full dependency path.

### Deployment & Output

90. ~~**Build output manifest**~~ ✅ — `manifest.json` generated during build, mapping source files to output files with content-hashed filenames for cache busting.

91. **Docker image generation** — Generate Dockerfile + .dockerignore for production deployment. Multi-stage build with minimal final image. `pledge build --docker` flag.

92. **Base path configuration** — `base: '/my-app/'` config for deploying under a subpath. All asset URLs and import paths adjusted automatically.

93. ~~**CSS critical path extraction**~~ ✅ — `extract_critical_css()` analyzes HTML class/id/tag usage and filters CSS rules; `inline_critical_css()` injects the result into `<head>`.

### Ecosystem & Extensibility

94. ~~**Plugin preset system** — `presets: ['react', 'tailwind']` config applies a bundle of plugins with sensible defaults. Community presets installable via npm. `pledgepack-preset-*` naming convention.~~ ✅

95. ~~**Vite plugin compatibility layer** — Run existing Vite plugins unmodified in pledgepack. `plugins: [{ vite: 'vite-plugin-svg' }]` config. Automatic API translation.~~ ✅

96. ~~**Rollup plugin adapter** — Run Rollup plugins in pledgepack via compatibility shim. `plugins: [{ rollup: '@rollup/plugin-json' }]` config. Widens plugin ecosystem instantly.~~ ✅

97. ~~**Custom transformer pipeline** — `transform: { pipeline: ['oxc', 'custom-transform', 'minify'] }` config. Insert custom transform steps at any point in the pipeline. WASM or JS transformers.~~ ✅

### Monorepo & Workspaces

98. ~~**Workspace-aware resolution** — Auto-detect npm/pnpm/yarn workspaces. Resolve `@myorg/ui` to local workspace package, not npm registry. `workspaces: true` config.~~ ✅

99. ~~**Cross-package HMR** — Hot reload changes in workspace packages and propagate to consuming app. No manual rebuild of dependent packages needed.~~ ✅

100. ~~**Shared build cache across workspace** — Cache transform results in a shared `.pledge/` at workspace root. All packages share the same cache directory. Faster incremental builds across packages.~~ ✅

### Observability & Monitoring

101. ~~**Build telemetry dashboard**~~ ✅ — `pledge dashboard` serves a web UI showing build history, cache hit rates, module counts, and build times over time. Data persisted in `.pledge/history.json`.

102. ~~**Bundle size budget CI integration**~~ ✅ — `pledge build --check-budgets` exits non-zero on budget violations. GitHub Actions annotation format output. PR comment generation with size diff.

103. ~~**Performance regression detection**~~ ✅ — Compare build times across commits. `pledge bench --baseline <ref>` flag. Warns when build time increases by more than configured threshold.

104. ~~**Module dependency graph visualization**~~ ✅ — `pledge analyze --graph` generates interactive force-directed graph of module dependencies. Circular dependency detection highlighted in red.

105. ~~**Build event webhooks**~~ ✅ — `webhooks: { on_build: 'https://api.example.com/build-done' }` config. POST build results to external service on completion. Slack/Discord notification support.

### Internationalization & Accessibility

106. ~~**i18n-aware bundling**~~ ✅ — Split bundles by locale. `i18n: { locales: ['en', 'fr', 'ja'], defaultLocale: 'en' }` config. Only load current locale's strings. `import messages from './messages.${locale}.json'` pattern.

107. ~~**RTL CSS auto-generation**~~ ✅ — Auto-generate RTL CSS from LTR stylesheets using logical properties. `css: { rtl: 'auto' }` config. `direction: rtl` support without manual CSS duplication.

108. ~~**a11y linting during build**~~ ✅ — Check for common accessibility issues in HTML output. Missing `alt` attributes, insufficient color contrast, missing ARIA labels. `a11y: { enabled: true }` config.

109. ~~**Build-time string encryption**~~ ✅ — Encrypt sensitive strings in source at build time, decrypt at runtime via injected shim. `encrypt: { keys: ['API_KEY'] }` config. Prevents plain-text secrets in bundles.

### Advanced Features

110. ~~**Web Components compilation**~~ ✅ — Compile Custom Elements with Shadow DOM from `.wc.tsx` files. Automatic `customElements.define()` registration. Shadow DOM CSS scoping.

111. ~~**Web Worker bundling**~~ ✅ — `import worker from './worker.ts?worker'` syntax. Bundle web workers as separate chunks with proper import URL generation. `new Worker(new URL('./worker.ts', import.meta.url))` pattern.

112. ~~**Shared worker bundling**~~ ✅ — `import worker from './worker.ts?sharedworker'` syntax. Bundle shared workers accessible across multiple browser tabs. Proper `SharedWorker` constructor output.

113. ~~**Service worker caching strategies**~~ ✅ — Configurable caching per route pattern: cache-first, network-first, stale-while-revalidate. `sw: { caching: [{ pattern: '/api/*', strategy: 'network-first' }] }` config. Extends existing SW generation.

114. ~~**Import glob expansion**~~ ✅ — `import.meta.glob('./pages/*.tsx')` syntax. Expands to a map of file paths to lazy import functions at build time. `glob: { eager: false }` config for preloading all or lazy loading.

115. ~~**Module federation support**~~ ✅ — Share modules across independently deployed apps. `federation: { name: 'host', remotes: { app1: 'http://cdn/app1.js' } }` config. Webpack Module Federation-compatible.

116. ~~**GraphQL code generation**~~ ✅ — `pledge build --codegen` generates TypeScript types from `.graphql` files. Schema-first development with type-safe queries. `graphql: { schema: 'schema.graphql' }` config.

117. ~~**Environment-specific builds**~~ ✅ — `pledge build --env staging` loads `.env.staging` and sets `process.env.NODE_ENV = 'staging'`. Multiple environment configs without code changes.

118. ~~**Post-build optimization hooks**~~ ✅ — `plugins: [{ name: 'seo', postBuild: true }]` API. Plugins run after build to optimize HTML meta tags, generate sitemaps, or submit to search engines.

119. ~~**Conditional exports resolution**~~ ✅ — Read `package.json` `exports` field with conditions. `exports: { conditions: ['production', 'browser'] }` config. Correct entry point per environment.

120. ~~**Build concurrency control**~~ ✅ — `build: { parallel: 4 }` config limits concurrent module transforms. Prevents OOM on large projects. Auto-detects optimal concurrency based on CPU cores.

---

## Roadmap v3: 85 Goals (36 Completed ✅)

### Pillar 1: Killer DX Story (19 Completed ✅)

1. ~~**Zero-config create**~~ ✅ — `pledge create` scaffolds project with interactive wizard, generates `package.json`, `pledge.config.ts`, entry files, `.env`, `.gitignore`. 7 framework templates: react, vue, svelte, solid, next, tanstack, vanilla.
2. ~~**Pre-built binary hot start**~~ ✅ — PledgePack is a compiled Rust binary; no Node.js startup overhead, no `node_modules` resolution delay on first run.
3. ~~**Lazy dependency install**~~ ✅ — `LazyPipeline` in `lazy_pipeline.rs` defers transform pipeline initialization until first module request. `DepBundler` pre-bundles deps on-demand.
4. ~~**Instant HMR on first load**~~ ✅ — Dev server serves modules on-demand (Vite-style), no full bundle needed for first page render.
6. ~~**Single binary, zero npm install for framework**~~ ✅ — `pledge create` scaffolds project without requiring `npm install pledgepack` first; binary ships everything.
7. ~~**Progressive template selection**~~ ✅ — Interactive TUI wizard using `dialoguer` with `Select` for framework choice while binary warms up in parallel.
9. ~~**Instant TypeScript**~~ ✅ — Oxc parser handles TS natively in Rust, zero `tsc` warmup or type-check delay for dev.
10. ~~**No Babel/SWC startup cost**~~ ✅ — Oxc parser is Rust-native, no JS transformer initialization overhead.
11. ~~**Built-in env loading**~~ ✅ — `EnvVars` in `env.rs` loads `.env` files in precedence order, injects into `import.meta.env.*`, supports `${VAR}` expansion, generates `.d.ts` declarations.
13. ~~**Dev server ready before browser opens**~~ ✅ — HTTP server starts before full file scanning completes; first request triggers lazy initialization via `lazy_pipeline.ensure_initialized()`.
14. ~~**No `pledge.config.ts` needed**~~ ✅ — `PledgeConfig::default()` provides sensible defaults when no config file is found.
16. ~~**No polyfill boot time**~~ ✅ — Polyfills injected only when actually needed during transform, not eagerly on startup.
17. ~~**Pre-bundled React**~~ ✅ — `DepBundler::pre_bundle()` in `dep_bundler.rs` scans for bare imports, resolves from `node_modules`, converts CJS→ESM, writes to `node_modules/.pledge-deps`.
18. ~~**Streaming HTML shell**~~ ✅ — Dev server sends initial `<html><head>` immediately while modules load, perceived instant load.
19. ~~**No webpack runtime**~~ ✅ — PledgePack uses its own minimal Rust-native module system, zero webpack runtime overhead.
20. ~~**Instant CSS**~~ ✅ — Tailwind/Lightning CSS compiled in-process, no PostCSS pipeline startup overhead.
21. ~~**No `node_modules` traversal on start**~~ ✅ — Custom resolver in `resolver/src/lib.rs` handles aliases, relative, absolute, workspace, and node_modules resolution without Node's recursive traversal.
22. ~~**Warm binary cache**~~ ✅ — `FunctionCache` in `cache/src/lib.rs` — two-tier cache (memory `DashMap` + filesystem `bincode`), persistent across restarts.
23. ~~**Parallel file scanning**~~ ✅ — `rayon` used for parallel module transforms in `BuildEngine::transform_modules_parallel()`.

### Pillar 2: Clear Differentiators (17 Completed ✅)

25. ~~**Native dev server**~~ ✅ — Rust `axum` HTTP server handles requests at bare-metal speed; no Node.js event loop overhead.
26. ~~**No `next dev` Node.js bottleneck**~~ ✅ — PledgePack dev server is Rust-native; Next.js dev server runs through Node.
27. ~~**Boa plugin sandbox**~~ ✅ — Plugins run in sandboxed `boa_engine` JS runtime, can't crash the dev server; Next.js plugins run in same Node process.
28. ~~**Binary distribution**~~ ✅ — One `pledge.exe` binary, no `node_modules` for the bundler; Next.js requires entire webpack/turbopack toolchain.
29. ~~**Zero JavaScript in the hot path**~~ ✅ — Module transformation, CSS, bundling all in Rust; Next.js has JS in the critical path.
30. ~~**Built-in test runner**~~ ✅ — `pledge test` runs Vitest-compatible tests with Boa engine; includes describe/it/test, expect(), hooks, snapshots, coverage, watch mode, UI mode, globals, test environments (node/jsdom/happy-dom).
31. ~~**Built-in bundle analyzer**~~ ✅ — `pledge analyze` generates interactive HTML treemap with module sizes, chunk breakdown, duplicate detection, circular dependency detection, dependency graph visualization.
32. ~~**Built-in LSP**~~ ✅ — `lsp_server.rs` provides import resolution, go-to-definition, diagnostics, hover, and document symbols; supports path aliases and node_modules resolution.
33. ~~**Built-in migration tooling**~~ ✅ — `pledge migrate` detects and converts Vite, Webpack, and Turbopack configurations to Pledge format via `migrate_config()` in `migrate.rs`.
34. ~~**Edge bundle generation**~~ ✅ — `pledge build --edge` outputs edge-compatible bundles via `edge.rs` — supports Cloudflare Workers, Vercel Edge, and Deno Deploy formats with platform-specific config files.
35. ~~**Native file watcher**~~ ✅ — Rust `notify` crate for file watching; no `chokidar` polling on Windows.
36. ~~**No `next.config.js` webpack overrides**~~ ✅ — PledgePack config is declarative; no need to eject or override webpack config.
37. ~~**Single CLI for everything**~~ ✅ — `pledge dev/build/test/analyze/migrate/doctor/bench/cache` all in single binary; Next.js splits across `next`, `vitest`, `bundle-analyzer`.
38. ~~**`pledge doctor`**~~ ✅ — `doctor.rs` runs 5 diagnostic categories (Config, Dependencies, Performance, Project, Security) with pass/warn/fail/info statuses.
39. ~~**`pledge bench`**~~ ✅ — `bench.rs` runs 5-build benchmark with min/max/avg/median, baseline comparison, regression detection with threshold, stores results in `.pledge/bench.json`.
40. ~~**`pledge cache`**~~ ✅ — `pledge cache clear` and `pledge cache stats` subcommands for inspecting and managing build cache.
41. ~~**True framework-agnostic bundler**~~ ✅ — PledgePack works without PledgeStack; supports React, Vue, Svelte, Solid, Astro, Next.js, TanStack, and vanilla TS/JS.

### Pillar 3: Plugin Ecosystem & Community (10 Completed ✅)

#### Framework Adapters & Language Support

43. ~~**pledgepack-plugin-mdx**~~ ✅ — `compile_mdx()` in `asset_pipeline.rs` extracts frontmatter (YAML key-value), converts markdown to JSX (headings, lists, code blocks with language classes, blockquotes, horizontal rules, inline formatting), preserves JSX components, generates ES module with named frontmatter exports + default `MDXContent` render function. `ModuleKind::Mdx` registered.

44. ~~**pledgepack-plugin-tailwind**~~ ✅ — `tailwind_v4.rs` implements Tailwind v4 CSS-first config: `@theme` blocks parsed to CSS custom properties, `@utility` expanded to class definitions, `@variant` expanded to variant-aware selectors, `@import "tailwindcss"` replaced with preflight + theme utilities. Tailwind v3 `@tailwind`/`@apply` directives processed in `postcss.rs` and `transform.rs`. Integrated with Lightning CSS for final optimization.

46. ~~**pledgepack-plugin-vue**~~ ✅ — `transform_vue()` in `transform.rs` extracts `<template>`, `<script setup>`, and `<style>` blocks from `.vue` SFCs. `compile_vue_template()` generates Vue 3 render functions with `h()` calls, supports directives (v-if, v-else, v-for, v-bind, v-on, v-model, v-show, v-text, v-html), mustache interpolation, self-closing tags. Scoped CSS with `data-v-pledge` attribute. Vue HMR with component-level hot replacement. `ModuleKind::Vue` registered.

47. ~~**pledgepack-plugin-svelte**~~ ✅ — `transform_svelte()` in transform dispatch with `ModuleKind::Svelte` registered. SFC block extraction via `extract_sfc_block()` shared with Vue. HMR boundary injection.

48. ~~**pledgepack-plugin-solid**~~ ✅ — `adapter-solid` crate with `SolidAdapter::transform()`. Solid JSX automatic runtime with `solid-js` import source. TypeScript type stripping via Oxc. HMR with reactive scope preservation via `window.__pledge_solid_hmr` boundaries.

#### CSS & Asset Plugins

49. ~~**pledgepack-plugin-postcss**~~ ✅ — `postcss.rs` with `PostCssConfig::from_file()` parses `postcss.config.js/ts/json/.postcssrc.json` and `package.json` postcss field. `process_css()` executes plugins: tailwindcss, autoprefixer (via Lightning CSS), postcss-nested/nesting, postcss-preset-env, cssnano, postcss-import. `BrowserslistConfig` loads from `.browserslistrc`/`package.json` and maps to Lightning CSS targets.

50. ~~**pledgepack-plugin-workbox**~~ ✅ — `service_worker.rs` with `generate_service_worker()` produces SW JS with install/activate/fetch handlers. 5 caching strategies: cache-first, network-first, stale-while-revalidate, network-only, cache-only. Precache URLs, runtime caching rules with regex patterns, offline fallback page, `skipWaiting`/`clients.claim`. Config via `sw: { caching: [...] }` in `pledge.config.ts`.

51. ~~**pledgepack-plugin-pwa**~~ ✅ — `WebAppManifest` struct with name, short_name, description, start_url, display mode, colors, icons. `generate_manifest()` produces `manifest.json`. `generate_pwa_tags()` produces HTML `<link rel="manifest">` + `<meta name="theme-color">` + SW registration script. Manifest icons (192x192, 512x512) with `any maskable` purpose.

53. ~~**pledgepack-plugin-image**~~ ✅ — `image_pipeline.rs` with `process_image()`: decodes image, resizes to 7 responsive widths (640–2048px) using Lanczos3 filter, encodes to WebP/JPEG (AVIF format decision logic in `asset_pipeline.rs`). `generate_srcset()`, `generate_picture_tag()`, `generate_img_tag()` with lazy loading + async decoding. Blur placeholder (LQIP) as base64 data URI. `ImageConfig` in config with `enabled`, `quality`, `webp`, `avif`, `responsive_widths`.

54. ~~**pledgepack-plugin-fonts**~~ ✅ — `fonts.rs` with `FontFormat` (WOFF2/WOFF/TTF/OTF/EOT), `FontSubset` (Latin, LatinExtended, Cyrillic, Greek, Vietnamese) with `unicode_range()` for `@font-face` generation. Font subsetting, `font-display: swap` injection, preload hints for critical fonts, WOFF2 optimization.

#### Data & Type Plugins

55. ~~**pledgepack-plugin-env-types**~~ ✅ — `env.rs` with `EnvVars::generate_dts()` produces `pledge-env.d.ts` with `ImportMetaEnv` interface. Type inference (boolean/number/string) from `.env` values. Built-in vars (`PLEDGE_DEV`, `PLEDGE_PROD`, `PLEDGE_MODE`, `MODE`, `DEV`, `PROD`, `SSR`). CLI `pledge generate-env-types` command. Auto-generated on build when `env_dts: true` (default).

56. ~~**pledgepack-plugin-graphql**~~ ✅ — `parse_graphql()` in `asset_pipeline.rs` extracts operations (query/mutation/subscription/fragment) with names and bodies. `graphql_to_module()` generates ES module with named exports per operation + default document export. `transform_graphql()` in `transform.rs` with `ModuleKind::Graphql`. `GraphqlCodegenConfig` in `advanced.rs` generates TypeScript interfaces + React hooks from schema. `GraphqlConfig` in `config.rs` with `schema`, `output`, `react_hooks` fields.

59. ~~**pledgepack-plugin-csv**~~ ✅ — `transform_csv()` in `asset_pipeline.rs` parses CSV with header row, generates ES module with named exports (headers as keys, rows as arrays) + default export (array of row objects). `transform_tsv()` handles tab-separated. `ModuleKind::Csv` and `ModuleKind::Tsv` registered.

60. ~~**pledgepack-plugin-yaml**~~ ✅ — `transform_yaml()` in `asset_pipeline.rs` uses `serde_yaml` for proper parsing of nested structures, lists, anchors, multi-line strings. Converts to `serde_json::Value` then generates ES module with named exports + default export. `ModuleKind::Yaml` registered.

### Pillar 1: Killer DX Story (5 Completed ✅)

#### Zero-Friction Onboarding

5. ~~**Cached create templates**~~ ✅ — `pledge create` caches template files to `~/.pledge/templates/<framework>/` on first creation. Subsequent creates with the same template copy from cache instead of regenerating from scratch. Uses `__PLEDGE_PROJECT_NAME__` placeholder in cached entry files for project name substitution. `copy_dir_recursive()` helper skips `node_modules`, `.pledge`, `.git` dirs during cache copy.

8. ~~**Pre-warmed module graph**~~ ✅ — After project creation, `prewarm_module_graph()` scans all source files (`.ts`, `.tsx`, `.js`, `.jsx`, `.css`, `.json`, `.vue`, `.svelte`, `.scss`, `.sass`) and writes `.pledge/module-graph.json` with module paths and kinds. Dev server can load this pre-built graph on startup for faster cold starts.

12. ~~**Flash create**~~ ✅ — `--flash` flag on `pledge create` skips the interactive wizard, uses `react` as default template (or provided template), and uses minimal output format. No git init, no README — just project files and a "cd <name> && pledge dev" hint.

#### Instant Feedback Loop

15. ~~**Instant route scanning**~~ ✅ — `scan_app_dir()` in `router.rs` is Rust-native, called by `router_handler` in dev server. Scans `app/` directory for file-based routes, layouts, loading states, and error boundaries. No JavaScript runtime involved in route table generation.

#### Performance Visibility

24. ~~**Time-to-first-paint metric in terminal**~~ ✅ — `serve()` in `dev-server/src/lib.rs` captures `std::time::Instant::now()` at startup. Prints `Ready in Xms` (green, bold) before both HTTP and HTTPS server start paths, making startup speed visible and measurable.

### Pillar 2: Clear Differentiators vs Next.js (1 Completed ✅)

#### Platform & Architecture

42. ~~**Open governance potential**~~ ✅ — PledgePack is not owned by a hosting company. Unlike Next.js's Vercel alignment, PledgePack's governance model is community-oriented. This is a conceptual differentiator, not a code feature.

### Pillar 3: Plugin Ecosystem & Community (9 Completed ✅)

#### Framework Adapters & Language Support

45. ~~**pledgepack-plugin-sass**~~ ✅ — `transform_sass()` in `transform.rs` uses `grass` crate (pure Rust Sass/SCSS compiler, v0.13) for compilation. Supports both `.scss` (SCSS syntax) and `.sass` (indented syntax) via `grass::InputSyntax`. Production mode uses `OutputStyle::Compressed`, dev uses `OutputStyle::Expanded`. CSS module support for `.module.scss`/`.module.sass` files via `generate_css_module_map()`. Source maps in dev via `css_features::generate_css_source_map()`. `ModuleKind::Sass` registered in `module.rs` and `module_graph.rs`.

### Pillar 3: Plugin Ecosystem & Community (8 Completed ✅)

#### CSS & Asset Plugins

52. ~~**pledgepack-plugin-favicons**~~ ✅ — `favicons.rs` module with `generate_favicons()` produces 16x16, 32x32, 180x180 (apple-touch-icon), 512x512 PNG sizes from a single source image using the `image` crate. Generates multi-resolution `favicon.ico` (16+32 embedded PNGs). `generate_favicon_html()` produces `<link>` tags for all sizes. `generate_manifest_icons()` produces PWA Web App Manifest icon entries with `any maskable` purpose.

#### Data & Type Plugins

57. ~~**pledgepack-plugin-prisma**~~ ✅ — `prisma.rs` module with `parse_schema()` parses Prisma schema files (datasource, generator, model, enum blocks). `generate_types()` produces TypeScript interfaces, enums, `PrismaClient` and `PrismaModel<T>` types with CRUD method signatures. `generate_query_logger()` produces dev-mode `$use` middleware that logs query model, action, and duration. `validate_schema()` checks for missing datasource/generator/ID fields.

58. ~~**pledgepack-plugin-drizzle**~~ ✅ — `drizzle.rs` module with `parse_schema()` parses Drizzle ORM TypeScript table definitions (`pgTable`, `sqliteTable`, `mysqlTable`). Extracts table names, column types, constraints (primaryKey, notNull, unique, default, references). `generate_migration()` produces SQL `CREATE TABLE` statements with proper types and constraints. `diff_schemas()` generates `ALTER TABLE` statements for schema changes (added/dropped columns, new/dropped tables).

61. ~~**pledgepack-plugin-toml**~~ ✅ — `transform_toml()` in `transform.rs` uses `toml` crate (v0.8) for parsing. Converts `toml::Value` to `serde_json::Value` for consistent serialization, then generates ES module with named exports for top-level table keys + default export. `ModuleKind::Toml` registered in `module.rs` and `module_graph.rs`. `.toml` extension mapped in `from_extension()`.

#### Specialized & Advanced Plugins

62. ~~**pledgepack-plugin-svgr**~~ ✅ — `svg_to_react_component()` in `svg.rs` converts SVG files to React components. Optimizes SVG first (remove comments, metadata, empty elements), then converts attributes to React-compatible camelCase (`class` → `className`, `stroke-width` → `strokeWidth`, etc.). Generates `export function ComponentName()` with JSX. Supports React, Vue, Svelte, Solid frameworks via `SvgComponentFramework` enum. `?sprite` suffix generates SVG sprite symbols.

63. ~~**pledgepack-plugin-shader**~~ ✅ — `transform_shader()` in `transform.rs` converts GLSL/WGSL shader files into ES module string exports. Supports `.glsl`, `.vert` (vertex), `.frag` (fragment), `.comp` (compute), `.wgsl` (WebGPU) extensions. Exports `shader` (default), `shaderType` ("vertex"/"fragment"/"compute"/"wgsl"/"glsl"), and `shaderSource` as template literal strings with proper escaping. `ModuleKind::Shader` registered in `module.rs` and `module_graph.rs`.

64. ~~**pledgepack-plugin-wasm**~~ ✅ — `transform_wasm()` in `transform.rs` generates async WASM instantiation code. Supports SIMD auto-detection via `WebAssembly.validate()` (#55), streaming compilation with `WebAssembly.instantiateStreaming()` (#74), and fallback for older browsers. Configurable via `build.wasm_simd` config: `"always"` (force SIMD), `"never"` (non-SIMD), `"auto"` (runtime detection). Generates `.simd.wasm` variant URL for SIMD path.

65. ~~**pledgepack-plugin-visualizer**~~ ✅ — `generate_flamegraph_html()` in `analyzer.rs` produces an interactive HTML flamegraph visualization. Groups modules by chunk, sorts by size descending, renders horizontal bars with width proportional to size. Click-to-expand chunk bars reveal per-module breakdown. Hover tooltips show module path, size, kind, and entry status. Color-coded by type (entry=red, CSS=green, JS=yellow, other=purple). Uses `crate::format_size()` for human-readable sizes. Complements existing treemap visualization.

### Pillar 3: Plugin Ecosystem & Community (4 Completed ✅)

#### Community & Tooling

66. ~~**PledgePack playground**~~ ✅ — `playground.rs` module with `serve_playground()` starts a local HTTP server (default port 8080) serving an interactive HTML page with a code editor (textarea), transform options, and live output. The playground generates a self-contained HTML page with CSS styling, JavaScript for transform simulation, and a simple HTTP server using `std::net::TcpListener`. CLI command: `pledge playground [--port <port>]`.

67. ~~**PledgePack plugin registry**~~ ✅ — `plugin_registry.rs` module with `search_plugins()` queries the npm registry API (`registry.npmjs.org/-/v1/search`) for `pledgepack-plugin-*` packages using `ureq` HTTP client. `install_plugin()` detects package manager (npm/pnpm/yarn) and runs install command. `list_installed_plugins()` scans `node_modules/` for installed pledgepack plugins. `format_plugin_list()` renders plugin info as a formatted table. CLI commands: `pledge plugin search [query]`, `pledge plugin install <name> [--dev]`, `pledge plugin list`.

68. ~~**Plugin template generator**~~ ✅ — `plugin_template.rs` module with `scaffold_plugin()` generates a complete plugin project: `package.json` with pledgepack plugin metadata, `index.js` with hook stubs (resolveId, load, transform, buildStart, buildEnd), `pledge.config.ts` for plugin dev config, `test.test.js` with basic test setup, and `README.md` with usage docs. `PluginHook` enum defines all available hooks. `PluginTemplateOptions` struct configures name, description, author, and hooks. CLI command: `pledge plugin create <name> [--description <desc>] [--author <author>]`.

69. ~~**Plugin documentation generator**~~ ✅ — `plugin_docs.rs` module with `generate_plugin_docs()` parses JavaScript/TypeScript plugin source files to extract: plugin name (from `name:` field), description (from JSDoc `@description`), hook signatures (function names, parameters, return types), and `@param`/`@returns` JSDoc annotations. `render_markdown()` produces formatted markdown output with plugin name, description, hook table, and parameter details. `parse_jsdoc()` and `parse_jsdoc_param()` helpers extract JSDoc tags. CLI command: `pledge plugin docs <file> [-o <output>]`.

### Pillar 4: Developer Tooling (6 Completed ✅)

#### Build & Analysis

71. ~~**Type checking during build**~~ ✅ — `type_check.rs` module with `run_type_check()` executes `tsc --noEmit --pretty false` and parses output for type errors. `TypeCheckResult` struct contains success flag, error list with file path, line/column, and message. `format_type_check_result()` renders formatted error output. `BuildConfig` extended with `type_check: bool` field. Integrated into build pipeline: runs after build completes, before optimization. Fails build on type errors. CLI flag: `pledge build --type-check`.

72. ~~**Type-aware tree shaking**~~ ✅ — `strip_type_only_imports()` in `type_check.rs` detects `import type` statements and removes them from runtime bundle output. Scans for `import type { ... }` and `import type ... from` patterns, preserving runtime imports while stripping type-only imports. Prevents type-only imports from inflating the bundle.

73. ~~**`.d.ts` bundling for library mode**~~ ✅ — `bundle_declarations()` in `type_check.rs` recursively inlines `.d.ts` file imports into a single declaration file. Resolves relative import paths, reads imported `.d.ts` files, and inlines their content. `bundle_type_declarations()` is the public API that takes a root project path and entry `.d.ts` file, producing a bundled declaration string.

74. ~~**Type-safe plugin API**~~ ✅ — `plugin_types.rs` module generates TypeScript declaration files for the plugin API. `generate_plugin_types()` produces interfaces for `PledgePlugin`, `PluginContext`, `ResolveIdResult`, `LoadResult`, `TransformResult`, `BuildStartContext`, `BuildEndContext`, `GenerateBundleContext`, and `RollupPlugin` (compatibility interface). `publish_plugin_types()` writes the declarations to `dist/pledgepack-plugin.d.ts`. Includes `PluginHook` type union and `PluginMeta` interface.

75. ~~**Visual regression testing**~~ ✅ — `visual_regression.rs` module with `run_visual_tests()` captures screenshots from a local dev server, compares against baselines using pixel diff, and generates HTML reports. `VisualRegressionConfig` controls enabled flag, threshold (default 0.01 = 1%), baseline directory (`.pledge/visual-baselines/`), and update mode. `ScreenshotReport` tracks passed/failed/new/updated counts. `format_visual_report()` renders CLI summary. `generate_visual_html_report()` produces interactive HTML with side-by-side comparisons. CLI flags: `pledge test --visual [--update-baselines]`.

#### Dev Experience

82. ~~**`pledge why` command**~~ ✅ — `find_import_chains()` in `analyzer.rs` traces import paths from entry points to a target module using BFS traversal of the dependency graph. `bfs_path()` performs breadth-first search on the module adjacency list. Returns all unique chains from entries to the target. CLI command: `pledge why <module>` — prints each chain with arrow notation showing the full dependency path.
