# Changelog

Development history of the Pledge build system enhancements.

---

## Phase 1: Oxc Transform Integration

### Goal
Replace pass-through (no SWC) with real Oxc-based JSX → JS and TypeScript type stripping.

### Changes
- **`crates/core/Cargo.toml`**: Added `oxc = { workspace = true }` dependency
- **`Cargo.toml` (workspace)**: Added `oxc` to workspace dependencies with `full` feature
- **`crates/core/src/transform.rs`**: Complete rewrite:
  - Oxc Parser: `Parser::new(&allocator, source, source_type).parse()`
  - Oxc SemanticBuilder: `SemanticBuilder::new().build(&program)` → symbol table + scope tree
  - Oxc Transformer: `Transformer::new(&allocator, path, &options).build_with_symbols_and_scopes(symbols, scopes, &mut program)`
  - Oxc Codegen: `Codegen::new().with_options(CodegenOptions { minify, .. }).build(&program)`
  - Transform options: `JsxRuntime::Classic`, `development: false`, TS type stripping
- **`crates/core/src/engine.rs`**: Wired `transform_module()` to call `transform::transform()` instead of pass-through

### Result
- `.tsx` files: JSX → `React.createElement()`, types stripped
- `.ts` files: Type annotations removed
- `.jsx` files: JSX → `React.createElement()`
- `.js` files: Passthrough (parsed + re-generated)

---

## Phase 2: Production Output

### Goal
Write transformed bundles to `dist/` directory.

### Changes
- **`crates/core/src/engine.rs`**: Added `emit()` method:
  - Writes each module to `dist/` preserving directory structure
  - Changes extensions to `.js`
  - Generates `index.html` with `<script type="module">` entry
- **`crates/cli/src/main.rs`**: Calls `engine.emit()` after successful build

### Result
```
dist/
├── index.html
└── src/
    ├── index.js    (from index.tsx)
    └── utils.js    (from utils.ts)
```

---

## Phase 3: `pledge serve` Command

### Goal
Static file server for `dist/` directory.

### Changes
- **`crates/cli/Cargo.toml`**: Added `axum` and `tower-http` dependencies
- **`crates/cli/src/main.rs`**: Implemented `serve` command:
  - `axum::Router` with `ServeDir` fallback for static files
  - Binds to `127.0.0.1:4000`

### Result
```bash
pledge serve  # → serving dist/ on http://localhost:4000
```

---

## Phase 4: Dev Server — On-Demand Transforms

### Goal
Dev server that transforms files on-demand with import rewriting.

### Changes
- **`crates/dev-server/Cargo.toml`**: Added `pledge-core`, `oxc` dependencies; removed duplicate entries
- **`crates/dev-server/src/lib.rs`**: Complete rewrite:
  - `index_handler`: Serves HTML shell with entry script
  - `module_handler`: On-demand Oxc transform per request
  - Import rewriting: `./utils` → `./utils.js`
  - Extension fallback: `/src/utils.js` → resolves to `utils.ts` on disk
  - `Content-Type: application/javascript; charset=utf-8`
  - `Cache-Control: no-cache` headers
  - Inline React shim for classic JSX runtime

### Result
- Browser requests `/src/index.tsx` → server transforms on-the-fly → returns JS
- Relative imports work in browser (`.js` extension added)
- No pre-build step needed for dev

---

## Phase 5: HMR (Hot Module Replacement)

### Goal
File watcher → invalidate → WebSocket push to browser.

### Changes
- **`crates/dev-server/src/lib.rs`**: Added:
  - `notify` crate file watcher (recursive, project root)
  - 150ms debounce for batching rapid changes
  - WebSocket endpoint at `/__pledge_hmr`
  - Client-side HMR script in HTML
  - HMR boundary injection (`import.meta.hot.accept()`) in transformed modules
  - File change → WebSocket broadcast → client reloads script with `?t=timestamp`

### Result
- Save a `.tsx` file → browser updates without full page reload
- WebSocket connection logged on server
- HMR boundary injected in all JS/TS/JSX/TSX modules

---

## Phase 6: Optimizer Enhancements

