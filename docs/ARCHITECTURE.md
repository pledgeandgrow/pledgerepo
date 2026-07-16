# Pledge Architecture

## Overview

Pledge is a hybrid Rust + Zig bundler that uses:
- **Zig** for hot paths (file I/O, module graph, SIMD scanning, hashing) via C ABI
- **Rust** for orchestration (build engine, resolver, cache, dev server, optimizer, plugin host)
- **Oxc** for JavaScript/TypeScript/JSX transformation (replacing SWC/esbuild)
- **Wasmtime** for sandboxed WASM plugins (replacing V8)

## Data Flow

```
User source files (src/*.tsx, *.ts)
        │
        ▼
   BuildEngine
   ├── Resolver ──► resolves import specifiers to file paths
   ├── Zig I/O ────► read_file() via C ABI (mmap, thread pool)
   ├── Transform ──► Oxc: parse → semantic → transform → codegen
   ├── Cache ──────► memory (DashMap) → disk (bincode) → transform
   └── Graph ──────► Zig arena-allocated module graph (0B/node)
        │
        ▼
   Optimizer
   ├── Tree shaking (reachability from entry)
   ├── Chunk splitting (entry / vendor / shared)
   └── Scope hoisting (ESM, no wrappers)
        │
        ▼
   Emitter ──► dist/ (JS files + index.html)
```

## Crate Dependency Graph

```
pledgepack-cli
├── pledgepack-core (engine, config, transform, pipeline, env, html, compression, analyzer, edge, dep_bundler, polyfills, transform_optimizations, css_features, css_in_js, tailwind_v4, asset_pipeline, plugin_system, output_distribution, service_worker, lsp_server, migrate, module_graph, remote, git_cache, watcher, hmr_diff, lazy_pipeline, middleware, doctor, config_validate, telemetry, budgets, bench, webhooks, i18n, rtl, a11y, encrypt)
│   ├── pledgepack-cache (function-level cache, memory + disk)
│   ├── pledgepack-native-sys (FFI to Zig)
│   ├── oxc (parser, semantic, transformer, codegen)
│   ├── lightningcss (CSS minification, CSS Modules)
│   ├── blake3 (content hashing for CSS Modules + cache keys)
│   ├── flate2 (gzip compression)
│   ├── brotli (Brotli compression)
│   ├── wasmtime (WASM plugin sandboxing)
│   ├── rayon (parallel transforms, parallel plugin execution)
│   └── dashmap (concurrent cache, concurrent plugin registry)
├── pledgepack-dev-server (axum, notify, tokio-tungstenite, reqwest, rustls)
│   ├── pledgepack-core
│   ├── pledgepack-native-sys
│   └── oxc
├── pledgepack-optimizer (tree shaking, chunk splitting)
│   └── pledgepack-core
├── pledgepack-resolver (tsconfig, exports, node_modules)
├── pledgepack-plugin-host (wasmtime, javy CLI integration)
├── pledgepack-js-plugin-host (boa_engine JS runtime, Vite-compatible plugin API)
├── pledgepack-adapter-react (Oxc JSX, Fast Refresh)
├── pledgepack-adapter-solid (Oxc JSX, solid-js automatic runtime)
├── pledgepack-adapter-next (App/Pages Router, SSR, API routes)
├── pledgepack-adapter-tanstack (file-based routing, route tree)
├── axum + tower-http (serve/analyze commands)
└── tokio (async runtime)
```

## Zig Native Layer (`native-sys/` + `native-sys/zig/*.zig`)

### C ABI Exports
- `graph_create()`, `graph_add_module()`, `graph_add_dependency()` — Arena-allocated module graph
- `read_file(path) → bytes` — Memory-mapped file I/O with thread pool fallback
- `find_imports(source) → Vec<String>` — SIMD-accelerated import scanning
- `hash_content(source) → u64` — Content hashing for cache keys
- `___chkstk_ms` — Windows x86_64 stack probing (required for Zig stack frames)

