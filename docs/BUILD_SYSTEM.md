# Build System

## `pledge build` Pipeline

```
1. Load config (pledge.config.ts → pledge.config.js → pledge.config.mjs → pledge.json → defaults)
2. Create BuildEngine with config
3. BuildEngine::build()
   ├── BFS from entry point (src/index.tsx)
   ├── For each module:
   │   ├── Resolve specifier → file path (Resolver)
   │   ├── Read file content (Zig read_file via C ABI)
   │   ├── Compute content hash (blake3)
   │   ├── Check memory cache (HashMap<u64, CachedOutput>)
   │   ├── If miss → check disk cache (FunctionCache/bincode)
   │   ├── If miss → transform:
   │   │   ├── Oxc Parser → AST
   │   │   ├── Oxc SemanticBuilder → symbols + scopes
   │   │   ├── Oxc Transformer → strip types, JSX → React.createElement
   │   │   ├── Oxc Codegen → JavaScript string
   │   │   ├── Env injection (import.meta.env.PLEDGE_* replacement)
   │   │   └── Source map generation (V3 with sourcesContent)
   │   ├── Store result in memory cache + disk cache
   │   ├── Extract imports → resolve → add to graph → enqueue
   │   └── Track stats (built vs cached)
   └── Return BuildResult { modules_built, modules_cached, duration_ms }
4. Run Optimizer
   ├── Tree shake (remove unreachable modules)
   ├── Split chunks (entry / vendor / shared)
   └── Return Vec<Chunk>
5. Emit to dist/
   ├── Write each module as .js file
   ├── Generate index.html (with hashed asset references)
   ├── Generate manifest.json
   └── Generate source maps
6. Post-build steps:
   ├── Generate pledge-env.d.ts (if env_dts enabled)
   ├── Process HTML entry point
   ├── Pre-bundle dependencies (scan node_modules, CJS→ESM)
   ├── Load JS plugins (buildStart hooks)
   ├── Generate edge bundle (if edge_target configured)
   ├── Generate service worker (if configured)
   ├── Generate Web App Manifest (if configured)
   ├── Check performance budgets (if configured)
   ├── Generate bundle size diff (if previous snapshot exists)
   ├── Multi-format output (if library mode configured — ESM/CJS/IIFE/UMD)
   ├── Record build telemetry (#101) — save to .pledge/history.json
   ├── Check bundle size budgets (#102) — if --check-budgets flag or budgets.enabled
   ├── Lint HTML for accessibility (#108) — if a11y.enabled
   ├── Send build event webhooks (#105) — if webhooks.onBuild/onError configured
   ├── Inject SRI hashes (#81) — if security.sri enabled
   ├── Generate CSP _headers file (#82) — if security.csp == "auto"
   └── Compress output (gzip .gz + brotli .br files)
```

## Transform Optimizations Pipeline

During step 3 (transform), the following optimizations are applied in order:

```
1. CSS tree shaking — extract_used_class_names() from JS/JSX/TSX, shake_css() removes unused selectors
2. Dead code elimination — eliminate_dead_code() removes unreachable branches (if false, if true)
3. Constant folding — fold_constants() evaluates compile-time expressions (1 + 2 → 3)
4. Optional chaining optimization — optimize_optional_chaining() simplifies redundant null checks
5. Cross-chunk variable hoisting — analyze_cross_chunk_hoisting() prepares shared variable declarations
6. Module-level memoization — ModuleTransformCache checks content + config hash before re-transforming
7. WASM target compilation — ?wasm import suffix generates JS glue code for WASM modules
8. i18n import transform (#106) — ${locale} patterns replaced with runtime locale detection shims
9. String encryption (#109) — sensitive string values encrypted with XOR + base64, runtime decrypt shim injected
```

## CSS Processing Pipeline