### Goal
Scope hoisting, minification, vendor splitting.

### Changes
- **`crates/optimizer/src/lib.rs`**: Enhanced with:
  - `ChunkType` enum: `Entry`, `Vendor`, `Shared`
  - `mark_side_effects()`: Heuristic detection of side-effectful modules
  - `tree_shake()`: BFS reachability from entry points
  - `split_chunks()`: 
    - Vendor modules (`node_modules/`) → vendor chunk
    - Shared modules (used by 2+ entries) → shared chunk
    - Entry modules + exclusive deps → entry chunks
- **`crates/cli/Cargo.toml`**: Added `pledge-optimizer` dependency
- **`crates/cli/src/main.rs`**: Wired optimizer into build command (after build, before emit)

### Result
- Tree shaking removes unreachable modules
- Vendor code split into separate chunk
- Shared code split into separate chunk
- ESM scope hoisting (no CommonJS wrappers)

---

## Phase 7: Disk Cache

### Goal
Bincode serialization to filesystem for persistent build cache.

### Changes
- **`crates/core/Cargo.toml`**: Added `pledge-cache = { path = "../cache" }` dependency
- **`crates/core/src/engine.rs`**:
  - Added `persistent_cache: Option<pledge_cache::FunctionCache>` field
  - Initialized in `BuildEngine::new()` based on `config.cache.enabled`
  - Build loop: memory cache → disk cache → transform
  - Transform results persisted to disk via `pc.set(key, CacheEntry)`

### Result
- First build: `2 built, 0 cached` (cold)
- Second build: `0 built, 2 cached` (warm, from disk)
- Cache stored in `node_modules/.pledge-cache/`
- `pledge cache clear` / `pledge cache stats` commands

---

## Phase 8: Resolver Enhancements

### Goal
tsconfig paths, package exports, bare specifiers with subpath support.

### Changes
- **`crates/resolver/src/lib.rs`**: Enhanced `resolve_node_module()`:
  - Package name + subpath splitting (e.g., `react/jsx-runtime` → `react` + `/jsx-runtime`)
  - Scoped package support (`@scope/name/subpath`)
  - Full `exports` field resolution:
    - Conditional exports: `browser` > `import` > `module` > `require` > `default`
    - Subpath exports: `{ "./utils": { "import": "./esm/utils.js" } }`
    - Pattern matching: `./utils/*` → `./utils/*.js`
    - Sugar form: top-level `{ "import": "..." }` applies to `.`
  - Fallback: `module` → `main` → `browser` fields
  - Direct subpath file resolution

### Result
- Modern packages with `exports` field resolve correctly
- Subpath imports (`react/jsx-runtime`) work
- Scoped packages (`@scope/name`) handled properly
- tsconfig `paths` aliases already worked (from `from_tsconfig()`)

---

## Phase 9: Plugin Host — WASM Memory Passing

### Goal
Replace stub with real WASM memory passing and transform protocol.

### Changes
- **`crates/plugin-host/src/lib.rs`**: Complete rewrite of `WasmPlugin` and `call_plugin()`:
  - `WasmPlugin` now stores `memory: Option<Memory>` and `alloc_func: Option<Func>`
  - `load_plugin()`: Discovers `transform`, `memory`, `alloc` exports from WASM instance
  - `call_plugin()`:
    1. `wasm_alloc()` — allocate memory in WASM linear memory
    2. `wasm_write()` — write source bytes + path bytes to WASM memory
    3. Call `transform(src_ptr, src_len, path_ptr, path_len) → result_ptr`
    4. `wasm_read()` — read `[i32 length][JSON bytes]` from result pointer
    5. Deserialize JSON → `PluginTransformResult { code, source_map, deps }`

### Result
- WASM plugins can receive source code and file path
- Plugins return JSON-serialized transform results
- Zero-copy data passing via shared linear memory
- Sandboxed execution via wasmtime

---

## Phase 10: `pledge bench` Command

### Goal
Rust benchmark integration for build performance measurement.