### Key Design Decisions
- **Arena allocation**: Module graph nodes have zero per-node allocation overhead
- **SIMD scanning**: Import specifiers found via 32-byte SIMD pattern matching
- **io_uring / IOCP**: Async file I/O on Linux (io_uring) and Windows (IOCP via thread pool)
- **Stack probing**: Custom `___chkstk_ms` implementation for Windows compatibility

## Rust Orchestration Layer

### BuildEngine (`crates/core/src/engine.rs`)
- BFS module graph traversal from entry point
- Per-module: resolve → read → transform → cache → enqueue dependencies
- Two-tier cache: memory (`HashMap`) → disk (`FunctionCache` with bincode)
- Emits transformed JS to `dist/` preserving directory structure

### Transform Pipeline (`crates/core/src/transform.rs`)
```
Source string
    │
    ├── Oxc Parser (SourceType from file extension)
    │       → AST (Program)
    │
    ├── Oxc SemanticBuilder
    │       → SymbolTable + ScopeTree
    │
    ├── Oxc Transformer (framework-aware JSX, TS type stripping)
    │       → React: JsxRuntime::Automatic (react/jsx-runtime)
    │       → Solid: JsxRuntime::Automatic, import_source=solid-js
    │       → Vue: JsxRuntime::Automatic, import_source=vue
    │       → Transformed AST
    │
    ├── Oxc Minifier (production only)
    │       → Dead code elimination, variable mangling
    │
    ├── Oxc Codegen (optional minify)
    │       → JavaScript string
    │
    └── Post-processing
            ├── Environment variable replacement (import.meta.env.PLEDGE_*)
            ├── Define replacement (compile-time constants from config.define)
            ├── import.meta.glob expansion (glob-based file imports for dynamic route/component discovery)
            ├── Dynamic import detection (Oxc AST ImportExpression visitor)
            ├── Web Worker transform (Worker + SharedWorker patterns)
            └── React Fast Refresh injection (dev mode, React only)
```

### Framework Adapters

#### Vue SFC (`transform_vue`)
- Extracts `<template>`, `<script setup>`, `<style scoped>` blocks
- TypeScript transform: `<script lang="ts">` blocks transformed with Oxc (type stripping)
- Compiles template to render function
- Extracts scoped CSS with `[data-v-pledge]` attribute selectors
- HMR boundary: `import.meta.hot.accept()` injected in dev mode

#### Svelte (`transform_svelte`)
- Extracts `<script>`, `<style>`, and markup from `.svelte` files
- TypeScript transform: `<script lang="ts">` blocks transformed with Oxc (type stripping)
- Generates DOM render function with mount/unmount lifecycle
- Scoped CSS with `[svelte-pledge]` attribute selectors
- HMR boundary: `import.meta.hot.accept()` injected in dev mode

#### Astro (`transform_astro`)
- Parses `---` frontmatter delimiters
- TypeScript transform: Frontmatter TS transformed with Oxc (type stripping)
- Compiles template to async render function
- Extracts `<style>` blocks as CSS
- HMR boundary: `import.meta.hot.accept()` injected in dev mode

#### Next.js (`crates/adapter-next/`)
- App Router: scans `app/` for page.tsx, layout.tsx, loading.tsx, error.tsx
- Pages Router: scans `pages/` for index.tsx, [param].tsx
- Generates client-side router with lazy imports
- Generates SSR manifest (JSON)

#### TanStack (`crates/adapter-tanstack/`)
- Scans `src/routes/` for file-based routes
- `$param` files → dynamic route segments
- Generates route tree with lazy imports
- Generates route manifest with parent/child relationships

### CSS Processing (`transform_css` + `process_postcss`)
- Lightning CSS: minification, nesting, autoprefixing
- PostCSS pipeline: `@tailwind` directives, `@apply` expansion
- 80+ Tailwind utility class mappings
- CSS Modules: `*.module.css` scoped class names with blake3 content hashing (`generate_css_module_map`)