```
1. Lightning CSS — minification, nesting, autoprefixing
2. CSS Modules — scoped class names with blake3 content hashing
3. PostCSS / Tailwind — @tailwind directives, @apply expansion
4. Tailwind v4 — @theme, @utility, @variant directive processing
5. CSS-in-JS extraction — styled-components, emotion, vanilla-extract compile-time extraction
6. CSS @layer — cascade layer management and ordering
7. Container queries — polyfill for older browsers
8. Critical CSS — extract_critical_css() + inline_critical_css() for faster FCP
9. CSS source maps — generate_css_source_map() maps to original source
10. PostCSS plugin caching — blake3 content hash for incremental processing
11. RTL CSS auto-generation (#107) — if css.rtl is 'auto' or 'manual', generates [dir="rtl"] scoped CSS from LTR output using logical property mappings
12. Dark mode CSS (#67) — generate_dark_mode_css() auto-generates dark variants from prefers-color-scheme or custom property inversion
13. Custom property optimization (#68) — optimize_custom_properties() inlines static vars, removes unused, minifies names (production)
14. Scoped CSS for React (#69) — scope_css_with_attribute() adds data-v-xxxxx attribute selectors
15. CSS Modules composes (#66) — parse_composes() + strip_composes() for cross-file composition resolution
16. CSS-in-JS runtime tree shaking (#73) — strip_css_in_js_runtime() removes runtime imports after static extraction
```

## Asset Pipeline

```
MDX files        → compile_mdx() — Markdown + JSX with frontmatter
GraphQL files    → parse_graphql() + graphql_to_module() with TypeScript types
YAML/CSV/TSV     → transform_yaml() / transform_csv() / transform_tsv() with typed exports
Images           → select_image_format() — WebP/AVIF auto-selection, generate_picture_element()
                   Responsive srcset (#79) — config.image.responsive_widths for custom breakpoints
                   Asset inlining (#80) — assets under build.assets_inline_limit auto-inlined as base64
Audio/Video      → transform_audio_asset() / transform_video_asset() with URL exports
                   Video poster (#78) — transform_video_asset() exports poster URL alongside src
PDF              → transform_pdf_asset() with inline base64 support
SVG              → optimize_svg() + ?sprite suffix (#77) generates <symbol> sprite sheet
Fonts            → optimize_fonts() (#76) with FontSubsetConfig, wired via build.font_subsetting
WASM             → transform_wasm() (#74) with WebAssembly.instantiateStreaming() + SIMD auto-detection
All assets       → AssetManifest with content-hashed output paths
```

## Plugin System

```
1. Plugin discovery — scan configured plugin paths
2. Plugin loading — WASM plugins via wasmtime, JS plugins via boa_engine
3. Hot reload — PluginHotReloader watches for file changes, reloads without restart
4. Sandboxing — SandboxLimits (memory, CPU time) + SandboxedFs (filesystem access)
5. Dependency resolution — PluginDependencyResolver with import maps for npm packages
6. Lifecycle hooks — LifecycleHookRegistry:
   ├── watchStart / watchChange / watchEnd (dev mode)
   ├── beforeTransform / afterTransform
   └── beforeBuild / afterBuild
7. Parallel execution — execute_parallel_transforms() via rayon thread pool
```

## Oxc Transform Details

### Source Type Detection
```rust
SourceType::from_path(path) →
  .tsx → SourceType::tsx()
  .ts  → SourceType::ts()
  .jsx → SourceType::jsx()
  .js  → SourceType::mjs()
```

### Transform Options
```rust
TransformOptions {
  jsx: {
    // Framework-aware:
    //   React → JsxRuntime::Classic (React.createElement, no react/jsx-runtime import)
    //   Solid → JsxRuntime::Automatic, import_source = "solid-js"
    //   Vue   → JsxRuntime::Automatic, import_source = "vue"
    runtime: JsxRuntime::Classic,
    development: false,
  },
  typescript: {
    only_remove_type_imports: false,
  },
}
```

### Classic vs Automatic JSX Runtime
- **Classic**: `React.createElement("div", null, "hello")` — requires `React` global
- **Automatic**: `import { jsx } from "react/jsx-runtime"` — requires React installed
- **Solid**: `import { createComponent } from "solid-js"` — automatic runtime with solid-js
- **Vue**: `import { jsx } from "vue"` — automatic runtime with vue
- Pledge defaults to **Classic** for React to avoid requiring React installation for simple projects

### Framework-Specific Transforms

#### Vue SFC (`.vue`)
- Extracts `<template>`, `<script setup>`, `<style scoped>` blocks via `extract_sfc_block()`
- Template compiled to render function
- Scoped CSS: `[data-v-pledge]` attribute selectors injected
- Output: JS module with render function + extracted CSS

#### Svelte (`.svelte`)
- Extracts `<script>`, `<style>`, and markup blocks
- Markup compiled to DOM render function with mount/unmount
- Scoped CSS: `[svelte-pledge]` attribute selectors
- Output: JS module with render function + extracted CSS

#### Astro (`.astro`)
- Parses `---` frontmatter delimiters
- Template compiled to async render function
- `<style>` blocks extracted as CSS
- Output: JS module with async render function + extracted CSS