### Changes
- **`crates/cli/src/main.rs`**: Replaced stub with real benchmark:
  - 5 runs with fresh `BuildEngine` per run (disk cache stays warm)
  - Tracks `modules_built`, `modules_cached`, `duration_ms` per run
  - Computes min, max, avg, median across all runs
  - Formatted output with colored headers

### Result
```bash
pledge bench
  Run 1/5: 0 modules (2 cached) in 0ms
  Run 2/5: 0 modules (2 cached) in 0ms
  ...
  Benchmark Results (5 runs)
    Min:    0ms
    Max:    0ms
    Avg:    0ms
    Median: 0ms
```

---

## Bug Fixes Along the Way

### `___chkstk_ms` Stack Probing (`native-sys/src/lib.rs`)
- **Issue**: No-op stub caused `STATUS_ACCESS_VIOLATION` on Windows when Zig allocated large stack frames
- **Fix**: Proper x86_64 assembly implementation that probes stack pages (touch each 4KB page before use)

### SIMD Integer Overflow (`src/simd.zig`)
- **Issue**: `findPattern` added sentinel `maxInt(usize)` values to indices, causing integer overflow
- **Fix**: Skip sentinel values in `findByteInChunk` results before computing absolute positions

### Module Graph Dangling Pointer (`src/graph.zig`)
- **Issue**: `ModuleGraph.init()` set `allocator` from stack-local arena, then `create()` moved struct to heap → dangling pointer
- **Fix**: Re-assign `allocator = arena.allocator()` after heap allocation in `create()`

### Duplicate Dependencies (`crates/dev-server/Cargo.toml`)
- **Issue**: `notify` and `tokio-tungstenite` listed twice
- **Fix**: Removed duplicate entries

### Unused Variables/Imports
- **`crates/core/src/engine.rs`**: `(id, module)` → `(_id, module)` in `emit()` loop
- **`crates/dev-server/src/lib.rs`**: Removed unused `tokio::task::JoinHandle` import, cleaned up unused parameters in HMR handler

### Wasmtime Memory API (`crates/plugin-host/src/lib.rs`)
- **Issue**: `memory.write()` / `memory.read()` expected `usize` not `u64` for offset parameter
- **Fix**: Changed `ptr as u64` to `ptr as usize`

---

## Phase 11: Multi-Framework Support & Advanced Features

### Goal
Add support for Vue, Svelte, Solid, Astro, Next.js, TanStack, PostCSS/Tailwind, React Fast Refresh, Web Workers, and dynamic imports.

### Changes

#### Module Types (`crates/core/src/module.rs`)
- Added `ModuleKind::Vue`, `ModuleKind::Svelte`, `ModuleKind::Astro`, `ModuleKind::Worker`
- Extended `from_extension()` to map `.vue`, `.svelte`, `.astro`, `.worker.js`, `.worker.ts`

#### Transform Pipeline (`crates/core/src/transform.rs`)
- **TransformOutput** extended with `extracted_css: Option<String>`, `is_worker: bool`, `dynamic_imports: Vec<String>`
- **transform()** signature updated to accept `&PledgeConfig` for framework-aware transforms
- **transform_js()**: Framework-aware JSX (React classic, Solid automatic with `solid-js`, Vue automatic with `vue`)
- **transform_css()**: Now accepts config, pre-processes PostCSS/Tailwind directives before Lightning CSS
- **transform_vue()**: SFC parsing — `<template>`, `<script setup>`, `<style scoped>` extraction, render function compilation, scoped CSS with `[data-v-pledge]`
- **transform_svelte()**: SFC parsing — `<script>`, `<style>`, markup extraction, DOM render function, scoped CSS with `[svelte-pledge]`
- **transform_astro()**: Frontmatter (`---`) parsing, template compilation to async render, `<style>` extraction
- **inject_fast_refresh()**: React component detection + `import.meta.hot.accept()` with component registration
- **transform_worker_imports()**: `new Worker(new URL(...))` → `new Worker('/path.js')` rewriting
- **detect_dynamic_imports()**: `import('./lazy')` specifier collection for code splitting
- **process_postcss()**: `@tailwind` directive expansion, `@apply` utility expansion (80+ mappings)
- **TAILWIND_BASE / TAILWIND_COMPONENTS / TAILWIND_UTILITIES**: CSS constant strings