### Resolver (`crates/resolver/src/lib.rs`)
- Resolution order: aliases (tsconfig) → relative → absolute → node_modules
- Package.json: `exports` (modern) → `module` → `main` → `browser`
- Exports conditions: `browser` > `import` > `module` > `require` > `default`
- Subpath patterns: `./utils/*` → `./utils/*.js`
- DashMap cache per (importer, specifier) pair

### Cache (`crates/cache/src/lib.rs`)
- `CacheKey`: blake3 hash of (content_hash, function_id, params)
- `CacheEntry`: { code, source_map, deps, created_at }
- Memory: `DashMap<CacheKey, CacheEntry>` — lock-free concurrent reads
- Disk: `bincode` serialization to `node_modules/.pledge-cache/`
- `FunctionCache::new(dir, persist)` — controls disk persistence

### Optimizer (`crates/optimizer/src/lib.rs`)
- **Reachability**: BFS from entry modules, mark all reachable
- **Side effects**: Heuristic detection (top-level assignments, console.log, etc.)
- **Chunk types**: Entry, Vendor (node_modules), Shared (2+ entries)
- **Scope hoisting**: ESM imports preserved, no CommonJS wrappers

### Dev Server (`crates/dev-server/src/lib.rs`)
- **Axum** router: `/` → index.html, `/__pledge_hmr` → WebSocket, `/__pledge_error` → error overlay, `/*` → module handler
- **On-demand transform**: Each HTTP request triggers full Oxc pipeline
- **AST-based import rewriting**: Oxc parser rewrites imports with string fallback (`./utils` → `./utils.js`)
- **Alias rewriting**: `@/components` → `/src/components` (resolve aliases)
- **Extension fallback**: `/src/utils.js` → resolves to `utils.ts` on disk
- **Import map injection**: Bare specifiers in `node_modules` resolved via import map in HTML
- **HTTPS support**: TLS via rustls + tokio-rustls (config: `https: { cert, key }`)
- **HMR**: `notify` crate watcher → debounce 150ms → WebSocket push to clients
- **CSS HMR**: CSS file changes send content via WebSocket, `<style>` tags updated in-place
- **Error overlay**: Transform errors sent via WebSocket with source context, file path, color-coded lines
- **Runtime error overlay**: `window.addEventListener('error')` and `window.addEventListener('unhandledrejection')` catch runtime browser errors and display them in the overlay with stack traces
- **Auto-open browser**: `open: true` config auto-opens default browser on dev server start (`start` on Windows, `open` on macOS, `xdg-open` on Linux)
- **Dev server proxy**: All HTTP methods (GET, POST, PUT, DELETE, PATCH) proxied via reqwest
- **WebSocket proxy**: `ws: true` on proxy config enables bidirectional WS proxying
- **Source maps**: `sourceMappingURL` comments appended to dev server responses
- **React Fast Refresh**: Component state preservation via `window.__pledge_fast_refresh`
- **React shim**: Minimal `React.createElement` injected in HTML (no React install needed)
- **Web Workers**: `new Worker(new URL(...))` patterns transformed for dev
- **Dynamic imports**: Oxc AST `ImportExpression` visitor for accurate detection

### Plugin Host (`crates/plugin-host/src/lib.rs`)
- **Wasmtime** engine loads `.wasm` plugin files
- Plugin exports: `transform(ptr, len, path_ptr, path_len) → result_ptr`, `alloc(len) → ptr`, `memory`
- Data passing: `alloc` → `memory.write` → `call transform` → `memory.read` → JSON deserialize
- Result format: `[i32 length][JSON bytes]` at returned pointer

### JS Plugin Host (`crates/js-plugin-host/src/lib.rs`)
- **Vite-compatible hooks**: `resolveId`, `load`, `transform`, `transformIndexHtml`, `configureServer`, `buildStart`, `buildEnd`, `generateBundle`
- **Embedded JS runtime**: `boa_engine` evaluates plugin source and executes hooks in-process
- **Console support**: `console.log()` injected for plugin debugging
- **Plugin parsing**: Scans JS/TS source for hook definitions, evaluates source in JS context
- **Hook execution**: `transform()` hook calls JS function and parses JSON result
- **Build lifecycle**: `buildStart()` / `buildEnd()` / `generateBundle()` called during build