### PostCSS / Tailwind Processing
- `@tailwind base` → Tailwind CSS reset (box-sizing, margins, borders)
- `@tailwind components` → `.container` responsive class
- `@tailwind utilities` → Full utility CSS subset (display, flex, spacing, typography, etc.)
- `@apply` expansion → 80+ utility class mappings
- Processed before Lightning CSS parsing

### Web Workers (#111, #112)
- `new Worker(new URL('./worker.ts', import.meta.url))` → `new Worker('/src/worker.js')`
- `new SharedWorker(new URL(...))` patterns also supported
- `.worker.js` / `.worker.ts` extensions detected as `ModuleKind::Worker`
- `?worker` import suffix: `import MyWorker from './worker.ts?worker'` → `const MyWorker = function() { return new Worker('/src/worker.js'); }`
- `?sharedworker` import suffix: `import MyWorker from './worker.ts?sharedworker'` → `const MyWorker = function() { return new SharedWorker('/src/worker.js'); }`
- Resolver strips `?worker`/`?sharedworker` suffixes during module resolution
- `ModuleKind::SharedWorker` for shared worker modules

### Web Components (#110)
- `.wc.tsx` / `.wc.jsx` files compiled to Custom Elements via `compile_web_component()`
- Automatic `customElements.define('tag-name', ClassName)` registration
- Shadow DOM with `mode: 'open'` — CSS scoped inside shadow root
- Component name extracted from `export function` / `export const` declarations
- Tag name auto-generated as kebab-case from component name
- `ModuleKind::WebComponent` for web component modules

### Service Worker Caching (#113)
- Per-route caching strategy configuration via `sw: { caching: [...] }` config
- Strategies: `cache-first`, `network-first`, `stale-while-revalidate`, `network-only`, `cache-only`
- `sw: { cache_name: 'pledge-sw', offline_fallback: '/offline.html' }` config
- Generates `sw.js` in output directory during build

### Module Federation (#115)
- `federation: { name: 'host', remotes: { app1: 'http://cdn/app1.js' }, shared: ['react'] }` config
- Host bootstrap: `__pledge_federation__` global with remotes/shared/init/loadRemote
- Remote entry: `__pledge_remote__` with exposes/shared/get/init
- Shared modules support `requiredVersion`, `singleton`, `eager` options
- `parse_federation_config()` parses JSON config into `FederationConfig` struct

### GraphQL Code Generation (#116)
- `pledge build --codegen` generates TypeScript types from `.graphql` schema files
- `graphql: { schema: 'schema.graphql', output: 'src/generated', react_hooks: true }` config
- Generates TypeScript interfaces for all GraphQL types
- Nullable fields typed as `T | null`, lists as `T[]`
- React hooks generated for Query operations (`useXxx` pattern)
- Output: `src/generated/graphql-types.ts`

### Environment-Specific Builds (#117)
- `pledge build --env staging` loads `.env.staging` file
- Env vars injected as `process.env.*` defines at build time
- `NODE_ENV` resolved from env file or production flag
- Multiple environments without code changes: `.env.development`, `.env.staging`, `.env.production`

### Post-Build Optimization Hooks (#118)
- `run_post_build_hooks()` executes after build emit
- Generates `sitemap.xml` from HTML chunks
- Injects missing HTML meta tags: viewport, description, charset
- `PostBuildContext` provides output dir, HTML path, chunks, assets
- `PostBuildResult` reports sitemap generation, HTML modification, warnings

### Conditional Exports Resolution (#119)
- `exports: { conditions: ['production', 'browser'] }` config
- Custom conditions checked first, then defaults: browser > import > module > require > default
- Supports sugar form (`{ "import": "...", "require": "..." }`) and subpath patterns (`"./components/*"`)
- Wildcard `*` pattern matching in export keys
- `Resolver::with_conditions()` constructor for custom conditions

### Build Concurrency Control (#120)
- `build: { parallel: 4 }` config limits concurrent module transforms
- Auto-detects CPU cores via `std::thread::available_parallelism()` when not configured
- Capped at 16 threads maximum
- Uses dedicated rayon thread pool via `ThreadPoolBuilder::install()`
- Prevents OOM on large projects with many modules

### Dynamic Import Detection
- Oxc AST `ImportExpression` visitor for accurate detection
- String-based fallback if parsing fails
- Only relative specifiers (`./`, `../`) tracked for chunk splitting
- Stored in `TransformOutput.dynamic_imports` for optimizer use