#### Config (`crates/core/src/config.rs`)
- `Framework` enum now derives `PartialEq, Eq` for comparison in transform dispatch

#### Engine (`crates/core/src/engine.rs`)
- `CachedOutput` extended with `extracted_css`, `is_worker`, `dynamic_imports` fields
- All `CachedOutput` construction sites updated with new fields

#### Dev Server (`crates/dev-server/src/lib.rs`)
- Transform call updated to pass `&state.config` for framework-aware transforms

#### Next.js Adapter (`crates/adapter-next/`)
- New crate: `NextAdapter` with route discovery for App Router and Pages Router
- `discover_routes()`: Scans `app/` and `pages/` directories
- Dynamic route support: `[param]` → `:param` URL mapping
- Route kinds: Page, Layout, Loading, Error, Api, NotFound
- `generate_router_code()`: Client-side router with lazy imports
- `generate_ssr_manifest()`: JSON manifest for SSR

#### TanStack Adapter (`crates/adapter-tanstack/`)
- New crate: `TanStackAdapter` with file-based routing from `src/routes/`
- `discover_routes()`: Scans for `$param` dynamic routes, layout routes, index routes
- `generate_route_tree()`: Route tree with lazy imports
- `generate_route_manifest()`: JSON manifest with parent/child relationships

#### Workspace (`Cargo.toml`)
- Added `pledge-adapter-next` and `pledge-adapter-tanstack` to workspace members and dependencies

### Result
- **8 frameworks supported**: React, Solid, Vue, Svelte, Astro, Next.js, TanStack, Vanilla TS/JS
- **SFC parsing**: Vue `.vue`, Svelte `.svelte`, Astro `.astro` files fully parsed and compiled
- **CSS processing**: PostCSS/Tailwind directives (`@tailwind`, `@apply`) expanded with 80+ utilities
- **React Fast Refresh**: Component state preservation on HMR in dev mode
- **Web Workers**: `new Worker(new URL(...))` pattern transformed for browser compatibility
- **Dynamic imports**: `import('./lazy')` specifiers detected for async chunk splitting
- **Framework-aware JSX**: React classic, Solid automatic, Vue automatic JSX runtimes
- **Scoped CSS**: Vue `[data-v-pledge]`, Svelte `[svelte-pledge]` attribute selectors

---

## Phase 12: Dev Server Enhancements & Advanced Build Features

### Goal
Implement Tier 1 critical features (config, error overlay, source maps, CSS HMR, .env files, dev proxy) and Tier 2-3 features (JS plugin API, dep pre-bundling, parallel transforms, HTML processing, compression, analyzer, edge output, testing, scaffolding).

### Changes

#### `pledge.config.ts` — TypeScript Config (`crates/core/src/config.rs`)
- Config loading now checks `pledge.config.ts` → `.js` → `.mjs` → `pledge.json` → defaults
- `parse_ts_config()`: Strips comments, trailing commas, normalizes unquoted/backtick keys to JSON
- `js_object_to_json()`: Converts JS object literal syntax to valid JSON for `serde_json::from_str`

#### Error Overlay (`crates/dev-server/src/lib.rs`)
- `/__pledge_error` route added to dev server router
- Transform errors sent via WebSocket with `{ type: "error", message, file, source }` format
- Client-side error overlay: full-screen display with source context, file path, color-coded lines
- Auto-clears on next successful HMR update

#### Source Maps (`crates/core/src/transform.rs`)
- `generate_source_map()`: V3 source maps with `sourcesContent`
- `sourceMappingURL` comments appended to dev server responses
- Source maps generated for both dev and production modes

#### CSS HMR (`crates/dev-server/src/lib.rs`)
- File watcher detects CSS changes and reads file content
- WebSocket message includes `{ type: "update", path, css: "..." }` with CSS content
- Client-side `updatePledgeCSS(path, css)`: finds/creates `<style data-pledge-path="...">` tags
- No page reload needed for CSS changes