### Javy Integration (`crates/plugin-host/src/lib.rs`)
- **JS-to-WASM compilation**: Shells out to `javy compile` CLI to produce WASM plugins
- **Fallback**: Falls back to embedded JS runtime if javy is not installed
- **Install**: `npm install -g @bytecodealliance/javy`

### Test Runner (`crates/js-plugin-host/src/test_runner.rs`)
- **Vitest-compatible API**: `describe`, `it`, `test`, `expect` with matchers (`toBe`, `toEqual`, `toBeTruthy`, `toContain`, `toHaveLength`, `toThrow`, `not` inverse matchers)
- **Lifecycle hooks**: `beforeAll`, `beforeEach`, `afterEach`, `afterAll`
- **Embedded JS runtime**: Tests run in `boa_engine` with `console.log` and `require()` shim
- **TypeScript stripping**: TS syntax automatically stripped for Boa compatibility
- **Mock support**: `vi.fn()`, `vi.mock()`, `vi.spyOn()`, `vi.stubGlobal()` for Vitest-compatible mocking
- **Snapshot testing**: `toMatchSnapshot()` and `toMatchInlineSnapshot()` with `SnapshotStore` for `.snap` file persistence, auto-update mode, and mismatch error reporting
- **Coverage reporting**: `CoverageReport` with text, JSON, HTML, and LCOV output formats; line/function/branch coverage tracking
- **Test setup files**: `test.setup_files` config for running setup code before each test file
- **Test environments**: `test.environment` config — `node` (default), `jsdom` (DOM shims: document, window, navigator, location, customElements, MutationObserver, getComputedStyle), `happy-dom` (lighter DOM shims)
- **Globals mode**: `test.globals: true` to run tests with global `describe`, `it`, `test`, `expect` without imports
- **Test isolation**: `test.isolation` config — `file` (each file in own Boa context), `pool` (shared pool), `none` (no isolation)
- **UI mode**: `pledge test --ui` generates HTML report with pass/fail/skip summary, per-test status, error details, and serves it at `localhost:5174` with auto-browser-open
- **Config integration**: `run_test_file_with_config()` accepts `TestConfig` for full configuration support

### Environment Variables (`crates/core/src/env.rs`)
- **File loading**: `.env` → `.env.local` → `.env.[mode]` → `.env.[mode].local` (highest precedence last)
- **Variable expansion**: `${VAR}` syntax for referencing other env vars
- **Injection**: `import.meta.env.PLEDGE_*` replaced in code during transform
- **Built-in vars**: `PLEDGE_DEV`, `PLEDGE_PROD`, `PLEDGE_MODE`, `MODE`, `DEV`, `PROD`, `SSR`
- **Type generation**: `generate_dts()` produces `pledge-env.d.ts` with typed `ImportMetaEnv` interface

### HTML Processing (`crates/core/src/html.rs`)
- **Parsing**: Extracts `<script type="module">` src, `<link rel="stylesheet">`, `<link rel="modulepreload">`, `<title>`, `<meta>` tags
- **Production HTML**: Replaces script src with hashed filenames, injects CSS `<link>` tags
- **HTML minification**: `minify_html()` removes comments, collapses whitespace, strips redundant spaces
- **Default generation**: `generate_default_html()` creates `index.html` with entry script and title

### Source Maps (`crates/core/src/transform.rs`)
- **V3 format**: Source maps with `sourcesContent` for debugging
- **Dev + production**: Generated in both modes

### Dependency Pre-Bundling (`crates/core/src/dep_bundler.rs`)
- **Scanning**: Recursively scans source files for bare (non-relative) imports
- **CJS → ESM**: Generates ESM interop wrappers for CommonJS modules
- **Resolution**: Reads `package.json` `module`/`main` fields, prefers ESM
- **Output**: Pre-bundled deps written to `node_modules/.pledge-deps/`

