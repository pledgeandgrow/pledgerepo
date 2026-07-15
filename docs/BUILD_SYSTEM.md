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
```

## Asset Pipeline

```
MDX files        → compile_mdx() — Markdown + JSX with frontmatter
GraphQL files    → parse_graphql() + graphql_to_module() with TypeScript types
YAML/CSV/TSV     → transform_yaml() / transform_csv() / transform_tsv() with typed exports
Images           → select_image_format() — WebP/AVIF auto-selection, generate_picture_element()
Audio/Video      → transform_audio_asset() / transform_video_asset() with URL exports
PDF              → transform_pdf_asset() with inline base64 support
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

### Web Workers
- `new Worker(new URL('./worker.ts', import.meta.url))` → `new Worker('/src/worker.js')`
- `new SharedWorker(new URL(...))` patterns also supported
- `.worker.js` / `.worker.ts` extensions detected as `ModuleKind::Worker`

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
- `.vue` → `.js` (SFC compiled, CSS extracted)
- `.svelte` → `.js` (SFC compiled, CSS extracted)
- `.astro` → `.js` (compiled, CSS extracted)
- `.css` → `.css` (Lightning CSS processed)
- `.json` → `.js` (named + default exports)
- `.wasm` → `.js` (async instantiation wrapper)
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
```

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