### React Fast Refresh (Dev Mode)
- AST-based component detection using Oxc (function declarations, arrow functions with capitalized names)
- Injects `import.meta.hot.accept()` with component registration
- Component state preserved via `window.__pledge_fast_refresh` registry
- Only injected in development mode for React framework

### Define / Compile-Time Constants
- Replace identifiers with literal values at build time via `apply_define()`
- Config: `define: { 'process.env.NODE_ID': '"production"', '__VERSION__': '"1.0.0"' }`
- Type inference: strings wrapped in quotes, numbers/booleans preserved

### import.meta.glob
- Glob-based file imports for dynamic route/component discovery
- `import.meta.glob('./pages/*.tsx')` expanded at transform time via `expand_import_meta_glob()`
- **Lazy mode** (default): Returns `{ './pages/Home.tsx': () => import('./pages/Home.tsx'), ... }`
- **Eager mode**: `{ eager: true }` returns `{ './pages/Home.tsx': moduleObject, ... }`
- **Query support**: `?raw` returns file content as string, `import` filter for import-only
- **Recursive**: `**` wildcard for recursive directory matching (e.g., `./components/**/*.tsx`)
- Replaced at transform time in the JS transform pipeline after env variable replacement

### Node.js Polyfills
- 20 built-in module polyfills available when `node_polyfills: true` in config
- Supports both `import 'path'` and `import 'node:path'` specifiers
- Browser-safe ESM polyfills using Web APIs (Web Crypto, TextEncoder, fetch, etc.)
- Modules: buffer, process, path, crypto, stream, util, events, url, os, fs, http, https, net, tls, zlib, querystring, string_decoder, timers, assert, child_process

## Caching

### Two-Tier Architecture
```
Request → Memory Cache (HashMap)
              Hit? → return cached output
              Miss? → Disk Cache (bincode)
                        Hit? → load into memory, return
                        Miss? → Transform
                                  → Store in memory + disk
```

### Cache Key
```rust
CacheKey = blake3(content_hash || function_id || params)
```
- `content_hash`: u64 hash of file source content
- `function_id`: "transform" (currently single function)
- `params`: file path string

### Cache Location
- Default: `node_modules/.pledge-cache/`
- Configurable via `pledge.config.ts`: `{ cache: { dir: '.pledge-cache', enabled: true } }`

### Cache Invalidation
- Content-based: File change → new content hash → cache miss → retransform
- Manual: `pledge cache clear` removes all disk cache files
- Automatic: Old entries are not garbage collected (future: TTL-based eviction)

## Production Output (`dist/`)

### File Structure
```
dist/
├── index.html          # Generated HTML shell (with hashed asset references)
├── manifest.json       # Source → output file mapping
└── src/
    ├── index.js        # Transformed from index.tsx
    ├── index.js.map    # Source map (V3 with sourcesContent)
    └── utils.js        # Transformed from utils.ts
```

### Compression Output
When `compress_gzip` and/or `compress_brotli` are enabled in config:
```
dist/
├── index.html.gz       # Gzip compressed (flate2)
├── index.html.br       # Brotli compressed (brotli crate)
├── src/
│   ├── index.js.gz     # Gzip compressed
│   ├── index.js.br     # Brotli compressed
│   └── ...
```
Compressible file types: `.js`, `.mjs`, `.css`, `.html`, `.json`, `.svg`, `.wasm`

### HTML Minification
- `minify_html()` removes HTML comments, collapses whitespace, strips redundant spaces between tags
- Applied during production builds for smaller HTML output

### Build Profiling
- Per-phase timing: Parse + Transform, Optimize, Emit phases timed individually
- Enable with `pledge build --profile` or `profile: true` in config
- Reports timing for each phase and total build duration

### Edge-Ready Output
When `edge_target` is configured, generates edge-function-compatible bundles:

| Target | Output File | Format |
|--------|-------------|--------|
| `cloudflare` | `worker.js` + `wrangler.toml` | Service Worker with `fetch` handler |
| `vercel` | `edge.js` + `vercel.json` | Edge function with `config.runtime = 'edge'` |
| `deno` | `mod.ts` + `deno.json` | `Deno.serve()` format |

### HTML Generation
The HTML processor (`crates/core/src/html.rs`) parses `index.html` as an entry point:
- Extracts `<script type="module">` src paths as entry points
- Extracts `<link rel="stylesheet">` and `<link rel="modulepreload">` hrefs
- Extracts `<title>` and `<meta>` tags
- In production: replaces script src with hashed filenames, injects CSS `<link>` tags
- HTML minification: `minify_html()` removes comments, collapses whitespace
- If no `index.html` exists, generates a default one with `generate_default_html()`