### Parallel Transforms (`crates/core/src/engine.rs`)
- **Rayon**: `transform_modules_parallel()` uses `rayon::par_iter` for multi-core processing
- **Batch**: All modules transformed in parallel, errors propagated

### Compression (`crates/core/src/compression.rs`)
- **Gzip**: Real gzip compression via `flate2` — `.gz` files for JS, CSS, HTML, JSON, SVG, WASM
- **Brotli**: Real Brotli compression via `brotli` crate — `.br` files generated alongside gzip
- **Stats**: File count, original/compressed sizes, compression ratios

### Node.js Polyfills (`crates/core/src/polyfills.rs`)
- **20 built-in modules**: buffer, process, path, crypto, stream, util, events, url, os, fs, http, https, net, tls, zlib, querystring, string_decoder, timers, assert, child_process
- **Browser-safe**: Minimal ESM-compatible polyfills using Web APIs (Web Crypto, TextEncoder, fetch, etc.)
- **node: prefix**: Supports both `import 'path'` and `import 'node:path'` specifiers

### Define / Compile-Time Constants (`crates/core/src/transform.rs`)
- **Constant replacement**: Replace identifiers with literal values at build time
- **Type inference**: Automatically wraps strings, preserves numbers/booleans
- **Config**: `define: { 'process.env.NODE_ID': '"production"' }`

### import.meta.glob (`crates/core/src/transform.rs:expand_import_meta_glob`)
- **Glob-based file imports**: `import.meta.glob('./pages/*.tsx')` expanded at transform time
- **Lazy mode**: Default — returns object mapping paths to `() => import('./pages/Home.tsx')` dynamic import functions
- **Eager mode**: `{ eager: true }` — returns object mapping paths to directly imported modules
- **Query support**: `?raw` query returns file content as string, `import` filter for import-only
- **Recursive wildcards**: `**` for recursive directory matching (e.g., `./components/**/*.tsx`)
- **Path keys**: Object keys are the matched file paths relative to the importing module

### Library Mode (`crates/core/src/config.rs`)
- **Multiple formats**: ESM, CJS, UMD, IIFE output formats
- **External dependencies**: Mark packages as external (not bundled)
- **Type declarations**: Optional `.d.ts` generation

### Build Profiling (`crates/core/src/pipeline.rs`)
- **Per-phase timing**: Parse + Transform, Optimize, Emit phases timed individually
- **Total build time**: End-to-end build duration reported
- **Enable**: `pledge build --profile` or `profile: true` in config

### Edge Output (`crates/core/src/edge.rs`)
- **Cloudflare Workers**: Service Worker format with `fetch` handler + `wrangler.toml`
- **Vercel Edge**: Edge function format with `config.runtime = 'edge'` + `vercel.json`
- **Deno Deploy**: `Deno.serve()` format + `deno.json`

### Build Analyzer (`crates/core/src/analyzer.rs`)
- **Per-module**: Original + transformed sizes, dependencies, module kind
- **Chunks**: Modules grouped by directory with size summaries
- **Duplicates**: Same module name in different paths flagged
- **HTML report**: `pledge analyze` serves interactive HTML at `localhost:4200`

### Transform Optimizations (`crates/core/src/transform_optimizations.rs`)
- **WASM target compilation**: `?wasm` import suffix detects WASM modules and generates JS glue code
- **Tree shaking with side-effects**: `analyze_side_effects()` checks for global writes, DOM access, console calls
- **Cross-chunk variable hoisting**: `analyze_cross_chunk_hoisting()` tracks variables imported across chunks
- **CSS tree shaking**: `extract_used_class_names()` finds className/class/:class attributes including template literals; `shake_css()` filters unused CSS rules
- **Dead code elimination**: `eliminate_dead_code()` handles `if (false)`, `if (true)`, strict comparisons, typeof checks
- **Constant folding**: `fold_constants()` handles numeric, string, boolean, and typeof expression folding
- **Optional chaining optimization**: `optimize_optional_chaining()` simplifies redundant null checks
- **Module-level memoization**: `ModuleTransformCache` with blake3 content + config hash keys and LRU eviction