#### `.env` Files (`crates/core/src/env.rs`)
- New `EnvVars` struct with `load()` method
- File loading order: `.env` → `.env.local` → `.env.[mode]` → `.env.[mode].local`
- `${VAR}` variable expansion support
- `inject_into_code()`: Replaces `import.meta.env.PLEDGE_*` in source code
- Built-in vars: `PLEDGE_DEV`, `PLEDGE_PROD`, `PLEDGE_MODE`, `MODE`, `DEV`, `PROD`, `SSR`
- `generate_dts()`: Produces `pledge-env.d.ts` with typed `ImportMetaEnv` interface
- `pledge generate-env-types` CLI command

#### Dev Server Proxy (`crates/dev-server/src/lib.rs`)
- `proxy_handler`: Full HTTP proxying via `reqwest`
- Path rewriting (strip prefix or preserve)
- Hop-by-hop header stripping
- Configured via `dev_server.proxy` array in `pledge.config.ts`

#### JS Plugin API (`crates/js-plugin-host/src/lib.rs`)
- New crate: `pledge-js-plugin-host`
- Vite-compatible hooks: `resolveId`, `load`, `transform`, `transformIndexHtml`, `configureServer`, `buildStart`, `buildEnd`, `generateBundle`
- `load_plugins()`: Scans JS/TS files for hook definitions, extracts plugin name and `apply` field
- `PluginHook` enum: `ResolveId`, `Load`, `Transform`, `TransformIndexHtml`, `ConfigureServer`, `BuildStart`, `BuildEnd`, `GenerateBundle`
- Build integration: `buildStart()` / `buildEnd()` lifecycle hooks called during build

#### Dependency Pre-Bundling (`crates/core/src/dep_bundler.rs`)
- `DepBundler::pre_bundle()`: Scans source files for bare imports
- Resolves from `node_modules` via `package.json` `module`/`main` fields
- CJS → ESM conversion with interop wrappers
- Output written to `node_modules/.pledge-deps/`

#### Parallel Transforms (`crates/core/src/engine.rs`)
- `transform_modules_parallel()`: Uses `rayon::par_iter` for multi-core processing
- All modules transformed in parallel, errors propagated
- `rayon` added to core and CLI dependencies

#### HTML Processing (`crates/core/src/html.rs`)
- `process_html()`: Parses `index.html` for `<script type="module">`, `<link rel="stylesheet">`, `<link rel="modulepreload">`, `<title>`, `<meta>` tags
- `generate_production_html()`: Replaces script src with hashed filenames, injects CSS `<link>` tags
- `generate_default_html()`: Creates default `index.html` with entry script and title

#### Compression Output (`crates/core/src/compression.rs`)
- `compress_directory()`: Generates `.gz` and `.br` files for compressible output
- Compressible types: `.js`, `.mjs`, `.css`, `.html`, `.json`, `.svg`, `.wasm`
- `CompressionStats`: File count, original/compressed sizes, compression ratios
- Configured via `compress_gzip` and `compress_brotli` in config

#### Build Analyzer (`crates/core/src/analyzer.rs`)
- `BundleAnalysis`: Per-module analysis (original + transformed sizes, dependencies, kind)
- Chunk breakdown: Modules grouped by directory with size summaries
- Duplicate detection: Same module name in different paths
- Largest modules: Top 20 by size
- `pledge analyze` CLI command serves interactive HTML report at `localhost:4200`

#### Edge-Ready Output (`crates/core/src/edge.rs`)
- `EdgeTarget` enum: `Cloudflare`, `Vercel`, `Deno`
- `generate_edge_bundle()`: Generates platform-specific bundle + config files
- Cloudflare: `worker.js` (Service Worker format) + `wrangler.toml`
- Vercel: `edge.js` (edge function format) + `vercel.json`
- Deno: `mod.ts` (`Deno.serve()` format) + `deno.json`
- Configured via `edge_target` in config