### Extension Mapping
- `.tsx` → `.js`
- `.ts` → `.js`
- `.jsx` → `.js`
- `.js` → `.js` (passthrough after transform)
- `.wc.tsx` / `.wc.jsx` → `.js` (Web Component compiled, Custom Element registered)
- `.vue` → `.js` (SFC compiled, CSS extracted)
- `.svelte` → `.js` (SFC compiled, CSS extracted)
- `.astro` → `.js` (compiled, CSS extracted)
- `.css` → `.css` (Lightning CSS processed)
- `.json` → `.js` (named + default exports)
- `.wasm` → `.js` (async instantiation wrapper)
- `.graphql` / `.gql` → `.js` (with `--codegen`: TypeScript types generated)
- `.png`/`.jpg`/`.svg`/etc. → URL string export (or base64 if `?inline`)

### Asset Hashing
- Content hash (blake3) appended to filenames: `logo-a1b2c3d4.png`
- `manifest.json` generated mapping source paths to hashed output paths
- Enables long-term browser caching with cache busting

### Library Mode
- `LibraryConfig` with ESM, CJS, UMD, IIFE output formats
- External dependencies: mark packages as external (not bundled)
- Optional `.d.ts` type declarations generation
- Config: `library: { entry, formats, name, external, declarations }`

### Single-File Bundle
- `emit_single_file()` concatenates all modules into one ESM file
- Topological sort ensures dependency order
- All imports inlined (no external chunk files)

## Optimizer

### Tree Shaking
1. Start from entry module IDs
2. BFS through dependency graph
3. Mark all reachable modules
4. Unreachable modules are excluded from chunks