### CSS Features (`crates/core/src/css_features.rs`)
- **CSS `@layer` management**: `parse_layers()` detects and orders cascade layers
- **Container queries polyfill**: `polyfill_container_queries()` for older browser support
- **Critical CSS extraction**: `extract_critical_css()` finds above-the-fold selectors; `inline_critical_css()` inlines into HTML
- **CSS source maps**: `generate_css_source_map()` maps output to original `.scss`/`.less`/`.css`

### CSS-in-JS (`crates/core/src/css_in_js.rs`)
- **Compile-time extraction**: styled-components, emotion, vanilla-extract patterns
- **JS object to CSS**: `js_object_to_css()` converts JS style objects to CSS declarations
- **Template literal extraction**: Parses tagged template literals for CSS content

### Tailwind v4 (`crates/core/src/tailwind_v4.rs`)
- **Oxide engine integration**: `@theme`, `@utility`, `@variant` directive support
- **Theme detection**: `detect_tailwind_v4_theme()` identifies v4 config patterns
- **Utility generation**: Dynamic utility class generation from theme tokens

### Asset Pipeline (`crates/core/src/asset_pipeline.rs`)
- **MDX compilation**: `compile_mdx()` — Markdown + JSX with frontmatter extraction
- **GraphQL loading**: `parse_graphql()` + `graphql_to_module()` with TypeScript type generation
- **YAML/CSV/TSV imports**: Typed named exports from data files
- **Image format auto-selection**: `select_image_format()` + `generate_picture_element()` for WebP/AVIF
- **Audio/video assets**: URL exports with metadata
- **PDF assets**: Inline base64 support
- **Asset manifest**: `AssetManifest` with content-hashed output paths

### Plugin System (`crates/core/src/plugin_system.rs`)
- **Hot reload**: `PluginHotReloader` watches plugin files and reloads without restart
- **WASM sandboxing**: `SandboxLimits` (memory, CPU time) + `SandboxedFs` (filesystem access control)
- **Dependency resolution**: `PluginDependencyResolver` with import maps for npm packages in WASM sandbox
- **Lifecycle hooks**: `LifecycleHookRegistry` — `watchStart`, `watchChange`, `watchEnd`, before/after transform/build
- **Parallel execution**: `execute_parallel_transforms()` via rayon thread pool

### Output Distribution (`crates/core/src/output_distribution.rs`)
- **Performance budgets**: `check_budget()` enforces per-entry and per-chunk size limits
- **Bundle size diff**: `diff_snapshots()` + `format_diff_report()` with regression detection
- **Source map explorer**: `build_source_map_tree()` + `generate_explorer_html()` with interactive treemap
- **Multi-format output**: `generate_multi_format()` — ESM, CJS, IIFE, UMD for library mode

### Service Worker (`crates/core/src/service_worker.rs`)
- **Service worker generation**: Precaching strategies (cache-first, network-first, stale-while-revalidate)
- **Web App Manifest**: `generate_manifest()` produces manifest.json with icons, theme, display mode

### LSP Server (`crates/core/src/lsp_server.rs`)
- **Import resolution**: `extract_import_path()` parses import/require statements
- **Go-to-definition**: Resolves module specifiers to file paths
- **Diagnostics**: Real-time error reporting with `DiagnosticSeverity`
- **Hover info**: Type and documentation on hover
- **Document symbols**: `SymbolKind` enumeration for outline view

### Migration Tooling (`crates/core/src/migrate.rs`)
- **Config migration**: `migrate_config()` from Vite/webpack/CRA/Next.js to `pledge.config.ts`
- **Dry run**: `--dry-run` flag shows what would be migrated without writing files
- **Framework detection**: Auto-detects framework from existing config files

### Incremental Build Graph (`crates/core/src/module_graph.rs`)
- **Content-hash change detection**: Only rebuild changed modules and transitive dependents
- **Persistent serialization**: `SerializableModuleGraph` saves/loads via bincode to `module_graph.bin`