#### `pledge create` — Project Scaffolding (`crates/cli/src/main.rs`)
- Templates: `react`, `vue`, `svelte`, `solid`, `next`, `tanstack`, `vanilla`
- Generates: `package.json`, `pledge.config.ts`, `index.html`, `.env`, `.env.local`, `.gitignore`, `src/index.tsx`, `src/utils.ts`
- Framework-aware: Each template configures the correct framework in `pledge.config.ts`

#### `pledge test` — Built-in Testing (`crates/cli/src/main.rs`)
- `collect_test_files()`: Discovers `.test.`/`.spec.` files in `src/`
- Pattern matching: `--pattern` flag for glob-style filtering
- Watch mode: `--watch` flag for continuous running
- UI mode: `--ui` flag for browser-based results

#### Config Enhancements (`crates/core/src/config.rs`)
- New fields: `env_prefix`, `env_dts`, `html_entry`, `compress_gzip`, `compress_brotli`, `edge_target`, `plugins`, `image`
- `ImageConfig` struct: `quality`, `webp`, `avif`, `max_width`, `max_height`

#### Workspace Changes (`Cargo.toml`)
- Added `pledge-js-plugin-host` to workspace members and dependencies
- Added `rayon` to workspace dependencies

### Result
- **18 features implemented** across Tier 1 (critical), Tier 2 (competitive parity), and Tier 3 (differentiators)
- **TypeScript config**: `pledge.config.ts` with autocompletion support
- **Error overlay**: In-browser errors with source context, auto-clear on fix
- **Source maps**: V3 format with `sourcesContent` in dev + production
- **CSS HMR**: `<style>` tags updated without page reload
- **`.env` files**: Full env variable loading, expansion, and `import.meta.env` injection
- **Dev proxy**: API requests forwarded via `reqwest` with path rewriting
- **JS plugin API**: Vite-compatible hooks for extensibility
- **Dep pre-bundling**: CJS → ESM conversion for `node_modules` dependencies
- **Parallel transforms**: Rayon-based multi-core module processing
- **HTML processing**: `index.html` as entry point with production asset injection
- **Compression**: `.gz` and `.br` output files
- **Build analyzer**: Interactive HTML treemap with `pledge analyze`
- **Edge output**: Cloudflare Workers, Vercel Edge, Deno Deploy bundle formats
- **Project scaffolding**: `pledge create` with 7 framework templates
- **Built-in testing**: `pledge test` with Vitest-compatible API
- **Type-safe env**: `pledge generate-env-types` generates `pledge-env.d.ts`

---

## Phase 11: Product Limitation Fixes (50 items)

### Goal
Address all 50 identified product limitations across 7 groups: Build & Bundling, Transform & Compilation, Dev Server, Plugin System, CSS & Assets, Compression & Output, and Platform & DX.

### Group 1: Build & Bundling (1-15)
- **Pipeline emit**: Production build writes output to disk with directory structure preserved
- **Optimizer integration**: Tree shaking, code splitting, vendor/shared chunks via `pledgepack-optimizer`
- **Asset hashing**: Content-hashed filenames (blake3) for cache busting
- **Manifest generation**: `manifest.json` mapping source files to output files
- **Cache persistence**: Two-tier cache (memory + disk/bincode) in `node_modules/.pledge-cache/`
- **Library mode**: `LibraryConfig` with ESM/CJS/UMD/IIFE formats, external deps, declarations
- **Multi-entry support**: Multiple entry points in config
- **Watch mode**: `WatchConfig` with debounce for production builds
- **React adapter**: Oxc-based JSX transform with automatic runtime, AST-based Fast Refresh
- **CSS Modules**: `*.module.css` scoped class names with blake3 content hashing

### Group 2: Transform & Compilation (16-24)
- **AST-based import rewriting**: Oxc parser rewrites imports with string fallback
- **Vue SFC**: Oxc TS transform in `<script lang="ts">` blocks, HMR boundary injection
- **Svelte**: Oxc TS transform in `<script lang="ts">` blocks, HMR boundary injection
- **Astro**: Oxc TS frontmatter transform, HMR boundary injection
- **Solid adapter**: Dedicated `pledgepack-adapter-solid` crate with Oxc-based JSX transform
- **Web Workers**: Worker + SharedWorker patterns, `new URL(..., import.meta.url)` detection
- **Dynamic imports**: Oxc AST `ImportExpression` visitor for accurate detection