### Chunk Splitting
```
Entry chunks:  Entry module + exclusive dependencies
Vendor chunk:  All modules in node_modules/
Shared chunk:  Modules used by 2+ entry points
Route chunks:  Per-route modules (#71) — split_by_routes() extracts shared route modules
```
- **Route-based splitting (#71)**: `detect_routes()` scans app/pages directories, `split_by_routes()` creates per-route chunks with a shared chunk for modules used across routes
- **Module prefetch (#72)**: `generate_prefetch_tags()` creates `<link rel="modulepreload">` and `<link rel="prefetch">` based on route chunks and prefetch strategy

### Scope Hoisting
- ESM `import`/`export` preserved (no CommonJS wrappers)
- Modules in the same chunk share scope
- No per-module function wrappers (unlike webpack's default)

## Parallel Transforms

The engine supports parallel module transforms using rayon:
```rust
engine.transform_modules_parallel(modules: Vec<(ModuleId, ResolvedModule)>)
```
- Uses `rayon::par_iter` for multi-core processing
- All modules transformed in parallel
- Errors propagated (first error stops collection)
- Falls back to sequential if single module

## Dependency Pre-Bundling

The dep bundler (`crates/core/src/dep_bundler.rs`) pre-bundles bare imports:
1. Scans source files for bare (non-relative) import specifiers
2. Resolves each from `node_modules` via `package.json` `module`/`main` fields
3. Converts CJS modules to ESM with interop wrappers
4. Writes pre-bundled output to `node_modules/.pledge-deps/`

CJS → ESM interop wrapper:
```javascript
const __pledge_cjs_module = {};
const module = { exports: __pledge_cjs_module };
// ... original CJS code ...
export default module.exports;
```

## Environment Variable Injection

The env module (`crates/core/src/env.rs`) loads `.env` files and injects variables:

### File Loading Order (highest precedence last)
1. `.env`
2. `.env.local`
3. `.env.[mode]` (e.g., `.env.production`)
4. `.env.[mode].local` (e.g., `.env.production.local`)

### Variable Expansion
```bash
PLEDGE_API_URL=http://localhost:8080
PLEDGE_FULL_URL=${PLEDGE_API_URL}/api/v1
```

### Code Injection
`import.meta.env.PLEDGE_*` references in source code are replaced with actual values during transform.

### Type Generation
`pledge generate-env-types` generates `pledge-env.d.ts`:
```typescript
interface ImportMetaEnv {
  readonly PLEDGE_API_URL: string;
  readonly PLEDGE_DEV: boolean;
  // ...
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}
```

## Test Runner (`crates/js-plugin-host/src/test_runner.rs`)

### Overview
The built-in test runner provides a Vitest-compatible testing experience using the `boa_engine` embedded JS runtime. Tests are run without external dependencies (no Node.js, Jest, or Vitest required).

### Configuration
In `pledge.config.ts`:
```typescript
export default defineConfig({
  test: {
    include: ['**/*.{test,spec}.{ts,tsx,js,jsx}'],
    exclude: ['node_modules', '.pledge', 'dist'],
    environment: 'node', // 'node' | 'jsdom' | 'happy-dom'
    globals: false, // Global describe/it/expect without imports
    setup_files: [], // e.g. ['./test/setup.ts']
    isolation: 'file', // 'file' | 'pool' | 'none'
    coverage: false, // Enable coverage collection
    coverage_reporter: 'text', // 'text' | 'json' | 'html' | 'lcov'
    snapshot: true, // Enable snapshot testing
    snapshot_dir: '__snapshots__',
    update_snapshots: false, // Auto-update snapshots
  },
});
```

### API Support
- **Test functions**: `describe`, `it`, `test`, `it.skip`, `test.skip`, `it.only`, `test.only`
- **Assertions**: `expect()` with `toBe`, `toEqual`, `toBeTruthy`, `toBeFalsy`, `toBeNull`, `toBeUndefined`, `toBeDefined`, `toContain`, `toHaveLength`, `toThrow`, and `not` inverse matchers
- **Lifecycle hooks**: `beforeAll`, `beforeEach`, `afterEach`, `afterAll`
- **Mocking**: `vi.fn()`, `vi.mock()`, `vi.spyOn()`, `vi.stubGlobal()`
- **Snapshot testing**: `toMatchSnapshot()`, `toMatchInlineSnapshot()` with `.snap` file persistence

### Test Environments
| Environment | Description |
|-------------|-------------|
| `node` (default) | No DOM shims, minimal `process` and `Buffer` stubs |
| `jsdom` | Full DOM shim: `document`, `window`, `navigator`, `location`, `customElements`, `MutationObserver`, `getComputedStyle`, `HTMLElement` |
| `happy-dom` | Lighter DOM shim: `document`, `window`, `navigator`, `location`, `customElements`, `MutationObserver` |

### Test Isolation
| Mode | Description |
|------|-------------|
| `file` (default) | Each test file runs in its own Boa JS context |
| `pool` | Shared pool of contexts for batch execution |
| `none` | No isolation — all tests share a single context |

### Coverage Reporting
- **Coverage tracking**: Line, function, and branch coverage via `__pledge_coverage` global
- **Report formats**: `text` (console output), `json` (machine-readable), `html` (styled report), `lcov` (for CI integration)
- **Config**: `test.coverage: true` to enable, `test.coverage_reporter` to select format

### UI Mode
- `pledge test --ui` generates an HTML report with:
  - Pass/fail/skip summary with colored indicators
  - Per-test file breakdown with suite and test names
  - Error messages and stack traces for failed tests
  - Execution duration per test
- Report served at `localhost:5174` with auto-browser-open
- Report also written to `.pledge/test-report.html`

### Snapshot Testing
- **`toMatchSnapshot()`**: Serializes value to JSON, compares against stored `.snap` file
- **`toMatchInlineSnapshot()`**: Compares against inline snapshot string
- **Auto-update**: `test.update_snapshots: true` or `-u` flag updates stale snapshots
- **Storage**: `.snap` files stored in `test.snapshot_dir` (default: `__snapshots__`)
- **Mismatch reporting**: Detailed diff shown on snapshot mismatch

## Observability & Monitoring (#101–#105)

### Build Telemetry Dashboard (#101)

`pledge dashboard` serves an interactive web UI at `localhost:4300` showing build history:

```
.pledge/history.json — persistent build records (max 100 entries)
```

Each build record includes:
- Timestamp, duration (ms), success/failure status
- Modules built vs cached, cache hit rate
- Bundle size (bytes)
- Error message (if failed)

The dashboard renders an SVG chart with build duration trend, cache hit rate, and a summary table of recent builds.

### Bundle Size Budget CI (#102)

`pledge build --check-budgets` or `budgets: { enabled: true }` in config:

```typescript
export default defineConfig({
  budgets: {
    enabled: true,
    maxBundleSize: 500_000,   // 500KB total
    maxChunkSize: 250_000,    // 250KB per chunk
    maxChunkCount: 10,        // max 10 chunks
    entryBudgets: {           // per-entry overrides
      'src/index.tsx': 200_000,
    },
  },
});
```

**CI integration**: When `GITHUB_ACTIONS` env is set, violations are emitted as `::error` annotations:
```
::error file=dist/src/index.js::Bundle size budget exceeded: 320KB > 250KB
```

### Performance Regression Detection (#103)

`pledge bench --baseline <ref> --threshold <pct>`:

```
pledge bench --baseline main --threshold 10
```

- Runs 5 build iterations, takes median duration
- Compares against stored baseline in `.pledge/bench.json`
- Exits non-zero if regression exceeds threshold (default: 10%)
- Use `pledge bench --save-baseline <ref>` to store a new baseline

### Module Dependency Graph (#104)

`pledge analyze --graph` generates an interactive force-directed dependency graph:

- Canvas-based physics simulation (Verlet integration)
- Nodes color-coded by type: entry (green), CSS (blue), module (gray), circular (red)
- Edges represent import relationships
- Circular dependencies detected via DFS and highlighted
- Served at `localhost:4200`

### Build Event Webhooks (#105)

```typescript
export default defineConfig({
  webhooks: {
    enabled: true,
    onBuild: 'https://hooks.slack.com/services/...',
    onError: 'https://discord.com/api/webhooks/...',
    headers: { 'Authorization': 'Bearer token' },
  },
});
```

- Auto-detects Slack vs Discord from URL format
- Slack: formatted as attachment with color-coded status, fields for duration/modules/bundle size
- Discord: formatted as embed with color, title, description, and fields
- Sent asynchronously after build completion

## Internationalization & Accessibility (#106–#109)

### i18n-Aware Bundling (#106)

```typescript
export default defineConfig({
  i18n: {
    enabled: true,
    locales: ['en', 'fr', 'ar'],
    defaultLocale: 'en',
    messagePattern: './locales/${locale}.json',
  },
});
```

- Import patterns containing `${locale}` are transformed at build time
- Only the default locale's messages are bundled; other locales loaded via dynamic import
- Runtime shim detects `document.documentElement.lang` or `navigator.language`

### RTL CSS Auto-Generation (#107)

```typescript
export default defineConfig({
  css: {
    rtl: 'auto',  // 'auto' | 'manual' | 'off'
  },
});
```

When enabled, for each CSS file emitted, a corresponding `.rtl.css` file is generated:

| LTR Property | RTL Property |
|---|---|
| `margin-left` | `margin-inline-start` |
| `margin-right` | `margin-inline-end` |
| `padding-left` | `padding-inline-start` |
| `padding-right` | `padding-inline-end` |
| `text-align: left` | `text-align: start` |
| `text-align: right` | `text-align: end` |
| `left: 10px` | `inset-inline-start: 10px` |
| `right: 10px` | `inset-inline-end: 10px` |
| `border-left` | `border-inline-start` |
| ... 20+ mappings | |

RTL output is scoped with `[dir="rtl"]` selector.

### Accessibility Linting (#108)

```typescript
export default defineConfig({
  a11y: {
    enabled: true,
    failOnError: true,
    checkAlt: true,
    checkAria: true,
    checkContrast: false,
  },
});
```

Checks performed on HTML output:
- **img-alt**: `<img>` tags missing `alt` attribute
- **button-aria-label**: Interactive `<button>` without text content or `aria-label`
- **input-label**: `<input>` without associated `<label>` or `aria-label`
- **html-lang**: `<html>` missing `lang` attribute
- **html-title**: Document missing `<title>` element
- **color-contrast**: Insufficient contrast ratios (optional)

### Build-Time String Encryption (#109)

```typescript
export default defineConfig({
  encrypt: {
    enabled: true,
    key: 'a1b2c3d4e5f6...',  // hex-encoded XOR key
    keys: ['API_KEY', 'SECRET_TOKEN'],  // variable names to encrypt
  },
});
```

- Scans code for string literals assigned to configured variable names
- Encrypts values using XOR cipher with base64 encoding
- Injects `__pledge_decrypt()` runtime shim in bundle output
- Encrypted values appear as `__pledge_decrypt("base64string")` in output
- Prevents plain-text secrets from appearing in bundle source

## JSON Schema Generation (`pledge schema`)

Generates a JSON Schema for the `pledge.config.ts` configuration, enabling IDE autocompletion and validation:

```bash
pledge schema              # Output to stdout
pledge schema --output schema.json  # Write to file
```

- Uses `schemars` crate to derive `JsonSchema` from `PledgeConfig` and all sub-structs/enums
- Schema covers all config fields: build, dev_server, cache, plugins, image, i18n, a11y, encrypt, etc.
- Can be used with VS Code JSON validation, JetBrains schema support, or any JSON Schema consumer

## PledgeStack Framework Adapter

PledgeStack is a Next.js-like full-stack framework with React frontend and Rust backend:

```
my-app/
├── app/                        # Frontend routes (React .tsx)
│   ├── page.tsx                # → GET /
│   ├── about/page.tsx          # → GET /about
│   └── blog/[slug]/page.tsx    # → GET /blog/:slug
├── server/                     # Backend (Rust .rs or .psx)
│   ├── api/
│   │   ├── users.rs            # → /api/users
│   │   └── auth.psx            # → /api/auth
│   ├── middleware/
│   │   └── auth.rs
│   └── lib.rs                  # Server entry point
├── components/                 # Co-located React components
├── lib/                        # Shared utilities
├── types/                      # TypeScript types
├── pledge.config.ts
├── Cargo.toml                  # Rust deps for server/
└── package.json                # JS deps for frontend
```

### Route Discovery
- `PledgeStackAdapter::discover()` scans `app/` and `server/` directories
- Frontend routes: `page.tsx` files in `app/` directory
- Backend routes: `#[route(GET, "/api/users")]` macros in `.rs` or `.psx` files
- Middleware: files in `server/middleware/`
- Server entry: `server/lib.rs`, `server/lib.psx`, `server/main.rs`, or `server/main.psx`

### `.psx` Extension
- PledgeStack eXtension — brands backend files, parallel to `.tsx` for frontend
- Treated as Rust source code (same syntax, same tooling with VS Code file association)
- Copied to `.rs` during build for `cargo build` compatibility

### Route Macro Formats
```rust
// Simple
#[route(GET, "/api/users")]
pub async fn list_users() { }

// Qualified
#[pledge::route(POST, "/api/auth")]
pub async fn login() { }

// Key-value
#[route(method = "DELETE", path = "/api/users/:id")]
pub async fn delete_user() { }
```

## Plugin Presets (#94)

Plugin presets bundle plugins and config defaults for specific ecosystems.

```ts
// pledge.config.ts
export default defineConfig({
  presets: ['react', 'tailwind'],  // applies both presets
})
```

**Built-in presets**: `react`, `tailwind`, `solid`, `vue`, `svelte`, `astro`

**Community presets**: Install `pledgepack-preset-{name}` from npm. The preset's `preset.json` is loaded automatically.

```json
// node_modules/pledgepack-preset-remix/preset.json
{
  "name": "remix",
  "plugins": ["./plugins/remix-router.js"],
  "config_defaults": { "framework": "react" },
  "description": "Remix preset with SSR routing"
}
```

## Custom Transformer Pipeline (#97)

Insert custom transform steps at any point in the pipeline.

```ts
export default defineConfig({
  transform_pipeline: {
    pipeline: ['my-wasm-transform', 'minify'],
    replace_default: false,  // insert into default pipeline
  }
})
```

When `replace_default: false` (default), custom steps are inserted after `oxc` and before `minify`:
```
oxc → [custom steps] → minify → tree-shake
```

When `replace_default: true`, only the configured steps run:
```
oxc → my-wasm-transform → minify
```

## Monorepo & Workspaces (#98–#100)

### Workspace-Aware Resolution (#98)

Auto-detects npm/pnpm/yarn workspaces and resolves local packages instead of hitting node_modules.

```ts
export default defineConfig({
  workspaces: {
    enabled: true,           // auto-detect workspace root
    root: './',              // explicit root (optional)
    cross_package_hmr: true, // #99
    shared_cache: true,      // #100
  }
})
```

Detection walks up the directory tree looking for:
- `package.json` with `"workspaces"` field (npm/yarn)
- `pnpm-workspace.yaml` (pnpm)
- `lerna.json` (lerna monorepos)

Bare specifiers like `@myorg/ui` resolve to the local workspace package, not the npm registry.

### Cross-Package HMR (#99)

When a file in a workspace package changes, HMR propagates to all consuming packages. The `HmrDependencyMap` scans source files, detects cross-package imports, and computes the full HMR update set.

### Shared Build Cache (#100)

When `shared_cache: true`, the build cache is placed at `{workspace_root}/.pledge/cache/` instead of per-package `node_modules/.pledge-cache/`. This enables cache reuse across packages for faster incremental builds.