### Remote Cache (`crates/core/src/remote.rs`)
- **S3/GCS/HTTP backends**: `RemoteCache` with automatic fallback
- **3-tier cache**: Memory → disk → remote, integrated in `BuildEngine`

### Git Cache Invalidation (`crates/core/src/git_cache.rs`)
- **Git tree hashes**: `GitCacheInvalidator` uses `git ls-files` and `git rev-parse HEAD^{tree}`
- **Faster invalidation**: Tree hash comparison instead of per-file content hashing

### Dev Server Optimizations
- **Native file watcher** (`crates/core/src/watcher.rs`): Platform-specific inotify/FSEvents/ReadDirectoryChangesW
- **HMR partial updates** (`crates/core/src/hmr_diff.rs`): LCS-based line-level diff via WebSocket
- **Cold boot optimization** (`crates/core/src/lazy_pipeline.rs`): Deferred Oxc/Lightning CSS initialization
- **Middleware chain** (`crates/core/src/middleware.rs`): Configurable request processing pipeline

### Observability & Monitoring (#101–#105)
- **Build telemetry dashboard** (`crates/core/src/telemetry.rs`): `pledge dashboard` command serves interactive web UI at `localhost:4300` with build history chart, cache hit rate, module counts, and build durations. Build records persisted to `.pledge/history.json` (max 100 entries).
- **Bundle size budget CI** (`crates/core/src/budgets.rs`): `pledge build --check-budgets` flag verifies total bundle size, per-chunk size, chunk count, and per-entry budgets. Exits non-zero on violations. Emits GitHub Actions `::error` annotations when `GITHUB_ACTIONS` env is set. Generates PR comment markdown with chunk size table.
- **Performance regression detection** (`crates/core/src/bench.rs`): `pledge bench --baseline <ref>` compares median build time against stored baseline. `--threshold` flag sets regression percentage (default 10%). Baseline results persisted in `.pledge/bench.json` keyed by git ref.
- **Module dependency graph** (`crates/core/src/analyzer.rs`): `pledge analyze --graph` generates interactive force-directed graph HTML with canvas-based physics simulation. Circular dependencies detected via DFS and highlighted in red. Legend distinguishes entry, CSS, module, and circular nodes.
- **Build event webhooks** (`crates/core/src/webhooks.rs`): `webhooks: { onBuild: URL, onError: URL }` config sends POST requests after builds. Auto-detects Slack and Discord webhook URL formats and generates appropriate message payloads. Custom headers supported via `webhooks.headers`.

### Internationalization & Accessibility (#106–#109)
- **i18n-aware bundling** (`crates/core/src/i18n.rs`): `i18n: { locales: [...], defaultLocale: 'en', messagePattern: './messages.${locale}.json' }` config enables locale-based bundle splitting. Transforms `${locale}` import patterns into runtime locale detection shims. Only the current locale's strings are loaded at runtime.
- **RTL CSS auto-generation** (`crates/core/src/rtl.rs`): `css: { rtl: 'auto' }` config auto-generates RTL CSS from LTR stylesheets using CSS logical properties. Converts `margin-left` → `margin-inline-start`, `padding-right` → `padding-inline-end`, `text-align: left` → `text-align: start`, and 20+ other physical-to-logical mappings. Generated as `[dir="rtl"]` scoped CSS files alongside LTR output.
- **Accessibility linting** (`crates/core/src/a11y.rs`): `a11y: { enabled: true, failOnError: true }` config checks HTML output for missing `alt` attributes on images, missing ARIA labels on interactive elements, insufficient color contrast, missing `<html lang>`, missing `<title>`, and form inputs without labels. Exits non-zero when `failOnError` is true and errors are found.
- **Build-time string encryption** (`crates/core/src/encrypt.rs`): `encrypt: { keys: ['API_KEY'], key: '<hex>' }` config encrypts sensitive string values at build time using XOR cipher with base64 encoding. Injects a runtime `__pledge_decrypt()` shim that decrypts values at runtime. Prevents plain-text secrets from appearing in bundle output.