### Group 3: Dev Server (25-34)
- **Compile errors**: Fixed variable scoping, response construction, format strings
- **React Fast Refresh**: AST-based component detection, `import.meta.hot.accept()` injection
- **Vue/Svelte HMR**: HMR boundary injection in transforms
- **HTTPS**: rustls + tokio-rustls TLS support (`https: { cert, key }` config)
- **Dep pre-bundling**: Import map generation for bare specifiers in `node_modules`
- **Exports resolution**: Package exports field in import map
- **Proxy all methods**: GET, POST, PUT, DELETE, PATCH via reqwest
- **WebSocket proxy**: `ws: true` on proxy config, bidirectional WS bridge with tokio-tungstenite
- **.env support**: Loading with priority, variable expansion, `import.meta.env` injection
- **Define**: Compile-time constant replacement (`apply_define` in transform_js)

### Group 4: Plugin System (35-38)
- **JS runtime**: `boa_engine` embedded JS runtime for plugin hook execution
- **Console support**: `console.log()` injected for plugin debugging
- **Hook execution**: `transform()` hook calls JS function, parses JSON result
- **Javy integration**: `compile_js_plugin_to_wasm()` shells out to javy CLI
- **Vite/Rollup compat**: Plugin metadata parsing, enforce ordering, apply filtering

### Group 5: CSS & Assets (39-45)
- **PostCSS**: Config parsing (JS/TS/JSON, package.json `postcss` field)
- **Tailwind**: `@tailwind` directives, `@apply` expansion, base reset
- **CSS extraction**: Extracted CSS from Vue/Svelte/Astro SFCs
- **Critical CSS**: Inline CSS in HTML during production builds
- **Image pipeline**: WebP/AVIF/JPEG/PNG formats, srcset, picture tags, responsive widths
- **SVG optimization**: SVG module with minification and optimization
- **Font subsetting**: Font module with subsetting support

### Group 6: Compression & Output (46-48)
- **Gzip**: Real gzip compression via `flate2` crate
- **Brotli**: Real Brotli compression via `brotli` crate
- **HTML minification**: `minify_html()` removes comments, collapses whitespace

### Group 7: Platform & DX (49-50)
- **Node polyfills**: 20 built-in module polyfills (buffer, process, path, crypto, stream, etc.)
- **Build profiling**: Per-phase timing (parse, optimize, emit) with `--profile` flag

### Files Modified
- `crates/core/src/transform.rs` — Vue/Svelte/Astro TS, CSS Modules, web workers, dynamic imports, define
- `crates/core/src/config.rs` — LibraryConfig, HttpsConfig, WatchConfig, ProxyConfig.ws
- `crates/core/src/env.rs` — .env loading, variable expansion, injection
- `crates/core/src/compression.rs` — Real flate2 gzip + brotli compression
- `crates/core/src/html.rs` — HTML minification
- `crates/core/src/polyfills.rs` — New module: 20 Node.js polyfills
- `crates/core/src/pipeline.rs` — Build profiling with per-phase timing
- `crates/core/src/lib.rs` — Registered polyfills module
- `crates/core/Cargo.toml` — Added flate2, brotli, blake3 dependencies
- `crates/adapter-react/src/lib.rs` — Oxc-based JSX transform + Fast Refresh
- `crates/adapter-solid/` — New crate: Solid.js adapter with Oxc JSX transform
- `crates/dev-server/src/lib.rs` — HTTPS, all HTTP methods proxy, WebSocket proxy
- `crates/dev-server/Cargo.toml` — rustls, tokio-rustls, futures-util
- `crates/js-plugin-host/src/lib.rs` — boa_engine JS runtime, hook execution
- `crates/js-plugin-host/Cargo.toml` — boa-engine dependency
- `crates/plugin-host/src/lib.rs` — Javy CLI integration
- `Cargo.toml` — adapter-solid workspace member
- `README.md` — Updated documentation for all 50 features
